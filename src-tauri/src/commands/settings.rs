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
    requests: State<'_, crate::ai::registry::AIRequestRegistry>,
    app: tauri::AppHandle,
    settings: AppSettings,
) -> AppResult<AppSettings> {
    let connection = database.connection()?;
    let previous = SqliteSettingsStorage::new(&connection).get()?;
    let saved = SqliteSettingsStorage::new(&connection).save(&settings)?;
    if previous.privacy_mode == "codex_online" && saved.privacy_mode == "local_only" {
        requests.cancel_all();
        use tauri::Manager;
        if let Some(window) = app.get_webview_window("ieee-webvpn") {
            let _ = window.close();
        }
    }
    logger.log(LogEvent::info(
        "settings",
        "save",
        "Local application settings were updated",
    ));
    Ok(saved)
}
