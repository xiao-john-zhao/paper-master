use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::{
    ai::{
        parser::{parse_analysis_response, parse_comparison_response},
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
        match self.fetch_models(settings).await {
            Ok(models) if models.is_empty() => ConnectionResult { state: "model_not_loaded".into(), message: "LM Studio is connected, but no model is loaded. Load a model in LM Studio and try again.".into(), latency_ms: Some(started.elapsed().as_millis()), models },
            Ok(models) => ConnectionResult { state: "connected".into(), message: format!("Connected to LM Studio. {} model{} available.", models.len(), if models.len() == 1 { "" } else { "s" }), latency_ms: Some(started.elapsed().as_millis()), models },
            Err(error) => ConnectionResult { state: if matches!(error.code, ErrorCode::LmstudioConnectionFailed) { "disconnected" } else { "connection_failed" }.into(), message: error.message, latency_ms: Some(started.elapsed().as_millis()), models: Vec::new() },
        }
    }

    async fn get_models(&self, settings: &AppSettings) -> AppResult<Vec<String>> {
        let models = self.fetch_models(settings).await?;
        if models.is_empty() {
            Err(AppError::new(
                ErrorCode::LmstudioModelNotFound,
                "LM Studio is running, but no model is loaded.",
            ))
        } else {
            Ok(models)
        }
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
        match parse_analysis_response(&content) {
            Ok(value) => Ok(ProviderResponse {
                value,
                http_status: Some(status),
                retry_count: 0,
            }),
            Err(first_error) => {
                for retry in 1..=AI_STRUCTURED_RETRY_COUNT {
                    let repair = format!("{REPAIR_ANALYSIS_PROMPT}\n\nRESPONSE:\n{content}");
                    (content, status) = self
                        .send_chat(
                            REPAIR_ANALYSIS_PROMPT,
                            &repair,
                            &model,
                            settings,
                            cancellation.clone(),
                        )
                        .await?;
                    if let Ok(value) = parse_analysis_response(&content) {
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
    async fn fetch_models(&self, settings: &AppSettings) -> AppResult<Vec<String>> {
        let client = client(settings)?;
        let url = endpoint(settings, "models")?;
        let response = client.get(url).send().await.map_err(request_error)?;
        if !response.status().is_success() {
            return Err(AppError::new(
                ErrorCode::LmstudioConnectionFailed,
                "LM Studio returned an error while listing models.",
            )
            .detail(format!("HTTP {}", response.status().as_u16())));
        }
        let body: ModelsResponse = response.json().await.map_err(|error| {
            AppError::new(
                ErrorCode::LmstudioConnectionFailed,
                "LM Studio returned an invalid models response.",
            )
            .detail(error.to_string())
        })?;
        Ok(body
            .data
            .into_iter()
            .map(|model| model.id)
            .filter(|id| !id.trim().is_empty())
            .collect())
    }

    async fn resolve_model(&self, settings: &AppSettings) -> AppResult<String> {
        let models = self.get_models(settings).await?;
        if settings.model.trim().is_empty() {
            return Ok(models[0].clone());
        }
        models
            .into_iter()
            .find(|model| model == &settings.model)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::LmstudioModelNotFound,
                    "The selected model is not loaded in LM Studio.",
                )
            })
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
    let mut base = url::Url::parse(&settings.base_url).map_err(|error| {
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

    #[test]
    fn only_loopback_endpoints_are_allowed() {
        let local = AppSettings::default();
        assert!(endpoint(&local, "models")
            .expect("local")
            .as_str()
            .ends_with("/v1/models"));
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
}
