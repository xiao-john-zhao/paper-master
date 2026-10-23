mod ai;
pub mod archive_cli;
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
        .manage(ai::AIProviderRegistry::new())
        .manage(ai::registry::AIRequestRegistry::new())
        .manage(services::analysis_queue::AnalysisQueue::default())
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
            services::analysis_queue::recover_interrupted(&database)?;
            app.manage(paths);
            app.manage(database);
            app.manage(logger);
            app.state::<ai::registry::AIRequestRegistry>()
                .attach_progress(app.handle().clone());
            services::analysis_queue::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::analysis_versions::list_analysis_versions,
            commands::analysis_versions::select_analysis_version,
            commands::analysis_versions::edit_analysis_version,
            commands::analysis_versions::extract_paper_locally,
            commands::analysis_versions::restore_analysis_version,
            commands::graph::save_graph_image,
            commands::paper::list_papers,
            commands::paper::search_papers,
            commands::paper::get_paper,
            commands::paper::update_paper,
            commands::paper::edit_paper_metadata,
            commands::paper::delete_paper,
            commands::paper::paper_exists_by_path,
            commands::paper::open_paper_pdf,
            commands::paper::relink_paper_pdf,
            commands::paper::get_author_context,
            commands::paper::get_team_links,
            commands::research::search_online_papers,
            commands::research::download_online_paper,
            commands::research::open_ieee_webvpn,
            commands::research::open_ieee_external,
            commands::research::open_relationship_source,
            commands::research::open_webvpn_credentials,
            commands::import::import_papers,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::database::test_database,
            commands::diagnostics::get_diagnostics,
            commands::diagnostics::open_log_folder,
            commands::diagnostics::copy_diagnostic_summary,
            commands::diagnostics::write_ui_log,
            commands::ai::check_ai_connection,
            commands::ai::get_ai_models,
            commands::ai::load_ai_model,
            commands::ai::analyze_paper,
            commands::ai::generate_paper_tags,
            commands::ai::compare_papers,
            commands::ai::edit_comparison_result,
            commands::ai::cancel_ai_request,
            commands::ai::cancel_paper_analysis,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run paper-master");
}
