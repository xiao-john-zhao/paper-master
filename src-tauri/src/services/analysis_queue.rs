use crate::{
    ai::{registry::AIRequestRegistry, AIProviderRegistry},
    commands::ai::run_analysis,
    database::{
        paper_repository::{PaperRepository, SqlitePaperRepository},
        Database,
    },
    diagnostics::{AppLogger, LogEvent},
    domain::{ai::AnalysisRequest, paper::PaperQuery},
    error::AppResult,
};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Default)]
pub struct AnalysisQueue(pub tokio::sync::Mutex<()>);

#[derive(serde::Serialize)]
pub struct QueueSummary {
    pub queued: usize,
    pub skipped: usize,
    pub no_text: usize,
}

pub fn enqueue_missing(database: &Database, ids: &[String]) -> AppResult<QueueSummary> {
    use crate::database::settings_storage::{SettingsStorage, SqliteSettingsStorage};
    let connection = database.connection()?;
    let transaction = connection.unchecked_transaction().map_err(|_| {
        crate::error::AppError::new(
            crate::error::ErrorCode::DatabaseWriteFailed,
            "无法开始分析排队。",
        )
    })?;
    let repo = SqlitePaperRepository::new(&transaction);
    let mut settings = SqliteSettingsStorage::new(&transaction).get()?;
    if settings.effective_provider() == "lmstudio" {
        settings.ai_provider = "lmstudio".into();
        settings.privacy_mode = "local_only".into();
    }
    let snapshot = serde_json::to_string(&settings).map_err(|_| {
        crate::error::AppError::new(
            crate::error::ErrorCode::InvalidSettings,
            "模型配置无法保存。",
        )
    })?;
    let mut result = QueueSummary {
        queued: 0,
        skipped: 0,
        no_text: 0,
    };
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    for id in unique {
        let Some(mut paper) = repo.find_by_id(id)? else {
            result.skipped += 1;
            continue;
        };
        let complete = crate::database::analysis_versions::list(&connection, id)?
            .iter()
            .any(|version| {
                version.provider != "extractive"
                    && version.provider != "mock"
                    && crate::ai::parser::parse_analysis_response(
                        &serde_json::to_string(&version.result).unwrap_or_default(),
                    )
                    .is_ok()
            });
        if complete || ["queued", "analyzing"].contains(&paper.analysis_status.as_str()) {
            result.skipped += 1;
            continue;
        }
        if repo.extracted_text(id)?.trim().is_empty() {
            result.no_text += 1;
            continue;
        }
        paper.analysis_status = "queued".into();
        paper.analysis_error = None;
        repo.update(&paper)?;
        transaction.execute("INSERT INTO settings(key,value_json,updated_at) VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json,updated_at=excluded.updated_at",rusqlite::params![format!("analysis-queue-settings:{id}"),snapshot,chrono::Utc::now().to_rfc3339()]).map_err(|_|crate::error::AppError::new(crate::error::ErrorCode::DatabaseWriteFailed,"分析任务配置无法保存。"))?;
        result.queued += 1;
    }
    transaction.commit().map_err(|_| {
        crate::error::AppError::new(
            crate::error::ErrorCode::DatabaseWriteFailed,
            "分析排队未提交。",
        )
    })?;
    Ok(result)
}

pub fn settings_for(
    connection: &rusqlite::Connection,
    id: &str,
    queued: bool,
) -> AppResult<crate::domain::settings::AppSettings> {
    use crate::database::settings_storage::{SettingsStorage, SqliteSettingsStorage};
    use rusqlite::OptionalExtension;
    if queued {
        let snapshot: Option<String> = connection
            .query_row(
                "SELECT value_json FROM settings WHERE key=?1",
                [format!("analysis-queue-settings:{id}")],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| {
                crate::error::AppError::new(
                    crate::error::ErrorCode::DatabaseReadFailed,
                    "分析任务配置读取失败。",
                )
            })?;
        if let Some(json) = snapshot {
            let mut settings: crate::domain::settings::AppSettings = serde_json::from_str(&json)
                .map_err(|_| {
                    crate::error::AppError::new(
                        crate::error::ErrorCode::InvalidSettings,
                        "分析任务配置无效。",
                    )
                })?;
            let current = SqliteSettingsStorage::new(connection).get()?;
            if current.privacy_mode != "codex_online" {
                settings.privacy_mode = "local_only".into();
            }
            return Ok(settings);
        }
    }
    SqliteSettingsStorage::new(connection).get()
}

pub fn recover_interrupted(database: &Database) -> AppResult<()> {
    // Existing rows and completed analyses are preserved; only interrupted jobs resume.
    database.connection()?.execute(
        "UPDATE papers SET analysis_status='queued', analysis_error=NULL WHERE analysis_status='analyzing'",
        [],
    ).map_err(|_| crate::error::AppError::new(crate::error::ErrorCode::DatabaseWriteFailed, "无法恢复分析队列。"))?;
    Ok(())
}

pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let queue = app.state::<AnalysisQueue>();
        let _worker = queue.0.lock().await;
        if let Err(error) = drain(
            app.state::<Database>().inner(),
            app.state::<AppLogger>().inner(),
            app.state::<AIProviderRegistry>().inner(),
            app.state::<AIRequestRegistry>().inner(),
            || {
                let _ = app.emit("papers-changed", ());
            },
        )
        .await
        {
            let mut event = LogEvent::error("ai.queue", "drain", "自动分析队列读取失败");
            event.error_code = Some(error.code_string().into());
            app.state::<AppLogger>().log(event);
        }
    });
}

pub async fn drain(
    database: &Database,
    logger: &AppLogger,
    providers: &AIProviderRegistry,
    requests: &AIRequestRegistry,
    changed: impl Fn(),
) -> AppResult<()> {
    loop {
        let pending = {
            let connection = database.connection()?;
            let mut papers = SqlitePaperRepository::new(&connection).list(&PaperQuery::default())?;
            papers.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
            papers
                .into_iter()
                .find(|paper| paper.analysis_status == "queued")
        };
        let Some(paper) = pending else {
            return Ok(());
        };
        let request = AnalysisRequest {
            request_id: format!("analysis-auto-{}", paper.id),
            paper_id: paper.id,
            ensure_only: true,
        };
        let _ = run_analysis(database, logger, providers, requests, &request, true).await;
        changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_batch_keeps_its_model_when_interactive_settings_change() {
        let database = Database::in_memory().unwrap();
        {
            let c = database.connection().unwrap();
            SqliteSettingsStorage::new(&c)
                .save(&AppSettings {
                    ai_provider: "lmstudio".into(),
                    model: "local-pinned-model".into(),
                    ..Default::default()
                })
                .unwrap();
            SqlitePaperRepository::new(&c)
                .create(&paper("p", "not_analyzed"), "Original source")
                .unwrap();
        }
        enqueue_missing(&database, &["p".into()]).unwrap();
        let c = database.connection().unwrap();
        SqliteSettingsStorage::new(&c)
            .save(&AppSettings {
                ai_provider: "codex".into(),
                privacy_mode: "codex_online".into(),
                ..Default::default()
            })
            .unwrap();
        let queued = settings_for(&c, "p", true).unwrap();
        assert_eq!(queued.effective_provider(), "lmstudio");
        assert_eq!(queued.model, "local-pinned-model");
        assert_eq!(queued.privacy_mode, "local_only");
        assert_eq!(
            settings_for(&c, "p", false).unwrap().effective_provider(),
            "codex"
        );
    }
    #[test]
    fn queues_extracts_once_skips_complete_models_and_reports_unreadable_sources() {
        let database = Database::in_memory().unwrap();
        {
            let c = database.connection().unwrap();
            let repo = SqlitePaperRepository::new(&c);
            for id in ["extract", "complete", "empty"] {
                repo.create(
                    &paper(id, "not_analyzed"),
                    if id == "empty" {
                        ""
                    } else {
                        "Original wireless evidence"
                    },
                )
                .unwrap();
            }
            let value:crate::domain::paper::PaperAnalysisResult=serde_json::from_value(serde_json::json!({"schema_version":"2","ai_summary":"研究概括","research_problem":"研究问题","research_scenario":"研究场景","core_method":"核心方法","key_innovations":["创新"],"key_contributions":["贡献"],"main_results":"主要结果","limitations":"局限","tags":["无线通信","通感一体化","波束成形"]})).unwrap();
            crate::database::analysis_versions::append(
                &c,
                "extract",
                "extractive",
                "not a model",
                &value,
            )
            .unwrap();
            crate::database::analysis_versions::append(
                &c,
                "complete",
                "lmstudio",
                "real-local-model",
                &value,
            )
            .unwrap();
        }
        let ids = vec![
            "extract".into(),
            "complete".into(),
            "empty".into(),
            "extract".into(),
        ];
        let summary = enqueue_missing(&database, &ids).unwrap();
        assert_eq!(summary.queued, 1);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.no_text, 1);
        let summary = enqueue_missing(&database, &ids).unwrap();
        assert_eq!(summary.queued, 0);
        assert_eq!(summary.skipped, 2);
    }
    use crate::{
        database::settings_storage::{SettingsStorage, SqliteSettingsStorage},
        domain::{paper::Paper, settings::AppSettings},
        services::paper_import::PaperImportService,
    };

    fn paper(id: &str, status: &str) -> Paper {
        Paper {
            id: id.into(),
            title: id.into(),
            file_path: format!("{id}.pdf"),
            file_name: format!("{id}.pdf"),
            publication_venue: None,
            authors: vec![],
            year: None,
            abstract_text: None,
            analysis: None,
            analysis_provider: None,
            tags: vec![],
            analysis_status: status.into(),
            analysis_error: None,
            pdf_diagnostics: None,
            created_at: id.into(),
            updated_at: id.into(),
        }
    }

    #[tokio::test]
    async fn queue_continues_after_failure_resumes_and_does_not_repeat_completed_jobs() {
        let database = Database::in_memory().unwrap();
        let temp =
            std::env::temp_dir().join(format!("paper-master-queue-{}", uuid::Uuid::new_v4()));
        let logger = AppLogger::new(&temp).unwrap();
        {
            let connection = database.connection().unwrap();
            SqliteSettingsStorage::new(&connection)
                .save(&AppSettings {
                    ai_provider: "mock".into(),
                    ..Default::default()
                })
                .unwrap();
            let repo = SqlitePaperRepository::new(&connection);
            repo.create(&paper("1-empty", "queued"), "").unwrap();
            repo.create(&paper("2-resume", "analyzing"), "A fictional paper studies a queue-aware wireless scheduler and fair resource allocation.").unwrap();
        }
        recover_interrupted(&database).unwrap();
        let providers = AIProviderRegistry::new();
        let registry = AIRequestRegistry::new();
        drain(&database, &logger, &providers, &registry, || {})
            .await
            .unwrap();
        let completed_at = {
            let connection = database.connection().unwrap();
            let repo = SqlitePaperRepository::new(&connection);
            assert_eq!(
                repo.find_by_id("1-empty").unwrap().unwrap().analysis_status,
                "failed"
            );
            let completed = repo.find_by_id("2-resume").unwrap().unwrap();
            assert!(completed.analysis.is_some());
            assert_eq!(completed.analysis_status, "analyzed");
            completed.updated_at
        };
        drain(&database, &logger, &providers, &registry, || {})
            .await
            .unwrap();
        let connection = database.connection().unwrap();
        assert_eq!(
            SqlitePaperRepository::new(&connection)
                .find_by_id("2-resume")
                .unwrap()
                .unwrap()
                .updated_at,
            completed_at
        );
    }

    #[tokio::test]
    #[ignore = "requires PAPER_MASTER_TEST_CORPUS and the local LM Studio model; reads source PDFs only"]
    async fn live_import_auto_analysis_and_evidence_comparison() {
        let corpus = std::env::var("PAPER_MASTER_TEST_CORPUS").expect("set corpus");
        let mut paths: Vec<_> = walkdir::WalkDir::new(corpus)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
            })
            .map(|entry| entry.path().to_path_buf())
            .collect();
        paths.sort();
        let database = Database::in_memory().unwrap();
        let temp = std::env::temp_dir().join(format!("paper-master-live-{}", uuid::Uuid::new_v4()));
        let logger = AppLogger::new(&temp).unwrap();
        let settings = AppSettings {
            model: std::env::var("PAPER_MASTER_LMSTUDIO_MODEL").unwrap_or_default(),
            max_output_tokens: 4096,
            max_analysis_input_characters: 12_000,
            ..Default::default()
        };
        SqliteSettingsStorage::new(&database.connection().unwrap())
            .save(&settings)
            .unwrap();
        let mut ids = Vec::new();
        for path in paths {
            let before = std::fs::read(&path).unwrap();
            let summary = PaperImportService::new(&database, &logger)
                .import_paths(vec![path.display().to_string()], "live-import".into());
            assert_eq!(
                std::fs::read(&path).unwrap(),
                before,
                "source PDF must be unchanged"
            );
            if let Some(paper) = summary
                .imported
                .first()
                .filter(|paper| paper.analysis_status == "queued")
            {
                ids.push(paper.id.clone());
            }
            if ids.len() == 2 {
                break;
            }
        }
        assert_eq!(ids.len(), 2);
        eprintln!("LIVE_STAGE imported=2 queued=2 database=in-memory");
        let providers = AIProviderRegistry::new();
        let registry = AIRequestRegistry::new();
        drain(&database, &logger, &providers, &registry, || {
            eprintln!("LIVE_STAGE automatic_job_finished");
        })
        .await
        .unwrap();
        {
            let connection = database.connection().unwrap();
            for id in &ids {
                let paper = SqlitePaperRepository::new(&connection)
                    .find_by_id(id)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    paper.analysis_status, "analyzed",
                    "analysis error: {:?}",
                    paper.analysis_error
                );
            }
        }
        eprintln!("LIVE_STAGE automatic_analyses_validated=2");
        let result = crate::commands::ai::compare_inner(
            &database,
            &logger,
            &providers,
            &crate::domain::ai::CompareRequest {
                paper_ids: ids,
                request_id: "live-compare".into(),
            },
            tokio_util::sync::CancellationToken::new(),
        )
        .await
        .expect("real semantic comparison with verified evidence");
        assert_eq!(result.schema_version, "2");
        assert!(result.evidence.len() >= 2);
        eprintln!(
            "LIVE_STAGE comparison_validated=true differences={} verified_quotes={}",
            result.key_differences.len(),
            result.evidence.len()
        );
        let logs = std::fs::read_to_string(temp.join("logs").join("paper-master.log")).unwrap();
        assert!(
            !logs.contains(&result.evidence[0].quote),
            "paper contents must not enter logs"
        );
    }
}
