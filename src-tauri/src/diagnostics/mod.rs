use chrono::Utc;
use serde::Serialize;
use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel { Trace, Debug, Info, Warn, Error }

#[derive(Debug, Clone, Serialize)]
pub struct LogEvent {
    pub timestamp: String,
    pub level: LogLevel,
    pub module: String,
    pub operation: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u128>,
}

impl LogEvent {
    pub fn info(module: impl Into<String>, operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(LogLevel::Info, module, operation, message)
    }

    pub fn error(module: impl Into<String>, operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(LogLevel::Error, module, operation, message)
    }

    pub fn new(level: LogLevel, module: impl Into<String>, operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            level,
            module: module.into(),
            operation: operation.into(),
            message: message.into(),
            error_code: None,
            request_id: None,
            duration_ms: None,
        }
    }
}

pub struct AppLogger {
    path: PathBuf,
    file: Mutex<std::fs::File>,
    recent_errors: Mutex<VecDeque<LogEvent>>,
}

impl AppLogger {
    pub fn new(app_data_dir: &Path) -> std::io::Result<Self> {
        let log_dir = app_data_dir.join("logs");
        fs::create_dir_all(&log_dir)?;
        let path = log_dir.join("paper-master.log");
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self { path, file: Mutex::new(file), recent_errors: Mutex::new(VecDeque::with_capacity(50)) })
    }

    pub fn log(&self, event: LogEvent) {
        if matches!(event.level, LogLevel::Error) {
            if let Ok(mut errors) = self.recent_errors.lock() {
                if errors.len() == 50 { errors.pop_front(); }
                errors.push_back(event.clone());
            }
        }
        if let (Ok(line), Ok(mut file)) = (serde_json::to_string(&event), self.file.lock()) {
            let _ = writeln!(file, "{line}");
        }
    }

    pub fn path(&self) -> &Path { &self.path }

    pub fn recent_errors(&self) -> Vec<LogEvent> {
        self.recent_errors.lock().map(|items| items.iter().cloned().collect()).unwrap_or_default()
    }
}

