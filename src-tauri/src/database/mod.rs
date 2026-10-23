pub mod analysis_versions;
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
}

impl Database {
    pub fn open(path: &Path, logger: &AppLogger) -> AppResult<Self> {
        let mut connection = Connection::open(path).map_err(|error| {
            AppError::new(
                ErrorCode::DatabaseReadFailed,
                "The local library database could not be opened.",
            )
            .detail(error.to_string())
        })?;
        let migration_version = migrations::run_migrations(&mut connection)?;
        logger.log(LogEvent::info(
            "database",
            "migrate",
            format!("Database schema is at version {migration_version}"),
        ));
        Ok(Self {
            path: path.to_path_buf(),
            connection: Mutex::new(connection),
            migration_version,
        })
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
        })
    }

    pub fn connection(&self) -> AppResult<MutexGuard<'_, Connection>> {
        self.connection.lock().map_err(|_| {
            AppError::new(
                ErrorCode::DatabaseReadFailed,
                "The local library database is busy.",
            )
            .detail("Database mutex was poisoned")
        })
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
