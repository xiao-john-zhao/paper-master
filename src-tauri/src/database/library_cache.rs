//! Version-neutral local recovery files. No source PDF or provider settings are copied.
use super::{
    analysis_versions,
    paper_repository::{PaperRepository, SqlitePaperRepository},
};
use crate::{
    domain::paper::{Paper, PaperAnalysisResult},
    error::{AppError, AppResult, ErrorCode},
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Default, Serialize)]
pub struct CacheStatus {
    pub directory: String,
    pub schema_version: u32,
    pub pending: usize,
    pub restored: usize,
    pub unreadable: usize,
    pub last_error: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct CachedVersion {
    #[serde(flatten)]
    version: analysis_versions::AnalysisVersion,
    original: Option<PaperAnalysisResult>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Entry {
    cache_schema_version: u32,
    pub(super) paper_id: String,
    // None is a durable deletion marker, so a restart cannot resurrect deleted papers.
    pub(super) paper: Option<Paper>,
    #[serde(default)]
    extracted_text: String,
    #[serde(default)]
    versions: Vec<CachedVersion>,
    #[serde(default)]
    metadata_edited: bool,
}

fn failure() -> AppError {
    AppError::new(ErrorCode::DatabaseWriteFailed, "论文已保存在数据库，但文件缓存同步失败。请在 Diagnostics 的论文缓存中重试，并检查磁盘空间和目录权限。")
}

pub fn directory(database_path: &Path) -> PathBuf {
    database_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("library-cache")
        .join("v1")
}

pub(super) fn file_name(id: &str) -> String {
    // Never interpolate an externally supplied paper ID into a filesystem path.
    format!(
        "{}.json",
        id.as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

pub(super) fn read(path: &Path) -> AppResult<Entry> {
    let data = fs::read(path).map_err(|_| failure())?;
    let entry: Entry = serde_json::from_slice(&data).map_err(|_| failure())?;
    if !(1..=SCHEMA_VERSION).contains(&entry.cache_schema_version)
        || entry.paper_id.is_empty()
        || entry.paper.as_ref().is_some_and(|p| p.id != entry.paper_id)
        || entry.versions.iter().any(|v| {
            v.version.paper_id != entry.paper_id
                || v.version.revision < 0
                || !matches!(v.version.result.schema_version.as_str(), "1" | "2")
        })
        || entry.versions.iter().filter(|v| v.version.active).count() > 1
    {
        return Err(failure());
    }
    Ok(entry)
}

pub(super) fn write(directory: &Path, entry: &Entry) -> AppResult<()> {
    fs::create_dir_all(directory).map_err(|_| failure())?;
    let path = directory.join(file_name(&entry.paper_id));
    let temporary = directory.join(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| failure())?;
        file.write_all(&serde_json::to_vec_pretty(entry).map_err(|_| failure())?)
            .map_err(|_| failure())?;
        file.sync_all().map_err(|_| failure())?;
        drop(file);
        if path.exists() {
            if let Ok(old) = read(&path) {
                if old.paper_id != entry.paper_id {
                    return Err(failure());
                }
                let backup_temp = directory.join(format!("{}.tmp", uuid::Uuid::new_v4()));
                // Keep tombstones in both generations to prevent accidental resurrection.
                fs::copy(
                    if entry.paper.is_none() {
                        &temporary
                    } else {
                        &path
                    },
                    &backup_temp,
                )
                .map_err(|_| failure())?;
                let rotated = fs::rename(&backup_temp, path.with_extension("json.bak"));
                if rotated.is_err() {
                    let _ = fs::remove_file(backup_temp);
                    return Err(failure());
                }
            } else {
                // Preserve unsupported or damaged data, including a still valid backup.
                let raw: serde_json::Value =
                    serde_json::from_slice(&fs::read(&path).map_err(|_| failure())?)
                        .unwrap_or_default();
                if raw
                    .get("cache_schema_version")
                    .and_then(|v| v.as_u64())
                    .is_some_and(|v| v > SCHEMA_VERSION as u64)
                {
                    return Err(failure());
                }
                fs::copy(
                    &path,
                    directory.join(format!(
                        "{}.invalid-{}",
                        file_name(&entry.paper_id),
                        uuid::Uuid::new_v4()
                    )),
                )
                .map_err(|_| failure())?;
            }
        }
        fs::rename(&temporary, &path).map_err(|_| failure())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub fn pending(connection: &Connection) -> AppResult<usize> {
    connection
        .query_row("SELECT COUNT(*) FROM library_cache_pending", [], |r| {
            r.get(0)
        })
        .map_err(|_| failure())
}

/// Hold SQLite's writer reservation until each file and its outbox acknowledgement agree.
/// Other application versions can still use the same SQLite file; their triggers enqueue work.
pub fn flush(connection: &Connection, directory: &Path) -> AppResult<()> {
    connection
        .execute_batch("BEGIN IMMEDIATE")
        .map_err(|_| failure())?;
    let result = (|| {
        let ids = {
            let mut query = connection
                .prepare("SELECT paper_id FROM library_cache_pending ORDER BY paper_id")
                .map_err(|_| failure())?;
            let rows = query
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|_| failure())?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|_| failure())?
        };
        let repo = SqlitePaperRepository::new(connection);
        for id in ids {
            let paper = repo.find_by_id(&id)?;
            let versions = analysis_versions::list(connection, &id)?
                .into_iter()
                .map(|version| {
                    let original: Option<String> = connection
                        .query_row(
                            "SELECT original_json FROM analysis_versions WHERE id=?1",
                            [&version.id],
                            |r| r.get(0),
                        )
                        .map_err(|_| failure())?;
                    Ok(CachedVersion {
                        version,
                        original: original
                            .map(|s| serde_json::from_str(&s))
                            .transpose()
                            .map_err(|_| failure())?,
                    })
                })
                .collect::<AppResult<Vec<_>>>()?;
            let edited: Option<String> = connection
                .query_row(
                    "SELECT value_json FROM settings WHERE key=?1",
                    [format!("metadata-edited:{id}")],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|_| failure())?;
            let entry = Entry {
                cache_schema_version: SCHEMA_VERSION,
                paper_id: id.clone(),
                extracted_text: if paper.is_some() {
                    repo.extracted_text(&id)?
                } else {
                    String::new()
                },
                paper,
                versions,
                metadata_edited: edited.as_deref() == Some("true"),
            };
            write(directory, &entry)?;
            connection
                .execute("DELETE FROM library_cache_pending WHERE paper_id=?1", [id])
                .map_err(|_| failure())?;
        }
        Ok(())
    })();
    match result {
        Ok(()) => {
            if connection.execute_batch("COMMIT").is_err() {
                let _ = connection.execute_batch("ROLLBACK");
                return Err(failure());
            }
            Ok(())
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

/// Restore missing papers/versions only. Current database rows and edits always win.
pub fn recover(connection: &Connection, directory: &Path) -> AppResult<(usize, usize)> {
    fs::create_dir_all(directory).map_err(|_| failure())?;
    let mut restored = 0;
    let mut unreadable = 0;
    let mut paths = std::collections::BTreeSet::new();
    for file in fs::read_dir(directory).map_err(|_| failure())? {
        let path = file.map_err(|_| failure())?.path();
        if path.extension().is_some_and(|e| e == "json") {
            paths.insert(path);
        } else if path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().ends_with(".json.bak"))
        {
            paths.insert(path.with_extension(""));
        }
    }
    for path in paths {
        let entry = match read(&path) {
            Ok(entry) => entry,
            Err(_) => {
                unreadable += 1;
                match read(&path.with_extension("json.bak")) {
                    Ok(entry) => entry,
                    Err(_) => continue,
                }
            }
        };
        if path
            .file_name()
            .is_none_or(|name| name != file_name(&entry.paper_id).as_str())
        {
            unreadable += 1;
            continue;
        }
        if entry.paper.is_none() {
            continue;
        }
        connection
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|_| failure())?;
        let result = restore_entry(connection, &entry);
        match result {
            Ok(count) => {
                connection.execute_batch("COMMIT").map_err(|_| failure())?;
                restored += count;
            }
            Err(_) => {
                let _ = connection.execute_batch("ROLLBACK");
                unreadable += 1;
            }
        }
    }
    Ok((restored, unreadable))
}

fn restore_entry(connection: &Connection, entry: &Entry) -> AppResult<usize> {
    let repo = SqlitePaperRepository::new(connection);
    // A pending deletion/update is newer than the cache, even when the write failed.
    let dirty: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_cache_pending WHERE paper_id=?1)",
            [&entry.paper_id],
            |r| r.get(0),
        )
        .map_err(|_| failure())?;
    if dirty {
        return Ok(0);
    }
    let Some(paper) = &entry.paper else {
        return Ok(0);
    };
    let existing = repo.find_by_id(&entry.paper_id)?;
    let missing_analysis = existing.as_ref().is_some_and(|p| p.analysis.is_none());
    let is_new = existing.is_none();
    if is_new {
        // Path uniqueness prevents attaching another paper's analysis to an existing record.
        repo.create(paper, &entry.extracted_text)?;
        if entry.metadata_edited {
            connection.execute("INSERT OR IGNORE INTO settings(key,value_json,updated_at) VALUES(?1,'true',?2)", params![format!("metadata-edited:{}", paper.id), paper.updated_at]).map_err(|_| failure())?;
        }
    }
    let mut count = usize::from(is_new);
    for cached in &entry.versions {
        let v = &cached.version;
        if let Some(profile) = &v.profile {
            connection
                .execute(
                    "INSERT OR IGNORE INTO settings(key,value_json,updated_at) VALUES(?1,?2,?3)",
                    params![
                        format!("analysis-profile:{}", v.id),
                        serde_json::to_string(profile).map_err(|_| failure())?,
                        v.updated_at
                    ],
                )
                .map_err(|_| failure())?;
        }

        let owner: Option<String> = connection
            .query_row(
                "SELECT paper_id FROM analysis_versions WHERE id=?1",
                [&v.id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| failure())?;
        if owner.is_some_and(|owner| owner != paper.id) {
            return Err(failure());
        }
        count += connection.execute("INSERT OR IGNORE INTO analysis_versions(id,paper_id,provider,model,created_at,updated_at,revision,edited,result_json,original_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)", params![v.id,v.paper_id,v.provider,v.model,v.created_at,v.updated_at,v.revision,v.edited,serde_json::to_string(&v.result).map_err(|_| failure())?,cached.original.as_ref().map(serde_json::to_string).transpose().map_err(|_| failure())?]).map_err(|_| failure())?;
    }
    if is_new {
        if let Some(active) = entry.versions.iter().find(|v| v.version.active) {
            // Retain original timestamps and the cached projection; no fresh generation/edit.
            connection
                .execute(
                    "INSERT INTO active_analyses(paper_id,version_id) VALUES(?1,?2)",
                    params![paper.id, active.version.id],
                )
                .map_err(|_| failure())?;
        }
    }
    if !is_new && missing_analysis {
        let has_active: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM active_analyses WHERE paper_id=?1)",
                [&paper.id],
                |r| r.get(0),
            )
            .map_err(|_| failure())?;
        if !has_active {
            if let Some(active) = entry.versions.iter().find(|v| v.version.active) {
                analysis_versions::select(connection, &paper.id, &active.version.id)?;
            }
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{database::Database, diagnostics::AppLogger};

    fn temp() -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("paper-master-cache-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }
    fn open(root: &Path) -> Database {
        Database::open(
            &root.join("paper-master.db"),
            &AppLogger::new(root).unwrap(),
        )
        .unwrap()
    }
    fn memory() -> Connection {
        let mut c = Connection::open_in_memory().unwrap();
        super::super::migrations::run_migrations(&mut c).unwrap();
        c
    }
    fn result(summary: &str) -> PaperAnalysisResult {
        PaperAnalysisResult {
            schema_version: "2".into(),
            research_problem: "研究问题".into(),
            research_scenario: "场景".into(),
            core_method: "方法".into(),
            key_innovations: vec!["创新".into()],
            key_contributions: vec!["贡献".into()],
            main_results: "结果".into(),
            limitations: "局限".into(),
            tags: vec!["ISAC".into()],
            ai_summary: summary.into(),
        }
    }
    fn insert(c: &Connection, id: &str) {
        c.execute("INSERT INTO papers(id,file_path,file_name,title,extracted_text,created_at,updated_at) VALUES(?1,?2,'原文.pdf','论文题名','Original evidence preserved','2026','2026')", params![id,format!("missing-original-{id}.pdf")]).unwrap();
    }

    #[test]
    fn cold_database_recovers_all_models_edits_original_selection_and_search() {
        let root = temp();
        let database = open(&root);
        let a;
        {
            let c = database.connection().unwrap();
            insert(&c, "p");
            a = analysis_versions::append(
                &c,
                "p",
                "lmstudio",
                "local-model",
                &result("原始生成总结"),
            )
            .unwrap();
            analysis_versions::append(&c, "p", "codex", "online-model", &result("另一模型总结"))
                .unwrap();
            let profile = crate::ai::adaptive::AnalysisProfile {
                reading: Some(crate::ai::staged::ReadingReport {
                    strategy: "segmented".into(),
                    source_characters: 12000,
                    covered_characters: 12000,
                    ..Default::default()
                }),
                attempts: 3,
                input_budget: 14000,
                input_characters: 12000,
                output_tokens: 4096,
                reasoning_effort: "none".into(),
                requested_context_length: 24576,
            };
            c.execute(
                "INSERT INTO settings VALUES(?1,?2,'2026')",
                params![
                    format!("analysis-profile:{}", a.id),
                    serde_json::to_string(&profile).unwrap()
                ],
            )
            .unwrap();
            analysis_versions::edit(&c, "p", &a.id, 0, &result("人工修改总结")).unwrap();
            analysis_versions::select(&c, "p", &a.id).unwrap();
            c.execute(
                "INSERT INTO settings VALUES('metadata-edited:p','true','2026')",
                [],
            )
            .unwrap();
        }
        assert_eq!(database.cache_status().unwrap().pending, 0);
        let cache = directory(database.path());
        let bytes = fs::read(cache.join(file_name("p"))).unwrap();
        drop(database);
        // Read-only reopening is idempotent and does not rotate healthy cache files.
        let database = open(&root);
        assert_eq!(fs::read(cache.join(file_name("p"))).unwrap(), bytes);
        drop(database);
        let c = memory();
        assert_eq!(recover(&c, &cache).unwrap(), (3, 0));
        let versions = analysis_versions::list(&c, "p").unwrap();
        assert_eq!(versions.len(), 2);
        let selected = versions.iter().find(|v| v.active).unwrap();
        assert_eq!(selected.id, a.id);
        assert!(selected.edited);
        assert_eq!(selected.revision, 1);
        assert_eq!(selected.profile.as_ref().unwrap().attempts, 3);
        assert_eq!(
            selected
                .profile
                .as_ref()
                .unwrap()
                .reading
                .as_ref()
                .unwrap()
                .covered_characters,
            12000
        );
        assert_eq!(selected.result.ai_summary, "人工修改总结");
        let original: String = c
            .query_row(
                "SELECT original_json FROM analysis_versions WHERE id=?1",
                [&a.id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(original.contains("原始生成总结"));
        let repo = SqlitePaperRepository::new(&c);
        assert_eq!(
            repo.extracted_text("p").unwrap(),
            "Original evidence preserved"
        );
        assert_eq!(
            repo.search(&crate::domain::paper::PaperQuery {
                text: Some("人工修改总结".into()),
                ..Default::default()
            })
            .unwrap()
            .len(),
            1
        );
        let edited: String = c
            .query_row(
                "SELECT value_json FROM settings WHERE key='metadata-edited:p'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(edited, "true");
        assert!(c
            .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
            .optional()
            .unwrap()
            .is_none());
        flush(&c, &cache).unwrap();
        assert_eq!(recover(&c, &cache).unwrap(), (0, 0));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn existing_database_edits_win_and_missing_versions_are_merged() {
        let root = temp();
        let cache = root.join("cache");
        let c = memory();
        insert(&c, "p");
        let v = analysis_versions::append(&c, "p", "mock", "m", &result("缓存旧总结")).unwrap();
        flush(&c, &cache).unwrap();
        // Simulate an older database snapshot with metadata edited after the snapshot.
        let current = memory();
        insert(&current, "p");
        current
            .execute("UPDATE papers SET title='较新的人工题名' WHERE id='p'", [])
            .unwrap();
        current
            .execute("DELETE FROM library_cache_pending", [])
            .unwrap();
        assert_eq!(recover(&current, &cache).unwrap(), (1, 0));
        assert_eq!(
            SqlitePaperRepository::new(&current)
                .find_by_id("p")
                .unwrap()
                .unwrap()
                .title,
            "较新的人工题名"
        );
        assert_eq!(analysis_versions::list(&current, "p").unwrap()[0].id, v.id);
        analysis_versions::edit(&current, "p", &v.id, 0, &result("较新的人工总结")).unwrap();
        current
            .execute("DELETE FROM library_cache_pending", [])
            .unwrap();
        recover(&current, &cache).unwrap();
        assert_eq!(
            analysis_versions::list(&current, "p").unwrap()[0]
                .result
                .ai_summary,
            "较新的人工总结"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn disk_failure_keeps_database_and_retry_outbox_then_syncs_latest_edits() {
        let root = temp();
        let database = open(&root);
        let cache = directory(database.path());
        fs::remove_dir(&cache).unwrap();
        fs::write(&cache, b"blocked directory").unwrap();
        {
            let c = database.connection().unwrap();
            insert(&c, "p");
            analysis_versions::append(&c, "p", "mock", "m", &result("必须保留的总结")).unwrap();
        }
        let status = database.cache_status().unwrap();
        assert_eq!(status.pending, 1);
        assert!(status.last_error.is_some());
        assert!(SqlitePaperRepository::new(&database.connection().unwrap())
            .find_by_id("p")
            .unwrap()
            .is_some());
        fs::remove_file(&cache).unwrap();
        let status = database.sync_cache().unwrap();
        assert_eq!(status.pending, 0);
        assert!(status.last_error.is_none());
        assert!(read(&cache.join(file_name("p")))
            .unwrap()
            .paper
            .unwrap()
            .analysis
            .unwrap()
            .ai_summary
            .contains("必须保留"));
        drop(database);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deletions_survive_restarts_and_corrupt_current_file() {
        let root = temp();
        let database = open(&root);
        let cache = directory(database.path());
        {
            let c = database.connection().unwrap();
            insert(&c, "p");
        }
        {
            let c = database.connection().unwrap();
            SqlitePaperRepository::new(&c).delete("p").unwrap();
        }
        assert!(read(&cache.join(file_name("p"))).unwrap().paper.is_none());
        assert!(read(&cache.join(file_name("p")).with_extension("json.bak"))
            .unwrap()
            .paper
            .is_none());
        fs::write(cache.join(file_name("p")), b"broken").unwrap();
        drop(database);
        let database = open(&root);
        assert_eq!(
            SqlitePaperRepository::new(&database.connection().unwrap())
                .counts()
                .unwrap()
                .0,
            0
        );
        assert_eq!(database.cache_status().unwrap().unreadable, 1);
        drop(database);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn one_corrupt_or_future_cache_does_not_hide_other_papers() {
        let root = temp();
        let cache = root.join("cache");
        let c = memory();
        for id in ["good", "corrupt", "future"] {
            insert(&c, id);
        }
        flush(&c, &cache).unwrap();
        c.execute("UPDATE papers SET title='Updated' WHERE id='corrupt'", [])
            .unwrap();
        flush(&c, &cache).unwrap();
        fs::write(cache.join(file_name("corrupt")), b"truncated").unwrap();
        let future_path = cache.join(file_name("future"));
        let mut future: serde_json::Value =
            serde_json::from_slice(&fs::read(&future_path).unwrap()).unwrap();
        future["cache_schema_version"] = 99.into();
        let future_bytes = serde_json::to_vec(&future).unwrap();
        fs::write(&future_path, &future_bytes).unwrap();
        let target = memory();
        assert_eq!(recover(&target, &cache).unwrap(), (2, 2));
        assert_eq!(SqlitePaperRepository::new(&target).counts().unwrap().0, 2);
        assert_eq!(fs::read(&future_path).unwrap(), future_bytes);
        insert(&target, "future");
        assert!(flush(&target, &cache).is_err());
        assert!(pending(&target).unwrap() > 0);
        assert_eq!(fs::read(&future_path).unwrap(), future_bytes);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_sqlite_writes_are_captured_on_next_launch_and_rollback_is_ignored() {
        let root = temp();
        let database = open(&root);
        let cache = directory(database.path());
        {
            let c = database.connection().unwrap();
            insert(&c, "p");
        }
        drop(database);
        let legacy = Connection::open(root.join("paper-master.db")).unwrap();
        legacy
            .execute(
                "UPDATE papers SET title='旧版应用修改后保留' WHERE id='p'",
                [],
            )
            .unwrap();
        legacy
            .execute_batch("BEGIN; UPDATE papers SET title='回滚文字' WHERE id='p'; ROLLBACK")
            .unwrap();
        drop(legacy);
        let database = open(&root);
        assert_eq!(
            read(&cache.join(file_name("p")))
                .unwrap()
                .paper
                .unwrap()
                .title,
            "旧版应用修改后保留"
        );
        assert_eq!(database.cache_status().unwrap().pending, 0);
        drop(database);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_current_cache_uses_backup_and_does_not_rewrite_existing_metadata() {
        let root = temp();
        let cache = root.join("cache");
        let c = memory();
        insert(&c, "p");
        analysis_versions::append(&c, "p", "mock", "m", &result("可恢复的总结")).unwrap();
        flush(&c, &cache).unwrap();
        c.execute("UPDATE papers SET title='第二次保存' WHERE id='p'", [])
            .unwrap();
        flush(&c, &cache).unwrap();
        fs::remove_file(cache.join(file_name("p"))).unwrap();
        let current = memory();
        insert(&current, "p");
        current
            .execute("UPDATE papers SET title='保留此人工题名' WHERE id='p'", [])
            .unwrap();
        current
            .execute("DELETE FROM library_cache_pending", [])
            .unwrap();
        assert_eq!(recover(&current, &cache).unwrap(), (1, 1));
        let paper = SqlitePaperRepository::new(&current)
            .find_by_id("p")
            .unwrap()
            .unwrap();
        assert_eq!(paper.title, "保留此人工题名");
        assert_eq!(paper.analysis.unwrap().ai_summary, "可恢复的总结");
        assert_eq!(paper.analysis_status, "analyzed");
        fs::remove_dir_all(root).unwrap();
    }
}
