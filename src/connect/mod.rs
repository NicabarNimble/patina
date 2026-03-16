//! Connection subsystem — lifecycle, domain model, and auth resolution.
//!
//! Three layers:
//! - **Acquisition**: per-provider credential obtainment (Provider trait)
//! - **Persistence**: ConnectionRecord + vault substrate (no secrets in TOML)
//! - **Consumption**: `resolve_auth()` → AuthPlan for broker dispatch
//!
//! # Types
//!
//! ```text
//! ConnectionRecord  →  resolve_auth()  →  AuthPlan
//!   (durable TOML)       (vault seam)      (execution-ready)
//! ```

mod access;
mod internal;
pub(crate) mod providers;

// Domain model types
pub use access::{require_auth_plan_domain, resolve_credential_for_domain};
pub use internal::model::{
    AuthConfig, AuthMethod, AuthPlan, ConnectError, ConnectionIdentity, ConnectionRecord,
    ConnectionScope, ConnectionStatus, ConnectionSummary, InjectionStrategy, ResolvedCredential,
};

// Provider types
pub use providers::{AcquisitionResult, Provider};

// Store operations (persistence layer)

/// Load a connection by name from `~/.patina/connections/{name}.toml`.
pub fn load(name: &str) -> Result<ConnectionRecord, ConnectError> {
    internal::store::load(name)
}

/// List all connections with computed status.
pub fn list() -> Result<Vec<ConnectionSummary>, ConnectError> {
    internal::store::list()
}

/// Create a new connection: writes TOML record + vault credential.
pub fn create(record: &ConnectionRecord, credential: &str) -> Result<(), ConnectError> {
    internal::store::create(record, credential)
}

/// Remove a connection: deletes TOML + vault entry (checks references first).
pub fn remove(name: &str, force: bool) -> Result<(), ConnectError> {
    internal::store::remove(name, force)
}

// Provider registry (acquisition layer)

/// Look up a provider by name (e.g., "github").
pub fn provider(name: &str) -> Result<Box<dyn Provider>, ConnectError> {
    providers::get(name).ok_or_else(|| ConnectError::UnknownProvider {
        provider: name.to_string(),
    })
}

/// List all registered provider names.
pub fn available_providers() -> Vec<&'static str> {
    providers::list_providers()
}

// Auth resolution (consumption layer)

/// Resolve a ConnectionRecord into an execution-ready AuthPlan.
///
/// This is the ONLY place that decrypts vault material for connection use.
/// FAIL CLOSED: missing credential = error, not warning.
pub fn resolve_auth(record: &ConnectionRecord) -> Result<AuthPlan, ConnectError> {
    internal::resolve::resolve_auth(record)
}
