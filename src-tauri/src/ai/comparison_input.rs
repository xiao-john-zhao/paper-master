use super::input_builder::PaperAnalysisInputBuilder;
use crate::{
    domain::paper::Paper,
    error::{AppError, AppResult, ErrorCode},
};

/// Shares one total character budget fairly across all selected papers.
/// Full PDF text remains local; only bounded source passages are sent to the local model.
pub fn build_comparison_input(papers: &[(Paper, String)], limit: usize) -> AppResult<String> {
    if !(2..=8).contains(&papers.len()) {
        return Err(invalid("请选择 2–8 篇论文。"));
    }
    let mut per_paper = limit.saturating_sub(200) / papers.len();
    loop {
        let mut items = Vec::new();
        for (index, (paper, text)) in papers.iter().enumerate() {
            let analysis = paper
                .analysis
                .as_ref()
                .ok_or_else(|| invalid("请先完成所有论文的内容分析。"))?;
            if text.trim().is_empty() {
                return Err(invalid(
                    "所选论文缺少原文证据，请重新导入可提取文本的 PDF。",
                ));
            }
            let mut summary =
                serde_json::to_value(analysis).map_err(|_| invalid("无法读取论文分析。"))?;
            if let Some(object) = summary.as_object_mut() {
                object.remove("tags");
            }
            bound_strings(&mut summary, (per_paper / 24).max(40));
            let evidence = PaperAnalysisInputBuilder::new(per_paper / 2).build(paper, text)?;
            items.push(serde_json::json!({ "paper_number": index + 1,
                "title": paper.title.chars().take(200).collect::<String>(),
                "analysis": summary, "source_excerpt": evidence }));
        }
        let input = serde_json::to_string(&serde_json::json!({"papers":items}))
            .map_err(|_| invalid("无法准备论文对比。"))?;
        if input.chars().count() <= limit {
            return Ok(input);
        }
        if per_paper < 500 {
            return Err(invalid("对比输入预算过小，请提高设置中的最大分析输入。"));
        }
        per_paper = per_paper * 4 / 5;
    }
}

fn bound_strings(value: &mut serde_json::Value, limit: usize) {
    match value {
        serde_json::Value::String(text) => {
            *text = text.chars().take(limit).collect();
        }
        serde_json::Value::Array(items) => {
            items.truncate(3);
            for item in items {
                bound_strings(item, limit);
            }
        }
        serde_json::Value::Object(object) => {
            for item in object.values_mut() {
                bound_strings(item, limit);
            }
        }
        _ => (),
    }
}

fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::PaperNotAnalyzed, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn all_eight_papers_share_one_budget_and_include_raw_evidence() {
        use crate::ai::{mock::MockAIProvider, AIProvider};
        let analysis = MockAIProvider
            .analyze_paper("fixture", &Default::default(), Default::default())
            .await
            .unwrap()
            .value;
        let papers: Vec<_> = (1..=8).map(|index| {
            let paper = Paper { id: index.to_string(), title: format!("论文{index}"), publication_venue: None,
                file_path: "fixture.pdf".into(), file_name: "fixture.pdf".into(), authors: vec![], year: None,
                abstract_text: None, analysis: Some(analysis.clone()), analysis_provider: None, tags: vec!["SHOULD_NOT_BE_COMPARISON_INPUT".into()],
                analysis_status: "analyzed".into(), analysis_error: None, pdf_diagnostics: None,
                created_at: "now".into(), updated_at: "now".into() };
            (paper, format!("ABSTRACT\nPaper{index} evidence about queue scheduling and beamforming.\nINTRODUCTION\n{}\nCONCLUSION\nLimitations need more validation.", "长篇科研正文".repeat(6000)))
        }).collect();
        let input = build_comparison_input(&papers, 24_000).unwrap();
        assert!(input.chars().count() <= 24_000);
        assert!(!input.contains("SHOULD_NOT_BE_COMPARISON_INPUT"));
        let decoded: serde_json::Value = serde_json::from_str(&input).unwrap();
        assert_eq!(decoded["papers"].as_array().unwrap().len(), 8);
        for (index, paper) in decoded["papers"].as_array().unwrap().iter().enumerate() {
            assert!(paper["source_excerpt"]
                .as_str()
                .unwrap()
                .contains(&format!("Paper{} evidence", index + 1)));
        }
    }
}
