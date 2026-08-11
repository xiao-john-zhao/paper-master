use serde::Serialize;
use tauri::State;

use crate::{database::Database, error::AppResult};

#[derive(Serialize)]
pub struct DatabaseCheck {
    pub status: String,
    pub migration_version: i64,
    pub path: String,
}

#[tauri::command]
pub fn test_database(database: State<'_, Database>) -> AppResult<DatabaseCheck> {
    Ok(DatabaseCheck {
        status: database.integrity_check()?,
        migration_version: database.migration_version(),
        path: database.path().display().to_string(),
    })
}
