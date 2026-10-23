use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct AnalysisRequest {
    pub paper_id: String,
    pub request_id: String,
    #[serde(default)]
    pub ensure_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompareRequest {
    pub paper_ids: Vec<String>,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PaperComparisonResult {
    pub schema_version: String,
    pub common_points: Vec<String>,
    pub key_differences: Vec<String>,
    pub problem_comparison: String,
    pub scenario_comparison: String,
    pub method_comparison: String,
    pub contribution_comparison: String,
    pub results_comparison: String,
    pub limitations_comparison: String,
    pub evidence: Vec<ComparisonEvidence>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ComparisonEvidence {
    pub paper_number: usize,
    pub quote: String,
}

impl PaperComparisonResult {
    pub fn response_json_schema() -> serde_json::Value {
        serde_json::to_value(schema_for!(PaperComparisonResult))
            .expect("PaperComparisonResult schema must serialize")
    }
}

#[derive(Debug, Clone, Default)]
pub struct AIResponseDiagnostics {
    pub http_status: Option<u16>,
    pub finish_reason: Option<String>,
    pub content_present: bool,
    pub content_length: usize,
    pub reasoning_present: bool,
    pub reasoning_length: usize,
    pub tool_calls_present: bool,
    pub duration_ms: Option<u128>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionResult {
    pub state: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u128>,
    pub models: Vec<String>,
    pub loaded_models: Vec<String>,
}
