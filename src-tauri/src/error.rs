use serde::Serialize;

use crate::domain::ai::AIResponseDiagnostics;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    PdfReadFailed,
    PdfTextEmpty,
    DatabaseWriteFailed,
    DatabaseReadFailed,
    LmstudioConnectionFailed,
    LmstudioModelNotFound,
    LmstudioInvalidResponse,
    AiFinalContentEmpty,
    AiOutputTokenLimit,
    AiSchemaValidationFailed,
    ModelStructuredOutputIncompatible,
    AiTimeout,
    AiCancelled,
    AiInvalidJson,
    FileNotFound,
    DuplicatePaper,
    PaperNotAnalyzed,
    InvalidSettings,
    MigrationFailed,
    Internal,
    CodexUnavailable,
    ResearchFailed,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip)]
    pub response_diagnostics: Option<AIResponseDiagnostics>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
            request_id: None,
            stage: None,
            response_diagnostics: None,
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

    pub fn stage(mut self, stage: impl Into<String>) -> Self {
        self.stage = Some(stage.into());
        self
    }

    pub fn response_diagnostics(mut self, diagnostics: AIResponseDiagnostics) -> Self {
        self.response_diagnostics = Some(diagnostics);
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
            ErrorCode::LmstudioInvalidResponse => "LMSTUDIO_INVALID_RESPONSE",
            ErrorCode::AiFinalContentEmpty => "AI_FINAL_CONTENT_EMPTY",
            ErrorCode::AiOutputTokenLimit => "AI_OUTPUT_TOKEN_LIMIT",
            ErrorCode::AiSchemaValidationFailed => "AI_SCHEMA_VALIDATION_FAILED",
            ErrorCode::ModelStructuredOutputIncompatible => "MODEL_STRUCTURED_OUTPUT_INCOMPATIBLE",
            ErrorCode::AiTimeout => "AI_TIMEOUT",
            ErrorCode::AiCancelled => "AI_CANCELLED",
            ErrorCode::AiInvalidJson => "AI_INVALID_JSON",
            ErrorCode::FileNotFound => "FILE_NOT_FOUND",
            ErrorCode::DuplicatePaper => "DUPLICATE_PAPER",
            ErrorCode::PaperNotAnalyzed => "PAPER_NOT_ANALYZED",
            ErrorCode::InvalidSettings => "INVALID_SETTINGS",
            ErrorCode::MigrationFailed => "MIGRATION_FAILED",
            ErrorCode::Internal => "INTERNAL",
            ErrorCode::CodexUnavailable => "CODEX_UNAVAILABLE",
            ErrorCode::ResearchFailed => "RESEARCH_FAILED",
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
