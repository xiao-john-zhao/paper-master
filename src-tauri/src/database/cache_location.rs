use super::library_cache;
use crate::error::{AppError, AppResult, ErrorCode};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
struct Location {
    schema_version: u32,
    directory: PathBuf,
}
fn error() -> AppError {
    AppError::new(
        ErrorCode::DatabaseWriteFailed,
        "缓存位置无法切换。请检查文件夹权限、磁盘空间及缓存格式，原位置仍保留。",
    )
}
fn pointer(database_path: &Path) -> PathBuf {
    database_path.with_file_name("cache-location.json")
}
pub fn resolve(database_path: &Path) -> AppResult<PathBuf> {
    let path = pointer(database_path);
    if !path.exists() {
        return Ok(library_cache::directory(database_path));
    }
    let location: Location =
        serde_json::from_slice(&fs::read(path).map_err(|_| error())?).map_err(|_| error())?;
    if location.schema_version != 1 || !location.directory.is_absolute() {
        return Err(error());
    }
    Ok(location.directory)
}
pub fn save(database_path: &Path, directory: &Path) -> AppResult<()> {
    let temporary =
        database_path.with_file_name(format!("cache-location-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| error())?;
        file.write_all(
            &serde_json::to_vec_pretty(&Location {
                schema_version: 1,
                directory: directory.into(),
            })
            .map_err(|_| error())?,
        )
        .map_err(|_| error())?;
        file.sync_all().map_err(|_| error())?;
        drop(file);
        fs::rename(&temporary, pointer(database_path)).map_err(|_| error())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

/// Copy only validated cache entries; destination snapshots are merged before export.
/// Retain the source directory so a failed copy or a moved external drive cannot destroy data.
pub fn copy_entries(source: &Path, destination: &Path) -> AppResult<()> {
    fs::create_dir_all(destination).map_err(|_| error())?;
    if !source.is_dir() {
        return Ok(());
    }
    for item in fs::read_dir(source).map_err(|_| error())? {
        let path = item.map_err(|_| error())?.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let entry = library_cache::read(&path)?;
        let name = path.file_name().ok_or_else(error)?;
        if name != library_cache::file_name(&entry.paper_id).as_str() {
            return Err(error());
        }
        let target = destination.join(name);
        if !target.exists() || entry.paper.is_none() {
            library_cache::write(destination, &entry)?;
        }
        let backup = path.with_extension("json.bak");
        let backup_target = target.with_extension("json.bak");
        if backup.is_file() && !backup_target.exists() {
            fs::copy(backup, backup_target).map_err(|_| error())?;
        }
    }
    Ok(())
}

pub fn copy_legacy_tombstones(
    connection: &rusqlite::Connection,
    source: &Path,
    destination: &Path,
) -> AppResult<()> {
    if !source.is_dir() {
        return Ok(());
    }
    for item in fs::read_dir(source).map_err(|_| error())? {
        let path = item.map_err(|_| error())?.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        if let Ok(entry) = library_cache::read(&path) {
            if entry.paper.is_none() {
                let exists: bool = connection
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM papers WHERE id=?1)",
                        [&entry.paper_id],
                        |r| r.get(0),
                    )
                    .map_err(|_| error())?;
                if !exists {
                    library_cache::write(destination, &entry)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        database::{
            paper_repository::{PaperRepository, SqlitePaperRepository},
            Database,
        },
        diagnostics::AppLogger,
    };
    fn fixture() -> (PathBuf, Database) {
        let root =
            std::env::temp_dir().join(format!("paper-cache-location-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let database = Database::open(
            &root.join("paper-master.db"),
            &AppLogger::new(&root).unwrap(),
        )
        .unwrap();
        {
            let c = database.connection().unwrap();
            c.execute("INSERT INTO papers(id,file_path,file_name,title,created_at,updated_at) VALUES('p','p.pdf','p.pdf','Original','now','now')",[]).unwrap();
        }
        (root, database)
    }
    #[test]
    fn relocates_preserves_source_routes_edits_and_recovers_without_database() {
        let (root, database) = fixture();
        let old = library_cache::directory(database.path());
        let target = root.join("portable cache");
        database.relocate_cache(&target).unwrap();
        assert!(old.is_dir());
        assert!(target.join("70.json").is_file());
        {
            let c = database.connection().unwrap();
            c.execute("UPDATE papers SET title='Latest edit' WHERE id='p'", [])
                .unwrap();
        }
        let json: serde_json::Value =
            serde_json::from_slice(&fs::read(target.join("70.json")).unwrap()).unwrap();
        assert_eq!(json["paper"]["title"], "Latest edit");
        drop(database);
        fs::rename(
            root.join("paper-master.db"),
            root.join("saved-before-cold-restore.db"),
        )
        .unwrap();
        let database = Database::open(
            &root.join("paper-master.db"),
            &AppLogger::new(&root).unwrap(),
        )
        .unwrap();
        assert_eq!(
            fs::canonicalize(database.cache_status().unwrap().directory).unwrap(),
            fs::canonicalize(&target).unwrap()
        );
        assert_eq!(
            SqlitePaperRepository::new(&database.connection().unwrap())
                .find_by_id("p")
                .unwrap()
                .unwrap()
                .title,
            "Latest edit"
        );
        drop(database);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn manually_moved_folder_can_be_selected_and_future_format_does_not_switch_pointer() {
        let (root, database) = fixture();
        let old = library_cache::directory(database.path());
        let moved = root.join("moved");
        fs::rename(&old, &moved).unwrap();
        database.relocate_cache(&moved).unwrap();
        let rejected = root.join("future");
        fs::create_dir_all(&rejected).unwrap();
        fs::write(
            rejected.join("78.json"),
            br#"{"cache_schema_version":99,"paper_id":"x","paper":null}"#,
        )
        .unwrap();
        assert!(database.relocate_cache(&rejected).is_err());
        assert_eq!(
            fs::canonicalize(resolve(database.path()).unwrap()).unwrap(),
            fs::canonicalize(moved).unwrap()
        );
        drop(database);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn older_binary_default_cache_deletions_do_not_resurrect_after_relocation() {
        let (root, database) = fixture();
        let default = library_cache::directory(database.path());
        let target = root.join("custom");
        database.relocate_cache(&target).unwrap();
        drop(database);
        {
            let c = rusqlite::Connection::open(root.join("paper-master.db")).unwrap();
            c.execute("DELETE FROM papers WHERE id='p'", []).unwrap();
            library_cache::flush(&c, &default).unwrap();
        }
        let database = Database::open(
            &root.join("paper-master.db"),
            &AppLogger::new(&root).unwrap(),
        )
        .unwrap();
        assert!(SqlitePaperRepository::new(&database.connection().unwrap())
            .find_by_id("p")
            .unwrap()
            .is_none());
        drop(database);
        fs::remove_dir_all(root).unwrap();
    }
}
