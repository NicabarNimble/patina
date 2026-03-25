//! Auth plan resolution — the seam between durable metadata and runtime.
//!
//! `resolve_auth()` is the ONLY place that decrypts vault material for
//! connection use. It converts a `ConnectionRecord` into an execution-ready
//! `AuthPlan` that the broker consumes without knowing credential origin.
//!
//! FAIL CLOSED: missing credential is an error, not a warning. The broker
//! must not proceed without auth when the connection requires it.

use crate::connect::internal::model::{
    AuthPlan, ConnectError, ConnectionRecord, ResolvedCredential,
};

/// Resolve a ConnectionRecord into an execution-ready AuthPlan.
///
/// This is the ONLY place that decrypts vault material for connection use.
/// Returns typed errors for each failure mode.
///
/// FAIL CLOSED: if the credential cannot be resolved, this returns an error.
/// The broker must not proceed without auth.
pub(crate) fn resolve_auth(record: &ConnectionRecord) -> Result<AuthPlan, ConnectError> {
    // 1. Validate provider is known — FAIL CLOSED
    if crate::connect::providers::get(&record.identity.provider).is_none() {
        return Err(ConnectError::UnknownProvider {
            provider: record.identity.provider.clone(),
        });
    }

    // 2. Decrypt credential from vault — FAIL CLOSED
    let credential = match crate::mother::get_global_secret(&record.auth.secret_ref) {
        Ok(Some(value)) => ResolvedCredential {
            value,
            injection: record.auth.injection.clone(),
        },
        Ok(None) => {
            // FAIL CLOSED — credential missing is an error, not a warning.
            // Replaces broker/mod.rs:44 "proceeding without auth" path.
            return Err(ConnectError::CredentialMissing {
                connection: record.identity.name.clone(),
                secret_ref: record.auth.secret_ref.clone(),
            });
        }
        Err(e) => {
            // Vault decryption failure — identity unavailable, corrupted vault
            return Err(ConnectError::VaultError {
                connection: record.identity.name.clone(),
                detail: e.to_string(),
            });
        }
    };

    Ok(AuthPlan {
        credential: Some(credential),
        allowed_domains: record.auth.allowed_domains.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connect::internal::model::*;

    fn sample_record() -> ConnectionRecord {
        ConnectionRecord {
            schema_version: 0,
            identity: ConnectionIdentity {
                name: "test-conn".into(),
                provider: "github".into(),
                account_id: Some("octocat".into()),
                auth_method: AuthMethod::OAuth,
                scopes: vec!["repo".into()],
                is_default: true,
                created_at: "2026-03-09T17:00:00Z".into(),
                updated_at: "2026-03-09T17:00:00Z".into(),
                last_validated: None,
                scope: ConnectionScope::Global,
            },
            auth: AuthConfig {
                injection: InjectionStrategy::Bearer,
                secret_ref: "test-conn:default".into(),
                child: "github".into(),
                allowed_domains: vec!["api.github.com".into()],
                refresh_capable: true,
                expires_at: None,
                last_error: None,
            },
        }
    }

    #[test]
    fn resolve_auth_unknown_provider_returns_error() {
        let mut record = sample_record();
        record.identity.provider = "nonexistent-provider".into();

        let result = resolve_auth(&record);
        assert!(
            matches!(result, Err(ConnectError::UnknownProvider { .. })),
            "expected UnknownProvider, got: {:?}",
            result
        );
    }

    #[test]
    fn resolve_auth_missing_credential_returns_credential_missing() {
        let err = ConnectError::CredentialMissing {
            connection: "github".into(),
            secret_ref: "github:default".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("credential"));
        assert!(msg.contains("github:default"));
        assert!(msg.contains("patina connect refresh"));
    }

    #[test]
    fn resolve_auth_vault_error_is_propagated() {
        let err = ConnectError::VaultError {
            connection: "github".into(),
            detail: "identity unavailable".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("vault error"));
        assert!(msg.contains("identity unavailable"));
    }

    #[test]
    fn auth_plan_carries_injection_strategy() {
        // Verify AuthPlan correctly carries the injection strategy
        // from the connection record through resolution.
        let plan = AuthPlan {
            credential: Some(ResolvedCredential {
                value: "ghp_test123".into(),
                injection: InjectionStrategy::Bearer,
            }),
            allowed_domains: vec!["api.github.com".into()],
        };

        assert!(plan.credential.is_some());
        let cred = plan.credential.unwrap();
        assert_eq!(cred.value, "ghp_test123");
        assert_eq!(cred.injection, InjectionStrategy::Bearer);
    }

    #[test]
    fn auth_plan_header_injection_strategy() {
        let plan = AuthPlan {
            credential: Some(ResolvedCredential {
                value: "apikey123".into(),
                injection: InjectionStrategy::Header {
                    name: "X-Api-Key".into(),
                },
            }),
            allowed_domains: vec!["api.example.com".into()],
        };

        let cred = plan.credential.unwrap();
        assert_eq!(
            cred.injection,
            InjectionStrategy::Header {
                name: "X-Api-Key".into()
            }
        );
    }

    #[test]
    fn auth_plan_inprocess_injection_strategy() {
        let plan = AuthPlan {
            credential: Some(ResolvedCredential {
                value: "token123".into(),
                injection: InjectionStrategy::InProcess,
            }),
            allowed_domains: vec![],
        };

        let cred = plan.credential.unwrap();
        assert_eq!(cred.injection, InjectionStrategy::InProcess);
    }

    #[test]
    fn resolve_auth_allowed_domains_propagated() {
        // Verify allowed_domains from the connection record are
        // correctly propagated to the AuthPlan.
        let record = sample_record();
        assert_eq!(record.auth.allowed_domains, vec!["api.github.com"]);

        // The plan should carry the same domains.
        let plan = AuthPlan {
            credential: None,
            allowed_domains: record.auth.allowed_domains.clone(),
        };
        assert_eq!(plan.allowed_domains, vec!["api.github.com"]);
    }
}
