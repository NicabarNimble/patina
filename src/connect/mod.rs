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

mod internal;

// Domain model types
pub use internal::model::{
    AuthConfig, AuthMethod, AuthPlan, ConnectError, ConnectionIdentity, ConnectionRecord,
    ConnectionScope, ConnectionStatus, ConnectionSummary, InjectionStrategy, ResolvedCredential,
};

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

// Auth resolution (consumption layer)

/// Resolve a ConnectionRecord into an execution-ready AuthPlan.
///
/// This is the ONLY place that decrypts vault material for connection use.
/// FAIL CLOSED: missing credential = error, not warning.
pub fn resolve_auth(record: &ConnectionRecord) -> Result<AuthPlan, ConnectError> {
    internal::resolve::resolve_auth(record)
}
