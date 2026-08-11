use std::path::PathBuf;

pub const DEFAULT_LM_STUDIO_URL: &str = "http://127.0.0.1:1234/v1";
pub const DEFAULT_TEMPERATURE: f32 = 0.15;
pub const DEFAULT_TIMEOUT_MS: u64 = 120_000;
pub const MAX_ANALYSIS_INPUT_CHARACTERS: usize = 24_000;
pub const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 8_192;
pub const AI_STRUCTURED_RETRY_COUNT: usize = 1;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub database_path: PathBuf,
    pub log_path: PathBuf,
}

impl AppPaths {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            database_path: data_dir.join("paper-master.db"),
            log_path: data_dir.join("logs").join("paper-master.log"),
            data_dir,
        }
    }
}
