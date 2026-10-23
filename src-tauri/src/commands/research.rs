use crate::services::progress::{emit as emit_progress, TaskProgress};
use crate::{
    ai::registry::AIRequestRegistry,
    config::AppPaths,
    database::{
        paper_repository::{PaperRepository, SqlitePaperRepository},
        settings_storage::{SettingsStorage, SqliteSettingsStorage},
        Database,
    },
    diagnostics::AppLogger,
    domain::paper::ImportSummary,
    error::{AppError, AppResult, ErrorCode},
    services::{
        paper_import::PaperImportService,
        research::{self, ResearchResult},
    },
};
use tauri::{webview::DownloadEvent, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

fn require_online(database: &Database) -> AppResult<()> {
    let connection = database.connection()?;
    if SqliteSettingsStorage::new(&connection).get()?.privacy_mode != "codex_online" {
        return Err(error("联网已关闭，请在设置中启用联网功能。"));
    }
    Ok(())
}

fn error(message: &str) -> AppError {
    AppError::new(ErrorCode::ResearchFailed, message)
}

#[tauri::command]
pub async fn search_online_papers(
    database: State<'_, Database>,
    requests: State<'_, AIRequestRegistry>,
    request: String,
    reference_paper_id: Option<String>,
    limit: usize,
    request_id: String,
) -> AppResult<ResearchResult> {
    let (settings, reference) = {
        let connection = database.connection()?;
        let settings = SqliteSettingsStorage::new(&connection).get()?;
        let reference = if let Some(id) = reference_paper_id {
            let repository = SqlitePaperRepository::new(&connection);
            let paper = repository
                .find_by_id(&id)?
                .ok_or_else(|| error("参考论文已从库中移除，请重新选择。"))?;
            let text = repository.extracted_text(&id)?;
            let corresponding =
                crate::services::authors::corresponding_authors(&paper.authors, &text).0;
            Some(research::SearchReference {
                title: paper.title.chars().take(500).collect(),
                authors: paper.authors.into_iter().take(30).collect(),
                corresponding_authors: corresponding,
            })
        } else {
            None
        };
        (settings, reference)
    };
    if settings.privacy_mode != "codex_online" {
        return Err(error(
            "请先在设置中启用 Codex 联网授权。检索需求及所选参考论文的题名/作者将用于理解需求和联网检索。",
        ));
    }
    let token = requests.register(&request_id);
    requests.report(TaskProgress::running(
        &request_id,
        "research",
        "等待检索资源",
    ));
    let result = async {
        let _permit = requests.acquire(&token).await?;
        require_online(&database)?;
        tokio::time::timeout(
            std::time::Duration::from_millis(settings.request_timeout_ms),
            research::search_observed(
                &request,
                reference.as_ref(),
                limit,
                settings.request_timeout_ms,
                token.clone(),
                |stage, done| {
                    requests.report(
                        TaskProgress::running(&request_id, "research", stage).counts(
                            done,
                            Some(3),
                            "steps",
                        ),
                    )
                },
            ),
        )
        .await
        .map_err(|_| {
            AppError::new(
                ErrorCode::AiTimeout,
                "理解需求和检索的总用时超过设置限制，请精简条件或重试。",
            )
        })?
    }
    .await;
    requests.remove(&request_id);
    match &result {
        Ok(value) => requests.report(
            TaskProgress::running(
                &request_id,
                "research",
                if value.intent.needs_clarification {
                    "请补充参考信息"
                } else {
                    "检索与来源校验完成"
                },
            )
            .counts(3, Some(3), "steps")
            .finish("completed"),
        ),
        Err(error) => requests.report(
            TaskProgress::running(&request_id, "research", &error.message).finish(
                if matches!(error.code, ErrorCode::AiCancelled) {
                    "cancelled"
                } else {
                    "failed"
                },
            ),
        ),
    }
    result.map_err(|error| error.request_id(request_id))
}

#[tauri::command]
pub async fn download_online_paper(
    app: tauri::AppHandle,
    paths: State<'_, AppPaths>,
    pdf_url: String,
    request_id: String,
    database: State<'_, Database>,
    requests: State<'_, AIRequestRegistry>,
) -> AppResult<ImportSummary> {
    require_online(&database)?;
    let token = requests.register(&request_id);
    if let Err(error) = require_online(&database) {
        requests.remove(&request_id);
        return Err(error);
    }
    emit_progress(
        &app,
        TaskProgress::running(&request_id, "download", "正在连接全文来源"),
    );
    let result = async {
        let directory = paths.data_dir.join("downloads");
        let file = tokio::select! {
            biased;
            _ = token.cancelled() => return Err(error("联网关闭或下载已取消。")),
            downloaded = research::download_observed(
            &pdf_url,
            &directory,
            |done, total| {
                emit_progress(
                    &app,
                    TaskProgress::running(&request_id, "download", "正在接收 PDF")
                        .counts(done, total, "bytes"),
                )
            },
        ) => downloaded?,
        };
        require_online(&database)?;
        emit_progress(
            &app,
            TaskProgress::running(&request_id, "download", "PDF 已下载并校验，正在导入论文库"),
        );
        import_download(app.clone(), file).await
    }
    .await;
    match &result {
        Ok(summary) => emit_progress(
            &app,
            TaskProgress::running(
                &request_id,
                "download",
                if summary.failed.is_empty() {
                    "下载与导入完成"
                } else {
                    "文件已下载，部分导入失败"
                },
            )
            .finish(if summary.failed.is_empty() {
                "completed"
            } else {
                "failed"
            }),
        ),
        Err(error) => emit_progress(
            &app,
            TaskProgress::running(&request_id, "download", &error.message).finish("failed"),
        ),
    }
    requests.remove(&request_id);
    result
}

async fn import_download(app: tauri::AppHandle, file: String) -> AppResult<ImportSummary> {
    let worker = app.clone();
    let summary = tauri::async_runtime::spawn_blocking(move || {
        let database = worker.state::<Database>();
        let logger = worker.state::<AppLogger>();
        PaperImportService::new(database.inner(), logger.inner())
            .import_paths(vec![file], format!("download-{}", uuid::Uuid::new_v4()))
    })
    .await
    .map_err(|_| error("PDF 已保存，但导入任务失败，请手动导入下载目录。"))?;
    crate::services::analysis_queue::start(app);
    Ok(summary)
}

#[tauri::command]
pub async fn open_ieee_webvpn(
    app: tauri::AppHandle,
    paths: State<'_, AppPaths>,
    ieee_url: Option<String>,
    database: State<'_, Database>,
) -> AppResult<()> {
    require_online(&database)?;
    let target = match ieee_url {
        Some(value) => research::vpn_url(&value)?,
        None => format!("{}/Xplore/home.jsp", research::IEEE_VPN_ROOT),
    };
    let url = url::Url::parse(&target).map_err(|_| error("WebVPN 地址无效。"))?;
    if let Some(window) = app.get_webview_window("ieee-webvpn") {
        window
            .navigate(url)
            .map_err(|_| error("无法导航到 IEEE 页面。"))?;
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }
    let directory = paths.data_dir.join("downloads");
    std::fs::create_dir_all(&directory).map_err(|_| error("无法创建论文下载目录。"))?;
    let download_directory = directory.clone();
    let popup_app = app.clone();
    let profile = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| error("无法定位本机私有会话目录。"))?
        .join("paper-master-private/webvpn-browser");
    std::fs::create_dir_all(&profile).map_err(|_| error("无法创建 WebVPN 私有会话目录。"))?;
    let pending_downloads = std::sync::Mutex::new(std::collections::HashMap::<
        String,
        std::collections::VecDeque<String>,
    >::new());
    // This external window has no Tauri IPC capability. Credentials/cookies stay in its browser profile.
    WebviewWindowBuilder::new(&app, "ieee-webvpn", WebviewUrl::External(url))
        .title("IEEE · 南京邮电大学 WebVPN（试验下载窗口）")
        .inner_size(1200.0, 850.0)
        .data_directory(profile)
        .on_navigation({
            let navigation_app = app.clone();
            move |url| {
                let allowed = require_online(navigation_app.state::<Database>().inner()).is_ok();
                allowed
                    && url.scheme() == "https"
                    && url.host_str() == Some("vpn.njupt.edu.cn")
                    && url.port() == Some(8443)
            }
        })
        .on_new_window(move |url, _| {
            // IEEE PDF links often use target=_blank. Keep them in the same controlled window
            // so the download callback and authenticated browser session are retained.
            if url.scheme() == "https"
                && url.host_str() == Some("vpn.njupt.edu.cn")
                && url.port() == Some(8443)
            {
                if let Some(window) = popup_app.get_webview_window("ieee-webvpn") {
                    let _ = window.navigate(url);
                }
            }
            tauri::webview::NewWindowResponse::Deny
        })
        .on_download(move |webview, event| {
            match event {
                DownloadEvent::Requested { url, destination } => {
                    if require_online(webview.app_handle().state::<Database>().inner()).is_err() {
                        return false;
                    }
                    if url.scheme() != "https"
                        || url.host_str() != Some("vpn.njupt.edu.cn")
                        || url.port() != Some(8443)
                    {
                        return false;
                    }
                    *destination =
                        download_directory.join(format!("ieee-{}.pdf", uuid::Uuid::new_v4()));
                    let id = destination
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    if let Ok(mut pending) = pending_downloads.lock() {
                        pending
                            .entry(url.to_string())
                            .or_default()
                            .push_back(id.clone());
                    }
                    emit_progress(
                        webview.app_handle(),
                        TaskProgress::running(
                            &id,
                            "download",
                            "学校浏览器正在下载（无字节进度接口）",
                        ),
                    );
                }
                DownloadEvent::Finished {
                    url,
                    path: Some(path),
                    success: true,
                    ..
                } => {
                    if let Ok(mut pending) = pending_downloads.lock() {
                        if let Some(ids) = pending.get_mut(url.as_str()) {
                            ids.pop_front();
                        }
                    }
                    if path.parent() != Some(download_directory.as_path()) {
                        return false;
                    }
                    let app = webview.app_handle().clone();
                    let id = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    emit_progress(
                        &app,
                        TaskProgress::running(&id, "download", "学校 PDF 下载结束，正在校验与导入"),
                    );
                    tauri::async_runtime::spawn(async move {
                        use std::io::Read;
                        let mut magic = [0u8; 5];
                        let valid = std::fs::metadata(&path)
                            .is_ok_and(|meta| meta.len() <= 80 * 1024 * 1024)
                            && std::fs::File::open(&path)
                                .and_then(|mut file| file.read_exact(&mut magic))
                                .is_ok()
                            && &magic == b"%PDF-";
                        if valid {
                            match import_download(app.clone(), path.to_string_lossy().to_string())
                                .await
                            {
                                Ok(summary) if summary.failed.is_empty() => {
                                    emit_progress(
                                        &app,
                                        TaskProgress::running(
                                            &id,
                                            "download",
                                            "学校 PDF 下载并导入完成",
                                        )
                                        .finish("completed"),
                                    );
                                    let _ = app.emit_to("main", "research-download", summary);
                                }
                                Ok(_) => {
                                    emit_progress(
                                        &app,
                                        TaskProgress::running(
                                            &id,
                                            "download",
                                            "PDF 已下载，导入失败",
                                        )
                                        .finish("failed"),
                                    );
                                    let _ = app.emit_to(
                                        "main",
                                        "research-download-error",
                                        "PDF 已保存，但导入失败，请手动导入下载目录。 ",
                                    );
                                }
                                Err(_) => {
                                    emit_progress(
                                        &app,
                                        TaskProgress::running(
                                            &id,
                                            "download",
                                            "PDF 已下载，导入失败",
                                        )
                                        .finish("failed"),
                                    );
                                    let _ = app.emit_to(
                                        "main",
                                        "research-download-error",
                                        "PDF 已保存，但导入失败，请手动导入。 ",
                                    );
                                }
                            }
                        } else {
                            emit_progress(
                                &app,
                                TaskProgress::running(
                                    &id,
                                    "download",
                                    "下载内容不是有效 PDF 或超出大小限制",
                                )
                                .finish("failed"),
                            );
                            let _ = app.emit_to(
                                "main",
                                "research-download-error",
                                "下载内容不是有效 PDF 或超过 80 MB，未导入。 ",
                            );
                        }
                    });
                }
                DownloadEvent::Finished {
                    url,
                    path,
                    success: false,
                } => {
                    let tracked = pending_downloads.lock().ok().and_then(|mut pending| {
                        pending
                            .get_mut(url.as_str())
                            .and_then(|ids| ids.pop_front())
                    });
                    let id = tracked.or_else(|| {
                        path.as_ref()
                            .filter(|path| path.parent() == Some(download_directory.as_path()))
                            .and_then(|path| path.file_stem())
                            .map(|stem| stem.to_string_lossy().to_string())
                    });
                    if let Some(id) = id {
                        emit_progress(
                            webview.app_handle(),
                            TaskProgress::running(&id, "download", "学校浏览器下载已取消或中断")
                                .finish("failed"),
                        );
                        let _ = webview.app_handle().emit_to(
                            "main",
                            "research-download-error",
                            "学校浏览器下载已取消或中断，请检查登录状态后重试。",
                        );
                    }
                }
                _ => {}
            }
            true
        })
        .build()
        .map_err(|_| error("无法打开 WebVPN 窗口。请使用系统浏览器访问学校 IEEE 入口。"))?;
    Ok(())
}

#[tauri::command]
pub fn open_ieee_external(
    app: tauri::AppHandle,
    database: State<'_, Database>,
    ieee_url: Option<String>,
) -> AppResult<()> {
    require_online(&database)?;
    let target = match ieee_url {
        Some(value) => research::vpn_url(&value)?,
        None => format!("{}/Xplore/home.jsp", research::IEEE_VPN_ROOT),
    };
    app.opener()
        .open_url(target, None::<&str>)
        .map_err(|_| error("无法启动系统浏览器。"))
}

#[tauri::command]
pub fn open_relationship_source(
    app: tauri::AppHandle,
    database: State<'_, Database>,
    url: String,
) -> AppResult<()> {
    require_online(&database)?;
    let url = research::relationship_source_url(&url)?;
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|_| error("无法打开作者关系来源。"))
}

#[tauri::command]
pub fn open_webvpn_credentials(app: tauri::AppHandle) -> AppResult<()> {
    let path = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| error("无法定位当前用户私有目录。"))?
        .join("paper-master-private/webvpn.txt");
    if !path.is_file() {
        return Err(error("本机未配置凭据文件。请在 %LOCALAPPDATA%\\paper-master-private\\webvpn.txt 中单独配置，不要放进工程或迁移包。"));
    }
    // Opens the private file locally; its contents never cross IPC or enter an AI prompt.
    app.opener()
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|_| error("无法打开私有凭据 TXT。"))
}

#[cfg(test)]
mod network_tests {
    use super::*;
    #[test]
    fn network_gate_follows_persisted_switch() {
        let database = Database::in_memory().unwrap();
        assert!(require_online(&database).is_err());
        let mut settings = crate::domain::settings::AppSettings::default();
        settings.privacy_mode = "codex_online".into();
        SqliteSettingsStorage::new(&database.connection().unwrap())
            .save(&settings)
            .unwrap();
        assert!(require_online(&database).is_ok());
        settings.privacy_mode = "local_only".into();
        SqliteSettingsStorage::new(&database.connection().unwrap())
            .save(&settings)
            .unwrap();
        assert!(require_online(&database).is_err());
    }
}
