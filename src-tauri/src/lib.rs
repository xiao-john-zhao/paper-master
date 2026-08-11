mod commands;
mod config;
mod diagnostics;
mod error;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().map_err(|error| error.to_string())?;
            std::fs::create_dir_all(&app_data_dir)?;
            let logger = diagnostics::AppLogger::new(&app_data_dir)?;
            logger.log(diagnostics::LogEvent::info(
                "application",
                "startup",
                "paper-master is starting",
            ));
            app.manage(config::AppPaths::new(app_data_dir));
            app.manage(logger);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::get_app_info])
        .run(tauri::generate_context!())
        .expect("failed to run paper-master");
}

