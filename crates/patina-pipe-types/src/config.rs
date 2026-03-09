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

/// Parameters for a pipe/ingest request (Mother → storage child).
///
/// Mother sends a bounded batch of records to a lakehouse child.
/// The child writes, dedup-checks, and returns a result.
/// Hard specification in [[raw-lake-ingestion]] DESIGN.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestParams {
    /// Absolute path to the lake root directory.
    pub lake_path: String,
    /// Persona owning this lake ("default" pre-federation).
    pub persona: String,
    /// Provider name (e.g., "github").
    pub provider: String,
    /// Source path within the lake (e.g., "NicabarNimble/patina").
    pub source_path: String,
    /// Schema name for validation.
    pub schema: String,
    /// Schema version string.
    pub schema_version: String,
    /// Per-fact-type identity fields for dedup (fact_type → field names).
    #[serde(default)]
    pub identity_fields: std::collections::HashMap<String, Vec<String>>,
    /// The records to ingest.
    pub records: Vec<IngestRecord>,
}

/// A single record in a pipe/ingest batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRecord {
    /// Event type (e.g., "github.issue").
    pub event_type: String,
    /// Pre-serialized JSON data string.
    pub data: String,
    /// Content hash for dedup ("blake3:<hex>").
    pub content_hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_params_round_trip_with_auth() {
        let original = InitializeParams {
            protocol_version: "1.0".to_string(),
            auth: Some(AuthConfig {
                token: "ghp_secret".to_string(),
                provider: "github".to_string(),
            }),
        };

        let json = serde_json::to_value(&original).unwrap();
        let restored: InitializeParams = serde_json::from_value(json).unwrap();

        assert_eq!(restored.protocol_version, "1.0");
        let auth = restored.auth.unwrap();
        assert_eq!(auth.token, "ghp_secret");
        assert_eq!(auth.provider, "github");
    }

    #[test]
    fn initialize_params_round_trip_without_auth() {
        let original = InitializeParams {
            protocol_version: "1.0".to_string(),
            auth: None,
        };

        let json = serde_json::to_value(&original).unwrap();
        // Verify auth is omitted from JSON (skip_serializing_if)
        assert!(json.get("auth").is_none());

        let restored: InitializeParams = serde_json::from_value(json).unwrap();
        assert_eq!(restored.protocol_version, "1.0");
        assert!(restored.auth.is_none());
    }

    #[test]
    fn fetch_params_round_trip_full() {
        let original = FetchParams {
            types: vec!["issues".to_string(), "prs".to_string()],
            since: Some("2026-03-07T00:00:00Z".to_string()),
            limit: 100,
            params: serde_json::json!({"owner": "NicabarNimble", "repo": "patina"}),
        };

        let json = serde_json::to_value(&original).unwrap();
        let restored: FetchParams = serde_json::from_value(json).unwrap();

        assert_eq!(restored.types, vec!["issues", "prs"]);
        assert_eq!(restored.since.as_deref(), Some("2026-03-07T00:00:00Z"));
        assert_eq!(restored.limit, 100);
        assert_eq!(restored.params["owner"], "NicabarNimble");
        assert_eq!(restored.params["repo"], "patina");
    }

    #[test]
    fn fetch_params_round_trip_minimal() {
        let original = FetchParams {
            types: vec![],
            since: None,
            limit: 10_000,
            params: serde_json::Value::Null,
        };

        let json = serde_json::to_value(&original).unwrap();
        // since omitted from JSON (skip_serializing_if)
        assert!(json.get("since").is_none());

        let restored: FetchParams = serde_json::from_value(json).unwrap();
        assert!(restored.types.is_empty());
        assert!(restored.since.is_none());
        assert_eq!(restored.limit, 10_000);
    }
}
