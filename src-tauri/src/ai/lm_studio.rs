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

#[derive(Clone, Copy)]
enum LmStudioApi {
    OpenAi,
    NativeV1,
    LegacyV0,
}

struct LmStudioEndpointBuilder {
    origin: url::Url,
}

#[derive(Debug)]
struct ChatCompletion {
    content: String,
    http_status: u16,
    endpoint: String,
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
        let mut completion = self
            .send_chat(
                ANALYSIS_SYSTEM_PROMPT,
                &prompt,
                &model,
                settings,
                cancellation.clone(),
            )
            .await?;
        let mut content = completion.content;
        match parse_analysis_response(&content)
            .and_then(|value| validate_analysis_tags(value, input))
        {
            Ok(value) => Ok(ProviderResponse {
                value,
                http_status: Some(completion.http_status),
                retry_count: 0,
                method: Some("POST".into()),
                endpoint: Some(completion.endpoint),
                model: Some(model.clone()),
            }),
            Err(first_error) => {
                for retry in 1..=AI_STRUCTURED_RETRY_COUNT {
                    let repair = format!(
                        "{REPAIR_ANALYSIS_PROMPT}\n\n论文内容：\n{input}\n\n待修复响应：\n{content}"
                    );
                    completion = self
                        .send_chat(
                            REPAIR_ANALYSIS_PROMPT,
                            &repair,
                            &model,
                            settings,
                            cancellation.clone(),
                        )
                        .await?;
                    content = completion.content;
                    if let Ok(value) = parse_analysis_response(&content)
                        .and_then(|value| validate_analysis_tags(value, input))
                    {
                        return Ok(ProviderResponse {
                            value,
                            http_status: Some(completion.http_status),
                            retry_count: retry,
                            method: Some("POST".into()),
                            endpoint: Some(completion.endpoint),
                            model: Some(model.clone()),
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
        let mut completion = self
            .send_chat(
                COMPARISON_SYSTEM_PROMPT,
                &prompt,
                &model,
                settings,
                cancellation.clone(),
            )
            .await?;
        let mut content = completion.content;
        match parse_comparison_response(&content) {
            Ok(value) => Ok(ProviderResponse {
                value,
                http_status: Some(completion.http_status),
                retry_count: 0,
                method: Some("POST".into()),
                endpoint: Some(completion.endpoint),
                model: Some(model.clone()),
            }),
            Err(first_error) => {
                for retry in 1..=AI_STRUCTURED_RETRY_COUNT {
                    let repair = format!("{REPAIR_COMPARISON_PROMPT}\n\nRESPONSE:\n{content}");
                    completion = self
                        .send_chat(
                            REPAIR_COMPARISON_PROMPT,
                            &repair,
                            &model,
                            settings,
                            cancellation.clone(),
                        )
                        .await?;
                    content = completion.content;
                    if let Ok(value) = parse_comparison_response(&content) {
                        return Ok(ProviderResponse {
                            value,
                            http_status: Some(completion.http_status),
                            retry_count: retry,
                            method: Some("POST".into()),
                            endpoint: Some(completion.endpoint),
                            model: Some(model.clone()),
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
            .get(build_lm_studio_url(
                settings,
                LmStudioApi::NativeV1,
                "models",
            )?)
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
            .get(build_lm_studio_url(
                settings,
                LmStudioApi::LegacyV0,
                "models",
            )?)
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
        let url = build_lm_studio_url(settings, LmStudioApi::OpenAi, "models")?;
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
            .post(build_lm_studio_url(
                settings,
                LmStudioApi::NativeV1,
                "models/load",
            )?)
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
    ) -> AppResult<ChatCompletion> {
        let client = client(settings)?;
        let url = build_lm_studio_url(settings, LmStudioApi::OpenAi, "chat/completions")?;
        let endpoint = url.path().to_string();
        let body = serde_json::json!({
            "model": model,
            "messages": [RequestMessage { role: "system", content: system }, RequestMessage { role: "user", content: user }],
            "temperature": settings.temperature,
            "stream": false
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
        let body: serde_json::Value = response.json().await.map_err(|error| {
            AppError::new(
                ErrorCode::LmstudioInvalidResponse,
                "LM Studio returned HTTP success with an invalid Chat Completions response.",
            )
            .detail(error.to_string())
        })?;
        let content = body
            .get("choices")
            .and_then(serde_json::Value::as_array)
            .filter(|choices| !choices.is_empty())
            .and_then(|choices| choices[0].get("message"))
            .and_then(serde_json::Value::as_object)
            .and_then(|message| message.get("content"))
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::LmstudioInvalidResponse,
                    "LM Studio returned HTTP success without a usable Chat Completions message.",
                )
                .detail("Expected a non-empty choices[0].message.content string.")
            })?
            .to_string();
        Ok(ChatCompletion {
            content,
            http_status: status,
            endpoint,
        })
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

fn build_lm_studio_url(
    settings: &AppSettings,
    api: LmStudioApi,
    suffix: &str,
) -> AppResult<url::Url> {
    LmStudioEndpointBuilder::from_settings(settings)?.build(api, suffix)
}

impl LmStudioEndpointBuilder {
    fn from_settings(settings: &AppSettings) -> AppResult<Self> {
        let mut origin = url::Url::parse(settings.base_url.trim()).map_err(|error| {
            AppError::new(ErrorCode::InvalidSettings, "LM Studio Base URL is invalid.")
                .detail(error.to_string())
        })?;
        if origin.scheme() != "http"
            || !matches!(origin.host_str(), Some("localhost") | Some("127.0.0.1"))
        {
            return Err(AppError::new(
                ErrorCode::InvalidSettings,
                "Local Only mode permits AI requests only to localhost or 127.0.0.1.",
            ));
        }
        let supplied_path = origin.path().trim_matches('/');
        if !supplied_path.is_empty() && supplied_path != "v1" {
            return Err(AppError::new(
                ErrorCode::InvalidSettings,
                "LM Studio Base URL must end at the server root or /v1.",
            ));
        }
        origin.set_path("/");
        origin.set_query(None);
        origin.set_fragment(None);
        Ok(Self { origin })
    }

    fn build(&self, api: LmStudioApi, suffix: &str) -> AppResult<url::Url> {
        if suffix.is_empty() || suffix.starts_with('/') {
            return Err(AppError::new(
                ErrorCode::InvalidSettings,
                "LM Studio endpoint path must be relative and non-empty.",
            ));
        }
        let prefix = match api {
            LmStudioApi::OpenAi => "/v1",
            LmStudioApi::NativeV1 => "/api/v1",
            LmStudioApi::LegacyV0 => "/api/v0",
        };
        let mut endpoint = self.origin.clone();
        endpoint.set_path(&format!("{prefix}/{suffix}"));
        Ok(endpoint)
    }
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
    fn openai_endpoints_are_normalized_for_every_supported_base_url() {
        for base_url in [
            "http://127.0.0.1:1234",
            "http://127.0.0.1:1234/",
            "http://127.0.0.1:1234/v1",
            "http://127.0.0.1:1234/v1/",
        ] {
            let settings = AppSettings {
                base_url: base_url.into(),
                ..Default::default()
            };
            assert_eq!(
                build_lm_studio_url(&settings, LmStudioApi::OpenAi, "models")
                    .expect("models endpoint")
                    .as_str(),
                "http://127.0.0.1:1234/v1/models"
            );
            assert_eq!(
                build_lm_studio_url(&settings, LmStudioApi::OpenAi, "chat/completions")
                    .expect("chat endpoint")
                    .as_str(),
                "http://127.0.0.1:1234/v1/chat/completions"
            );
        }
    }

    #[test]
    fn only_loopback_endpoints_and_relative_suffixes_are_allowed() {
        let external = AppSettings {
            base_url: "http://example.com/v1".into(),
            ..Default::default()
        };
        assert_eq!(
            build_lm_studio_url(&external, LmStudioApi::OpenAi, "models")
                .expect_err("external")
                .code_string(),
            "INVALID_SETTINGS"
        );
        let local = AppSettings::default();
        assert_eq!(
            build_lm_studio_url(&local, LmStudioApi::OpenAi, "/chat/completions")
                .expect_err("root-relative path")
                .code_string(),
            "INVALID_SETTINGS"
        );
    }

    #[tokio::test]
    async fn rejects_http_200_without_a_chat_completions_envelope() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("request");
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /v1/chat/completions "));
            write_json_response(&mut stream, r#"{"unexpected":"endpoint"}"#);
        });
        let settings = AppSettings {
            base_url: format!("http://{address}"),
            ..Default::default()
        };
        let error = LMStudioProvider
            .send_chat(
                "Return JSON.",
                "Return an object.",
                "test-model",
                &settings,
                CancellationToken::new(),
            )
            .await
            .expect_err("invalid envelope");
        assert_eq!(error.code_string(), "LMSTUDIO_INVALID_RESPONSE");
        server.join().expect("server");
    }

    #[tokio::test]
    async fn accepts_a_valid_chat_completions_envelope() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("request");
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /v1/chat/completions "));
            write_json_response(
                &mut stream,
                r#"{"choices":[{"message":{"role":"assistant","content":"{\"ok\":true}"}}]}"#,
            );
        });
        let settings = AppSettings {
            base_url: format!("http://{address}/v1/"),
            ..Default::default()
        };
        let completion = LMStudioProvider
            .send_chat(
                "Return JSON.",
                "Return an object.",
                "test-model",
                &settings,
                CancellationToken::new(),
            )
            .await
            .expect("valid envelope");
        assert_eq!(completion.endpoint, "/v1/chat/completions");
        assert_eq!(completion.http_status, 200);
        assert_eq!(completion.content, r#"{"ok":true}"#);
        server.join().expect("server");
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

    fn write_json_response(stream: &mut TcpStream, body: &str) {
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).expect("response");
    }
}
