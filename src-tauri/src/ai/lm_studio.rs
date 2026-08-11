use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::{
    ai::{
        parser::{parse_analysis_response, parse_comparison_response, validate_analysis_tags},
        prompt::{
            build_analysis_user_prompt, build_comparison_user_prompt, ANALYSIS_SYSTEM_PROMPT,
            COMPARISON_SYSTEM_PROMPT, REPAIR_ANALYSIS_PROMPT, REPAIR_COMPARISON_PROMPT,
        },
        AIProvider, ProviderResponse,
    },
    config::AI_STRUCTURED_RETRY_COUNT,
    domain::{
        ai::{ConnectionResult, PaperComparisonResult},
        paper::PaperAnalysisResult,
        settings::AppSettings,
    },
    error::{AppError, AppResult, ErrorCode},
};

#[derive(Default)]
pub struct LMStudioProvider;

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelItem>,
}
#[derive(Deserialize)]
struct ModelItem {
    id: String,
}

#[derive(Deserialize)]
struct NativeModelsResponse {
    #[serde(default)]
    models: Vec<NativeModelItem>,
}

#[derive(Deserialize)]
struct NativeModelItem {
    key: String,
    #[serde(rename = "type")]
    model_type: String,
    #[serde(default)]
    loaded_instances: Vec<LoadedModelInstance>,
}

#[derive(Deserialize)]
struct LoadedModelInstance {
    id: String,
}

#[derive(Deserialize)]
struct LegacyModelsResponse {
    #[serde(default)]
    data: Vec<LegacyModelItem>,
}

#[derive(Deserialize)]
struct LegacyModelItem {
    id: String,
    #[serde(rename = "type", default)]
    model_type: String,
    #[serde(default)]
    state: String,
}

#[derive(Deserialize)]
struct LoadModelResponse {
    model_instance_id: String,
}

#[derive(Debug, Clone)]
struct ModelAvailability {
    id: String,
    loaded_instance: Option<String>,
}
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}
#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}
#[derive(Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

#[derive(Serialize)]
struct RequestMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[async_trait]
impl AIProvider for LMStudioProvider {
    fn id(&self) -> &'static str {
        "lmstudio"
    }

    async fn check_connection(&self, settings: &AppSettings) -> ConnectionResult {
        let started = Instant::now();
        match self.fetch_model_catalog(settings).await {
            Ok(catalog) if catalog.is_empty() => ConnectionResult {
                state: "model_not_loaded".into(),
                message:
                    "已连接 LM Studio，但没有找到可用的本地语言模型。请先在 LM Studio 下载模型。"
                        .into(),
                latency_ms: Some(started.elapsed().as_millis()),
                models: Vec::new(),
                loaded_models: Vec::new(),
            },
            Ok(catalog) => {
                let models = catalog
                    .iter()
                    .map(|model| model.id.clone())
                    .collect::<Vec<_>>();
                let loaded_models = catalog
                    .iter()
                    .filter_map(|model| model.loaded_instance.clone())
                    .collect::<Vec<_>>();
                ConnectionResult {
                    state: if loaded_models.is_empty() {
                        "model_not_loaded"
                    } else {
                        "connected"
                    }
                    .into(),
                    message: if loaded_models.is_empty() {
                        format!(
                            "已发现 {} 个本地模型，尚未加载。选择模型后点击“加载并测试”。",
                            models.len()
                        )
                    } else {
                        format!(
                            "LM Studio 连接正常，已加载 {} 个模型。",
                            loaded_models.len()
                        )
                    },
                    latency_ms: Some(started.elapsed().as_millis()),
                    models,
                    loaded_models,
                }
            }
            Err(error) => ConnectionResult {
                state: if matches!(error.code, ErrorCode::LmstudioConnectionFailed) {
                    "disconnected"
                } else {
                    "connection_failed"
                }
                .into(),
                message: error.message,
                latency_ms: Some(started.elapsed().as_millis()),
                models: Vec::new(),
                loaded_models: Vec::new(),
            },
        }
    }

    async fn get_models(&self, settings: &AppSettings) -> AppResult<Vec<String>> {
        let models = self
            .fetch_model_catalog(settings)
            .await?
            .into_iter()
            .map(|model| model.id)
            .collect::<Vec<_>>();
        if models.is_empty() {
            Err(AppError::new(
                ErrorCode::LmstudioModelNotFound,
                "LM Studio 已启动，但没有找到已下载的语言模型。",
            ))
        } else {
            Ok(models)
        }
    }

    async fn load_model(&self, settings: &AppSettings) -> AppResult<String> {
        self.resolve_and_load_model(settings).await
    }

    async fn analyze_paper(
        &self,
        input: &str,
        settings: &AppSettings,
        cancellation: CancellationToken,
    ) -> AppResult<ProviderResponse<PaperAnalysisResult>> {
        let model = self.resolve_model(settings).await?;
        let prompt = build_analysis_user_prompt(input);
        let (mut content, mut status) = self
            .send_chat(
                ANALYSIS_SYSTEM_PROMPT,
                &prompt,
                &model,
                settings,
                cancellation.clone(),
            )
            .await?;
        match parse_analysis_response(&content)
            .and_then(|value| validate_analysis_tags(value, input))
        {
            Ok(value) => Ok(ProviderResponse {
                value,
                http_status: Some(status),
                retry_count: 0,
            }),
            Err(first_error) => {
                for retry in 1..=AI_STRUCTURED_RETRY_COUNT {
                    let repair = format!(
                        "{REPAIR_ANALYSIS_PROMPT}\n\n论文内容：\n{input}\n\n待修复响应：\n{content}"
                    );
                    (content, status) = self
                        .send_chat(
                            REPAIR_ANALYSIS_PROMPT,
                            &repair,
                            &model,
                            settings,
                            cancellation.clone(),
                        )
                        .await?;
                    if let Ok(value) = parse_analysis_response(&content)
                        .and_then(|value| validate_analysis_tags(value, input))
                    {
                        return Ok(ProviderResponse {
                            value,
                            http_status: Some(status),
                            retry_count: retry,
                        });
                    }
                }
                Err(first_error)
            }
        }
    }

    async fn generate_tags(
        &self,
        input: &str,
        settings: &AppSettings,
        cancellation: CancellationToken,
    ) -> AppResult<Vec<String>> {
        Ok(self
            .analyze_paper(input, settings, cancellation)
            .await?
            .value
            .tags)
    }

    async fn compare_papers(
        &self,
        input: &str,
        settings: &AppSettings,
        cancellation: CancellationToken,
    ) -> AppResult<ProviderResponse<PaperComparisonResult>> {
        let model = self.resolve_model(settings).await?;
        let prompt = build_comparison_user_prompt(input);
        let (mut content, mut status) = self
            .send_chat(
                COMPARISON_SYSTEM_PROMPT,
                &prompt,
                &model,
                settings,
                cancellation.clone(),
            )
            .await?;
        match parse_comparison_response(&content) {
            Ok(value) => Ok(ProviderResponse {
                value,
                http_status: Some(status),
                retry_count: 0,
            }),
            Err(first_error) => {
                for retry in 1..=AI_STRUCTURED_RETRY_COUNT {
                    let repair = format!("{REPAIR_COMPARISON_PROMPT}\n\nRESPONSE:\n{content}");
                    (content, status) = self
                        .send_chat(
                            REPAIR_COMPARISON_PROMPT,
                            &repair,
                            &model,
                            settings,
                            cancellation.clone(),
                        )
                        .await?;
                    if let Ok(value) = parse_comparison_response(&content) {
                        return Ok(ProviderResponse {
                            value,
                            http_status: Some(status),
                            retry_count: retry,
                        });
                    }
                }
                Err(first_error)
            }
        }
    }
}

impl LMStudioProvider {
    async fn fetch_model_catalog(
        &self,
        settings: &AppSettings,
    ) -> AppResult<Vec<ModelAvailability>> {
        if let Some(models) = self.fetch_native_models(settings).await? {
            return Ok(models);
        }
        if let Some(models) = self.fetch_legacy_models(settings).await? {
            return Ok(models);
        }
        self.fetch_openai_models(settings).await
    }

    async fn fetch_native_models(
        &self,
        settings: &AppSettings,
    ) -> AppResult<Option<Vec<ModelAvailability>>> {
        let response = client(settings)?
            .get(native_endpoint(settings, "models")?)
            .send()
            .await
            .map_err(request_error)?;
        if matches!(response.status().as_u16(), 404 | 405) {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(AppError::new(
                ErrorCode::LmstudioConnectionFailed,
                "LM Studio 在读取本地模型列表时返回错误。",
            )
            .detail(format!("HTTP {}", response.status().as_u16())));
        }
        let body: NativeModelsResponse = response.json().await.map_err(|error| {
            AppError::new(
                ErrorCode::LmstudioConnectionFailed,
                "LM Studio 返回了无法识别的模型列表。",
            )
            .detail(error.to_string())
        })?;
        Ok(Some(
            body.models
                .into_iter()
                .filter(|model| model.model_type != "embedding")
                .filter(|model| !model.key.trim().is_empty())
                .map(|model| ModelAvailability {
                    id: model.key,
                    loaded_instance: model
                        .loaded_instances
                        .into_iter()
                        .next()
                        .map(|instance| instance.id),
                })
                .collect(),
        ))
    }

    async fn fetch_legacy_models(
        &self,
        settings: &AppSettings,
    ) -> AppResult<Option<Vec<ModelAvailability>>> {
        let response = client(settings)?
            .get(legacy_endpoint(settings, "models")?)
            .send()
            .await
            .map_err(request_error)?;
        if matches!(response.status().as_u16(), 404 | 405) {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Ok(None);
        }
        let body: LegacyModelsResponse = response.json().await.map_err(|error| {
            AppError::new(
                ErrorCode::LmstudioConnectionFailed,
                "LM Studio 返回了无法识别的兼容模型列表。",
            )
            .detail(error.to_string())
        })?;
        Ok(Some(
            body.data
                .into_iter()
                .filter(|model| !matches!(model.model_type.as_str(), "embeddings" | "embedding"))
                .filter(|model| !model.id.trim().is_empty())
                .map(|model| ModelAvailability {
                    loaded_instance: (model.state == "loaded").then(|| model.id.clone()),
                    id: model.id,
                })
                .collect(),
        ))
    }

    async fn fetch_openai_models(
        &self,
        settings: &AppSettings,
    ) -> AppResult<Vec<ModelAvailability>> {
        let client = client(settings)?;
        let url = endpoint(settings, "models")?;
        let response = client.get(url).send().await.map_err(request_error)?;
        if !response.status().is_success() {
            return Err(AppError::new(
                ErrorCode::LmstudioConnectionFailed,
                "LM Studio 在读取兼容模型列表时返回错误。",
            )
            .detail(format!("HTTP {}", response.status().as_u16())));
        }
        let body: ModelsResponse = response.json().await.map_err(|error| {
            AppError::new(
                ErrorCode::LmstudioConnectionFailed,
                "LM Studio 返回了无法识别的兼容模型列表。",
            )
            .detail(error.to_string())
        })?;
        Ok(body
            .data
            .into_iter()
            .filter(|model| !model.id.trim().is_empty())
            .map(|model| ModelAvailability {
                loaded_instance: Some(model.id.clone()),
                id: model.id,
            })
            .collect())
    }

    async fn resolve_model(&self, settings: &AppSettings) -> AppResult<String> {
        self.resolve_and_load_model(settings).await
    }

    async fn resolve_and_load_model(&self, settings: &AppSettings) -> AppResult<String> {
        let catalog = self.fetch_model_catalog(settings).await?;
        if catalog.is_empty() {
            return Err(AppError::new(
                ErrorCode::LmstudioModelNotFound,
                "LM Studio 中没有可用的本地语言模型。",
            ));
        }
        let selected = if settings.model.trim().is_empty() {
            catalog
                .iter()
                .find(|model| model.loaded_instance.is_some())
                .unwrap_or(&catalog[0])
        } else {
            catalog
                .iter()
                .find(|model| {
                    model.id == settings.model
                        || model.loaded_instance.as_deref() == Some(settings.model.as_str())
                })
                .ok_or_else(|| {
                    AppError::new(
                        ErrorCode::LmstudioModelNotFound,
                        "设置中选择的模型在 LM Studio 中不存在，请刷新模型列表后重新选择。",
                    )
                    .detail(format!("Selected model: {}", settings.model))
                })?
        };
        if let Some(instance) = &selected.loaded_instance {
            return Ok(instance.clone());
        }
        Ok(self
            .load_native_model(settings, &selected.id)
            .await?
            .unwrap_or_else(|| selected.id.clone()))
    }

    async fn load_native_model(
        &self,
        settings: &AppSettings,
        model: &str,
    ) -> AppResult<Option<String>> {
        let response = client(settings)?
            .post(native_endpoint(settings, "models/load")?)
            .json(&serde_json::json!({ "model": model }))
            .send()
            .await
            .map_err(request_error)?;
        if matches!(response.status().as_u16(), 404 | 405) {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(AppError::new(
                ErrorCode::LmstudioModelNotFound,
                "LM Studio 无法加载所选模型，请检查内存/显存和模型文件。",
            )
            .detail(format!("HTTP {}", response.status().as_u16())));
        }
        let body: LoadModelResponse = response.json().await.map_err(|error| {
            AppError::new(
                ErrorCode::LmstudioConnectionFailed,
                "LM Studio 返回了无法识别的模型加载结果。",
            )
            .detail(error.to_string())
        })?;
        Ok(Some(body.model_instance_id))
    }

    async fn send_chat(
        &self,
        system: &str,
        user: &str,
        model: &str,
        settings: &AppSettings,
        cancellation: CancellationToken,
    ) -> AppResult<(String, u16)> {
        let client = client(settings)?;
        let url = endpoint(settings, "chat/completions")?;
        let body = serde_json::json!({
            "model": model,
            "messages": [RequestMessage { role: "system", content: system }, RequestMessage { role: "user", content: user }],
            "temperature": settings.temperature,
            "stream": false,
            "response_format": { "type": "json_object" }
        });
        let request = client.post(url).json(&body).send();
        let response = tokio::select! {
            _ = cancellation.cancelled() => return Err(AppError::new(ErrorCode::AiCancelled, "AI analysis was cancelled.")),
            value = request => value.map_err(request_error)?,
        };
        let status = response.status().as_u16();
        if !response.status().is_success() {
            return Err(AppError::new(
                ErrorCode::LmstudioConnectionFailed,
                "LM Studio rejected the analysis request.",
            )
            .detail(format!("HTTP {status}")));
        }
        let body: ChatResponse = response.json().await.map_err(|error| {
            AppError::new(
                ErrorCode::AiInvalidJson,
                "LM Studio returned an invalid completion envelope.",
            )
            .detail(error.to_string())
        })?;
        let content = body
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::AiInvalidJson,
                    "LM Studio returned an empty final response.",
                )
            })?;
        Ok((content, status))
    }
}

fn client(settings: &AppSettings) -> AppResult<Client> {
    Ok(Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_millis(settings.request_timeout_ms))
        .user_agent("paper-master/0.1")
        .build()
        .map_err(|error| {
            AppError::new(
                ErrorCode::Internal,
                "The local AI client could not be initialized.",
            )
            .detail(error.to_string())
        })?)
}

fn endpoint(settings: &AppSettings, suffix: &str) -> AppResult<url::Url> {
    let mut base = local_base_url(settings)?;
    if !base.path().ends_with('/') {
        base.set_path(&format!("{}/", base.path()));
    }
    base.join(suffix).map_err(|error| {
        AppError::new(
            ErrorCode::InvalidSettings,
            "LM Studio endpoint could not be constructed.",
        )
        .detail(error.to_string())
    })
}

fn native_endpoint(settings: &AppSettings, suffix: &str) -> AppResult<url::Url> {
    management_endpoint(settings, "api/v1", suffix)
}

fn legacy_endpoint(settings: &AppSettings, suffix: &str) -> AppResult<url::Url> {
    management_endpoint(settings, "api/v0", suffix)
}

fn management_endpoint(
    settings: &AppSettings,
    api_path: &str,
    suffix: &str,
) -> AppResult<url::Url> {
    let mut base = local_base_url(settings)?;
    base.set_path(&format!("/{api_path}/"));
    base.set_query(None);
    base.set_fragment(None);
    base.join(suffix).map_err(|error| {
        AppError::new(
            ErrorCode::InvalidSettings,
            "LM Studio management endpoint could not be constructed.",
        )
        .detail(error.to_string())
    })
}

fn local_base_url(settings: &AppSettings) -> AppResult<url::Url> {
    let base = url::Url::parse(&settings.base_url).map_err(|error| {
        AppError::new(ErrorCode::InvalidSettings, "LM Studio Base URL is invalid.")
            .detail(error.to_string())
    })?;
    if base.scheme() != "http" || !matches!(base.host_str(), Some("localhost") | Some("127.0.0.1"))
    {
        return Err(AppError::new(
            ErrorCode::InvalidSettings,
            "Local Only mode permits AI requests only to localhost or 127.0.0.1.",
        ));
    }
    Ok(base)
}

fn request_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        AppError::new(ErrorCode::AiTimeout, "The local AI request timed out. Increase the timeout or retry with a shorter paper input.").detail(error.to_string())
    } else {
        AppError::new(
            ErrorCode::LmstudioConnectionFailed,
            "Cannot connect to LM Studio. Please start the LM Studio local server.",
        )
        .detail(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    #[test]
    fn only_loopback_endpoints_are_allowed() {
        let local = AppSettings::default();
        assert!(endpoint(&local, "models")
            .expect("local")
            .as_str()
            .ends_with("/v1/models"));
        assert!(native_endpoint(&local, "models/load")
            .expect("native")
            .as_str()
            .ends_with("/api/v1/models/load"));
        let external = AppSettings {
            base_url: "http://example.com/v1".into(),
            ..Default::default()
        };
        assert_eq!(
            endpoint(&external, "models")
                .expect_err("external")
                .code_string(),
            "INVALID_SETTINGS"
        );
    }

    #[tokio::test]
    async fn discovers_and_explicitly_loads_a_native_lm_studio_model() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("address");
        let server = thread::spawn(move || {
            for index in 0..2 {
                let (mut stream, _) = listener.accept().expect("request");
                let request = read_request(&mut stream);
                let body = if index == 0 {
                    assert!(request.starts_with("GET /api/v1/models "));
                    r#"{"models":[{"type":"llm","key":"qwen/test","loaded_instances":[]},{"type":"embedding","key":"embed/test","loaded_instances":[]}]}"#
                } else {
                    assert!(request.starts_with("POST /api/v1/models/load "));
                    assert!(request.contains(r#""model":"qwen/test""#));
                    r#"{"type":"llm","model_instance_id":"qwen/test","load_time_seconds":0.1,"status":"loaded"}"#
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(response.as_bytes()).expect("response");
            }
        });
        let settings = AppSettings {
            base_url: format!("http://{address}/v1"),
            model: "qwen/test".into(),
            ..Default::default()
        };
        let loaded = LMStudioProvider
            .load_model(&settings)
            .await
            .expect("loaded model");
        assert_eq!(loaded, "qwen/test");
        server.join().expect("server");
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 2048];
        loop {
            let read = stream.read(&mut buffer).expect("read");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            let text = String::from_utf8_lossy(&request);
            let Some(header_end) = text.find("\r\n\r\n") else {
                continue;
            };
            let content_length = text[..header_end]
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|value| value.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            if request.len() >= header_end + 4 + content_length {
                break;
            }
        }
        String::from_utf8(request).expect("utf8 request")
    }
}
