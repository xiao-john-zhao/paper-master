use regex::Regex;

use crate::{
    domain::{ai::PaperComparisonResult, paper::PaperAnalysisResult},
    error::{AppError, AppResult, ErrorCode},
};

pub fn parse_analysis_response(raw: &str) -> AppResult<PaperAnalysisResult> {
    let json = sanitize_json(raw)?;
    let decoded: serde_json::Value = serde_json::from_str(&json).map_err(|error| {
        invalid_json(format!("Analysis JSON parsing failed: {error}")).stage("JSON_PARSED")
    })?;
    let mut value: PaperAnalysisResult = serde_json::from_value(decoded).map_err(|error| {
        invalid_schema(format!("Analysis schema validation failed: {error}"))
            .stage("SCHEMA_VALIDATED")
    })?;
    value.tags = unique_nonempty(value.tags);
    value.key_innovations = unique_nonempty(value.key_innovations);
    value.key_contributions = unique_nonempty(value.key_contributions);
    if [
        &value.ai_summary,
        &value.research_problem,
        &value.research_scenario,
        &value.core_method,
        &value.main_results,
        &value.limitations,
    ]
    .into_iter()
    .chain(value.key_innovations.iter())
    .chain(value.key_contributions.iter())
    .any(|text| crate::domain::paper::is_extractive_placeholder(text))
    {
        return Err(invalid_schema(
            "Model output contains archive placeholders instead of analysis.",
        )
        .stage("SCHEMA_VALIDATED"));
    }
    if value.research_problem.trim().is_empty()
        || value.research_scenario.trim().is_empty()
        || value.core_method.trim().is_empty()
        || value.main_results.trim().is_empty()
        || value.limitations.trim().is_empty()
        || value.key_innovations.is_empty()
        || value.key_contributions.is_empty()
    {
        return Err(
            invalid_schema("Analysis JSON contains empty required fields.")
                .stage("SCHEMA_VALIDATED"),
        );
    }
    if !(3..=8).contains(&value.tags.len()) {
        return Err(
            invalid_schema("Analysis JSON must contain 3 to 8 tags.").stage("SCHEMA_VALIDATED")
        );
    }
    let summary_is_chinese = contains_cjk(&value.ai_summary);
    if ![
        value.research_problem.as_str(),
        value.research_scenario.as_str(),
        value.core_method.as_str(),
        value.main_results.as_str(),
        value.limitations.as_str(),
    ]
    .into_iter()
    .all(contains_cjk)
        || !value.key_innovations.iter().all(|item| contains_cjk(item))
        || !value
            .key_contributions
            .iter()
            .all(|item| contains_cjk(item))
        || !summary_is_chinese
    {
        return Err(
            invalid_schema("Analysis fields must be written in Simplified Chinese.")
                .stage("SCHEMA_VALIDATED"),
        );
    }
    value.schema_version = "2".to_string();
    Ok(value)
}

pub fn parse_comparison_response(raw: &str) -> AppResult<PaperComparisonResult> {
    let json = sanitize_json(raw)?;
    let decoded: serde_json::Value = serde_json::from_str(&json).map_err(|error| {
        invalid_json(format!("Comparison JSON parsing failed: {error}")).stage("JSON_PARSED")
    })?;
    let mut value: PaperComparisonResult = serde_json::from_value(decoded).map_err(|error| {
        invalid_schema(format!("Comparison schema validation failed: {error}"))
            .stage("SCHEMA_VALIDATED")
    })?;
    value.common_points = unique_nonempty(value.common_points);
    value.key_differences = unique_nonempty(value.key_differences);
    let narratives = [
        &value.problem_comparison,
        &value.scenario_comparison,
        &value.method_comparison,
        &value.contribution_comparison,
        &value.results_comparison,
        &value.limitations_comparison,
        &value.summary,
    ];
    if value.schema_version != "2"
        || value.common_points.is_empty()
        || value.key_differences.len() < 2
        || value.evidence.is_empty()
        || narratives.iter().any(|item| item.trim().is_empty())
    {
        return Err(
            invalid_schema("Comparison JSON contains empty required fields.")
                .stage("SCHEMA_VALIDATED"),
        );
    }
    if !value.common_points.iter().all(|item| contains_cjk(item))
        || !value.key_differences.iter().all(|item| contains_cjk(item))
        || !narratives.iter().all(|item| contains_cjk(item))
    {
        return Err(
            invalid_schema("Comparison fields must be written in Simplified Chinese.")
                .stage("SCHEMA_VALIDATED"),
        );
    }
    Ok(value)
}

pub fn validate_comparison_evidence(
    value: PaperComparisonResult,
    input: &str,
) -> AppResult<PaperComparisonResult> {
    let source: serde_json::Value =
        serde_json::from_str(input).map_err(|_| invalid_schema("Invalid comparison input."))?;
    let papers = source["papers"]
        .as_array()
        .ok_or_else(|| invalid_schema("Missing comparison sources."))?;
    let normalized = |text: &str| text.split_whitespace().collect::<String>();
    let references = Regex::new(r"论文\s*([0-9]+)").expect("static regex");
    let mut covered = std::collections::HashSet::new();
    for difference in &value.key_differences {
        let ids: std::collections::HashSet<usize> = references
            .captures_iter(difference)
            .filter_map(|capture| capture[1].parse().ok())
            .collect();
        if ids.len() < 2 || ids.iter().any(|id| *id == 0 || *id > papers.len()) {
            return Err(invalid_schema(
                "Each difference must identify at least two valid paper numbers.",
            ));
        }
        covered.extend(ids);
    }
    if covered.len() != papers.len() {
        return Err(invalid_schema(
            "Differences must cover every selected paper.",
        ));
    }
    for evidence in &value.evidence {
        let paper = evidence
            .paper_number
            .checked_sub(1)
            .and_then(|index| papers.get(index))
            .ok_or_else(|| invalid_schema("Evidence refers to an unknown paper."))?;
        let quote = normalized(&evidence.quote);
        if !(20..=180).contains(&evidence.quote.chars().count())
            || !normalized(paper["source_excerpt"].as_str().unwrap_or("")).contains(&quote)
        {
            return Err(invalid_schema(
                "Evidence must be a verbatim passage from the corresponding paper excerpt.",
            ));
        }
    }
    if (1..=papers.len()).any(|id| !value.evidence.iter().any(|item| item.paper_number == id)) {
        return Err(invalid_schema("Every paper must have source evidence."));
    }
    Ok(value)
}

pub fn validate_analysis_tags(
    mut value: PaperAnalysisResult,
    source_content: &str,
) -> AppResult<PaperAnalysisResult> {
    let (paper_content, managed_vocabulary) = split_managed_tag_context(source_content);
    for tag in &mut value.tags {
        if let Some(canonical) = managed_vocabulary
            .iter()
            .find(|existing| normalize_evidence_text(existing) == normalize_evidence_text(tag))
        {
            *tag = canonical.clone();
        }
    }
    value
        .tags
        .retain(|tag| tag_is_supported(tag, paper_content));
    value.tags = unique_nonempty(value.tags);
    if !(3..=8).contains(&value.tags.len()) {
        return Err(invalid_schema(
            "Fewer than three generated tags have explicit support in the paper content.",
        )
        .stage("SCHEMA_VALIDATED"));
    }
    Ok(value)
}

fn split_managed_tag_context(source: &str) -> (&str, Vec<String>) {
    const START: &str = "[MANAGED TAG VOCABULARY START — NAMING ONLY, NOT PAPER EVIDENCE]";
    const END: &str = "[MANAGED TAG VOCABULARY END]";
    let Some(start) = source.find(START) else {
        return (source, Vec::new());
    };
    let paper = &source[..start];
    let vocabulary = source[start + START.len()..]
        .split(END)
        .next()
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- "))
        .map(|line| line.rsplit_once(" (used ").map_or(line, |(tag, _)| tag))
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    (paper, vocabulary)
}

pub fn sanitize_json(raw: &str) -> AppResult<String> {
    let without_reasoning = strip_reasoning(raw);
    let without_fence = without_reasoning
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let start = without_fence
        .find('{')
        .ok_or_else(|| invalid_json("The model response does not contain a JSON object."))?;
    let end = without_fence
        .rfind('}')
        .ok_or_else(|| invalid_json("The model response contains an incomplete JSON object."))?;
    if end < start {
        return Err(invalid_json(
            "The model response contains an invalid JSON object.",
        ));
    }
    let candidate = &without_fence[start..=end];
    Ok(Regex::new(r",\s*([}\]])")
        .expect("static regex")
        .replace_all(candidate, "$1")
        .to_string())
}

fn strip_reasoning(raw: &str) -> String {
    let mut value = raw.to_string();
    while let Some(start) = value.find("<think>") {
        if let Some(relative_end) = value[start + 7..].find("</think>") {
            let end = start + 7 + relative_end + 8;
            value.replace_range(start..end, "");
        } else {
            value = value
                .find('{')
                .map(|index| value[index..].to_string())
                .unwrap_or_default();
            break;
        }
    }
    value
}

fn unique_nonempty(values: Vec<String>) -> Vec<String> {
    let mut output = Vec::new();
    for value in values {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty()
            && !output
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(&trimmed))
        {
            output.push(trimmed);
        }
    }
    output
}

fn contains_cjk(value: &str) -> bool {
    value.chars().any(
        |character| matches!(character as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF),
    )
}

fn tag_is_supported(tag: &str, source_content: &str) -> bool {
    let normalized_tag = normalize_evidence_text(tag);
    if normalized_tag.is_empty()
        || matches!(
            normalized_tag.as_str(),
            "论文" | "研究" | "方法" | "结果" | "分析" | "模型" | "技术"
        )
    {
        return false;
    }
    let normalized_source = normalize_evidence_text(source_content);
    if normalized_source.contains(&normalized_tag) {
        return true;
    }
    Regex::new(r"[\(（]([^\)）]+)[\)）]")
        .expect("static regex")
        .captures_iter(tag)
        .filter_map(|capture| capture.get(1))
        .map(|evidence| normalize_evidence_text(evidence.as_str()))
        .filter(|evidence| evidence.len() >= 2)
        .any(|evidence| normalized_source.contains(&evidence))
}

fn normalize_evidence_text(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|character| character.is_alphanumeric())
        .collect()
}

fn invalid_json(detail: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::AiInvalidJson, "The local model returned an invalid structured result. Try again or use Mock provider for diagnostics.").detail(detail)
}

fn invalid_schema(detail: impl Into<String>) -> AppError {
    AppError::new(
        ErrorCode::AiSchemaValidationFailed,
        "The local model returned JSON that does not match the paper analysis schema.",
    )
    .detail(detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"{"schema_version":"2","research_problem":"研究问题","research_scenario":"应用场景","core_method":"核心方法","key_innovations":["主要创新"],"key_contributions":["主要贡献"],"main_results":"主要结果","limitations":"研究局限","tags":["标签一","标签二","标签三"],"ai_summary":"一句话总结",}"#;

    #[test]
    fn removes_fences_reasoning_and_trailing_commas() {
        let raw = format!("<think>private reasoning</think>\n```json\n{VALID}\n```");
        assert_eq!(
            parse_analysis_response(&raw).expect("analysis").tags.len(),
            3
        );
    }

    #[test]
    fn rejects_missing_tags() {
        let raw = VALID.replace("[\"标签一\",\"标签二\",\"标签三\"]", "[\"标签一\"]");
        assert_eq!(
            parse_analysis_response(&raw)
                .expect_err("tags")
                .code_string(),
            "AI_SCHEMA_VALIDATION_FAILED"
        );
    }

    #[test]
    fn requires_all_five_summary_modules() {
        let raw = VALID.replace("\"research_scenario\":\"应用场景\",", "");
        assert_eq!(
            parse_analysis_response(&raw)
                .expect_err("scenario")
                .code_string(),
            "AI_SCHEMA_VALIDATION_FAILED"
        );
    }

    #[test]
    fn validates_structured_comparison() {
        let value = comparison_fixture();
        assert_eq!(value.common_points, vec!["研究目标相同"]);
    }

    fn comparison_fixture() -> PaperComparisonResult {
        parse_comparison_response(r#"{"schema_version":"2","common_points":["研究目标相同"],"key_differences":["论文1与论文2的方法不同。","论文1与论文2的实验条件不同。"],"problem_comparison":"研究问题存在差异。","scenario_comparison":"场景不同。","method_comparison":"两者方法存在差异。","contribution_comparison":"两者贡献可以互补。","results_comparison":"不同条件不可直接比较。","limitations_comparison":"提供的材料未报告。","evidence":[{"paper_number":1,"quote":"This paper proposes a queue-aware scheduler."},{"paper_number":2,"quote":"This paper optimizes energy using beamforming."}],"summary":"这是有依据的对比。"}"#).expect("comparison")
    }

    #[test]
    fn comparison_rejects_invented_or_misattributed_evidence() {
        let source = serde_json::json!({"papers":[
            {"source_excerpt":"This paper proposes a queue-aware scheduler."},
            {"source_excerpt":"This paper optimizes energy using beamforming."}
        ]})
        .to_string();
        assert!(validate_comparison_evidence(comparison_fixture(), &source).is_ok());
        let mut invented = comparison_fixture();
        invented.evidence[0].quote = "This paper achieves a fictional perfect accuracy.".into();
        assert!(validate_comparison_evidence(invented, &source).is_err());
        let mut wrong_paper = comparison_fixture();
        wrong_paper.evidence[0].paper_number = 2;
        assert!(validate_comparison_evidence(wrong_paper, &source).is_err());
        let mut vague = comparison_fixture();
        vague.key_differences[0] = "两者关键词不同。".into();
        assert!(validate_comparison_evidence(vague, &source).is_err());
    }

    #[test]
    fn rejects_english_only_generated_content() {
        let raw = VALID.replace("研究问题", "Research problem");
        assert_eq!(
            parse_analysis_response(&raw)
                .expect_err("language")
                .code_string(),
            "AI_SCHEMA_VALIDATION_FAILED"
        );
    }

    #[test]
    fn rejects_syntactically_invalid_json_before_schema_validation() {
        assert_eq!(
            parse_analysis_response(r#"{"research_problem": "未完成"#)
                .expect_err("invalid JSON")
                .code_string(),
            "AI_INVALID_JSON"
        );
    }

    #[test]
    fn keeps_only_tags_with_paper_evidence() {
        let mut value = parse_analysis_response(VALID).expect("analysis");
        value.tags = vec![
            "集成感知与通信（ISAC）".into(),
            "多无人机（UAV）".into(),
            "资源分配".into(),
            "量子医学".into(),
        ];
        let validated = validate_analysis_tags(value, "本文研究 ISAC 多 UAV 网络中的资源分配问题。\n\n[MANAGED TAG VOCABULARY START — NAMING ONLY, NOT PAPER EVIDENCE]\n- 集成感知与通信（ISAC） (used 4)\n- 量子医学 (used 2)\n[MANAGED TAG VOCABULARY END]\n")
            .expect("supported tags");
        assert_eq!(validated.tags.len(), 3);
        assert_eq!(validated.tags[0], "集成感知与通信（ISAC）");
        assert!(!validated.tags.iter().any(|tag| tag == "量子医学"));
    }
}
