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
            message: "Mock 模型已就绪。".into(),
            latency_ms: Some(0),
            models: vec!["paper-master-mock-v1".into()],
            loaded_models: vec!["paper-master-mock-v1".into()],
        }
    }
    async fn get_models(&self, _settings: &AppSettings) -> AppResult<Vec<String>> {
        Ok(vec!["paper-master-mock-v1".into()])
    }
    async fn load_model(&self, _settings: &AppSettings) -> AppResult<String> {
        Ok("paper-master-mock-v1".into())
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
            _ = tokio::time::sleep(std::time::Duration::from_millis(250)) => Ok(ProviderResponse { value: PaperComparisonResult { common_points: vec!["这些论文都关注本地、可靠的科研工作流。".into()], key_differences: vec!["各论文在上下文构建、证据检索与评估重点上采用了不同方案。".into()], method_comparison: "这些方法采用互补的信息选择与处理策略，并在输入组织方式上存在差异。".into(), contribution_comparison: "各论文分别在输入构建、证据排序或评估框架方面提供了可组合的贡献。".into(), summary: "这些研究具有互补性，可共同用于构建本地化的论文管理与分析流程。".into() }, http_status: None, retry_count: 0 }),
        }
    }
}

fn mock_analysis() -> PaperAnalysisResult {
    PaperAnalysisResult {
        schema_version: "2".into(),
        research_problem: "如何在本地模型上下文受限的条件下可靠分析长篇学术论文？".into(),
        research_scenario: "面向在个人电脑上使用本地大模型分析长篇学术 PDF 的私密科研场景。".into(),
        core_method: "采用章节感知的输入构建器，优先保留题目、摘要、引言、结论和少量正文证据。"
            .into(),
        key_innovations: vec![
            "将章节优先级与确定性字符预算结合，减少统一截断造成的关键证据丢失。".into(),
            "把 PDF 解析诊断与模型服务诊断纳入同一条本地分析链路。".into(),
        ],
        key_contributions: vec![
            "提出有长度上限的本地文档分析流程。".into(),
            "用结构化诊断区分解析器故障与模型服务故障。".into(),
            "通过可替换接口支持不同本地模型提供方。".into(),
        ],
        main_results: "该方法在全部内容留在本机的前提下生成稳定的结构化分析结果。".into(),
        limitations: "扫描型 PDF 仍需要 OCR，当前版本尚未覆盖。".into(),
        tags: vec![
            "本地大模型（LLM）".into(),
            "科学文档分析".into(),
            "章节感知解析".into(),
            "隐私计算".into(),
        ],
        ai_summary: Some("一个面向本地模型的私密、章节感知型学术论文结构化分析流程。".into()),
    }
}
