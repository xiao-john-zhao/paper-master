pub mod input_builder;
pub mod lm_studio;
pub mod mock;
pub mod parser;
pub mod prompt;
pub mod registry;

use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};
use tokio_util::sync::CancellationToken;

use crate::{
    domain::settings::AppSettings,
    domain::{
        ai::{AIResponseDiagnostics, ConnectionResult, PaperComparisonResult},
        paper::PaperAnalysisResult,
    },
    error::{AppError, AppResult, ErrorCode},
};

#[derive(Debug, Clone)]
pub struct ProviderResponse<T> {
    pub value: T,
    pub http_status: Option<u16>,
    pub retry_count: usize,
    pub method: Option<String>,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub response_diagnostics: Option<AIResponseDiagnostics>,
}

#[async_trait]
pub trait AIProvider: Send + Sync {
    fn id(&self) -> &'static str;
    async fn check_connection(&self, settings: &AppSettings) -> ConnectionResult;
    async fn get_models(&self, settings: &AppSettings) -> AppResult<Vec<String>>;
    async fn load_model(&self, settings: &AppSettings) -> AppResult<String>;
    async fn analyze_paper(
        &self,
        input: &str,
        settings: &AppSettings,
        cancellation: CancellationToken,
    ) -> AppResult<ProviderResponse<PaperAnalysisResult>>;
    async fn generate_tags(
        &self,
        input: &str,
        settings: &AppSettings,
        cancellation: CancellationToken,
    ) -> AppResult<Vec<String>>;
    async fn compare_papers(
        &self,
        input: &str,
        settings: &AppSettings,
        cancellation: CancellationToken,
    ) -> AppResult<ProviderResponse<PaperComparisonResult>>;
}

pub struct AIProviderRegistry {
    providers: HashMap<&'static str, Arc<dyn AIProvider>>,
}

impl AIProviderRegistry {
    pub fn new() -> Self {
        let mut providers: HashMap<&'static str, Arc<dyn AIProvider>> = HashMap::new();
        providers.insert("lmstudio", Arc::new(lm_studio::LMStudioProvider::default()));
        providers.insert("mock", Arc::new(mock::MockAIProvider));
        Self { providers }
    }

    pub fn get(&self, id: &str) -> AppResult<Arc<dyn AIProvider>> {
        self.providers.get(id).cloned().ok_or_else(|| {
            AppError::new(
                ErrorCode::InvalidSettings,
                "The selected AI provider is unavailable.",
            )
        })
    }
}
