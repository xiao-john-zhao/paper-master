use regex::Regex;

use crate::{
    domain::{ai::PaperComparisonResult, paper::PaperAnalysisResult},
    error::{AppError, AppResult, ErrorCode},
};

pub fn parse_analysis_response(raw: &str) -> AppResult<PaperAnalysisResult> {
    let json = sanitize_json(raw)?;
    let mut value: PaperAnalysisResult = serde_json::from_str(&json)
        .map_err(|error| invalid(format!("Analysis JSON parsing failed: {error}")))?;
    value.tags = unique_nonempty(value.tags);
    value.key_contributions = unique_nonempty(value.key_contributions);
    if value.research_problem.trim().is_empty()
        || value.core_method.trim().is_empty()
        || value.main_results.trim().is_empty()
        || value.limitations.trim().is_empty()
        || value.key_contributions.is_empty()
    {
        return Err(invalid("Analysis JSON contains empty required fields."));
    }
    if !(3..=8).contains(&value.tags.len()) {
        return Err(invalid("Analysis JSON must contain 3 to 8 tags."));
    }
    value.schema_version = "1".to_string();
    Ok(value)
}

pub fn parse_comparison_response(raw: &str) -> AppResult<PaperComparisonResult> {
    let json = sanitize_json(raw)?;
    let mut value: PaperComparisonResult = serde_json::from_str(&json)
        .map_err(|error| invalid(format!("Comparison JSON parsing failed: {error}")))?;
    value.common_points = unique_nonempty(value.common_points);
    value.key_differences = unique_nonempty(value.key_differences);
    if value.common_points.is_empty()
        || value.key_differences.is_empty()
        || value.method_comparison.trim().is_empty()
        || value.contribution_comparison.trim().is_empty()
        || value.summary.trim().is_empty()
    {
        return Err(invalid("Comparison JSON contains empty required fields."));
    }
    Ok(value)
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
        .ok_or_else(|| invalid("The model response does not contain a JSON object."))?;
    let end = without_fence
        .rfind('}')
        .ok_or_else(|| invalid("The model response contains an incomplete JSON object."))?;
    if end < start {
        return Err(invalid(
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

fn invalid(detail: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::AiInvalidJson, "The local model returned an invalid structured result. Try again or use Mock provider for diagnostics.").detail(detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"{"research_problem":"Problem","core_method":"Method","key_contributions":["One"],"main_results":"Results","limitations":"Limits","tags":["a","b","c"],}"#;

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
        let raw = VALID.replace("[\"a\",\"b\",\"c\"]", "[\"a\"]");
        assert_eq!(
            parse_analysis_response(&raw)
                .expect_err("tags")
                .code_string(),
            "AI_INVALID_JSON"
        );
    }
}
