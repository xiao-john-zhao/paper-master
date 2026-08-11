use chrono::Utc;
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};
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
pub async fn load_ai_model(
    database: State<'_, Database>,
    providers: State<'_, AIProviderRegistry>,
    settings: Option<AppSettings>,
) -> AppResult<ConnectionResult> {
    let settings = match settings {
        Some(value) => value,
        None => load_settings(database.inner())?,
    };
    let provider = providers.get(&settings.ai_provider)?;
    let loaded_model = provider.load_model(&settings).await?;
    let mut connection = provider.check_connection(&settings).await;
    if !connection.loaded_models.contains(&loaded_model) {
        connection.loaded_models.push(loaded_model);
    }
    connection.state = "connected".into();
    connection.message = "所选模型已成功加载，LM Studio 可以开始分析。".into();
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
    let (settings, mut paper, extracted_text, managed_tags) = {
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
            let mut transport_event = LogEvent::info(
                if provider.id() == "lmstudio" {
                    "ai.lmstudio"
                } else {
                    "ai.mock"
                },
                "response",
                "AI provider response received",
            );
            transport_event.request_id = Some(request.request_id.clone());
            transport_event.provider = Some(provider.id().to_string());
            transport_event.model = Some(if settings.model.is_empty() {
                "auto".into()
            } else {
                settings.model.clone()
            });
            transport_event.http_status = response.http_status;
            transport_event.duration_ms = Some(started.elapsed().as_millis());
            logger.log(transport_event);

            let mut parse_event =
                LogEvent::info("ai.parser", "validate", "Structured AI response validated");
            parse_event.request_id = Some(request.request_id.clone());
            parse_event.parse_success = Some(true);
            parse_event.retry_count = Some(response.retry_count);
            logger.log(parse_event);

            paper.analysis = Some(response.value.clone());
            paper.tags = response.value.tags.clone();
            paper.analysis_status = "analyzed".to_string();
            paper.analysis_error = None;
            paper.updated_at = Utc::now().to_rfc3339();
            let connection = database.connection()?;
            SqlitePaperRepository::new(&connection).update(&paper)?;
            drop(connection);
            let mut save_event = LogEvent::info(
                "paper.repository",
                "save_analysis",
                "Structured analysis saved",
            );
            save_event.request_id = Some(request.request_id.clone());
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
    validate_compare_paper_ids(&request.paper_ids)?;
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
    let (settings, papers) = {
        let connection = database.connection()?;
        let repository = SqlitePaperRepository::new(&connection);
        let settings = SqliteSettingsStorage::new(&connection).get()?;
        let mut papers = Vec::with_capacity(request.paper_ids.len());
        for paper_id in &request.paper_ids {
            papers.push(repository.find_by_id(paper_id)?.ok_or_else(|| {
                AppError::new(ErrorCode::FileNotFound, "所选论文已不在本地文献库中。")
            })?);
        }
        (settings, papers)
    };
    let structured_papers = papers
        .iter()
        .enumerate()
        .map(|(index, paper)| {
            let analysis = paper.analysis.as_ref().ok_or_else(|| {
                AppError::new(
                    ErrorCode::PaperNotAnalyzed,
                    format!("请先分析《{}》，再执行多篇论文对比。", paper.title),
                )
            })?;
            Ok(serde_json::json!({
                "paper_number": index + 1,
                "title": paper.title,
                "analysis": analysis,
            }))
        })
        .collect::<AppResult<Vec<_>>>()?;
    let input = serde_json::to_string_pretty(&serde_json::json!({
        "papers": structured_papers
    }))
    .map_err(|error| {
        AppError::new(ErrorCode::Internal, "无法准备已保存的论文分析用于对比。")
            .detail(error.to_string())
    })?;
    let provider = providers.get(&settings.ai_provider)?;
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
    match provider.compare_papers(&input, &settings, token).await {
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
            authors: Vec::new(),
            year: None,
            abstract_text: None,
            analysis: None,
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
            analysis_status: "not_analyzed".into(),
            analysis_error: None,
            pdf_diagnostics: None,
            created_at: "now".into(),
            updated_at: "now".into(),
        }
    }
}
