use serde::Serialize;
use tauri::State;

use crate::{config::AppPaths, diagnostics::AppLogger};

#[derive(Serialize)]
pub struct AppInfo {
    name: &'static str,
    version: &'static str,
    environment: &'static str,
    data_directory: String,
    log_file: String,
}

#[tauri::command]
pub fn get_app_info(paths: State<'_, AppPaths>, logger: State<'_, AppLogger>) -> AppInfo {
    AppInfo {
        name: "paper-master",
        version: env!("CARGO_PKG_VERSION"),
        environment: if cfg!(debug_assertions) {
            "Development"
        } else {
            "Production"
        },
        data_directory: paths.data_dir.display().to_string(),
        log_file: logger.path().display().to_string(),
    }
}
pub mod database;
pub mod import;
pub mod paper;
pub mod settings;
