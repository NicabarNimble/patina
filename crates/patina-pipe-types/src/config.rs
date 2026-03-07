use serde::{Deserialize, Serialize};

/// Parameters sent by Mother during pipe/initialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
}

/// Authentication configuration delivered to a child.
///
/// Uses plain String for token — the security boundary is Mother's code
/// (Zeroizing<String> on decrypt, drop after serialization to child stdin),
/// not the types crate. See DESIGN.md decision #2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub token: String,
    pub provider: String,
}

/// Parameters for a pipe/fetch request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchParams {
    pub types: Vec<String>,
    /// Opaque cursor — child-owned, Mother stores and passes back.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
    pub limit: u64,
    /// Provider-specific parameters (e.g., owner/repo for GitHub).
    #[serde(default)]
    pub params: serde_json::Value,
}
