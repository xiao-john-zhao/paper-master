use std::time::Instant;
use tauri::State;

use crate::{
    database::{
        paper_repository::{PaperRepository, SqlitePaperRepository},
        Database,
    },
    diagnostics::{AppLogger, LogEvent, LogLevel},
    domain::paper::{Paper, PaperQuery},
    error::AppResult,
};

#[tauri::command]
pub fn list_papers(database: State<'_, Database>, query: PaperQuery) -> AppResult<Vec<Paper>> {
    let started = Instant::now();
    let connection = database.connection()?;
    let result = SqlitePaperRepository::new(&connection).list(&query);
    drop(connection);
    result.map(|papers| {
        let _ = started;
        papers
    })
}

#[tauri::command]
pub fn search_papers(
    database: State<'_, Database>,
    logger: State<'_, AppLogger>,
    query: PaperQuery,
) -> AppResult<Vec<Paper>> {
    let started = Instant::now();
    let connection = database.connection()?;
    let result = SqlitePaperRepository::new(&connection).search(&query);
    if let Ok(papers) = &result {
        let mut event = LogEvent::new(
            LogLevel::Info,
            "search.keyword",
            "search",
            format!("Keyword search returned {} papers", papers.len()),
        );
        event.duration_ms = Some(started.elapsed().as_millis());
        logger.log(event);
    }
    result
}

#[tauri::command]
pub fn get_paper(database: State<'_, Database>, id: String) -> AppResult<Option<Paper>> {
    let connection = database.connection()?;
    SqlitePaperRepository::new(&connection).find_by_id(&id)
}

#[tauri::command]
pub fn update_paper(
    database: State<'_, Database>,
    logger: State<'_, AppLogger>,
    paper: Paper,
) -> AppResult<Paper> {
    let connection = database.connection()?;
    let updated = SqlitePaperRepository::new(&connection).update(&paper)?;
    logger.log(LogEvent::info(
        "paper.repository",
        "update",
        format!("Updated paper {}", paper.id),
    ));
    Ok(updated)
}

#[tauri::command]
pub fn delete_paper(
    database: State<'_, Database>,
    logger: State<'_, AppLogger>,
    id: String,
) -> AppResult<()> {
    let connection = database.connection()?;
    SqlitePaperRepository::new(&connection).delete(&id)?;
    logger.log(LogEvent::info(
        "paper.repository",
        "delete",
        format!("Removed paper {id} from the library"),
    ));
    Ok(())
}

#[tauri::command]
pub fn paper_exists_by_path(database: State<'_, Database>, path: String) -> AppResult<bool> {
    let connection = database.connection()?;
    SqlitePaperRepository::new(&connection).exists_by_path(&path)
}
