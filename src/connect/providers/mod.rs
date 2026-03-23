//! Provider registry — acquisition interface for connection providers.
//!
//! Each provider implements the `Provider` trait, which defines how to
//! acquire credentials interactively (OAuth, device flow) or manually
//! (user-supplied token). The registry maps provider names to implementations.
//!
//! Adding a new provider requires:
//! 1. Create `src/connect/providers/<name>.rs`
//! 2. Implement `Provider` trait
//! 3. Add to `get()` in this module

pub(crate) mod github;

use crate::connect::internal::model::{AuthMethod, ConnectError, InjectionStrategy};

/// Result of a successful credential acquisition.
///
/// Returned by `Provider::acquire()` and `Provider::acquire_manual()`.
/// Contains everything needed to populate a `ConnectionRecord`.
pub struct AcquisitionResult {
    /// The secret value to store in vault.
    pub credential: String,
    /// Who authenticated (e.g., GitHub login). Populated by `probe_account()`.
    pub account_id: Option<String>,
    /// How the credential was obtained.
    pub auth_method: AuthMethod,
    /// OAuth/API scopes granted.
    pub scopes: Vec<String>,
    /// ISO 8601 expiry, if known.
    pub expires_at: Option<String>,
}

/// Provider-specific credential acquisition.
///
/// Each provider implements this trait. The connect module calls it
/// during `patina connect <provider>`. The trait is the boundary
/// between provider-specific code and the uniform connection model.
pub trait Provider {
    /// Provider identifier (e.g., "github").
    fn name(&self) -> &str;

    /// Acquire a credential interactively.
    /// Returns the raw credential string to store in vault.
    fn acquire(&self) -> Result<AcquisitionResult, ConnectError>;

    /// Acquire a credential from a manually-provided value.
    /// For --manual mode (user has a token already).
    fn acquire_manual(&self, token: &str) -> Result<AcquisitionResult, ConnectError>;

    /// Probe account identity (e.g., call /user endpoint).
    /// Returns a display name for the authenticated account.
    fn probe_account(&self, credential: &str) -> Result<Option<String>, ConnectError>;

    /// Default OAuth/API scopes for this provider.
    fn default_scopes(&self) -> Vec<String>;

    /// Default injection strategy for this provider.
    fn default_injection(&self) -> InjectionStrategy;

    /// Allowed API domains for this provider.
    fn allowed_domains(&self) -> Vec<String>;
}

/// Look up a provider by name. Returns None for unknown providers.
pub(crate) fn get(name: &str) -> Option<Box<dyn Provider>> {
    match name {
        "github" => Some(Box::new(github::GitHubProvider)),
        _ => None,
    }
}

/// List all registered provider names.
pub(crate) fn list_providers() -> Vec<&'static str> {
    vec!["github"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_github_provider() {
        let p = get("github");
        assert!(p.is_some());
        assert_eq!(p.unwrap().name(), "github");
    }

    #[test]
    fn get_unknown_returns_none() {
        assert!(get("slack").is_none());
        assert!(get("").is_none());
    }

    #[test]
    fn list_includes_github() {
        let providers = list_providers();
        assert!(providers.contains(&"github"));
    }

    /// Mock provider for testing the trait interface.
    struct MockProvider {
        provider_name: &'static str,
        scopes: Vec<String>,
    }

    impl Provider for MockProvider {
        fn name(&self) -> &str {
            self.provider_name
        }

        fn acquire(&self) -> Result<AcquisitionResult, ConnectError> {
            Ok(AcquisitionResult {
                credential: "mock-token-123".into(),
                account_id: Some("mock-user".into()),
                auth_method: AuthMethod::OAuth,
                scopes: self.scopes.clone(),
                expires_at: None,
            })
        }

        fn acquire_manual(&self, token: &str) -> Result<AcquisitionResult, ConnectError> {
            Ok(AcquisitionResult {
                credential: token.to_string(),
                account_id: None,
                auth_method: AuthMethod::Manual,
                scopes: vec![],
                expires_at: None,
            })
        }

        fn probe_account(&self, _credential: &str) -> Result<Option<String>, ConnectError> {
            Ok(Some("mock-user".into()))
        }

        fn default_scopes(&self) -> Vec<String> {
            self.scopes.clone()
        }

        fn default_injection(&self) -> InjectionStrategy {
            InjectionStrategy::Bearer
        }

        fn allowed_domains(&self) -> Vec<String> {
            vec!["api.mock.com".into()]
        }
    }

    #[test]
    fn mock_provider_acquire_populates_result() {
        let provider = MockProvider {
            provider_name: "mock",
            scopes: vec!["read".into(), "write".into()],
        };

        let result = provider.acquire().unwrap();
        assert_eq!(result.credential, "mock-token-123");
        assert_eq!(result.account_id, Some("mock-user".into()));
        assert_eq!(result.auth_method, AuthMethod::OAuth);
        assert_eq!(result.scopes, vec!["read", "write"]);
        assert!(result.expires_at.is_none());
    }

    #[test]
    fn mock_provider_acquire_manual_uses_provided_token() {
        let provider = MockProvider {
            provider_name: "mock",
            scopes: vec![],
        };

        let result = provider.acquire_manual("user-token-abc").unwrap();
        assert_eq!(result.credential, "user-token-abc");
        assert_eq!(result.auth_method, AuthMethod::Manual);
        assert!(result.account_id.is_none());
    }

    #[test]
    fn mock_provider_defaults_populate_connection_fields() {
        let provider = MockProvider {
            provider_name: "custom",
            scopes: vec!["scope-a".into(), "scope-b".into()],
        };

        assert_eq!(provider.name(), "custom");
        assert_eq!(provider.default_scopes(), vec!["scope-a", "scope-b"]);
        assert_eq!(provider.default_injection(), InjectionStrategy::Bearer);
        assert_eq!(provider.allowed_domains(), vec!["api.mock.com"]);
    }

    #[test]
    fn mock_provider_probe_account_returns_identity() {
        let provider = MockProvider {
            provider_name: "mock",
            scopes: vec![],
        };

        let account = provider.probe_account("any-token").unwrap();
        assert_eq!(account, Some("mock-user".into()));
    }

    #[test]
    fn provider_trait_is_object_safe() {
        // Verify the trait can be used as a trait object (Box<dyn Provider>)
        let provider: Box<dyn Provider> = Box::new(MockProvider {
            provider_name: "mock",
            scopes: vec![],
        });
        assert_eq!(provider.name(), "mock");
        let _result = provider.acquire().unwrap();
    }
}
