use std::time::Instant;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

use crate::{
    database::{
        paper_repository::{PaperRepository, SqlitePaperRepository},
        Database,
    },
    diagnostics::{AppLogger, LogEvent, LogLevel},
    domain::paper::{Paper, PaperQuery},
    error::{AppError, AppResult, ErrorCode},
};

#[tauri::command]
pub fn get_team_links(
    database: State<'_, Database>,
) -> AppResult<Vec<crate::services::authors::TeamLink>> {
    let connection = database.connection()?;
    let repository = SqlitePaperRepository::new(&connection);
    let library = repository
        .list(&PaperQuery::default())?
        .into_iter()
        .map(|paper| {
            // Team evidence comes from the author header/footnotes, never reference lists.
            let text: String = repository
                .extracted_text(&paper.id)?
                .chars()
                .take(16000)
                .collect();
            Ok((paper, text))
        })
        .collect::<AppResult<Vec<_>>>()?;
    Ok(crate::services::authors::team_links(&library))
}

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
pub fn open_paper_pdf(
    app: tauri::AppHandle,
    database: State<'_, Database>,
    id: String,
) -> AppResult<()> {
    let connection = database.connection()?;
    let paper = SqlitePaperRepository::new(&connection)
        .find_by_id(&id)?
        .ok_or_else(|| AppError::new(ErrorCode::FileNotFound, "论文已从库中移除。"))?;
    let path = crate::services::paper_files::existing_pdf(&paper.file_path)?;
    // Rust opener is intentionally scoped by the library record, rather than a blanket JS file scope.
    app.opener()
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|_| {
            AppError::new(
                ErrorCode::PdfReadFailed,
                "系统未能打开 PDF，请安装或选择默认 PDF 阅读器，或重新关联文件。",
            )
        })
}

#[tauri::command]
pub fn relink_paper_pdf(
    database: State<'_, Database>,
    id: String,
    path: String,
) -> AppResult<Paper> {
    let path = crate::services::paper_files::existing_pdf(&path)?;
    let connection = database.connection()?;
    let repository = SqlitePaperRepository::new(&connection);
    let mut paper = repository
        .find_by_id(&id)?
        .ok_or_else(|| AppError::new(ErrorCode::FileNotFound, "论文已从库中移除。"))?;
    // Relinking repairs a moved file; require the same name to avoid silently mixing analyses.
    if !path.file_name().is_some_and(|name| {
        name.to_string_lossy()
            .eq_ignore_ascii_case(&paper.file_name)
    }) {
        return Err(AppError::new(
            ErrorCode::PdfReadFailed,
            "请选择同名的原始 PDF。另一篇论文请使用导入功能。",
        ));
    }
    paper.file_path = path.to_string_lossy().to_string();
    paper.updated_at = chrono::Utc::now().to_rfc3339();
    repository.update(&paper)
}

#[tauri::command]
pub fn get_author_context(
    database: State<'_, Database>,
    id: String,
) -> AppResult<crate::services::authors::AuthorContext> {
    let connection = database.connection()?;
    let repository = SqlitePaperRepository::new(&connection);
    let paper = repository
        .find_by_id(&id)?
        .ok_or_else(|| AppError::new(ErrorCode::FileNotFound, "论文已从库中移除。"))?;
    let text = repository.extracted_text(&id)?;
    let library = repository
        .list(&PaperQuery::default())?
        .into_iter()
        .map(|other| {
            let source = repository.extracted_text(&other.id)?;
            Ok((other, source))
        })
        .collect::<AppResult<Vec<_>>>()?;
    Ok(crate::services::authors::build_context(
        &paper, &text, &library,
    ))
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

#[derive(serde::Deserialize)]
pub struct EditableMetadata {
    title: String,
    authors: Vec<String>,
    #[serde(rename = "abstract")]
    abstract_text: Option<String>,
    publication_venue: Option<String>,
    year: Option<i32>,
}
#[tauri::command]
pub fn edit_paper_metadata(
    database: State<'_, Database>,
    id: String,
    metadata: EditableMetadata,
) -> AppResult<Paper> {
    if metadata.title.trim().is_empty()
        || metadata.title.len() > 4000
        || metadata.authors.len() > 100
        || metadata.authors.iter().any(|a| a.len() > 500)
        || metadata
            .abstract_text
            .as_ref()
            .is_some_and(|a| a.len() > 100000)
        || metadata
            .publication_venue
            .as_ref()
            .is_some_and(|a| a.len() > 4000)
        || metadata.year.is_some_and(|y| !(1900..=2100).contains(&y))
    {
        return Err(AppError::new(
            ErrorCode::AiSchemaValidationFailed,
            "请检查题名、年份或文字长度。",
        ));
    }
    let connection = database.connection()?;
    let repo = SqlitePaperRepository::new(&connection);
    let mut paper = repo
        .find_by_id(&id)?
        .ok_or_else(|| AppError::new(ErrorCode::FileNotFound, "论文已移除。"))?;
    paper.title = metadata.title.trim().into();
    paper.authors = metadata
        .authors
        .into_iter()
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
        .collect();
    paper.abstract_text = metadata.abstract_text;
    paper.publication_venue = metadata.publication_venue;
    paper.year = metadata.year;
    paper.updated_at = chrono::Utc::now().to_rfc3339();
    let saved = repo.update(&paper)?;
    connection.execute("INSERT INTO settings(key,value_json,updated_at) VALUES(?1,'true',?2) ON CONFLICT(key) DO UPDATE SET updated_at=excluded.updated_at",rusqlite::params![format!("metadata-edited:{id}"),paper.updated_at]).map_err(|_|AppError::new(ErrorCode::DatabaseWriteFailed,"无法保存元数据编辑标记。"))?;
    Ok(saved)
}
