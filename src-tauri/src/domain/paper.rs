use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperAnalysisResult {
    #[serde(default = "analysis_schema_version")]
    pub schema_version: String,
    pub research_problem: String,
    #[serde(default)]
    pub research_scenario: String,
    pub core_method: String,
    #[serde(default)]
    pub key_innovations: Vec<String>,
    pub key_contributions: Vec<String>,
    pub main_results: String,
    pub limitations: String,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_summary: Option<String>,
}

fn analysis_schema_version() -> String {
    "2".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfDiagnostics {
    pub page_count: usize,
    pub extracted_character_count: usize,
    pub detected_sections: Vec<String>,
    pub parser_status: String,
    pub warnings: Vec<String>,
    pub fallback_used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub id: String,
    pub file_path: String,
    pub file_name: String,
    pub title: String,
    pub authors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    #[serde(rename = "abstract", skip_serializing_if = "Option::is_none")]
    pub abstract_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis: Option<PaperAnalysisResult>,
    pub tags: Vec<String>,
    pub analysis_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_diagnostics: Option<PdfDiagnostics>,
    pub created_at: String,
    pub updated_at: String,
}

// Keep the frontend domain name while avoiding Rust's reserved semantic ambiguity.
impl Paper {
    pub fn abstract_value(&self) -> Option<&str> {
        self.abstract_text.as_deref()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaperQuery {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub filter: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportFailure {
    pub path: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportSummary {
    pub operation_id: String,
    pub imported: Vec<Paper>,
    pub duplicates: Vec<String>,
    pub failed: Vec<ImportFailure>,
}
