use tauri::State;

use crate::{
    database::{
        settings_storage::{SettingsStorage, SqliteSettingsStorage},
        Database,
    },
    diagnostics::{AppLogger, LogEvent},
    domain::settings::AppSettings,
    error::AppResult,
};

#[tauri::command]
pub fn get_settings(database: State<'_, Database>) -> AppResult<AppSettings> {
    let connection = database.connection()?;
    SqliteSettingsStorage::new(&connection).get()
}

#[tauri::command]
pub fn save_settings(
    database: State<'_, Database>,
    logger: State<'_, AppLogger>,
    settings: AppSettings,
) -> AppResult<AppSettings> {
    let connection = database.connection()?;
    let saved = SqliteSettingsStorage::new(&connection).save(&settings)?;
    logger.log(LogEvent::info(
        "settings",
        "save",
        "Local application settings were updated",
    ));
    Ok(saved)
}
