pub mod analysis_versions;
pub mod library_cache;
pub mod migrations;
pub mod paper_repository;
pub mod settings_storage;

use rusqlite::Connection;
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

use crate::{
    diagnostics::{AppLogger, LogEvent},
    error::{AppError, AppResult, ErrorCode},
};

pub struct Database {
    path: PathBuf,
    connection: Mutex<Connection>,
    migration_version: i64,
    cache_directory: Option<PathBuf>,
    cache_status: Mutex<library_cache::CacheStatus>,
}

/// Every write path (including imports, background analysis and future commands) shares
/// this boundary. The durable SQLite outbox survives a crash or an unavailable disk.
pub struct DatabaseConnection<'a> {
    connection: MutexGuard<'a, Connection>,
    database: &'a Database,
}
impl std::ops::Deref for DatabaseConnection<'_> {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        &self.connection
    }
}
impl Drop for DatabaseConnection<'_> {
    fn drop(&mut self) {
        if self.connection.is_autocommit() {
            if let Some(directory) = &self.database.cache_directory {
                if library_cache::pending(&self.connection).unwrap_or(0) > 0 {
                    let result = library_cache::flush(&self.connection, directory);
                    self.database.record_cache_result(&self.connection, result);
                }
            }
        }
    }
}

impl Database {
    pub fn open(path: &Path, logger: &AppLogger) -> AppResult<Self> {
        Self::open_internal(path, logger, false)
    }

    pub fn open_with_backup(path: &Path, logger: &AppLogger) -> AppResult<Self> {
        Self::open_internal(path, logger, true)
    }

    fn open_internal(path: &Path, logger: &AppLogger, backup: bool) -> AppResult<Self> {
        let existed = path.is_file();
        let flags = if existed {
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
        } else {
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
        };
        // An existing file disappearing between inspection and open must raise an error.
        let mut connection = Connection::open_with_flags(path, flags).map_err(|error| {
            AppError::new(
                ErrorCode::DatabaseReadFailed,
                "The local library database could not be opened.",
            )
            .detail(error.to_string())
        })?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| {
                AppError::new(ErrorCode::DatabaseReadFailed, "本地数据库忙，请稍后重试。")
            })?;
        let has_papers: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='papers')",
                [],
                |r| r.get(0),
            )
            .map_err(|_| AppError::new(ErrorCode::DatabaseReadFailed, "数据库结构无法读取。"))?;
        let before: Option<i64> = if has_papers {
            Some(
                connection
                    .query_row("SELECT COUNT(*) FROM papers", [], |r| r.get(0))
                    .map_err(|_| {
                        AppError::new(ErrorCode::DatabaseReadFailed, "已有论文记录无法读取。")
                    })?,
            )
        } else {
            None
        };
        if existed && backup {
            let backup_path = path.with_file_name(format!(
                "paper-master-before-v{}-{}.db",
                env!("CARGO_PKG_VERSION"),
                uuid::Uuid::new_v4()
            ));
            connection
                .execute("VACUUM INTO ?1", [backup_path.to_string_lossy().as_ref()])
                .map_err(|_| {
                    AppError::new(
                        ErrorCode::DatabaseWriteFailed,
                        "数据库备份失败，未开始缓存迁移。",
                    )
                })?;
        }
        // Keep this same SQLite handle from backup through migrations and cache export;
        // do not close the final WAL reader and then reopen a different file implicitly.
        let migration_version = migrations::run_migrations(&mut connection)?;
        if let Some(before) = before {
            let after: i64 = connection
                .query_row("SELECT COUNT(*) FROM papers", [], |r| r.get(0))
                .map_err(|_| {
                    AppError::new(ErrorCode::DatabaseReadFailed, "迁移后的论文数量无法校验。")
                })?;
            if before != after {
                return Err(AppError::new(
                    ErrorCode::MigrationFailed,
                    "迁移前后论文数量不一致，已停止初始化。原数据库备份保留在应用数据目录。",
                ));
            }
        }
        logger.log(LogEvent::info(
            "database",
            "migrate",
            format!("Database schema is at version {migration_version}"),
        ));
        let directory = library_cache::directory(path);
        let database = Self {
            path: path.to_path_buf(),
            connection: Mutex::new(connection),
            migration_version,
            cache_directory: Some(directory.clone()),
            cache_status: Mutex::new(library_cache::CacheStatus {
                directory: directory.display().to_string(),
                schema_version: library_cache::SCHEMA_VERSION,
                ..Default::default()
            }),
        };
        // Cache failure must not hide an intact SQLite library. It remains visible in the UI.
        if database.recover_cache().is_err() {
            logger.log(LogEvent::info(
                "library.cache",
                "recovery_pending",
                "Local cache needs attention; SQLite library remains available",
            ));
        }
        Ok(database)
    }

    #[cfg(test)]
    pub fn in_memory() -> AppResult<Self> {
        let mut connection = Connection::open_in_memory().map_err(|error| {
            AppError::new(ErrorCode::DatabaseReadFailed, "Test database failed")
                .detail(error.to_string())
        })?;
        let migration_version = migrations::run_migrations(&mut connection)?;
        Ok(Self {
            path: PathBuf::from(":memory:"),
            connection: Mutex::new(connection),
            migration_version,
            cache_directory: None,
            cache_status: Mutex::new(Default::default()),
        })
    }

    pub fn connection(&self) -> AppResult<DatabaseConnection<'_>> {
        self.connection
            .lock()
            .map(|connection| DatabaseConnection {
                connection,
                database: self,
            })
            .map_err(|_| {
                AppError::new(
                    ErrorCode::DatabaseReadFailed,
                    "The local library database is busy.",
                )
                .detail("Database mutex was poisoned")
            })
    }

    fn record_cache_result(&self, connection: &Connection, result: AppResult<()>) {
        if let Ok(mut status) = self.cache_status.lock() {
            status.pending = library_cache::pending(connection).unwrap_or(0);
            status.last_error = result.err().map(|error| error.message);
        }
    }

    pub fn cache_status(&self) -> AppResult<library_cache::CacheStatus> {
        let connection = self.connection()?;
        let mut status = self
            .cache_status
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal, "缓存状态不可用。"))?
            .clone();
        status.pending = library_cache::pending(&connection)?;
        Ok(status)
    }

    pub fn sync_cache(&self) -> AppResult<library_cache::CacheStatus> {
        let directory = self
            .cache_directory
            .as_ref()
            .ok_or_else(|| AppError::new(ErrorCode::FileNotFound, "当前运行环境没有文件缓存。"))?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::new(ErrorCode::DatabaseReadFailed, "数据库忙。"))?;
        connection
            .execute(
                "INSERT OR IGNORE INTO library_cache_pending SELECT id FROM papers",
                [],
            )
            .map_err(|_| AppError::new(ErrorCode::DatabaseWriteFailed, "缓存同步未开始。"))?;
        let result = library_cache::flush(&connection, directory);
        self.record_cache_result(&connection, result.clone());
        result?;
        drop(connection);
        self.cache_status()
    }

    pub fn recover_cache(&self) -> AppResult<library_cache::CacheStatus> {
        let directory = self
            .cache_directory
            .as_ref()
            .ok_or_else(|| AppError::new(ErrorCode::FileNotFound, "当前运行环境没有文件缓存。"))?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| AppError::new(ErrorCode::DatabaseReadFailed, "数据库忙。"))?;
        // Flush pending writes/deletions before reading older snapshots.
        let result = (|| {
            library_cache::flush(&connection, directory)?;
            let (restored, unreadable) = library_cache::recover(&connection, directory)?;
            if let Ok(mut status) = self.cache_status.lock() {
                status.restored = restored;
                status.unreadable = unreadable;
            }
            library_cache::seed_missing(&connection, directory)?;
            library_cache::flush(&connection, directory)
        })();
        self.record_cache_result(&connection, result.clone());
        result?;
        drop(connection);
        self.cache_status()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn migration_version(&self) -> i64 {
        self.migration_version
    }

    pub fn integrity_check(&self) -> AppResult<String> {
        self.connection()?
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|error| {
                AppError::new(
                    ErrorCode::DatabaseReadFailed,
                    "The database integrity check failed.",
                )
                .detail(error.to_string())
            })
    }
}
