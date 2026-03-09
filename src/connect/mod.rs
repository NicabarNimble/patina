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
    ConnectionScope, ConnectionStatus, InjectionStrategy, ResolvedCredential,
};
