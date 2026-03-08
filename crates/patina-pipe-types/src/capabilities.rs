use serde::{Deserialize, Serialize};

/// Child-declared capabilities returned during pipe/initialize.
///
/// Tells Mother what this child can do — which data types it produces,
/// whether it supports incremental fetching, and who the provider is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub provider: String,
    pub data_types: Vec<String>,
    pub supports_incremental: bool,
}

/// Health check response from pipe/health.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub status: HealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Health states for pipe/health responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Ok,
    Degraded,
    Down,
}
