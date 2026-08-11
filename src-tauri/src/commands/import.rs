use tauri::State;

use crate::{
    database::Database, diagnostics::AppLogger, domain::paper::ImportSummary,
    services::paper_import::PaperImportService,
};

#[tauri::command]
pub async fn import_papers(
    database: State<'_, Database>,
    logger: State<'_, AppLogger>,
    paths: Vec<String>,
    operation_id: String,
) -> Result<ImportSummary, String> {
    Ok(PaperImportService::new(database.inner(), logger.inner()).import_paths(paths, operation_id))
}
