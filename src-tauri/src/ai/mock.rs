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
            _ = tokio::time::sleep(std::time::Duration::from_millis(350)) => Ok(ProviderResponse { value: mock_analysis(), http_status: None, retry_count: 0, method: None, endpoint: None, model: Some("paper-master-mock-v1".into()), response_diagnostics: None }),
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
            _ = tokio::time::sleep(std::time::Duration::from_millis(250)) => Ok(ProviderResponse { value: PaperComparisonResult {
                schema_version: "2".into(), common_points: vec!["Mock 演示：这些论文都关注可靠科研工作流。".into()],
                key_differences: vec!["Mock 演示：论文1与论文2的输入组织方式不同。".into(), "Mock 演示：论文1与论文2的评估重点不同。".into()],
                problem_comparison: "Mock 演示：研究问题不同。".into(), scenario_comparison: "Mock 演示：适用条件不同。".into(),
                method_comparison: "Mock 演示：方法互补。".into(), contribution_comparison: "Mock 演示：贡献互补。".into(),
                results_comparison: "Mock 演示：实验条件不同，不可直接比较。".into(), limitations_comparison: "Mock 演示：均有适用范围限制。".into(),
                evidence: vec![], summary: "Mock 演示数据，仅用于界面开发；切换 LM Studio 获取真实论文对比。".into()
            }, http_status: None, retry_count: 0, method: None, endpoint: None, model: Some("paper-master-mock-v1".into()), response_diagnostics: None }),
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
        ai_summary: "一个面向本地模型的私密、章节感知型学术论文结构化分析流程。".into(),
    }
}
