//! Connection domain model — types for the persistence and consumption layers.
//!
//! Two metadata kinds on ConnectionRecord:
//! - Identity (human-facing): who connected, to what, how, when
//! - Auth config (machine-facing): how the broker should use this credential
//!
//! AuthPlan is the consumption-layer output: an execution-ready value
//! that the broker consumes without knowing the credential's origin.

use serde::{Deserialize, Serialize};
use std::fmt;

// =============================================================================
// Connection Record — Persistence Layer
// =============================================================================

/// Durable connection record. Two metadata kinds: identity + auth config.
///
/// Serialized to TOML as `[identity]` + `[auth]` sections with a
/// top-level `schema_version`. No secrets — vault holds encrypted
/// credential values separately.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionRecord {
    /// Schema version for forward compatibility (current: 0).
    pub schema_version: u64,

    /// Connection identity metadata (human-facing).
    pub identity: ConnectionIdentity,

    /// Durable auth configuration (machine-facing).
    pub auth: AuthConfig,
}

/// Connection identity metadata — who is connected, to what, how, when.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionIdentity {
    /// Unique key and filename stem (e.g., "github").
    pub name: String,
    /// Which provider definition (e.g., "github").
    pub provider: String,
    /// Who authenticated (e.g., "octocat"). Populated by probe_account().
    pub account_id: Option<String>,
    /// How the credential was obtained.
    pub auth_method: AuthMethod,
    /// OAuth/API scopes (e.g., ["repo", "read:org"]).
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Default connection for this provider.
    #[serde(default)]
    pub is_default: bool,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 last-updated timestamp.
    pub updated_at: String,
    /// ISO 8601 last successful health check.
    pub last_validated: Option<String>,
    /// Connection scope (v1: always Global).
    #[serde(default)]
    pub scope: ConnectionScope,
}

/// How a credential was obtained.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    /// Acquired via OAuth device flow or similar.
    OAuth,
    /// User pasted a token / provided a vault secret reference.
    Manual,
}

/// Connection scope — where this connection lives.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionScope {
    /// User-level: `~/.patina/connections/` (v1 default).
    #[default]
    Global,
}

// =============================================================================
// Auth Config — Durable, machine-facing configuration
// =============================================================================

/// Durable auth configuration. Tells the broker HOW to use this
/// connection's credential at runtime. No decrypted secrets here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthConfig {
    /// How to inject the credential into HTTP requests.
    pub injection: InjectionStrategy,
    /// Vault secret name (e.g., "github:default").
    pub secret_ref: String,
    /// Deprecated: formerly named the native child binary to spawn.
    /// Retained for backwards compatibility with stored connection TOML files.
    /// Ignored at runtime — connector capabilities are resolved via provider.
    #[serde(default)]
    pub child: String,
    /// API domains this connection may access.
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// Whether `patina connect refresh` can re-acquire.
    #[serde(default)]
    pub refresh_capable: bool,
    /// ISO 8601 expiry, if known at acquisition time.
    pub expires_at: Option<String>,
    /// Last auth failure message (for status display).
    pub last_error: Option<String>,
}

/// How to inject a credential into HTTP requests.
/// Drives broker/http.rs dispatch — strategy, not provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum InjectionStrategy {
    /// `Authorization: Bearer {token}`
    Bearer,
    /// Custom header: `{name}: {value}`
    Header { name: String },
    /// Raw token delivered via pipe/initialize to child, not HTTP.
    InProcess,
}

// =============================================================================
// Auth Plan — Consumption Layer
// =============================================================================

/// Execution-ready auth for the broker. No TOML, no vault access,
/// no provider knowledge — just what the broker needs to run.
#[derive(Debug, Clone)]
pub struct AuthPlan {
    /// Resolved credential + injection strategy. None if auth not required.
    pub credential: Option<ResolvedCredential>,
    /// API domains allowed for proxied HTTP.
    pub allowed_domains: Vec<String>,
}

/// Decrypted credential paired with its injection strategy.
#[derive(Debug, Clone)]
pub struct ResolvedCredential {
    /// The decrypted secret value.
    pub value: String,
    /// How to inject this credential.
    pub injection: InjectionStrategy,
}

// =============================================================================
// Connection Status — computed from durable metadata
// =============================================================================

/// Connection health, computed from metadata (no live network checks).
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    /// Credential exists in vault, no errors.
    Connected,
    /// Credential not found in vault.
    Missing,
    /// `expires_at` is in the past.
    Expired,
    /// `last_error` is set (last auth attempt failed).
    Errored,
    /// Never validated (`last_validated` is None).
    Unchecked,
}

/// Summary for connection listing — name, provider, account, computed status.
#[derive(Debug, Clone)]
pub struct ConnectionSummary {
    pub name: String,
    pub provider: String,
    pub account_id: Option<String>,
    pub status: ConnectionStatus,
}

// =============================================================================
// Errors
// =============================================================================

/// Typed errors for connection operations.
/// Each variant produces an actionable error message.
#[derive(Debug)]
pub enum ConnectError {
    /// Connection TOML not found at expected path.
    NotFound { name: String },
    /// Connection TOML exists but fails to parse.
    MalformedConfig { name: String, detail: String },
    /// Vault secret referenced by connection doesn't exist.
    CredentialMissing {
        connection: String,
        secret_ref: String,
    },
    /// Vault decryption failed (identity unavailable, corrupted vault).
    VaultError { connection: String, detail: String },
    /// Connection is referenced by sources.toml entries.
    InUse {
        connection: String,
        references: Vec<String>,
    },
    /// I/O error during store operations.
    IoError { detail: String },
    /// Provider not found in registry.
    UnknownProvider { provider: String },
    /// Credential acquisition failed (OAuth flow, manual entry, etc.).
    AcquisitionFailed { provider: String, detail: String },
    /// User cancelled the operation.
    Cancelled,
}

impl fmt::Display for ConnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectError::NotFound { name } => {
                write!(f, "connection '{}' not found", name)
            }
            ConnectError::MalformedConfig { name, detail } => {
                write!(f, "connection '{}' has invalid config: {}", name, detail)
            }
            ConnectError::CredentialMissing {
                connection,
                secret_ref,
            } => {
                write!(
                    f,
                    "connection '{}': credential '{}' not found in vault \
                     (run `patina connect refresh {}` to re-acquire)",
                    connection, secret_ref, connection
                )
            }
            ConnectError::VaultError { connection, detail } => {
                write!(f, "connection '{}': vault error: {}", connection, detail)
            }
            ConnectError::InUse {
                connection,
                references,
            } => {
                write!(
                    f,
                    "connection '{}' is referenced by:\n{}",
                    connection,
                    references
                        .iter()
                        .map(|r| format!("  {}", r))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
            ConnectError::IoError { detail } => {
                write!(f, "connection I/O error: {}", detail)
            }
            ConnectError::UnknownProvider { provider } => {
                write!(f, "unknown provider '{}' (available: github)", provider)
            }
            ConnectError::AcquisitionFailed { provider, detail } => {
                write!(
                    f,
                    "credential acquisition failed for '{}': {}",
                    provider, detail
                )
            }
            ConnectError::Cancelled => {
                write!(f, "operation cancelled by user")
            }
        }
    }
}

impl std::error::Error for ConnectError {}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_oauth_record() -> ConnectionRecord {
        ConnectionRecord {
            schema_version: 0,
            identity: ConnectionIdentity {
                name: "github".into(),
                provider: "github".into(),
                account_id: Some("octocat".into()),
                auth_method: AuthMethod::OAuth,
                scopes: vec!["repo".into(), "read:org".into()],
                is_default: true,
                created_at: "2026-03-09T17:00:00Z".into(),
                updated_at: "2026-03-09T17:00:00Z".into(),
                last_validated: None,
                scope: ConnectionScope::Global,
            },
            auth: AuthConfig {
                injection: InjectionStrategy::Bearer,
                secret_ref: "github:default".into(),
                child: "github".into(),
                allowed_domains: vec!["api.github.com".into()],
                refresh_capable: true,
                expires_at: None,
                last_error: None,
            },
        }
    }

    fn sample_manual_record() -> ConnectionRecord {
        ConnectionRecord {
            schema_version: 0,
            identity: ConnectionIdentity {
                name: "github-ci".into(),
                provider: "github".into(),
                account_id: None,
                auth_method: AuthMethod::Manual,
                scopes: vec![],
                is_default: false,
                created_at: "2026-03-09T18:00:00Z".into(),
                updated_at: "2026-03-09T18:00:00Z".into(),
                last_validated: None,
                scope: ConnectionScope::Global,
            },
            auth: AuthConfig {
                injection: InjectionStrategy::Bearer,
                secret_ref: "github-ci-token".into(),
                child: "github".into(),
                allowed_domains: vec!["api.github.com".into()],
                refresh_capable: false,
                expires_at: None,
                last_error: None,
            },
        }
    }

    #[test]
    fn toml_round_trip_oauth() {
        let record = sample_oauth_record();
        let toml_str = toml::to_string_pretty(&record).unwrap();
        let parsed: ConnectionRecord = toml::from_str(&toml_str).unwrap();
        assert_eq!(record, parsed);
    }

    #[test]
    fn toml_round_trip_manual() {
        let record = sample_manual_record();
        let toml_str = toml::to_string_pretty(&record).unwrap();
        let parsed: ConnectionRecord = toml::from_str(&toml_str).unwrap();
        assert_eq!(record, parsed);
    }

    #[test]
    fn toml_format_has_expected_sections() {
        let record = sample_oauth_record();
        let toml_str = toml::to_string_pretty(&record).unwrap();

        // Must have [identity] and [auth] sections
        assert!(
            toml_str.contains("[identity]"),
            "missing [identity] section"
        );
        assert!(toml_str.contains("[auth]"), "missing [auth] section");
        // Must have schema_version at top level
        assert!(
            toml_str.contains("schema_version = 0"),
            "missing schema_version"
        );
    }

    #[test]
    fn toml_oauth_method_serializes_lowercase() {
        let record = sample_oauth_record();
        let toml_str = toml::to_string_pretty(&record).unwrap();
        assert!(
            toml_str.contains("auth_method = \"oauth\""),
            "auth_method should serialize as lowercase 'oauth', got:\n{}",
            toml_str
        );
    }

    #[test]
    fn toml_manual_method_serializes_lowercase() {
        let record = sample_manual_record();
        let toml_str = toml::to_string_pretty(&record).unwrap();
        assert!(
            toml_str.contains("auth_method = \"manual\""),
            "auth_method should serialize as lowercase 'manual'"
        );
    }

    #[test]
    fn toml_injection_bearer_format() {
        let record = sample_oauth_record();
        let toml_str = toml::to_string_pretty(&record).unwrap();
        // Tagged enum with type = "bearer"
        assert!(
            toml_str.contains("[auth.injection]"),
            "injection should be a table"
        );
        assert!(
            toml_str.contains("type = \"bearer\""),
            "injection type should be 'bearer'"
        );
    }

    #[test]
    fn toml_injection_header_round_trip() {
        let mut record = sample_oauth_record();
        record.auth.injection = InjectionStrategy::Header {
            name: "X-Api-Key".into(),
        };
        let toml_str = toml::to_string_pretty(&record).unwrap();
        let parsed: ConnectionRecord = toml::from_str(&toml_str).unwrap();
        assert_eq!(record, parsed);
        assert!(toml_str.contains("type = \"header\""));
        assert!(toml_str.contains("name = \"X-Api-Key\""));
    }

    #[test]
    fn toml_injection_inprocess_round_trip() {
        let mut record = sample_oauth_record();
        record.auth.injection = InjectionStrategy::InProcess;
        let toml_str = toml::to_string_pretty(&record).unwrap();
        let parsed: ConnectionRecord = toml::from_str(&toml_str).unwrap();
        assert_eq!(record, parsed);
        assert!(toml_str.contains("type = \"inprocess\""));
    }

    #[test]
    fn parse_hand_authored_toml() {
        // Simulate what a user would hand-write (minimal format)
        let input = r#"
schema_version = 0

[identity]
name = "github"
provider = "github"
auth_method = "manual"
created_at = "2026-03-09T17:00:00Z"
updated_at = "2026-03-09T17:00:00Z"

[auth]
injection = { type = "bearer" }
secret_ref = "github-token"
child = "github"
allowed_domains = ["api.github.com"]
"#;
        let record: ConnectionRecord = toml::from_str(input).unwrap();
        assert_eq!(record.identity.name, "github");
        assert_eq!(record.identity.auth_method, AuthMethod::Manual);
        assert_eq!(record.auth.injection, InjectionStrategy::Bearer);
        assert_eq!(record.auth.secret_ref, "github-token");
        assert!(!record.auth.refresh_capable); // default false
        assert!(record.identity.scopes.is_empty()); // default empty
        assert!(!record.identity.is_default); // default false
        assert_eq!(record.identity.scope, ConnectionScope::Global); // default
    }

    #[test]
    fn connect_error_display_messages() {
        let err = ConnectError::CredentialMissing {
            connection: "github".into(),
            secret_ref: "github:default".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("github"));
        assert!(msg.contains("github:default"));
        assert!(msg.contains("patina connect refresh"));

        let err = ConnectError::InUse {
            connection: "github".into(),
            references: vec!["/home/user/project → source \"gh\"".into()],
        };
        let msg = err.to_string();
        assert!(msg.contains("is referenced by"));
        assert!(msg.contains("/home/user/project"));
    }

    #[test]
    fn connection_status_variants() {
        // Verify all variants exist and are distinct
        let statuses = [
            ConnectionStatus::Connected,
            ConnectionStatus::Missing,
            ConnectionStatus::Expired,
            ConnectionStatus::Errored,
            ConnectionStatus::Unchecked,
        ];
        for (i, a) in statuses.iter().enumerate() {
            for (j, b) in statuses.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }
}
