use crate::{
    domain::settings::AppSettings,
    error::{AppError, ErrorCode},
};

/// A fresh plan is made for every paper; no global downgrade is persisted.
pub fn profiles(settings: &AppSettings) -> Vec<AppSettings> {
    let mut result = vec![settings.clone()];
    if !settings.adaptive_analysis || settings.effective_provider() != "lmstudio" {
        return result;
    }
    let mut next = settings.clone();
    for effort in ["medium", "low", "none"] {
        let rank = |s: &str| match s {
            "xhigh" => 4,
            "high" => 3,
            "medium" => 2,
            "low" => 1,
            _ => 0,
        };
        if rank(effort) < rank(&next.reasoning_effort) {
            next.reasoning_effort = effort.into();
            result.push(next.clone());
        }
    }
    // Keep the output ceiling: a truncated answer must never be 'fixed' by cutting it.
    for percent in [70, 45, 25] {
        next.max_analysis_input_characters =
            (settings.max_analysis_input_characters * percent / 100).max(4000);
        next.model_context_length = (settings.model_context_length * percent as u32 / 100)
            .max(settings.max_output_tokens + 4096)
            .min(settings.model_context_length);
        if next.max_analysis_input_characters < result.last().unwrap().max_analysis_input_characters
        {
            result.push(next.clone());
        }
    }
    result
}

pub fn retryable(error: &AppError) -> bool {
    match error.code {
        ErrorCode::AiTimeout
        | ErrorCode::AiOutputTokenLimit
        | ErrorCode::AiFinalContentEmpty
        | ErrorCode::AiInvalidJson
        | ErrorCode::AiSchemaValidationFailed
        | ErrorCode::LmstudioInvalidResponse => true,
        ErrorCode::LmstudioConnectionFailed | ErrorCode::LmstudioModelNotFound => error
            .response_diagnostics
            .as_ref()
            .and_then(|d| d.http_status)
            .is_some_and(|status| [400, 413, 422, 500, 503, 507].contains(&status)),
        _ => false,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalysisProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading: Option<super::staged::ReadingReport>,
    pub attempts: usize,
    pub input_budget: usize,
    pub input_characters: usize,
    pub output_tokens: u32,
    pub reasoning_effort: String,
    pub requested_context_length: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn each_paper_starts_high_and_keeps_output_budget() {
        let s = AppSettings {
            reasoning_effort: "high".into(),
            max_analysis_input_characters: 32000,
            ..Default::default()
        };
        let p = profiles(&s);
        assert_eq!(p[0].reasoning_effort, "high");
        assert_eq!(p[1].max_analysis_input_characters, 32000);
        assert_eq!(p.last().unwrap().max_analysis_input_characters, 8000);
        assert!(p.iter().all(|p| p.max_output_tokens == s.max_output_tokens));
        assert_eq!(profiles(&s)[0].max_analysis_input_characters, 32000);
        assert!(!retryable(&AppError::new(
            ErrorCode::AiCancelled,
            "cancelled"
        )));
        assert!(!retryable(&AppError::new(
            ErrorCode::LmstudioConnectionFailed,
            "offline"
        )));
    }
}
