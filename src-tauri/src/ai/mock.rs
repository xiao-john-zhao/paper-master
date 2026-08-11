use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::{
    ai::{AIProvider, ProviderResponse},
    domain::{
        ai::{ConnectionResult, PaperComparisonResult},
        paper::PaperAnalysisResult,
        settings::AppSettings,
    },
    error::{AppError, AppResult, ErrorCode},
};

pub struct MockAIProvider;

#[async_trait]
impl AIProvider for MockAIProvider {
    fn id(&self) -> &'static str {
        "mock"
    }
    async fn check_connection(&self, _settings: &AppSettings) -> ConnectionResult {
        ConnectionResult {
            state: "connected".into(),
            message: "Mock provider is ready.".into(),
            latency_ms: Some(0),
            models: vec!["paper-master-mock-v1".into()],
        }
    }
    async fn get_models(&self, _settings: &AppSettings) -> AppResult<Vec<String>> {
        Ok(vec!["paper-master-mock-v1".into()])
    }
    async fn analyze_paper(
        &self,
        _input: &str,
        _settings: &AppSettings,
        cancellation: CancellationToken,
    ) -> AppResult<ProviderResponse<PaperAnalysisResult>> {
        tokio::select! {
            _ = cancellation.cancelled() => Err(AppError::new(ErrorCode::AiCancelled, "AI analysis was cancelled.")),
            _ = tokio::time::sleep(std::time::Duration::from_millis(350)) => Ok(ProviderResponse { value: mock_analysis(), http_status: None, retry_count: 0 }),
        }
    }
    async fn generate_tags(
        &self,
        _input: &str,
        _settings: &AppSettings,
        _cancellation: CancellationToken,
    ) -> AppResult<Vec<String>> {
        Ok(mock_analysis().tags)
    }
    async fn compare_papers(
        &self,
        _input: &str,
        _settings: &AppSettings,
        cancellation: CancellationToken,
    ) -> AppResult<ProviderResponse<PaperComparisonResult>> {
        tokio::select! {
            _ = cancellation.cancelled() => Err(AppError::new(ErrorCode::AiCancelled, "AI comparison was cancelled.")),
            _ = tokio::time::sleep(std::time::Duration::from_millis(250)) => Ok(ProviderResponse { value: PaperComparisonResult { common_points: vec!["Both papers target reliable local scientific workflows.".into()], key_differences: vec!["Paper A focuses on context construction while Paper B focuses on evidence retrieval.".into()], method_comparison: "The methods use complementary selection strategies.".into(), contribution_comparison: "One contributes input construction; the other contributes evidence ranking.".into(), summary: "The two approaches are complementary and can be combined in a local research workflow.".into() }, http_status: None, retry_count: 0 }),
        }
    }
}

fn mock_analysis() -> PaperAnalysisResult {
    PaperAnalysisResult { schema_version: "1".into(), research_problem: "How can long scientific papers be analyzed reliably with a bounded local model context?".into(), core_method: "A section-aware input builder prioritizes the title, abstract, introduction, conclusion, and selected body text.".into(), key_contributions: vec!["A bounded local document-analysis pipeline.".into(), "Structured diagnostics separating parser and provider failures.".into(), "A replaceable provider interface for local models.".into()], main_results: "The method produces stable structured results while keeping all content on the local machine.".into(), limitations: "Scanned PDFs require OCR, which is outside v0.1.".into(), tags: vec!["local LLM".into(), "scientific NLP".into(), "document analysis".into(), "privacy".into()], ai_summary: Some("A private, section-aware workflow for structured scientific-paper analysis with local models.".into()) }
}
