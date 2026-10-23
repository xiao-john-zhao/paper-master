use crate::{
    database::{
        analysis_versions::{self, AnalysisVersion},
        Database,
    },
    domain::paper::PaperAnalysisResult,
    error::AppResult,
};
use tauri::State;

#[tauri::command]
pub fn list_analysis_versions(
    database: State<'_, Database>,
    paper_id: String,
) -> AppResult<Vec<AnalysisVersion>> {
    analysis_versions::list(&*database.connection()?, &paper_id)
}
#[tauri::command]
pub fn select_analysis_version(
    database: State<'_, Database>,
    paper_id: String,
    id: String,
) -> AppResult<AnalysisVersion> {
    analysis_versions::select(&*database.connection()?, &paper_id, &id)
}
#[tauri::command]
pub fn edit_analysis_version(
    database: State<'_, Database>,
    paper_id: String,
    id: String,
    revision: i64,
    result: PaperAnalysisResult,
) -> AppResult<AnalysisVersion> {
    analysis_versions::edit(&*database.connection()?, &paper_id, &id, revision, &result)
}
#[tauri::command]
pub fn extract_paper_locally(
    database: State<'_, Database>,
    paper_id: String,
) -> AppResult<AnalysisVersion> {
    crate::services::archive::extract(&database, &paper_id)
}

#[tauri::command]
pub fn restore_analysis_version(
    database: State<'_, Database>,
    paper_id: String,
    id: String,
    revision: i64,
) -> AppResult<AnalysisVersion> {
    let connection = database.connection()?;
    let json: String = connection
        .query_row(
            "SELECT original_json FROM analysis_versions WHERE id=?1 AND paper_id=?2",
            rusqlite::params![id, paper_id],
            |row| row.get(0),
        )
        .map_err(|_| {
            crate::error::AppError::new(
                crate::error::ErrorCode::DatabaseReadFailed,
                "没有可恢复的原始生成内容。",
            )
        })?;
    let result = serde_json::from_str(&json).map_err(|_| {
        crate::error::AppError::new(
            crate::error::ErrorCode::DatabaseReadFailed,
            "原始生成内容格式无效。",
        )
    })?;
    analysis_versions::edit(&connection, &paper_id, &id, revision, &result)
}
