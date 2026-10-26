use serde::{Deserialize, Serialize};

use crate::config::{
    DEFAULT_LM_STUDIO_URL, DEFAULT_MAX_OUTPUT_TOKENS, DEFAULT_TEMPERATURE, DEFAULT_TIMEOUT_MS,
    MAX_ANALYSIS_INPUT_CHARACTERS,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub ai_provider: String,
    #[serde(default = "default_auto_analyze")]
    pub auto_analyze_on_import: bool,
    #[serde(default = "default_reasoning_effort")]
    pub reasoning_effort: String,
    #[serde(default = "default_context_length")]
    pub model_context_length: u32,
    pub base_url: String,
    #[serde(default)]
    pub api_base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_model: String,
    #[serde(default = "default_api_format")]
    pub api_response_format: String,
    #[serde(default)]
    pub api_reasoning_supported: bool,
    #[serde(default = "default_auto_analyze")]
    pub adaptive_analysis: bool,
    pub model: String,
    pub temperature: f32,
    pub request_timeout_ms: u64,
    pub max_analysis_input_characters: usize,
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u32,
    pub log_level: String,
    pub privacy_mode: String,
}

impl AppSettings {
    pub fn effective_provider(&self) -> &str {
        if self.ai_provider == "codex" && self.privacy_mode != "codex_online" {
            "lmstudio"
        } else {
            &self.ai_provider
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            ai_provider: "lmstudio".to_string(),
            auto_analyze_on_import: true,
            reasoning_effort: default_reasoning_effort(),
            model_context_length: default_context_length(),
            base_url: DEFAULT_LM_STUDIO_URL.to_string(),
            api_base_url: String::new(),
            api_key: String::new(),
            api_model: String::new(),
            api_response_format: default_api_format(),
            api_reasoning_supported: false,
            adaptive_analysis: true,
            model: String::new(),
            temperature: DEFAULT_TEMPERATURE,
            request_timeout_ms: DEFAULT_TIMEOUT_MS,
            max_analysis_input_characters: MAX_ANALYSIS_INPUT_CHARACTERS,
            max_output_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
            log_level: if cfg!(debug_assertions) {
                "DEBUG"
            } else {
                "INFO"
            }
            .to_string(),
            privacy_mode: "local_only".to_string(),
        }
    }
}

fn default_max_output_tokens() -> u32 {
    DEFAULT_MAX_OUTPUT_TOKENS
}

fn default_api_format() -> String {
    "json_object".into()
}

fn default_auto_analyze() -> bool {
    true
}
fn default_reasoning_effort() -> String {
    "none".into()
}

fn default_context_length() -> u32 {
    24_576
}
