use crate::{
    database::{
        paper_repository::{PaperRepository, SqlitePaperRepository},
        Database,
    },
    diagnostics::AppLogger,
    error::AppResult,
};
use std::path::PathBuf;

pub fn execute(args: &[String]) -> AppResult<()> {
    let folder = PathBuf::from(&args[1]);
    let data_dir = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("app.papermaster.desktop");
    std::fs::create_dir_all(&data_dir).map_err(|_| {
        crate::error::AppError::new(
            crate::error::ErrorCode::DatabaseWriteFailed,
            "无法创建本地数据目录。",
        )
    })?;
    // Back up the existing SQLite database with SQLite itself, including committed WAL data.
    let path = data_dir.join("paper-master.db");
    if path.is_file() {
        let conn = rusqlite::Connection::open(&path).map_err(|_| {
            crate::error::AppError::new(
                crate::error::ErrorCode::DatabaseReadFailed,
                "无法备份原数据库。",
            )
        })?;
        let backup = data_dir.join(format!(
            "paper-master-before-v0.4.0-{}.db",
            chrono::Utc::now().format("%Y%m%dT%H%M%SZ")
        ));
        conn.execute("VACUUM INTO ?1", [backup.to_string_lossy().as_ref()])
            .map_err(|_| {
                crate::error::AppError::new(
                    crate::error::ErrorCode::DatabaseWriteFailed,
                    "数据库备份失败；未开始归档。",
                )
            })?;
    }
    let logger = AppLogger::new(&data_dir).map_err(|_| {
        crate::error::AppError::new(
            crate::error::ErrorCode::DatabaseWriteFailed,
            "无法创建本地诊断目录。",
        )
    })?;
    let database = Database::open(&path, &logger)?;
    let summary = crate::services::paper_import::PaperImportService::new(&database, &logger)
        .import_observed(
            vec![folder.to_string_lossy().into()],
            format!("archive-{}", uuid::Uuid::new_v4()),
            |done, total, _| {
                if done % 25 == 0 || done == total {
                    eprintln!("ARCHIVE_PROGRESS {done}/{total}");
                }
            },
        );
    let all_ids = {
        let conn = database.connection()?;
        SqlitePaperRepository::new(&conn)
            .list(&Default::default())?
            .into_iter()
            .map(|p| p.id)
            .collect::<Vec<_>>()
    };
    for id in all_ids {
        crate::services::archive::repair_metadata(&database, &id)?;
    }
    let papers = {
        let conn = database.connection()?;
        SqlitePaperRepository::new(&conn).list(&Default::default())?
    };
    let mut archived = 0;
    let mut no_text = 0;
    let mut entries = Vec::new();
    for paper in &papers {
        if !PathBuf::from(&paper.file_path).starts_with(&folder) {
            continue;
        }
        let version = crate::services::archive::extract(&database, &paper.id)?;
        archived += 1;
        let empty = paper
            .pdf_diagnostics
            .as_ref()
            .is_none_or(|d| d.extracted_character_count == 0);
        if empty {
            no_text += 1;
        }
        entries.push(serde_json::json!({"id":paper.id,"file":paper.file_path,"title":paper.title,"authors":paper.authors,"tags":version.result.tags,"needs_text_review":empty,"diagnostics":paper.pdf_diagnostics}));
    }
    let duplicate = crate::services::paper_import::PaperImportService::new(&database, &logger)
        .import_paths(
            vec![folder.to_string_lossy().into()],
            "archive-reimport-check".into(),
        );
    let library = {
        let conn = database.connection()?;
        let repo = SqlitePaperRepository::new(&conn);
        papers
            .iter()
            .map(|p| {
                Ok((
                    p.clone(),
                    repo.extracted_text(&p.id)?.chars().take(16000).collect(),
                ))
            })
            .collect::<AppResult<Vec<_>>>()?
    };
    let team_links = crate::services::authors::team_links(&library);
    let report = serde_json::json!({"potential_team_links":team_links,"version":"0.4.0","source":folder,"data_directory":data_dir,"archived":archived,"new_imports":summary.imported.len(),"existing":summary.duplicates.len(),"failures":summary.failed,"needs_text_review":no_text,"reimport_duplicates":duplicate.duplicates.len(),"reimport_new":duplicate.imported.len(),"integrity":database.integrity_check()?,"entries":entries});
    let report_path = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("archive-report.json"));
    if let Some(parent) = report_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&report_path, serde_json::to_vec_pretty(&report).unwrap()).map_err(|_| {
        crate::error::AppError::new(
            crate::error::ErrorCode::DatabaseWriteFailed,
            "归档完成，但报告未保存。",
        )
    })?;
    eprintln!(
        "ARCHIVE_RESULT archived={archived} no_text={no_text} integrity=ok report={}",
        report_path.display()
    );
    Ok(())
}

pub fn analyze_one(id: &str) -> AppResult<()> {
    let directory = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("app.papermaster.desktop");
    let logger = AppLogger::new(&directory).map_err(|_| {
        crate::error::AppError::new(
            crate::error::ErrorCode::DatabaseWriteFailed,
            "日志目录不可用。",
        )
    })?;
    let database = Database::open(&directory.join("paper-master.db"), &logger)?;
    let providers = crate::ai::AIProviderRegistry::new();
    let requests = crate::ai::registry::AIRequestRegistry::new();
    let request = crate::domain::ai::AnalysisRequest {
        paper_id: id.into(),
        request_id: format!("sample-{}", uuid::Uuid::new_v4()),
        ensure_only: false,
    };
    tauri::async_runtime::block_on(crate::commands::ai::run_analysis(
        &database, &logger, &providers, &requests, &request, false,
    ))?;
    eprintln!("MODEL_SAMPLE_SAVED paper_id={id}");
    Ok(())
}
