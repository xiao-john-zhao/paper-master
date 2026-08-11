use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    PdfReadFailed,
    PdfTextEmpty,
    DatabaseWriteFailed,
    DatabaseReadFailed,
    LmstudioConnectionFailed,
    LmstudioModelNotFound,
    AiTimeout,
    AiCancelled,
    AiInvalidJson,
    FileNotFound,
    DuplicatePaper,
    InvalidSettings,
    MigrationFailed,
    Internal,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
            request_id: None,
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn code_string(&self) -> &'static str {
        match self.code {
            ErrorCode::PdfReadFailed => "PDF_READ_FAILED",
            ErrorCode::PdfTextEmpty => "PDF_TEXT_EMPTY",
            ErrorCode::DatabaseWriteFailed => "DATABASE_WRITE_FAILED",
            ErrorCode::DatabaseReadFailed => "DATABASE_READ_FAILED",
            ErrorCode::LmstudioConnectionFailed => "LMSTUDIO_CONNECTION_FAILED",
            ErrorCode::LmstudioModelNotFound => "LMSTUDIO_MODEL_NOT_FOUND",
            ErrorCode::AiTimeout => "AI_TIMEOUT",
            ErrorCode::AiCancelled => "AI_CANCELLED",
            ErrorCode::AiInvalidJson => "AI_INVALID_JSON",
            ErrorCode::FileNotFound => "FILE_NOT_FOUND",
            ErrorCode::DuplicatePaper => "DUPLICATE_PAPER",
            ErrorCode::InvalidSettings => "INVALID_SETTINGS",
            ErrorCode::MigrationFailed => "MIGRATION_FAILED",
            ErrorCode::Internal => "INTERNAL",
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

pub type AppResult<T> = Result<T, AppError>;
