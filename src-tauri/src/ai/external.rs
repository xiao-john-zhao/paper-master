//! Opt-in OpenAI-compatible transport. LM Studio's loopback rules stay separate.
use super::{lm_studio::LMStudioProvider, AIProvider, ProviderResponse};
use crate::{
    domain::{
        ai::{ConnectionResult, PaperComparisonResult},
        paper::PaperAnalysisResult,
        settings::AppSettings,
    },
    error::{AppError, AppResult, ErrorCode},
};
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

pub fn endpoint(settings: &AppSettings, suffix: &str) -> AppResult<url::Url> {
    if settings.privacy_mode != "codex_online" {
        return Err(AppError::new(
            ErrorCode::InvalidSettings,
            "请先启用联网授权并保存，再使用外部 API。",
        ));
    }
    configured_endpoint(settings, suffix)
}

pub fn configured_endpoint(settings: &AppSettings, suffix: &str) -> AppResult<url::Url> {
    let invalid = || {
        AppError::new(
            ErrorCode::InvalidSettings,
            "API 地址须为 HTTPS，或本机回环 HTTP 地址；不能包含密码、查询参数或片段。",
        )
    };
    let mut url = url::Url::parse(settings.api_base_url.trim()).map_err(|_| invalid())?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if url.host_str().is_none()
        || !(url.scheme() == "https" || (url.scheme() == "http" && local))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid());
    }
    url.set_path(&format!("{}/{suffix}", url.path().trim_end_matches('/')));
    Ok(url)
}

pub struct ExternalProvider;
#[async_trait]
impl AIProvider for ExternalProvider {
    fn id(&self) -> &'static str {
        "openai"
    }
    async fn check_connection(&self, s: &AppSettings) -> ConnectionResult {
        let mut result = LMStudioProvider.check_connection(s).await;
        result.message = if result.state == "connected" {
            format!("API 连接成功，读取到 {} 个模型。", result.models.len())
        } else {
            "未能读取 API 模型列表。请检查地址、授权、密钥；若服务不支持 /models，可手动填写模型后直接分析。".into()
        };
        result
    }
    async fn get_models(&self, s: &AppSettings) -> AppResult<Vec<String>> {
        LMStudioProvider.get_models(s).await
    }
    async fn load_model(&self, s: &AppSettings) -> AppResult<String> {
        endpoint(s, "models")?;
        if s.api_model.trim().is_empty() {
            return Err(AppError::new(
                ErrorCode::InvalidSettings,
                "请填写 API 模型名称。",
            ));
        }
        Ok(s.api_model.clone())
    }
    async fn analyze_paper(
        &self,
        input: &str,
        s: &AppSettings,
        c: CancellationToken,
    ) -> AppResult<ProviderResponse<PaperAnalysisResult>> {
        LMStudioProvider.analyze_paper(input, s, c).await
    }
    async fn generate_tags(
        &self,
        input: &str,
        s: &AppSettings,
        c: CancellationToken,
    ) -> AppResult<Vec<String>> {
        LMStudioProvider.generate_tags(input, s, c).await
    }
    async fn compare_papers(
        &self,
        input: &str,
        s: &AppSettings,
        c: CancellationToken,
    ) -> AppResult<ProviderResponse<PaperComparisonResult>> {
        LMStudioProvider.compare_papers(input, s, c).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requires_consent_and_preserves_custom_api_prefix() {
        let mut s = AppSettings {
            ai_provider: "openai".into(),
            api_base_url: "https://api.example.org/compatible/v1/".into(),
            ..Default::default()
        };
        assert!(endpoint(&s, "models").is_err());
        s.privacy_mode = "codex_online".into();
        assert_eq!(
            endpoint(&s, "chat/completions").unwrap().as_str(),
            "https://api.example.org/compatible/v1/chat/completions"
        );
        for url in [
            "http://example.org/v1",
            "https://key@example.org",
            "https://example.org?key=secret",
            "https://example.org/#fragment",
        ] {
            s.api_base_url = url.into();
            assert!(endpoint(&s, "models").is_err());
        }
    }
}
