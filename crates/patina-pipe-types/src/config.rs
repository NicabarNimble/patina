use serde::{Deserialize, Serialize};

/// Parameters sent by Mother during pipe/initialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
    /// Typed capability grant for DuckLake children.
    /// Concrete before generic — no Option<Value> at trust boundaries.
    /// Per [[initialize-is-capability-grant]].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ducklake: Option<DuckLakeGrant>,
}

/// Capability grant for the DuckLake child.
/// Two toys: connector (authority to fetch) and storage (authority to write).
/// Fail closed on missing or malformed.
/// Per [[children-have-agency-toys-are-capabilities]].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuckLakeGrant {
    pub connector: ConnectorToy,
    pub storage: StorageToy,
}

/// Connector toy — indivisible capability bundle.
/// The child derives HTTP enforcement from this grant;
/// the proxy is not a separate toy.
/// Per [[connector-toy-is-indivisible-authority]].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorToy {
    /// Executable identity: "github-connector"
    pub binary: String,
    /// Secret material: OAuth token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
    /// How credential reaches the API
    pub injection: GrantInjection,
    /// Policy boundary: approved domains
    pub allowed_domains: Vec<String>,
    /// Behavior scope: { owner, repo }
    pub params: serde_json::Value,
    /// Behavior scope: ["issues", "prs"]
    pub types: Vec<String>,
}

/// How a credential is injected — wire format for capability grants.
///
/// Parallel to connect::InjectionStrategy but lives in pipe-types
/// (no dependency on the main binary).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum GrantInjection {
    Bearer,
    Header { name: String },
    #[serde(rename = "inprocess")]
    InProcess,
}

/// Storage toy — where the child writes results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageToy {
    /// ~/.patina/lakes/<name>/
    pub lake_path: String,
}

/// Result of pipe/run — the child's full report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    /// Per-type results.
    #[serde(default)]
    pub types: std::collections::HashMap<String, TypeReport>,
    /// Escalation to Mother (auth failures, missing binaries).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalation: Option<Escalation>,
}

/// Per-type fetch+store result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeReport {
    pub status: String,
    #[serde(default)]
    pub written: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Escalation from child to Mother — things the child can't fix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Escalation {
    #[serde(rename = "type")]
    pub escalation_type: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
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
            ducklake: None,
        };

        let json = serde_json::to_value(&original).unwrap();
        let restored: InitializeParams = serde_json::from_value(json).unwrap();

        assert_eq!(restored.protocol_version, "1.0");
        let auth = restored.auth.unwrap();
        assert_eq!(auth.token, "ghp_secret");
        assert_eq!(auth.provider, "github");
        assert!(restored.ducklake.is_none());
    }

    #[test]
    fn initialize_params_round_trip_without_auth() {
        let original = InitializeParams {
            protocol_version: "1.0".to_string(),
            auth: None,
            ducklake: None,
        };

        let json = serde_json::to_value(&original).unwrap();
        // Verify auth and ducklake are omitted from JSON (skip_serializing_if)
        assert!(json.get("auth").is_none());
        assert!(json.get("ducklake").is_none());

        let restored: InitializeParams = serde_json::from_value(json).unwrap();
        assert_eq!(restored.protocol_version, "1.0");
        assert!(restored.auth.is_none());
        assert!(restored.ducklake.is_none());
    }

    #[test]
    fn ducklake_grant_round_trip() {
        let grant = DuckLakeGrant {
            connector: ConnectorToy {
                binary: "github-connector".to_string(),
                credential: Some("ghp_secret".to_string()),
                injection: GrantInjection::Bearer,
                allowed_domains: vec!["api.github.com".to_string()],
                params: serde_json::json!({"owner": "anthropics", "repo": "claude-code"}),
                types: vec!["issues".to_string(), "prs".to_string()],
            },
            storage: StorageToy {
                lake_path: "/tmp/test-lake".to_string(),
            },
        };

        let json = serde_json::to_value(&grant).unwrap();
        let restored: DuckLakeGrant = serde_json::from_value(json).unwrap();

        assert_eq!(restored.connector.binary, "github-connector");
        assert_eq!(restored.connector.credential.as_deref(), Some("ghp_secret"));
        assert_eq!(restored.connector.allowed_domains, vec!["api.github.com"]);
        assert_eq!(restored.connector.types, vec!["issues", "prs"]);
        assert_eq!(restored.storage.lake_path, "/tmp/test-lake");
    }

    #[test]
    fn initialize_params_with_ducklake_grant() {
        let original = InitializeParams {
            protocol_version: "1.0".to_string(),
            auth: None,
            ducklake: Some(DuckLakeGrant {
                connector: ConnectorToy {
                    binary: "github-connector".to_string(),
                    credential: None,
                    injection: GrantInjection::InProcess,
                    allowed_domains: vec![],
                    params: serde_json::json!({}),
                    types: vec!["issues".to_string()],
                },
                storage: StorageToy {
                    lake_path: "/tmp/lake".to_string(),
                },
            }),
        };

        let json = serde_json::to_value(&original).unwrap();
        assert!(json.get("ducklake").is_some());
        assert!(json.get("auth").is_none());

        let restored: InitializeParams = serde_json::from_value(json).unwrap();
        let grant = restored.ducklake.unwrap();
        assert_eq!(grant.connector.binary, "github-connector");
        assert!(grant.connector.credential.is_none());
        assert_eq!(grant.storage.lake_path, "/tmp/lake");
    }

    #[test]
    fn grant_injection_variants_round_trip() {
        // Bearer
        let json = serde_json::to_value(&GrantInjection::Bearer).unwrap();
        assert_eq!(json["type"], "bearer");
        let _: GrantInjection = serde_json::from_value(json).unwrap();

        // Header
        let json =
            serde_json::to_value(&GrantInjection::Header { name: "X-Api-Key".into() }).unwrap();
        assert_eq!(json["type"], "header");
        assert_eq!(json["name"], "X-Api-Key");
        let _: GrantInjection = serde_json::from_value(json).unwrap();

        // InProcess
        let json = serde_json::to_value(&GrantInjection::InProcess).unwrap();
        assert_eq!(json["type"], "inprocess");
        let _: GrantInjection = serde_json::from_value(json).unwrap();
    }

    #[test]
    fn run_result_round_trip() {
        let mut types = std::collections::HashMap::new();
        types.insert(
            "issues".to_string(),
            TypeReport {
                status: "ok".to_string(),
                written: 847,
                cursor: Some("2026-03-12T00:00:00Z".to_string()),
                error: None,
            },
        );
        types.insert(
            "prs".to_string(),
            TypeReport {
                status: "failed".to_string(),
                written: 0,
                cursor: None,
                error: Some("rate limited".to_string()),
            },
        );

        let result = RunResult {
            types,
            escalation: None,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(json.get("escalation").is_none());

        let restored: RunResult = serde_json::from_value(json).unwrap();
        assert_eq!(restored.types["issues"].written, 847);
        assert_eq!(restored.types["prs"].status, "failed");
    }

    #[test]
    fn escalation_round_trip() {
        let result = RunResult {
            types: std::collections::HashMap::new(),
            escalation: Some(Escalation {
                escalation_type: "auth_failed".to_string(),
                message: "GitHub API returned 401".to_string(),
                action: Some("patina connect refresh github".to_string()),
            }),
        };

        let json = serde_json::to_value(&result).unwrap();
        let restored: RunResult = serde_json::from_value(json).unwrap();
        let esc = restored.escalation.unwrap();
        assert_eq!(esc.escalation_type, "auth_failed");
        assert_eq!(esc.action.as_deref(), Some("patina connect refresh github"));
    }

    #[test]
    fn connector_toy_no_credential_omitted() {
        let toy = ConnectorToy {
            binary: "test".to_string(),
            credential: None,
            injection: GrantInjection::Bearer,
            allowed_domains: vec![],
            params: serde_json::json!({}),
            types: vec![],
        };
        let json = serde_json::to_value(&toy).unwrap();
        assert!(json.get("credential").is_none());
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
