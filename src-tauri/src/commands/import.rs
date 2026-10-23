use tauri::{AppHandle, Manager};

use crate::{
    database::Database, diagnostics::AppLogger, domain::paper::ImportSummary,
    services::paper_import::PaperImportService,
};

#[tauri::command]
pub async fn import_papers(
    app: AppHandle,
    paths: Vec<String>,
    operation_id: String,
) -> Result<ImportSummary, String> {
    let worker = app.clone();
    let summary =
        tauri::async_runtime::spawn_blocking(move || {
            let database = worker.state::<Database>();
            let logger = worker.state::<AppLogger>();
            let summary = PaperImportService::new(database.inner(), logger.inner())
                .import_observed(paths, operation_id.clone(), |done, total, stage| {
                    crate::services::progress::emit(
                        &worker,
                        crate::services::progress::TaskProgress::running(
                            &operation_id,
                            "import",
                            stage,
                        )
                        .counts(done as u64, Some(total as u64), "files"),
                    );
                });
            let stage = format!(
                "导入 {} 篇，跳过 {} 篇，失败 {} 项",
                summary.imported.len(),
                summary.duplicates.len(),
                summary.failed.len()
            );
            crate::services::progress::emit(
                &worker,
                crate::services::progress::TaskProgress::running(&operation_id, "import", &stage)
                    .finish(
                        if summary.imported.is_empty() && !summary.failed.is_empty() {
                            "failed"
                        } else {
                            "completed"
                        },
                    ),
            );
            summary
        })
        .await
        .map_err(|_| "导入任务意外退出，请重试。".to_string())?;
    crate::services::analysis_queue::start(app);
    Ok(summary)
}
