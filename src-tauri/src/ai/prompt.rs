pub const ANALYSIS_SYSTEM_PROMPT: &str = r#"Analyze only the supplied scientific paper content. Return one JSON object with exactly these fields:
research_problem (string), core_method (string), key_contributions (array of strings), main_results (string), limitations (string), tags (array of 3-8 strings), ai_summary (string).
Rules: Use only supported information. Do not invent numerical results or methods. Keep the paper's technical terminology. Describe what the authors actually propose. Output English. Do not output reasoning, chain-of-thought, Markdown, or any text outside the JSON object."#;

pub const COMPARISON_SYSTEM_PROMPT: &str = r#"Compare only the two supplied structured paper analyses. Return one JSON object with exactly these fields: common_points (array of strings), key_differences (array of strings), method_comparison (string), contribution_comparison (string), summary (string). Output English. Do not invent evidence. Do not output reasoning, Markdown, or text outside JSON."#;

pub const REPAIR_ANALYSIS_PROMPT: &str = r#"Convert the response below into valid JSON with fields research_problem, core_method, key_contributions, main_results, limitations, tags, and ai_summary. Preserve supported final conclusions only. Remove reasoning. Return JSON only."#;

pub const REPAIR_COMPARISON_PROMPT: &str = r#"Convert the response below into valid JSON with fields common_points, key_differences, method_comparison, contribution_comparison, and summary. Remove reasoning. Return JSON only."#;

pub fn build_analysis_user_prompt(input: &str) -> String {
    format!("PAPER CONTENT START\n{input}\nPAPER CONTENT END\nReturn the requested JSON object.")
}

pub fn build_comparison_user_prompt(input: &str) -> String {
    format!("STRUCTURED PAPER ANALYSES START\n{input}\nSTRUCTURED PAPER ANALYSES END\nReturn the requested comparison JSON object.")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prompt_forbids_reasoning_and_invention() {
        assert!(ANALYSIS_SYSTEM_PROMPT.contains("Do not invent numerical results"));
        assert!(ANALYSIS_SYSTEM_PROMPT.contains("Do not output reasoning"));
    }
}
