use serde::Serialize;
use std::{
    io::Write,
    process::{Command, Stdio},
};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::{
    ai::AIProviderRegistry,
    database::{
        paper_repository::SqlitePaperRepository,
        settings_storage::{SettingsStorage, SqliteSettingsStorage},
        Database,
    },
    diagnostics::{AppLogger, LogEvent, LogLevel},
    error::{AppError, AppResult, ErrorCode},
};

#[derive(Serialize)]
pub struct DiagnosticSnapshot {
    application_version: &'static str,
    environment: &'static str,
    database_path: String,
    database_status: String,
    migration_version: i64,
    lm_studio_base_url: String,
    lm_studio_status: String,
    current_model: String,
    paper_count: usize,
    analyzed_paper_count: usize,
    last_ai_request_status: String,
    last_import_status: String,
    recent_errors: Vec<LogEvent>,
    log_file_location: String,
}

#[tauri::command]
pub async fn get_diagnostics(
    database: State<'_, Database>,
    logger: State<'_, AppLogger>,
    providers: State<'_, AIProviderRegistry>,
) -> AppResult<DiagnosticSnapshot> {
    let (settings, counts, last_ai, last_import) = {
        let connection = database.connection()?;
        let settings = SqliteSettingsStorage::new(&connection).get()?;
        let counts = SqlitePaperRepository::new(&connection).counts()?;
        let last_ai = latest_operation(&connection, &["analysis", "compare"])?;
        let last_import = latest_operation(&connection, &["import"])?;
        (settings, counts, last_ai, last_import)
    };
    let connection = providers
        .get(&settings.ai_provider)?
        .check_connection(&settings)
        .await;
    Ok(DiagnosticSnapshot {
        application_version: env!("CARGO_PKG_VERSION"),
        environment: if cfg!(debug_assertions) {
            "Development"
        } else {
            "Production"
        },
        database_path: database.path().display().to_string(),
        database_status: database.integrity_check()?,
        migration_version: database.migration_version(),
        lm_studio_base_url: settings.base_url,
        lm_studio_status: connection.state,
        current_model: if settings.model.is_empty() {
            "Auto (first loaded model)".into()
        } else {
            settings.model
        },
        paper_count: counts.0,
        analyzed_paper_count: counts.1,
        last_ai_request_status: last_ai.unwrap_or_else(|| "No AI request recorded".into()),
        last_import_status: last_import.unwrap_or_else(|| "No import recorded".into()),
        recent_errors: logger.recent_errors(),
        log_file_location: logger.path().display().to_string(),
    })
}

#[tauri::command]
pub fn open_log_folder(app: AppHandle, logger: State<'_, AppLogger>) -> AppResult<()> {
    let folder = logger
        .path()
        .parent()
        .ok_or_else(|| AppError::new(ErrorCode::FileNotFound, "The log folder is unavailable."))?;
    app.opener()
        .open_path(folder.to_string_lossy().to_string(), None::<&str>)
        .map_err(|error| {
            AppError::new(
                ErrorCode::FileNotFound,
                "The log folder could not be opened.",
            )
            .detail(error.to_string())
        })
}

#[tauri::command]
pub fn copy_diagnostic_summary(
    database: State<'_, Database>,
    logger: State<'_, AppLogger>,
) -> AppResult<String> {
    let (paper_count, analyzed_count) = {
        let connection = database.connection()?;
        SqlitePaperRepository::new(&connection).counts()?
    };
    let summary = format!(
        "paper-master {}\nEnvironment: {}\nDatabase status: {}\nMigration version: {}\nPaper count: {}\nAnalyzed count: {}\nLog file: {}",
        env!("CARGO_PKG_VERSION"),
        if cfg!(debug_assertions) { "Development" } else { "Production" },
        database.integrity_check()?, database.migration_version(), paper_count, analyzed_count, logger.path().display(),
    );
    let mut child = Command::new("clip.exe")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| {
            AppError::new(
                ErrorCode::Internal,
                "The diagnostic summary could not be copied.",
            )
            .detail(error.to_string())
        })?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| AppError::new(ErrorCode::Internal, "The Windows clipboard is unavailable."))?
        .write_all(summary.as_bytes())
        .map_err(|error| {
            AppError::new(
                ErrorCode::Internal,
                "The diagnostic summary could not be copied.",
            )
            .detail(error.to_string())
        })?;
    let status = child.wait().map_err(|error| {
        AppError::new(
            ErrorCode::Internal,
            "The clipboard operation did not finish.",
        )
        .detail(error.to_string())
    })?;
    if !status.success() {
        return Err(AppError::new(
            ErrorCode::Internal,
            "The diagnostic summary could not be copied.",
        ));
    }
    Ok(summary)
}

#[derive(serde::Deserialize)]
pub struct UiLogInput {
    operation: String,
    message: String,
    request_id: Option<String>,
}

#[tauri::command]
pub fn write_ui_log(logger: State<'_, AppLogger>, event: UiLogInput) {
    let mut log = LogEvent::new(LogLevel::Info, "ui", event.operation, event.message);
    log.request_id = event.request_id;
    logger.log(log);
}

fn latest_operation(
    connection: &rusqlite::Connection,
    kinds: &[&str],
) -> AppResult<Option<String>> {
    let placeholders = kinds.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("SELECT operation_id,status,summary FROM operation_status WHERE kind IN ({placeholders}) ORDER BY updated_at DESC LIMIT 1");
    let mut statement = connection.prepare(&sql).map_err(|error| {
        AppError::new(
            ErrorCode::DatabaseReadFailed,
            "Operation diagnostics could not be read.",
        )
        .detail(error.to_string())
    })?;
    let mut rows = statement
        .query(rusqlite::params_from_iter(kinds.iter().copied()))
        .map_err(|error| {
            AppError::new(
                ErrorCode::DatabaseReadFailed,
                "Operation diagnostics could not be read.",
            )
            .detail(error.to_string())
        })?;
    match rows.next().map_err(|error| {
        AppError::new(
            ErrorCode::DatabaseReadFailed,
            "Operation diagnostics could not be read.",
        )
        .detail(error.to_string())
    })? {
        Some(row) => Ok(Some(format!(
            "{} · {} · {}",
            row.get::<_, String>(1).unwrap_or_default(),
            row.get::<_, String>(2).unwrap_or_default(),
            row.get::<_, String>(0).unwrap_or_default()
        ))),
        None => Ok(None),
    }
}
