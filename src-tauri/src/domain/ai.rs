use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct AnalysisRequest {
    pub paper_id: String,
    pub request_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompareRequest {
    pub paper_ids: Vec<String>,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperComparisonResult {
    pub common_points: Vec<String>,
    pub key_differences: Vec<String>,
    pub method_comparison: String,
    pub contribution_comparison: String,
    pub summary: String,
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
