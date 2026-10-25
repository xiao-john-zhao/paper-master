use crate::services::progress::TaskProgress;
use chrono::Utc;
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};
use tauri::State;

use crate::{
    ai::{
        comparison_input::build_comparison_input, input_builder::PaperAnalysisInputBuilder,
        registry::AIRequestRegistry, AIProviderRegistry,
    },
    database::{
        paper_repository::{PaperRepository, SqlitePaperRepository},
        settings_storage::{SettingsStorage, SqliteSettingsStorage},
        Database,
    },
    diagnostics::{AppLogger, LogEvent, LogLevel},
    domain::{
        ai::{
            AIResponseDiagnostics, AnalysisRequest, CompareRequest, ConnectionResult,
            PaperComparisonResult,
        },
        paper::{Paper, PaperAnalysisResult, PaperQuery},
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
        .get(settings.effective_provider())?
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
        .get(settings.effective_provider())?
        .get_models(&settings)
        .await
}

#[tauri::command]
pub async fn load_ai_model(
    database: State<'_, Database>,
    providers: State<'_, AIProviderRegistry>,
    settings: Option<AppSettings>,
) -> AppResult<ConnectionResult> {
    let settings = match settings {
        Some(value) => value,
        None => load_settings(database.inner())?,
    };
    let provider = providers.get(settings.effective_provider())?;
    let loaded_model = provider.load_model(&settings).await?;
    let mut connection = provider.check_connection(&settings).await;
    if !connection.loaded_models.contains(&loaded_model) {
        connection.loaded_models.push(loaded_model);
    }
    connection.state = "connected".into();
    connection.message = if settings.ai_provider == "codex" {
        "Codex CLI 登录状态已检查。"
    } else {
        "所选模型已成功加载，可以开始分析。"
    }
    .into();
    Ok(connection)
}

#[tauri::command]
pub async fn analyze_paper(
    database: State<'_, Database>,
    logger: State<'_, AppLogger>,
    providers: State<'_, AIProviderRegistry>,
    requests: State<'_, AIRequestRegistry>,
    request: AnalysisRequest,
) -> AppResult<PaperAnalysisResult> {
    run_analysis(
        database.inner(),
        logger.inner(),
        providers.inner(),
        requests.inner(),
        &request,
        false,
    )
    .await
}

#[tauri::command]
pub fn queue_missing_analyses(
    app: tauri::AppHandle,
    database: State<'_, Database>,
    paper_ids: Vec<String>,
) -> AppResult<crate::services::analysis_queue::QueueSummary> {
    let settings = SqliteSettingsStorage::new(&*database.connection()?).get()?;
    if settings.effective_provider() == "mock" {
        return Err(AppError::new(
            ErrorCode::InvalidSettings,
            "请先选择 LM Studio 或 Codex 真实模型，演示模型不用于补齐分析。",
        ));
    }
    let result = crate::services::analysis_queue::enqueue_missing(&database, &paper_ids)?;
    crate::services::analysis_queue::start(app);
    Ok(result)
}

pub(crate) async fn run_analysis(
    database: &Database,
    logger: &AppLogger,
    providers: &AIProviderRegistry,
    requests: &AIRequestRegistry,
    request: &AnalysisRequest,
    queued_only: bool,
) -> AppResult<PaperAnalysisResult> {
    let token = requests.register(&request.request_id);
    requests.report(TaskProgress::running(
        &request.request_id,
        "analysis",
        "等待分析资源",
    ));
    let result = async {
        let _permit = requests.acquire(&token).await?;
        if request.ensure_only {
            let connection = database.connection()?;
            if let Some(paper) =
                SqlitePaperRepository::new(&connection).find_by_id(&request.paper_id)?
            {
                let settings = crate::services::analysis_queue::settings_for(
                    &connection,
                    &request.paper_id,
                    queued_only,
                )?;
                let has_model = crate::database::analysis_versions::list(&connection, &paper.id)?
                    .iter()
                    .any(|v| {
                        v.active
                            && v.provider == settings.effective_provider()
                            && (settings.effective_provider() != "lmstudio"
                                || settings.model.is_empty()
                                || v.model == settings.model)
                    });
                if paper.analysis_status == "analyzed" && has_model {
                    if let Some(analysis) = paper.analysis {
                        return Ok(analysis);
                    }
                }
            }
        }
        if queued_only {
            let connection = database.connection()?;
            let current = SqlitePaperRepository::new(&connection).find_by_id(&request.paper_id)?;
            if !current.is_some_and(|paper| paper.analysis_status == "queued") {
                return Err(AppError::new(
                    ErrorCode::AiCancelled,
                    "自动分析任务已取消或已处理。",
                ));
            }
        }
        analyze_inner(
            database,
            logger,
            providers,
            requests,
            request,
            token,
            queued_only,
        )
        .await
    }
    .await;
    requests.remove(&request.request_id);
    match &result {
        Ok(_) => requests.report(
            TaskProgress::running(&request.request_id, "analysis", "分析完成并已保存")
                .counts(4, Some(4), "steps")
                .finish("completed"),
        ),
        Err(error) => requests.report(
            TaskProgress::running(&request.request_id, "analysis", &error.message).finish(
                if matches!(error.code, ErrorCode::AiCancelled) {
                    "cancelled"
                } else {
                    "failed"
                },
            ),
        ),
    }
    if let Err(error) = &result {
        // Includes preparation/model-discovery failures, before inference has started.
        let connection = database.connection()?;
        let repository = SqlitePaperRepository::new(&connection);
        if let Some(mut paper) = repository.find_by_id(&request.paper_id)? {
            if !queued_only || matches!(paper.analysis_status.as_str(), "queued" | "analyzing") {
                paper.analysis_status = "failed".into();
                paper.analysis_error = Some(error.message.clone());
                paper.updated_at = Utc::now().to_rfc3339();
                repository.update(&paper)?;
            }
        }
    }
    result
}

async fn analyze_inner(
    database: &Database,
    logger: &AppLogger,
    providers: &AIProviderRegistry,
    requests: &AIRequestRegistry,
    request: &AnalysisRequest,
    token: tokio_util::sync::CancellationToken,
    queued_only: bool,
) -> AppResult<PaperAnalysisResult> {
    let started = Instant::now();
    requests.report(
        TaskProgress::running(&request.request_id, "analysis", "读取原文与构建分析材料").counts(
            0,
            Some(4),
            "steps",
        ),
    );
    let (settings, mut paper, extracted_text, managed_tags) = {
        let connection = database.connection()?;
        let repository = SqlitePaperRepository::new(&connection);
        let settings = crate::services::analysis_queue::settings_for(
            &connection,
            &request.paper_id,
            queued_only,
        )?;
        let paper = repository.find_by_id(&request.paper_id)?.ok_or_else(|| {
            AppError::new(
                ErrorCode::FileNotFound,
                "The selected paper no longer exists in the library.",
            )
        })?;
        let text = repository.extracted_text(&request.paper_id)?;
        let managed_tags = build_managed_tag_context(
            &repository.list(&PaperQuery::default())?,
            &request.paper_id,
            settings.max_analysis_input_characters / 8,
        );
        (settings, paper, text, managed_tags)
    };
    let paper_budget = settings
        .max_analysis_input_characters
        .saturating_sub(managed_tags.chars().count());
    let mut input = PaperAnalysisInputBuilder::new(paper_budget).build(&paper, &extracted_text)?;
    input.push_str(&managed_tags);
    requests.report(
        TaskProgress::running(
            &request.request_id,
            "analysis",
            "材料已准备，正在请求所选 AI 服务",
        )
        .counts(1, Some(4), "steps"),
    );
    log_pipeline_stage(
        logger,
        &request.request_id,
        "PDF_INPUT_READY",
        "Paper input is ready for AI analysis",
        Some(input.chars().count()),
    );
    log_pipeline_stage(
        logger,
        &request.request_id,
        "REQUEST_BUILT",
        "Structured AI request was built",
        Some(input.chars().count()),
    );
    let provider = providers.get(settings.effective_provider())?;
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
    start.stage = Some("REQUEST_SENT".into());
    logger.log(start);

    requests.report(
        TaskProgress::running(
            &request.request_id,
            "analysis",
            "模型正在阅读与总结（当前阶段耗时不定）",
        )
        .counts(2, Some(4), "steps"),
    );

    let response = tokio::select! {
        biased;
        _ = token.cancelled() => Err(AppError::new(ErrorCode::AiCancelled, "AI 分析已取消。")),
        response = provider.analyze_paper(&input, &settings, token.clone()) => response,
    };
    match response {
        Ok(response) => {
            requests.report(
                TaskProgress::running(
                    &request.request_id,
                    "analysis",
                    "结构和证据已校验，正在保存结果",
                )
                .counts(3, Some(4), "steps"),
            );
            let mut transport_event = LogEvent::info(
                match provider.id() {
                    "lmstudio" => "ai.lmstudio",
                    "codex" => "ai.codex",
                    _ => "ai.mock",
                },
                "response",
                "AI provider response received",
            );
            transport_event.request_id = Some(request.request_id.clone());
            transport_event.provider = Some(provider.id().to_string());
            transport_event.method = response.method.clone();
            transport_event.endpoint = response.endpoint.clone();
            transport_event.model = response.model.clone().or_else(|| {
                Some(if settings.model.is_empty() {
                    "auto".into()
                } else {
                    settings.model.clone()
                })
            });
            transport_event.http_status = response.http_status;
            transport_event.stage = Some("RESPONSE_RECEIVED".into());
            if let Some(diagnostics) = response.response_diagnostics.as_ref() {
                attach_response_diagnostics(&mut transport_event, diagnostics);
            } else {
                transport_event.duration_ms = Some(started.elapsed().as_millis());
            }
            logger.log(transport_event);

            let mut extraction_event = LogEvent::info(
                match provider.id() {
                    "lmstudio" => "ai.lmstudio",
                    "codex" => "ai.codex",
                    _ => "ai.mock",
                },
                "extract_final_content",
                "Final AI response content extracted",
            );
            extraction_event.request_id = Some(request.request_id.clone());
            extraction_event.provider = Some(provider.id().to_string());
            extraction_event.stage = Some("FINAL_CONTENT_EXTRACTED".into());
            if let Some(diagnostics) = response.response_diagnostics.as_ref() {
                attach_response_diagnostics(&mut extraction_event, diagnostics);
            }
            logger.log(extraction_event);

            let mut json_event =
                LogEvent::info("ai.parser", "parse_json", "AI response JSON parsed");
            json_event.request_id = Some(request.request_id.clone());
            json_event.stage = Some("JSON_PARSED".into());
            json_event.parse_success = Some(true);
            logger.log(json_event);

            let mut parse_event =
                LogEvent::info("ai.parser", "validate", "Structured AI response validated");
            parse_event.request_id = Some(request.request_id.clone());
            parse_event.stage = Some("SCHEMA_VALIDATED".into());
            parse_event.parse_success = Some(true);
            parse_event.retry_count = Some(response.retry_count);
            logger.log(parse_event);

            let connection = database.connection()?;
            let saved_version = crate::database::analysis_versions::append(
                &connection,
                &paper.id,
                provider.id(),
                response
                    .model
                    .as_deref()
                    .unwrap_or(if provider.id() == "codex" {
                        ""
                    } else {
                        &settings.model
                    }),
                &response.value,
            )?;
            drop(connection);
            paper.analysis = Some(saved_version.result.clone());
            paper.tags = response.value.tags.clone();
            paper.analysis_status = "analyzed".to_string();
            paper.analysis_error = None;
            paper.updated_at = Utc::now().to_rfc3339();
            let mut save_event = LogEvent::info(
                "paper.repository",
                "save_analysis",
                "Structured analysis saved",
            );
            save_event.request_id = Some(request.request_id.clone());
            save_event.stage = Some("DATABASE_SAVED".into());
            logger.log(save_event);
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
            event.method = response.method.clone();
            event.endpoint = response.endpoint.clone();
            event.model = response.model.clone().or_else(|| {
                Some(if settings.model.is_empty() {
                    "auto".into()
                } else {
                    settings.model.clone()
                })
            });
            event.input_character_count = Some(input.chars().count());
            event.http_status = response.http_status;
            event.parse_success = Some(true);
            event.retry_count = Some(response.retry_count);
            event.duration_ms = Some(started.elapsed().as_millis());
            event.stage = Some("ANALYSIS_COMPLETED".into());
            logger.log(event);
            Ok(saved_version.result)
        }
        Err(error) => {
            paper.analysis_status = "failed".to_string();
            paper.analysis_error = Some(error.message.clone());
            paper.updated_at = Utc::now().to_rfc3339();
            if let Ok(connection) = database.connection() {
                let repository = SqlitePaperRepository::new(&connection);
                if let Ok(Some(mut current)) = repository.find_by_id(&paper.id) {
                    current.analysis_status = "failed".into();
                    current.analysis_error = Some(error.message.clone());
                    current.updated_at = Utc::now().to_rfc3339();
                    let _ = repository.update(&current);
                }
            }
            record_operation(
                database,
                &request.request_id,
                "analysis",
                "failed",
                error.code_string(),
            );
            if let Some(diagnostics) = error.response_diagnostics.as_ref() {
                let mut response_event =
                    LogEvent::info("ai.lmstudio", "response", "AI provider response received");
                response_event.request_id = Some(request.request_id.clone());
                response_event.provider = Some(provider.id().to_string());
                response_event.method = Some("POST".into());
                response_event.endpoint = Some("/v1/chat/completions".into());
                response_event.model = Some(if settings.model.is_empty() {
                    "auto".into()
                } else {
                    settings.model.clone()
                });
                response_event.stage = Some("RESPONSE_RECEIVED".into());
                attach_response_diagnostics(&mut response_event, diagnostics);
                logger.log(response_event);
            }
            let mut event = LogEvent::new(
                LogLevel::Error,
                "ai.provider",
                "analyze",
                "AI analysis failed; see the error code and stage.",
            );
            event.error_code = Some(error.code_string().to_string());
            event.request_id = Some(request.request_id.clone());
            event.provider = Some(provider.id().to_string());
            if provider.id() == "lmstudio" {
                event.method = Some("POST".into());
                event.endpoint = Some("/v1/chat/completions".into());
            }
            event.model = Some(if settings.model.is_empty() {
                "auto".into()
            } else {
                settings.model.clone()
            });
            event.input_character_count = Some(input.chars().count());
            event.parse_success = Some(false);
            event.duration_ms = Some(started.elapsed().as_millis());
            event.stage = error.stage.clone();
            if let Some(diagnostics) = error.response_diagnostics.as_ref() {
                attach_response_diagnostics(&mut event, diagnostics);
            }
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
        let (settings, paper, text, managed_tags) = {
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
            let managed_tags = build_managed_tag_context(
                &repository.list(&PaperQuery::default())?,
                &paper_id,
                settings.max_analysis_input_characters / 8,
            );
            (settings, paper, text, managed_tags)
        };
        let paper_budget = settings
            .max_analysis_input_characters
            .saturating_sub(managed_tags.chars().count());
        let mut input = PaperAnalysisInputBuilder::new(paper_budget).build(&paper, &text)?;
        input.push_str(&managed_tags);
        providers
            .get(settings.effective_provider())?
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
    validate_compare_paper_ids(&request.paper_ids)?;
    requests.report(TaskProgress::running(
        &request.request_id,
        "compare",
        "等待资源并准备比较材料",
    ));
    let token = requests.register(&request.request_id);
    let result = async {
        let _permit = requests.acquire(&token).await?;
        requests.report(TaskProgress::running(
            &request.request_id,
            "compare",
            "模型正在比较问题、场景、方法和原文证据",
        ));
        compare_inner(
            database.inner(),
            logger.inner(),
            providers.inner(),
            &request,
            token,
        )
        .await
    }
    .await;
    requests.remove(&request.request_id);
    match &result {
        Ok(_) => requests.report(
            TaskProgress::running(&request.request_id, "compare", "比较完成，原文证据已校验")
                .finish("completed"),
        ),
        Err(error) => requests.report(
            TaskProgress::running(&request.request_id, "compare", &error.message).finish(
                if matches!(error.code, ErrorCode::AiCancelled) {
                    "cancelled"
                } else {
                    "failed"
                },
            ),
        ),
    }
    result
}

pub(crate) async fn compare_inner(
    database: &Database,
    logger: &AppLogger,
    providers: &AIProviderRegistry,
    request: &CompareRequest,
    token: tokio_util::sync::CancellationToken,
) -> AppResult<PaperComparisonResult> {
    let started = Instant::now();
    let (settings, papers) = {
        let connection = database.connection()?;
        let repository = SqlitePaperRepository::new(&connection);
        let settings = SqliteSettingsStorage::new(&connection).get()?;
        let mut papers = Vec::with_capacity(request.paper_ids.len());
        for paper_id in &request.paper_ids {
            let paper = repository.find_by_id(paper_id)?.ok_or_else(|| {
                AppError::new(ErrorCode::FileNotFound, "所选论文已不在本地文献库中。")
            })?;
            papers.push((paper, repository.extracted_text(paper_id)?));
        }
        (settings, papers)
    };
    let input = build_comparison_input(&papers, settings.max_analysis_input_characters)?;
    let provider = providers.get(settings.effective_provider())?;
    record_operation(
        database,
        &request.request_id,
        "compare",
        "running",
        &format!("{} 篇论文对比已开始", papers.len()),
    );
    let mut start = LogEvent::info("ai.provider", "compare", "多篇论文结构化对比已开始");
    start.request_id = Some(request.request_id.clone());
    start.provider = Some(provider.id().to_string());
    start.model = Some(if settings.model.is_empty() {
        "auto".into()
    } else {
        settings.model.clone()
    });
    start.input_character_count = Some(input.chars().count());
    logger.log(start);
    let response = tokio::select! {
        biased;
        _ = token.cancelled() => Err(AppError::new(ErrorCode::AiCancelled, "论文对比已取消。")),
        response = provider.compare_papers(&input, &settings, token.clone()) => response,
    };
    match response {
        Ok(response) => {
            record_operation(
                database,
                &request.request_id,
                "compare",
                "success",
                &format!("{} 篇论文对比已完成", papers.len()),
            );
            let mut event = LogEvent::info("ai.provider", "compare", "多篇论文结构化对比已完成");
            event.request_id = Some(request.request_id.clone());
            event.provider = Some(provider.id().to_string());
            event.method = response.method.clone();
            event.endpoint = response.endpoint.clone();
            event.model = response.model.clone().or_else(|| {
                Some(if settings.model.is_empty() {
                    "auto".into()
                } else {
                    settings.model.clone()
                })
            });
            event.input_character_count = Some(input.chars().count());
            event.http_status = response.http_status;
            event.parse_success = Some(true);
            event.retry_count = Some(response.retry_count);
            event.duration_ms = Some(started.elapsed().as_millis());
            if let Some(diagnostics) = response.response_diagnostics.as_ref() {
                attach_response_diagnostics(&mut event, diagnostics);
            }
            logger.log(event);
            let report = serde_json::json!({"paper_ids":request.paper_ids,"provider":provider.id(),"model":response.model,"original":response.value,"current":response.value});
            let connection = database.connection()?;
            connection
                .execute(
                    "INSERT INTO settings(key,value_json,updated_at) VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json,updated_at=excluded.updated_at",
                    rusqlite::params![
                        format!("comparison-report:{}", request.request_id),
                        report.to_string(),
                        Utc::now().to_rfc3339()
                    ],
                )
                .map_err(|_| AppError::new(ErrorCode::DatabaseWriteFailed, "对比结果无法保存。"))?;
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
                "AI analysis failed; see the error code and stage.",
            );
            event.error_code = Some(error.code_string().to_string());
            event.request_id = Some(request.request_id.clone());
            event.provider = Some(provider.id().to_string());
            if provider.id() == "lmstudio" {
                event.method = Some("POST".into());
                event.endpoint = Some("/v1/chat/completions".into());
            }
            event.model = Some(if settings.model.is_empty() {
                "auto".into()
            } else {
                settings.model.clone()
            });
            event.parse_success = Some(false);
            event.duration_ms = Some(started.elapsed().as_millis());
            event.stage = error.stage.clone();
            if let Some(diagnostics) = error.response_diagnostics.as_ref() {
                attach_response_diagnostics(&mut event, diagnostics);
            }
            logger.log(event);
            Err(error.request_id(&request.request_id))
        }
    }
}

#[tauri::command]
pub fn cancel_ai_request(requests: State<'_, AIRequestRegistry>, request_id: String) -> bool {
    requests.cancel(&request_id)
}

#[tauri::command]
pub fn cancel_paper_analysis(
    database: State<'_, Database>,
    requests: State<'_, AIRequestRegistry>,
    paper_id: String,
) -> AppResult<()> {
    requests.cancel(&format!("analysis-auto-{paper_id}"));
    let connection = database.connection()?;
    let repository = SqlitePaperRepository::new(&connection);
    if let Some(mut paper) = repository.find_by_id(&paper_id)? {
        if matches!(paper.analysis_status.as_str(), "queued" | "analyzing") {
            paper.analysis_status = "failed".into();
            paper.analysis_error = Some("自动分析已取消，可点击重新分析。".into());
            repository.update(&paper)?;
        }
    }
    Ok(())
}

fn load_settings(database: &Database) -> AppResult<AppSettings> {
    let connection = database.connection()?;
    SqliteSettingsStorage::new(&connection).get()
}

fn log_pipeline_stage(
    logger: &AppLogger,
    request_id: &str,
    stage: &str,
    message: &str,
    input_character_count: Option<usize>,
) {
    let mut event = LogEvent::info("ai.pipeline", "stage", message);
    event.request_id = Some(request_id.to_string());
    event.stage = Some(stage.to_string());
    event.input_character_count = input_character_count;
    logger.log(event);
}

fn attach_response_diagnostics(event: &mut LogEvent, diagnostics: &AIResponseDiagnostics) {
    event.http_status = diagnostics.http_status;
    event.finish_reason = diagnostics.finish_reason.clone();
    event.content_present = Some(diagnostics.content_present);
    event.content_length = Some(diagnostics.content_length);
    event.reasoning_present = Some(diagnostics.reasoning_present);
    event.reasoning_length = Some(diagnostics.reasoning_length);
    event.tool_calls_present = Some(diagnostics.tool_calls_present);
    event.duration_ms = diagnostics.duration_ms;
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

fn build_managed_tag_context(papers: &[Paper], current_paper_id: &str, budget: usize) -> String {
    let mut groups = HashMap::<String, HashMap<String, usize>>::new();
    for paper in papers.iter().filter(|paper| paper.id != current_paper_id) {
        for tag in &paper.tags {
            let tag = tag.trim();
            if !tag.is_empty() {
                *groups
                    .entry(canonical_tag_key(tag))
                    .or_default()
                    .entry(tag.to_string())
                    .or_default() += 1;
            }
        }
    }
    let mut tags = groups
        .into_values()
        .map(|variants| {
            let total: usize = variants.values().sum();
            let canonical = variants
                .into_iter()
                .max_by(|(tag_a, count_a), (tag_b, count_b)| {
                    count_a
                        .cmp(count_b)
                        .then_with(|| {
                            canonical_tag_style_score(tag_a).cmp(&canonical_tag_style_score(tag_b))
                        })
                        .then_with(|| tag_b.cmp(tag_a))
                })
                .map(|(tag, _)| tag)
                .unwrap_or_default();
            (canonical, total)
        })
        .collect::<Vec<_>>();
    tags.sort_by(|(tag_a, count_a), (tag_b, count_b)| {
        count_b.cmp(count_a).then_with(|| tag_a.cmp(tag_b))
    });
    let mut context =
        String::from("\n\n[MANAGED TAG VOCABULARY START — NAMING ONLY, NOT PAPER EVIDENCE]\n");
    for (tag, count) in tags.into_iter().take(100) {
        let line = format!("- {tag} (used {count})\n");
        if context.chars().count() + line.chars().count() > budget.max(200) {
            break;
        }
        context.push_str(&line);
    }
    context.push_str("[MANAGED TAG VOCABULARY END]\n");
    if context.lines().count() <= 2 {
        String::new()
    } else {
        context
    }
}

fn canonical_tag_key(tag: &str) -> String {
    tag.chars()
        .flat_map(char::to_lowercase)
        .filter(|character| character.is_alphanumeric())
        .collect()
}

fn canonical_tag_style_score(tag: &str) -> u8 {
    u8::from(tag.contains('（') && tag.contains('）'))
}

fn validate_compare_paper_ids(paper_ids: &[String]) -> AppResult<()> {
    let unique_ids = paper_ids.iter().collect::<HashSet<_>>();
    if !(2..=8).contains(&unique_ids.len()) || unique_ids.len() != paper_ids.len() {
        return Err(AppError::new(
            ErrorCode::PaperNotAnalyzed,
            "请选择 2–8 篇互不相同的论文进行对比。",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_tag_context_excludes_current_paper_and_orders_by_frequency() {
        let papers = vec![
            paper("current", &["当前专用标签"]),
            paper("one", &["集成感知与通信（ISAC）", "资源分配"]),
            paper("two", &["集成感知与通信(ISAC)"]),
        ];
        let context = build_managed_tag_context(&papers, "current", 1000);
        assert!(context.contains("集成感知与通信（ISAC） (used 2)"));
        assert!(context.contains("资源分配 (used 1)"));
        assert!(!context.contains("当前专用标签"));
        assert!(context.find("used 2").unwrap() < context.find("used 1").unwrap());
    }

    #[test]
    fn comparison_accepts_two_to_eight_unique_papers() {
        assert!(validate_compare_paper_ids(&["a".into(), "b".into()]).is_ok());
        assert!(validate_compare_paper_ids(
            &(0..8)
                .map(|index| format!("paper-{index}"))
                .collect::<Vec<_>>()
        )
        .is_ok());
        assert!(validate_compare_paper_ids(&["a".into()]).is_err());
        assert!(validate_compare_paper_ids(&["a".into(), "a".into()]).is_err());
        assert!(validate_compare_paper_ids(
            &(0..9)
                .map(|index| format!("paper-{index}"))
                .collect::<Vec<_>>()
        )
        .is_err());
    }

    fn paper(id: &str, tags: &[&str]) -> Paper {
        Paper {
            id: id.into(),
            file_path: format!("{id}.pdf"),
            file_name: format!("{id}.pdf"),
            title: id.into(),
            publication_venue: None,
            authors: Vec::new(),
            year: None,
            abstract_text: None,
            analysis: None,
            analysis_provider: None,
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
            analysis_status: "not_analyzed".into(),
            analysis_error: None,
            pdf_diagnostics: None,
            created_at: "now".into(),
            updated_at: "now".into(),
        }
    }
}

#[tauri::command]
pub fn edit_comparison_result(
    database: State<'_, Database>,
    request_id: String,
    mut result: PaperComparisonResult,
) -> AppResult<PaperComparisonResult> {
    if serde_json::to_vec(&result).map_or(true, |s| s.len() > 300000)
        || result.schema_version != "2"
    {
        return Err(AppError::new(
            ErrorCode::AiSchemaValidationFailed,
            "对比内容过长或格式无效。",
        ));
    }
    let connection = database.connection()?;
    let key = format!("comparison-report:{request_id}");
    let json: String = connection
        .query_row(
            "SELECT value_json FROM settings WHERE key=?1",
            [&key],
            |r| r.get(0),
        )
        .map_err(|_| AppError::new(ErrorCode::FileNotFound, "请先生成对比结果。"))?;
    let mut report: serde_json::Value = serde_json::from_str(&json)
        .map_err(|_| AppError::new(ErrorCode::DatabaseReadFailed, "对比记录无效。"))?;
    let original: PaperComparisonResult = serde_json::from_value(report["original"].clone())
        .map_err(|_| AppError::new(ErrorCode::DatabaseReadFailed, "原始对比记录无效。"))?;
    result.evidence = original.evidence;
    report["current"] = serde_json::to_value(&result).unwrap();
    connection
        .execute(
            "UPDATE settings SET value_json=?1,updated_at=?2 WHERE key=?3",
            rusqlite::params![report.to_string(), Utc::now().to_rfc3339(), key],
        )
        .map_err(|_| AppError::new(ErrorCode::DatabaseWriteFailed, "对比编辑保存失败。"))?;
    Ok(result)
}
