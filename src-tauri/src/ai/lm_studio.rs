use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::{
    ai::{
        parser::{
            parse_analysis_response, parse_comparison_response, validate_analysis_tags,
            validate_comparison_evidence,
        },
        prompt::{
            build_analysis_user_prompt, build_comparison_user_prompt, ANALYSIS_SYSTEM_PROMPT,
            COMPARISON_SYSTEM_PROMPT, REPAIR_ANALYSIS_PROMPT, REPAIR_COMPARISON_PROMPT,
        },
        AIProvider, ProviderResponse,
    },
    config::AI_STRUCTURED_RETRY_COUNT,
    domain::{
        ai::{AIResponseDiagnostics, ConnectionResult, PaperComparisonResult},
        paper::PaperAnalysisResult,
        settings::AppSettings,
    },
    error::{AppError, AppResult, ErrorCode},
};

#[derive(Default)]
pub struct LMStudioProvider;

fn source_tag_candidates(input: &str) -> String {
    let source = input
        .split("[MANAGED TAG VOCABULARY START")
        .next()
        .unwrap_or(input);
    let terms = regex::Regex::new(r"\b[A-Za-z][A-Za-z-]{2,30}\b").expect("static regex");
    let mut counts = std::collections::HashMap::<String, (usize, String)>::new();
    for found in terms.find_iter(source) {
        let term = found.as_str();
        let lower = term.to_ascii_lowercase();
        if [
            "the",
            "and",
            "for",
            "with",
            "from",
            "that",
            "this",
            "are",
            "was",
            "were",
            "which",
            "can",
            "has",
            "have",
            "not",
            "our",
            "their",
            "these",
            "those",
            "also",
            "been",
            "will",
            "more",
            "such",
            "than",
            "into",
            "each",
            "based",
            "using",
            "paper",
            "title",
            "abstract",
            "introduction",
            "conclusion",
            "selected",
            "body",
            "method",
            "result",
            "evidence",
        ]
        .contains(&lower.as_str())
        {
            continue;
        }
        let entry = counts.entry(lower).or_insert((0, term.into()));
        entry.0 += 1;
    }
    let mut ranked: Vec<_> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(a.0.cmp(&b.0)));
    ranked
        .into_iter()
        .take(40)
        .map(|(_, (_, term))| term)
        .collect::<Vec<_>>()
        .join(", ")
}

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
    #[serde(alias = "instance_id")]
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
    diagnostics: AIResponseDiagnostics,
}

#[derive(Clone, Copy)]
enum StructuredOutputSchema<'a> {
    PaperAnalysis,
    PaperComparison(&'a str),
    EvidenceNote,
    SemanticAudit,
}

impl StructuredOutputSchema<'_> {
    fn name(self) -> &'static str {
        match self {
            Self::PaperAnalysis => "paper_analysis_result",
            Self::PaperComparison(_) => "paper_comparison_result",
            Self::EvidenceNote => "paper_evidence_note",
            Self::SemanticAudit => "paper_semantic_audit",
        }
    }

    fn schema(self) -> serde_json::Value {
        match self {
            Self::PaperAnalysis => PaperAnalysisResult::response_json_schema(),
            Self::PaperComparison(input) => comparison_response_schema(input),
            Self::EvidenceNote => super::staged::note_schema(),
            Self::SemanticAudit => super::staged::audit_schema(),
        }
    }
}

pub(crate) fn comparison_response_schema(input: &str) -> serde_json::Value {
    let mut schema = PaperComparisonResult::response_json_schema();
    let Ok(source) = serde_json::from_str::<serde_json::Value>(input) else {
        return schema;
    };
    let Some(papers) = source["papers"].as_array() else {
        return schema;
    };
    let choices: Vec<_> = papers
        .iter()
        .enumerate()
        .map(|(index, paper)| {
            let excerpt = paper["source_excerpt"].as_str().unwrap_or("");
            let title = paper["title"].as_str().unwrap_or("");
            let mut quotes: Vec<String> = excerpt
                .lines()
                .map(str::trim)
                .filter(|line| !line.starts_with('[') && *line != title)
                .map(|line| line.chars().take(160).collect::<String>())
                .filter(|line| line.chars().count() >= 20)
                .collect();
            quotes.dedup();
            // Offer passages throughout the excerpt rather than only its beginning.
            if quotes.len() > 24 {
                quotes = (0..24)
                    .map(|n| quotes[n * (quotes.len() - 1) / 23].clone())
                    .collect();
            }
            if quotes.is_empty() {
                quotes.push(excerpt.chars().take(160).collect());
            }
            serde_json::json!({"type":"object", "additionalProperties":false,
            "properties":{"paper_number":{"type":"integer","const":index+1},
                "quote":{"type":"string","enum":quotes}},
            "required":["paper_number","quote"]})
        })
        .collect();
    schema["properties"]["evidence"] = serde_json::json!({"type":"array", "minItems":papers.len(), "maxItems":papers.len()*2,
        "items":{"oneOf":choices}});
    schema
}

#[derive(Serialize)]
struct RequestMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[async_trait]
impl AIProvider for LMStudioProvider {
    fn supports_staged_analysis(&self) -> bool {
        true
    }
    fn id(&self) -> &'static str {
        "lmstudio"
    }

    async fn read_evidence(
        &self,
        input: &str,
        settings: &AppSettings,
        cancellation: CancellationToken,
    ) -> AppResult<super::staged::EvidenceNote> {
        let model = self.resolve_model(settings).await?;
        let result = self
            .send_chat(
                super::staged::READ_PROMPT,
                input,
                &model,
                settings,
                cancellation,
                StructuredOutputSchema::EvidenceNote,
            )
            .await?;
        serde_json::from_str(&result.content)
            .map_err(|_| AppError::new(ErrorCode::AiInvalidJson, "分段证据返回了无效 JSON。"))
    }
    async fn audit_analysis(
        &self,
        input: &str,
        settings: &AppSettings,
        cancellation: CancellationToken,
    ) -> AppResult<super::staged::SemanticAudit> {
        let model = self.resolve_model(settings).await?;
        let result = self
            .send_chat(
                super::staged::AUDIT_PROMPT,
                input,
                &model,
                settings,
                cancellation,
                StructuredOutputSchema::SemanticAudit,
            )
            .await?;
        serde_json::from_str(&result.content)
            .map_err(|_| AppError::new(ErrorCode::AiInvalidJson, "语义复核返回了无效 JSON。"))
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
                StructuredOutputSchema::PaperAnalysis,
            )
            .await?;
        let mut content = completion.content.clone();
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
                response_diagnostics: Some(completion.diagnostics),
            }),
            Err(first_error) => {
                for retry in 1..=AI_STRUCTURED_RETRY_COUNT {
                    let candidates = source_tag_candidates(input);
                    let repair = format!(
                        "{REPAIR_ANALYSIS_PROMPT}\n\n本次校验要求：至少三个标签的括号内必须包含论文原文中的连续术语，不能自创缩写。请从下面实际出现的英文术语中选择与论文主旨相符的术语，写成中文名称（原文术语）；候选仅用于定位原文，不代表建议采用全部词语。\n{candidates}\n\n论文内容：\n{input}\n\n待修复响应：\n{content}"
                    );
                    completion = self
                        .send_chat(
                            REPAIR_ANALYSIS_PROMPT,
                            &repair,
                            &model,
                            settings,
                            cancellation.clone(),
                            StructuredOutputSchema::PaperAnalysis,
                        )
                        .await?;
                    content = completion.content.clone();
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
                            response_diagnostics: Some(completion.diagnostics),
                        });
                    }
                }
                Err(first_error.response_diagnostics(completion.diagnostics))
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
                StructuredOutputSchema::PaperComparison(input),
            )
            .await?;
        let mut content = completion.content.clone();
        match parse_comparison_response(&content)
            .and_then(|value| validate_comparison_evidence(value, input))
        {
            Ok(value) => Ok(ProviderResponse {
                value,
                http_status: Some(completion.http_status),
                retry_count: 0,
                method: Some("POST".into()),
                endpoint: Some(completion.endpoint),
                model: Some(model.clone()),
                response_diagnostics: Some(completion.diagnostics),
            }),
            Err(first_error) => {
                for retry in 1..=AI_STRUCTURED_RETRY_COUNT {
                    let repair = format!("{REPAIR_COMPARISON_PROMPT}\n\n原始论文材料：\n{input}\n\n待修复响应：\n{content}");
                    completion = self
                        .send_chat(
                            COMPARISON_SYSTEM_PROMPT,
                            &repair,
                            &model,
                            settings,
                            cancellation.clone(),
                            StructuredOutputSchema::PaperComparison(input),
                        )
                        .await?;
                    content = completion.content.clone();
                    if let Ok(value) = parse_comparison_response(&content)
                        .and_then(|value| validate_comparison_evidence(value, input))
                    {
                        return Ok(ProviderResponse {
                            value,
                            http_status: Some(completion.http_status),
                            retry_count: retry,
                            method: Some("POST".into()),
                            endpoint: Some(completion.endpoint),
                            model: Some(model.clone()),
                            response_diagnostics: Some(completion.diagnostics),
                        });
                    }
                }
                Err(first_error.response_diagnostics(completion.diagnostics))
            }
        }
    }
}

impl LMStudioProvider {
    async fn fetch_model_catalog(
        &self,
        settings: &AppSettings,
    ) -> AppResult<Vec<ModelAvailability>> {
        if settings.ai_provider == "openai" {
            return self.fetch_openai_models(settings).await;
        }
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
        if settings.ai_provider == "openai" {
            super::external::endpoint(settings, "chat/completions")?;
            if settings.api_model.trim().is_empty() {
                return Err(AppError::new(
                    ErrorCode::InvalidSettings,
                    "请填写 API 模型名称。",
                ));
            }
            return Ok(settings.api_model.clone());
        }
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
            .json(&serde_json::json!({ "model": model, "context_length": settings.model_context_length }))
            .send()
            .await
            .map_err(request_error)?;
        if matches!(response.status().as_u16(), 404 | 405) {
            return Ok(None);
        }
        if !response.status().is_success() {
            let status = response.status().as_u16();
            return Err(AppError::new(
                ErrorCode::LmstudioModelNotFound,
                "LM Studio 无法加载所选模型，请检查内存/显存和模型文件。",
            )
            .detail(format!("HTTP {status}"))
            .response_diagnostics(AIResponseDiagnostics {
                http_status: Some(status),
                ..Default::default()
            }));
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
        output_schema: StructuredOutputSchema<'_>,
    ) -> AppResult<ChatCompletion> {
        let client = client(settings)?;
        let url = build_lm_studio_url(settings, LmStudioApi::OpenAi, "chat/completions")?;
        let endpoint = url.path().to_string();
        let response_format = serde_json::json!({
            "type": "json_schema",
            "json_schema": {
                "name": output_schema.name(),
                "strict": true,
                "schema": output_schema.schema(),
            }
        });
        let mut body = serde_json::json!({
            "model": model,
            "messages": [RequestMessage { role: "system", content: system }, RequestMessage { role: "user", content: user }],
            "temperature": settings.temperature,
            "reasoning_effort": if settings.reasoning_effort == "off" { "none" } else { &settings.reasoning_effort },
            "max_tokens": settings.max_output_tokens,
            "response_format": response_format,
            "stream": false
        });
        if settings.ai_provider == "openai" {
            if settings.api_response_format == "json_object" {
                body["response_format"] = serde_json::json!({"type":"json_object"});
            }
            if !settings.api_reasoning_supported {
                body.as_object_mut().unwrap().remove("reasoning_effort");
            }
        }
        let started = Instant::now();
        let request = client.post(url).json(&body).send();
        let response = tokio::select! {
            _ = cancellation.cancelled() => return Err(AppError::new(ErrorCode::AiCancelled, "AI analysis was cancelled.")),
            value = request => value.map_err(|error| request_error(error).stage("REQUEST_SENT"))?,
        };
        let status = response.status().as_u16();
        if !response.status().is_success() {
            let diagnostics = AIResponseDiagnostics {
                http_status: Some(status),
                duration_ms: Some(started.elapsed().as_millis()),
                ..Default::default()
            };
            return Err(AppError::new(
                ErrorCode::LmstudioConnectionFailed,
                if settings.ai_provider == "openai" {
                    "外部 API 拒绝请求，请核对密钥、模型、额度及输出格式。"
                } else {
                    "LM Studio 拒绝分析请求。"
                },
            )
            .detail(format!("HTTP {status}"))
            .stage("RESPONSE_RECEIVED")
            .response_diagnostics(diagnostics));
        }
        let body: serde_json::Value = response.json().await.map_err(|error| {
            AppError::new(
                ErrorCode::LmstudioInvalidResponse,
                "LM Studio returned HTTP success with an invalid Chat Completions response.",
            )
            .detail(error.to_string())
            .stage("RESPONSE_RECEIVED")
            .response_diagnostics(AIResponseDiagnostics {
                http_status: Some(status),
                duration_ms: Some(started.elapsed().as_millis()),
                ..Default::default()
            })
        })?;
        extract_chat_completion(&body, status, endpoint, started.elapsed().as_millis())
    }

    #[cfg(test)]
    async fn minimal_structured_output_check(
        &self,
        settings: &AppSettings,
    ) -> AppResult<(PaperAnalysisResult, AIResponseDiagnostics)> {
        const MINIMAL_INPUT: &str = r#"Return an analysis of the following fictional paper metadata.

Title:
Fair Scheduling in Wireless Networks

Abstract:
This paper studies queue fairness and proposes a deficit-based scheduler."#;
        let model = self.resolve_model(settings).await?;
        let completion = self
            .send_chat(
                ANALYSIS_SYSTEM_PROMPT,
                MINIMAL_INPUT,
                &model,
                settings,
                CancellationToken::new(),
                StructuredOutputSchema::PaperAnalysis,
            )
            .await
            .map_err(|error| {
                if matches!(&error.code, ErrorCode::AiFinalContentEmpty) {
                    let mut incompatible = AppError::new(
                        ErrorCode::ModelStructuredOutputIncompatible,
                        "The selected model did not produce final structured content.",
                    )
                    .stage("FINAL_CONTENT_EXTRACTED");
                    if let Some(diagnostics) = error.response_diagnostics {
                        incompatible = incompatible.response_diagnostics(diagnostics);
                    }
                    incompatible
                } else {
                    error
                }
            })?;
        let diagnostics = completion.diagnostics.clone();
        let result = parse_analysis_response(&completion.content)
            .map_err(|error| error.response_diagnostics(completion.diagnostics))?;
        Ok((result, diagnostics))
    }
}

fn extract_chat_completion(
    body: &serde_json::Value,
    status: u16,
    endpoint: String,
    duration_ms: u128,
) -> AppResult<ChatCompletion> {
    let choices = body
        .get("choices")
        .and_then(serde_json::Value::as_array)
        .filter(|choices| !choices.is_empty())
        .ok_or_else(|| invalid_chat_envelope(status, duration_ms, "Expected choices[0]."))?;
    let choice = choices[0].as_object().ok_or_else(|| {
        invalid_chat_envelope(status, duration_ms, "Expected choices[0] to be an object.")
    })?;
    let message = choice
        .get("message")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            invalid_chat_envelope(status, duration_ms, "Expected choices[0].message.")
        })?;
    let finish_reason = choice
        .get("finish_reason")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);
    let reasoning = message
        .get("reasoning_content")
        .or_else(|| message.get("reasoning"));
    let reasoning_length = reasoning.map(diagnostic_value_length).unwrap_or_default();
    let reasoning_present = reasoning_length > 0;
    let tool_calls_present = message.get("tool_calls").is_some_and(|value| {
        !value.is_null() && value.as_array().is_none_or(|items| !items.is_empty())
    });
    let content_value = message.get("content");
    let content = match content_value {
        Some(value) if value.is_null() => String::new(),
        Some(value) => value
            .as_str()
            .ok_or_else(|| {
                invalid_chat_envelope(
                    status,
                    duration_ms,
                    "Expected choices[0].message.content to be a string or null.",
                )
            })?
            .to_string(),
        None => String::new(),
    };
    let content_length = content.chars().count();
    let content_present = !content.trim().is_empty();
    let diagnostics = AIResponseDiagnostics {
        http_status: Some(status),
        finish_reason: finish_reason.clone(),
        content_present,
        content_length,
        reasoning_present,
        reasoning_length,
        tool_calls_present,
        duration_ms: Some(duration_ms),
    };

    if finish_reason.as_deref() == Some("length") {
        return Err(AppError::new(
            ErrorCode::AiOutputTokenLimit,
            "The local model reached the configured output token limit before completing the final answer.",
        )
        .detail("choices[0].finish_reason was length.")
        .stage("FINAL_CONTENT_EXTRACTED")
        .response_diagnostics(diagnostics));
    }
    if !content_present {
        return Err(AppError::new(
            ErrorCode::AiFinalContentEmpty,
            "The local model completed processing but did not return a final answer. Please retry or adjust the model configuration.",
        )
        .detail("choices[0].message.content was empty; reasoning content was not used.")
        .stage("FINAL_CONTENT_EXTRACTED")
        .response_diagnostics(diagnostics));
    }

    Ok(ChatCompletion {
        content,
        http_status: status,
        endpoint,
        diagnostics,
    })
}

fn invalid_chat_envelope(status: u16, duration_ms: u128, detail: &str) -> AppError {
    AppError::new(
        ErrorCode::LmstudioInvalidResponse,
        "LM Studio returned HTTP success with an invalid Chat Completions response.",
    )
    .detail(detail)
    .stage("RESPONSE_RECEIVED")
    .response_diagnostics(AIResponseDiagnostics {
        http_status: Some(status),
        duration_ms: Some(duration_ms),
        ..Default::default()
    })
}

fn diagnostic_value_length(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Null => 0,
        serde_json::Value::String(text) => text.chars().count(),
        other => serde_json::to_string(other)
            .map(|text| text.chars().count())
            .unwrap_or_default(),
    }
}

fn client(settings: &AppSettings) -> AppResult<Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    if settings.ai_provider == "openai" && !settings.api_key.is_empty() {
        let mut value =
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", settings.api_key))
                .map_err(|_| AppError::new(ErrorCode::InvalidSettings, "API 密钥包含无效字符。"))?;
        value.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, value);
    }
    Ok(Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .default_headers(headers)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_millis(settings.request_timeout_ms))
        .user_agent("paper-master/0.2.1")
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
    if settings.ai_provider == "openai" {
        return super::external::endpoint(settings, suffix);
    }
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
                StructuredOutputSchema::PaperAnalysis,
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
            assert!(request.contains(r#""response_format":{"json_schema":"#));
            assert!(request.contains(r#""type":"json_schema""#));
            assert!(request.contains(r#""name":"paper_analysis_result""#));
            assert!(request.contains(r#""additionalProperties":false"#));
            assert!(request.contains(r#""max_tokens":8192"#));
            write_json_response(
                &mut stream,
                r#"{"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"{\"ok\":true}"}}]}"#,
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
                StructuredOutputSchema::PaperAnalysis,
            )
            .await
            .expect("valid envelope");
        assert_eq!(completion.endpoint, "/v1/chat/completions");
        assert_eq!(completion.http_status, 200);
        assert_eq!(completion.content, r#"{"ok":true}"#);
        assert_eq!(
            completion.diagnostics.finish_reason.as_deref(),
            Some("stop")
        );
        assert!(completion.diagnostics.content_present);
        assert!(!completion.diagnostics.reasoning_present);
        server.join().expect("server");
    }

    #[tokio::test]
    async fn rejects_empty_final_content_without_using_reasoning() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("request");
            let _request = read_request(&mut stream);
            write_json_response(
                &mut stream,
                r#"{"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"","reasoning_content":"private chain of thought that must never be used"}}]}"#,
            );
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
                StructuredOutputSchema::PaperAnalysis,
            )
            .await
            .expect_err("empty final content");
        assert_eq!(error.code_string(), "AI_FINAL_CONTENT_EMPTY");
        assert_eq!(error.stage.as_deref(), Some("FINAL_CONTENT_EXTRACTED"));
        let diagnostics = error.response_diagnostics.expect("safe diagnostics");
        assert!(!diagnostics.content_present);
        assert_eq!(diagnostics.content_length, 0);
        assert!(diagnostics.reasoning_present);
        assert!(diagnostics.reasoning_length > 0);
        server.join().expect("server");
    }

    #[tokio::test]
    async fn distinguishes_output_token_limit_from_empty_content() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let address = listener.local_addr().expect("address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("request");
            let _request = read_request(&mut stream);
            write_json_response(
                &mut stream,
                r#"{"choices":[{"finish_reason":"length","message":{"role":"assistant","content":"","reasoning_content":"unfinished reasoning"}}]}"#,
            );
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
                StructuredOutputSchema::PaperAnalysis,
            )
            .await
            .expect_err("token limit");
        assert_eq!(error.code_string(), "AI_OUTPUT_TOKEN_LIMIT");
        assert_eq!(
            error
                .response_diagnostics
                .expect("safe diagnostics")
                .finish_reason
                .as_deref(),
            Some("length")
        );
        server.join().expect("server");
    }

    #[test]
    fn generated_paper_schema_is_strict_and_complete() {
        let schema = PaperAnalysisResult::response_json_schema();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["additionalProperties"], false);
        let required = schema["required"].as_array().expect("required fields");
        for field in [
            "schema_version",
            "research_problem",
            "research_scenario",
            "core_method",
            "key_innovations",
            "key_contributions",
            "main_results",
            "limitations",
            "tags",
            "ai_summary",
        ] {
            assert!(required.iter().any(|item| item == field), "missing {field}");
        }
    }

    #[test]
    fn comparison_schema_constrains_quotes_to_their_own_paper() {
        let sources = serde_json::json!({"papers":[
            {"title":"A", "source_excerpt":"[TITLE]\nA\nThis paper uses queue-aware scheduling for fairness."},
            {"title":"B", "source_excerpt":"[TITLE]\nB\nThis paper uses energy-efficient beamforming for sensing."}
        ]}).to_string();
        let schema = comparison_response_schema(&sources);
        assert_eq!(schema["properties"]["evidence"]["minItems"], 2);
        let choices = schema["properties"]["evidence"]["items"]["oneOf"]
            .as_array()
            .unwrap();
        assert_eq!(choices[0]["properties"]["paper_number"]["const"], 1);
        assert_eq!(choices[1]["properties"]["paper_number"]["const"], 2);
        assert!(choices[0]["properties"]["quote"]["enum"][0]
            .as_str()
            .unwrap()
            .contains("queue-aware"));
        assert!(!choices[1].to_string().contains("queue-aware"));
    }

    #[tokio::test]
    #[ignore = "requires a running local LM Studio server and a structured-output-capable model"]
    async fn live_minimal_structured_output_smoke() {
        let mut settings = AppSettings::default();
        settings.base_url = std::env::var("PAPER_MASTER_LMSTUDIO_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:1234/v1".into());
        settings.model = std::env::var("PAPER_MASTER_LMSTUDIO_MODEL")
            .unwrap_or_else(|_| "deepseek-r1-distill-llama-8b".into());
        settings.request_timeout_ms = std::env::var("PAPER_MASTER_LMSTUDIO_TIMEOUT_MS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(300_000);
        let (result, diagnostics) = LMStudioProvider
            .minimal_structured_output_check(&settings)
            .await
            .expect("model must return schema-valid final content");
        eprintln!(
            "content_present={} content_length={} reasoning_present={} reasoning_length={} finish_reason={}",
            diagnostics.content_present,
            diagnostics.content_length,
            diagnostics.reasoning_present,
            diagnostics.reasoning_length,
            diagnostics.finish_reason.as_deref().unwrap_or("none")
        );
        assert!(diagnostics.content_present);
        assert!(diagnostics.content_length > 0);
        assert_ne!(diagnostics.finish_reason.as_deref(), Some("length"));
        assert_eq!(result.schema_version, "2");
    }

    #[tokio::test]
    #[ignore = "requires the local paper database and a running LM Studio server"]
    async fn live_real_paper_structured_output_smoke() {
        let database_path = std::env::var_os("PAPER_MASTER_REAL_PAPER_DB")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("APPDATA").map(|directory| {
                    std::path::PathBuf::from(directory)
                        .join("app.papermaster.desktop")
                        .join("paper-master.db")
                })
            })
            .expect("set PAPER_MASTER_REAL_PAPER_DB or APPDATA");
        let connection = rusqlite::Connection::open_with_flags(
            &database_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .expect("open local paper database read-only");
        let (title, abstract_text, extracted_text): (String, Option<String>, String) = connection
            .query_row(
                "SELECT title,abstract_text,extracted_text FROM papers WHERE length(extracted_text)>1000 ORDER BY updated_at DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("a real extracted paper");
        let bounded_text = extracted_text.chars().take(24_000).collect::<String>();
        let input = format!(
            "Title: {title}\nAbstract: {}\n\nPaper content:\n{bounded_text}",
            abstract_text.unwrap_or_else(|| "Not reported".into())
        );
        let mut settings = AppSettings::default();
        settings.base_url = std::env::var("PAPER_MASTER_LMSTUDIO_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:1234/v1".into());
        settings.model = std::env::var("PAPER_MASTER_LMSTUDIO_MODEL")
            .unwrap_or_else(|_| "deepseek-r1-distill-llama-8b".into());
        settings.request_timeout_ms = std::env::var("PAPER_MASTER_LMSTUDIO_TIMEOUT_MS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(300_000);
        let response = LMStudioProvider
            .analyze_paper(&input, &settings, CancellationToken::new())
            .await
            .expect("real paper must return a schema-valid final analysis");
        let diagnostics = response
            .response_diagnostics
            .expect("LM Studio response diagnostics");
        eprintln!(
            "title={title:?} content_present={} content_length={} reasoning_present={} reasoning_length={} finish_reason={} retry_count={}",
            diagnostics.content_present,
            diagnostics.content_length,
            diagnostics.reasoning_present,
            diagnostics.reasoning_length,
            diagnostics.finish_reason.as_deref().unwrap_or("none"),
            response.retry_count
        );
        assert!(diagnostics.content_present);
        assert_eq!(response.value.schema_version, "2");
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
                    r#"{"type":"llm","instance_id":"qwen/test","load_time_seconds":0.1,"status":"loaded"}"#
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

    #[tokio::test]
    async fn external_api_uses_bearer_custom_path_and_json_mode_without_native_load() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert!(request.starts_with("POST /custom/v1/chat/completions "));
            assert!(request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-only-key"));
            let body: serde_json::Value =
                serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
            assert_eq!(body["model"], "remote-model");
            assert_eq!(body["response_format"]["type"], "json_object");
            assert!(body.get("reasoning_effort").is_none());
            write_json_response(
                &mut stream,
                r#"{"choices":[{"finish_reason":"stop","message":{"content":"{}"}}]}"#,
            );
        });
        let s = AppSettings {
            ai_provider: "openai".into(),
            privacy_mode: "codex_online".into(),
            api_base_url: format!("http://{address}/custom/v1"),
            api_key: "test-only-key".into(),
            api_model: "remote-model".into(),
            ..Default::default()
        };
        assert_eq!(
            LMStudioProvider.resolve_model(&s).await.unwrap(),
            "remote-model"
        );
        LMStudioProvider
            .send_chat(
                "return JSON",
                "test",
                "remote-model",
                &s,
                CancellationToken::new(),
                StructuredOutputSchema::PaperAnalysis,
            )
            .await
            .unwrap();
        server.join().unwrap();
        let mut offline = s;
        offline.privacy_mode = "local_only".into();
        assert!(LMStudioProvider.resolve_model(&offline).await.is_err());
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
