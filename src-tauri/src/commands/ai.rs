use chrono::Utc;
use std::time::Instant;
use tauri::State;

use crate::{
    ai::{
        input_builder::PaperAnalysisInputBuilder, registry::AIRequestRegistry, AIProviderRegistry,
    },
    database::{
        paper_repository::{PaperRepository, SqlitePaperRepository},
        settings_storage::{SettingsStorage, SqliteSettingsStorage},
        Database,
    },
    diagnostics::{AppLogger, LogEvent, LogLevel},
    domain::{
        ai::{AnalysisRequest, CompareRequest, ConnectionResult, PaperComparisonResult},
        paper::PaperAnalysisResult,
        settings::AppSettings,
    },
    error::{AppError, AppResult, ErrorCode},
};

#[tauri::command]
pub async fn check_ai_connection(
    database: State<'_, Database>,
    providers: State<'_, AIProviderRegistry>,
    settings: Option<AppSettings>,
) -> AppResult<ConnectionResult> {
    let settings = match settings {
        Some(value) => value,
        None => load_settings(database.inner())?,
    };
    Ok(providers
        .get(&settings.ai_provider)?
        .check_connection(&settings)
        .await)
}

#[tauri::command]
pub async fn get_ai_models(
    database: State<'_, Database>,
    providers: State<'_, AIProviderRegistry>,
    settings: Option<AppSettings>,
) -> AppResult<Vec<String>> {
    let settings = match settings {
        Some(value) => value,
        None => load_settings(database.inner())?,
    };
    providers
        .get(&settings.ai_provider)?
        .get_models(&settings)
        .await
}

#[tauri::command]
pub async fn analyze_paper(
    database: State<'_, Database>,
    logger: State<'_, AppLogger>,
    providers: State<'_, AIProviderRegistry>,
    requests: State<'_, AIRequestRegistry>,
    request: AnalysisRequest,
) -> AppResult<PaperAnalysisResult> {
    let token = requests.register(&request.request_id);
    let result = analyze_inner(
        database.inner(),
        logger.inner(),
        providers.inner(),
        &request,
        token,
    )
    .await;
    requests.remove(&request.request_id);
    result
}

async fn analyze_inner(
    database: &Database,
    logger: &AppLogger,
    providers: &AIProviderRegistry,
    request: &AnalysisRequest,
    token: tokio_util::sync::CancellationToken,
) -> AppResult<PaperAnalysisResult> {
    let started = Instant::now();
    let (settings, mut paper, extracted_text) = {
        let connection = database.connection()?;
        let repository = SqlitePaperRepository::new(&connection);
        let settings = SqliteSettingsStorage::new(&connection).get()?;
        let paper = repository.find_by_id(&request.paper_id)?.ok_or_else(|| {
            AppError::new(
                ErrorCode::FileNotFound,
                "The selected paper no longer exists in the library.",
            )
        })?;
        let text = repository.extracted_text(&request.paper_id)?;
        (settings, paper, text)
    };
    let input = PaperAnalysisInputBuilder::new(settings.max_analysis_input_characters)
        .build(&paper, &extracted_text)?;
    let provider = providers.get(&settings.ai_provider)?;
    paper.analysis_status = "analyzing".to_string();
    paper.analysis_error = None;
    paper.updated_at = Utc::now().to_rfc3339();
    {
        let connection = database.connection()?;
        SqlitePaperRepository::new(&connection).update(&paper)?;
    }
    record_operation(
        database,
        &request.request_id,
        "analysis",
        "running",
        "AI analysis started",
    );
    let mut start = LogEvent::info("ai.provider", "analyze", "AI analysis request started");
    start.request_id = Some(request.request_id.clone());
    start.provider = Some(provider.id().to_string());
    start.model = Some(if settings.model.is_empty() {
        "auto".into()
    } else {
        settings.model.clone()
    });
    start.input_character_count = Some(input.chars().count());
    logger.log(start);

    match provider.analyze_paper(&input, &settings, token).await {
        Ok(response) => {
            paper.analysis = Some(response.value.clone());
            paper.tags = response.value.tags.clone();
            paper.analysis_status = "analyzed".to_string();
            paper.analysis_error = None;
            paper.updated_at = Utc::now().to_rfc3339();
            let connection = database.connection()?;
            SqlitePaperRepository::new(&connection).update(&paper)?;
            drop(connection);
            record_operation(
                database,
                &request.request_id,
                "analysis",
                "success",
                "AI analysis completed",
            );
            let mut event = LogEvent::info(
                "ai.provider",
                "analyze",
                "AI analysis completed and structured result was saved",
            );
            event.request_id = Some(request.request_id.clone());
            event.provider = Some(provider.id().to_string());
            event.model = Some(if settings.model.is_empty() {
                "auto".into()
            } else {
                settings.model.clone()
            });
            event.input_character_count = Some(input.chars().count());
            event.http_status = response.http_status;
            event.parse_success = Some(true);
            event.retry_count = Some(response.retry_count);
            event.duration_ms = Some(started.elapsed().as_millis());
            logger.log(event);
            Ok(response.value)
        }
        Err(error) => {
            paper.analysis_status = "failed".to_string();
            paper.analysis_error = Some(error.message.clone());
            paper.updated_at = Utc::now().to_rfc3339();
            if let Ok(connection) = database.connection() {
                let _ = SqlitePaperRepository::new(&connection).update(&paper);
            }
            record_operation(
                database,
                &request.request_id,
                "analysis",
                "failed",
                error.code_string(),
            );
            let mut event = LogEvent::new(
                LogLevel::Error,
                "ai.provider",
                "analyze",
                error
                    .detail
                    .clone()
                    .unwrap_or_else(|| error.message.clone()),
            );
            event.error_code = Some(error.code_string().to_string());
            event.request_id = Some(request.request_id.clone());
            event.provider = Some(provider.id().to_string());
            event.model = Some(if settings.model.is_empty() {
                "auto".into()
            } else {
                settings.model.clone()
            });
            event.input_character_count = Some(input.chars().count());
            event.parse_success = Some(false);
            event.duration_ms = Some(started.elapsed().as_millis());
            logger.log(event);
            Err(error.request_id(&request.request_id))
        }
    }
}

#[tauri::command]
pub async fn generate_paper_tags(
    database: State<'_, Database>,
    providers: State<'_, AIProviderRegistry>,
    requests: State<'_, AIRequestRegistry>,
    paper_id: String,
    request_id: String,
) -> AppResult<Vec<String>> {
    let token = requests.register(&request_id);
    let result = async {
        let (settings, paper, text) = {
            let connection = database.connection()?;
            let repository = SqlitePaperRepository::new(&connection);
            let settings = SqliteSettingsStorage::new(&connection).get()?;
            let paper = repository.find_by_id(&paper_id)?.ok_or_else(|| {
                AppError::new(
                    ErrorCode::FileNotFound,
                    "The selected paper no longer exists.",
                )
            })?;
            let text = repository.extracted_text(&paper_id)?;
            (settings, paper, text)
        };
        let input = PaperAnalysisInputBuilder::new(settings.max_analysis_input_characters)
            .build(&paper, &text)?;
        providers
            .get(&settings.ai_provider)?
            .generate_tags(&input, &settings, token)
            .await
    }
    .await;
    requests.remove(&request_id);
    result
}

#[tauri::command]
pub async fn compare_papers(
    database: State<'_, Database>,
    logger: State<'_, AppLogger>,
    providers: State<'_, AIProviderRegistry>,
    requests: State<'_, AIRequestRegistry>,
    request: CompareRequest,
) -> AppResult<PaperComparisonResult> {
    if request.paper_a_id == request.paper_b_id {
        return Err(AppError::new(
            ErrorCode::PaperNotAnalyzed,
            "Select two different papers to compare.",
        ));
    }
    let token = requests.register(&request.request_id);
    let result = compare_inner(
        database.inner(),
        logger.inner(),
        providers.inner(),
        &request,
        token,
    )
    .await;
    requests.remove(&request.request_id);
    result
}

async fn compare_inner(
    database: &Database,
    logger: &AppLogger,
    providers: &AIProviderRegistry,
    request: &CompareRequest,
    token: tokio_util::sync::CancellationToken,
) -> AppResult<PaperComparisonResult> {
    let started = Instant::now();
    let (settings, paper_a, paper_b) = {
        let connection = database.connection()?;
        let repository = SqlitePaperRepository::new(&connection);
        let settings = SqliteSettingsStorage::new(&connection).get()?;
        let paper_a = repository.find_by_id(&request.paper_a_id)?.ok_or_else(|| {
            AppError::new(
                ErrorCode::FileNotFound,
                "Paper A no longer exists in the library.",
            )
        })?;
        let paper_b = repository.find_by_id(&request.paper_b_id)?.ok_or_else(|| {
            AppError::new(
                ErrorCode::FileNotFound,
                "Paper B no longer exists in the library.",
            )
        })?;
        (settings, paper_a, paper_b)
    };
    let analysis_a = paper_a.analysis.as_ref().ok_or_else(|| {
        AppError::new(
            ErrorCode::PaperNotAnalyzed,
            format!("Analyze “{}” before comparing papers.", paper_a.title),
        )
    })?;
    let analysis_b = paper_b.analysis.as_ref().ok_or_else(|| {
        AppError::new(
            ErrorCode::PaperNotAnalyzed,
            format!("Analyze “{}” before comparing papers.", paper_b.title),
        )
    })?;
    let input = serde_json::to_string_pretty(&serde_json::json!({
        "paper_a": { "title": paper_a.title, "analysis": analysis_a },
        "paper_b": { "title": paper_b.title, "analysis": analysis_b }
    }))
    .map_err(|error| {
        AppError::new(
            ErrorCode::Internal,
            "The stored analyses could not be prepared for comparison.",
        )
        .detail(error.to_string())
    })?;
    let provider = providers.get(&settings.ai_provider)?;
    record_operation(
        database,
        &request.request_id,
        "compare",
        "running",
        "Paper comparison started",
    );
    let mut start = LogEvent::info(
        "ai.provider",
        "compare",
        "Structured paper comparison started",
    );
    start.request_id = Some(request.request_id.clone());
    start.provider = Some(provider.id().to_string());
    start.model = Some(if settings.model.is_empty() {
        "auto".into()
    } else {
        settings.model.clone()
    });
    start.input_character_count = Some(input.chars().count());
    logger.log(start);
    match provider.compare_papers(&input, &settings, token).await {
        Ok(response) => {
            record_operation(
                database,
                &request.request_id,
                "compare",
                "success",
                "Paper comparison completed",
            );
            let mut event = LogEvent::info(
                "ai.provider",
                "compare",
                "Structured paper comparison completed",
            );
            event.request_id = Some(request.request_id.clone());
            event.provider = Some(provider.id().to_string());
            event.model = Some(if settings.model.is_empty() {
                "auto".into()
            } else {
                settings.model.clone()
            });
            event.input_character_count = Some(input.chars().count());
            event.http_status = response.http_status;
            event.parse_success = Some(true);
            event.retry_count = Some(response.retry_count);
            event.duration_ms = Some(started.elapsed().as_millis());
            logger.log(event);
            Ok(response.value)
        }
        Err(error) => {
            record_operation(
                database,
                &request.request_id,
                "compare",
                "failed",
                error.code_string(),
            );
            let mut event = LogEvent::new(
                LogLevel::Error,
                "ai.provider",
                "compare",
                error
                    .detail
                    .clone()
                    .unwrap_or_else(|| error.message.clone()),
            );
            event.error_code = Some(error.code_string().to_string());
            event.request_id = Some(request.request_id.clone());
            event.provider = Some(provider.id().to_string());
            event.parse_success = Some(false);
            event.duration_ms = Some(started.elapsed().as_millis());
            logger.log(event);
            Err(error.request_id(&request.request_id))
        }
    }
}

#[tauri::command]
pub fn cancel_ai_request(requests: State<'_, AIRequestRegistry>, request_id: String) -> bool {
    requests.cancel(&request_id)
}

fn load_settings(database: &Database) -> AppResult<AppSettings> {
    let connection = database.connection()?;
    SqliteSettingsStorage::new(&connection).get()
}

fn record_operation(
    database: &Database,
    operation_id: &str,
    kind: &str,
    status: &str,
    summary: &str,
) {
    if let Ok(connection) = database.connection() {
        let _ = connection.execute(
            "INSERT INTO operation_status(operation_id,kind,status,summary,updated_at) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(operation_id) DO UPDATE SET status=excluded.status,summary=excluded.summary,updated_at=excluded.updated_at",
            rusqlite::params![operation_id, kind, status, summary, Utc::now().to_rfc3339()],
        );
    }
}
