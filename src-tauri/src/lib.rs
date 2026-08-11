mod commands;
mod config;
mod database;
mod diagnostics;
mod domain;
mod error;
mod pdf;
mod services;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| error.to_string())?;
            std::fs::create_dir_all(&app_data_dir)?;
            let logger = diagnostics::AppLogger::new(&app_data_dir)?;
            logger.log(diagnostics::LogEvent::info(
                "application",
                "startup",
                "paper-master is starting",
            ));
            let paths = config::AppPaths::new(app_data_dir);
            let database = database::Database::open(&paths.database_path, &logger)
                .map_err(|error| format!("{}: {}", error.code_string(), error))?;
            app.manage(paths);
            app.manage(database);
            app.manage(logger);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::paper::list_papers,
            commands::paper::search_papers,
            commands::paper::get_paper,
            commands::paper::update_paper,
            commands::paper::delete_paper,
            commands::paper::paper_exists_by_path,
            commands::import::import_papers,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::database::test_database,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run paper-master");
}
