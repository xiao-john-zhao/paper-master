use serde::{Deserialize, Serialize};

use crate::config::{
    DEFAULT_LM_STUDIO_URL, DEFAULT_MAX_OUTPUT_TOKENS, DEFAULT_TEMPERATURE, DEFAULT_TIMEOUT_MS,
    MAX_ANALYSIS_INPUT_CHARACTERS,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub ai_provider: String,
    pub base_url: String,
    pub model: String,
    pub temperature: f32,
    pub request_timeout_ms: u64,
    pub max_analysis_input_characters: usize,
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u32,
    pub log_level: String,
    pub privacy_mode: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            ai_provider: "lmstudio".to_string(),
            base_url: DEFAULT_LM_STUDIO_URL.to_string(),
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
