//! Mother - Cross-project awareness layer
//!
//! This module consolidates mother functionality:
//! - **Client**: HTTP client for communicating with `patina serve` daemon
//! - **Graph**: Local SQLite storage for project relationships
//!
//! # Client Usage (containers → mother)
//!
//! Set `PATINA_MOTHER=host:port` to enable remote queries.
//! ```ignore
//! use patina::mother;
//!
//! if mother::is_configured() {
//!     let response = mother::scry(request)?;
//! }
//! ```
//!
//! # Graph Usage (local relationship storage)
//!
//! ```ignore
//! use patina::mother::{Graph, EdgeType};
//!
//! let graph = Graph::open()?;
//! graph.add_edge("patina", "dojo", EdgeType::TestsWith, Some("benchmark"))?;
//! let related = graph.get_related("patina", &[EdgeType::Uses, EdgeType::TestsWith])?;
//! ```

pub mod broker;
pub mod doctor_runtime;
mod internal;
pub mod skills;

// Bridge module: state logic now lives in the mother crate.
// Re-export so existing `crate::mother::state::*` paths continue to compile.
pub(crate) mod state {
    pub use mother_crate::state::*;
}

use anyhow::Result;
use patina_protocol::{
    BuiltinChild, BuiltinChildAction, BuiltinChildRequest, BuiltinChildResult,
    SecretsAuthorityOperation, SecretsDispatchRequest,
};

// Child trait exports
pub use crate::child::runtime::{
    CallCorrelation, Child, ChildCallRequest, ChildHealth, ChildRequest, ChildResponse, MotherHost,
    PendingEvent, TaskIntent, TaskIntentKind, Toy,
};
pub use mother_crate::state::{
    ChildInstallRecord, ChildInstallUpdate, ChildRegistryEntryRecord, ChildRegistryEntryUpdate,
    ChildRegistrySourceRecord, ChildRegistrySourceUpdate, InterfaceKindId, LakeCursorUpdate,
    MotherRuntimeStore, MotherSessionParticipant, MotherSessionRecord, MotherSessionStatus,
    ProjectBeliefStateRecord, ProjectBeliefStateUpdate, ProjectChildAssignmentRecord,
    ProjectChildAssignmentUpdate, ProjectUid, QueuedTask, RunStatus, TaskStatus, VoiceUid,
};
pub use mother_crate::toys::{GrantedIngressSource, GrantedToys};

// Graph exports
pub use crate::beliefs::{
    BeliefEntry, BeliefStatus, Edge, EdgeType, EdgeUsageStats, Graph, Node, NodeType, WeightChange,
    WeightLearningReport, DEFAULT_ALPHA, MIN_SAMPLES, WEIGHT_MAX, WEIGHT_MIN,
};

// Client exports
pub use internal::{Client, ScryRequest, ScryResponse, ScryResultJson};

/// Default port for mother daemon
pub const DEFAULT_PORT: u16 = 50051;

/// Environment variable for mother address
pub const ENV_MOTHER: &str = "PATINA_MOTHER";

/// Optional explicit mother address alias
pub const ENV_MOTHER_ADDR: &str = "PATINA_MOTHER_ADDR";

/// Optional explicit UDS socket override for local mother
pub const ENV_MOTHER_SOCKET: &str = "PATINA_MOTHER_SOCKET";

/// Optional explicit token file override for TCP auth
pub const ENV_MOTHER_TOKEN_FILE: &str = "PATINA_MOTHER_TOKEN_FILE";

/// Legacy environment variable (deprecated, use PATINA_MOTHER)
const ENV_MOTHER_LEGACY: &str = "PATINA_MOTHERSHIP";

/// Check if mother is configured via environment
pub fn is_configured() -> bool {
    // Warn if using legacy env var
    if std::env::var(ENV_MOTHER_LEGACY).is_ok() && std::env::var(ENV_MOTHER).is_err() {
        eprintln!("⚠️  PATINA_MOTHERSHIP is deprecated, use PATINA_MOTHER instead");
    }
    std::env::var(ENV_MOTHER).is_ok()
        || std::env::var(ENV_MOTHER_ADDR).is_ok()
        || std::env::var(ENV_MOTHER_LEGACY).is_ok()
}

/// Get the mother address from environment
/// Returns None if not configured
pub fn get_address() -> Option<String> {
    std::env::var(ENV_MOTHER)
        .or_else(|_| std::env::var(ENV_MOTHER_ADDR))
        .or_else(|_| std::env::var(ENV_MOTHER_LEGACY))
        .ok()
}

/// Resolve the mother address for command/control-plane calls.
pub fn resolve_control_plane_address() -> String {
    get_address().unwrap_or_else(|| format!("localhost:{}", DEFAULT_PORT))
}

/// Create a mother client using canonical control-plane resolution.
pub fn control_plane_client() -> Client {
    Client::new(resolve_control_plane_address())
}

/// Create a client connected to the configured mother
/// Returns None if no mother address env is set
pub fn connect() -> Option<Client> {
    get_address().map(Client::new)
}

/// Check if the mother is reachable (health check)
pub fn is_available() -> bool {
    if let Some(client) = connect() {
        client.health().is_ok()
    } else {
        false
    }
}

/// Query the mother with scry
/// Returns Err if mother is not configured or unreachable
pub fn scry(request: ScryRequest) -> Result<ScryResponse> {
    let client = connect().ok_or_else(|| anyhow::anyhow!("PATINA_MOTHER not set"))?;
    client.scry(request)
}

/// Read a global secret via Mother secrets authority.
pub fn get_global_secret(name: &str) -> Result<Option<String>> {
    let client = control_plane_client();
    let request = BuiltinChildRequest::new(
        BuiltinChild::SecretsAuthority,
        BuiltinChildAction::SecretsDispatch(SecretsDispatchRequest {
            operation: SecretsAuthorityOperation::GetGlobalSecret {
                name: name.to_string(),
            },
        }),
    );
    let response = client.child_action_typed(&request).map_err(|e| {
        anyhow::anyhow!(
            "secrets-authority unavailable via Mother (start with `patina mother start`): {}",
            e
        )
    })?;
    let response = match response.result {
        BuiltinChildResult::Dispatch { payload } => payload,
        other => anyhow::bail!(
            "Unexpected typed response from secrets-authority: {:?}",
            other
        ),
    };

    Ok(response
        .get("value")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_env_vars_cleared<T>(f: impl FnOnce() -> T) -> T {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        let old_mother = std::env::var_os(ENV_MOTHER);
        let old_addr = std::env::var_os(ENV_MOTHER_ADDR);
        let old_legacy = std::env::var_os(ENV_MOTHER_LEGACY);

        unsafe {
            std::env::remove_var(ENV_MOTHER);
            std::env::remove_var(ENV_MOTHER_ADDR);
            std::env::remove_var(ENV_MOTHER_LEGACY);
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        match old_mother {
            Some(value) => unsafe { std::env::set_var(ENV_MOTHER, value) },
            None => unsafe { std::env::remove_var(ENV_MOTHER) },
        }
        match old_addr {
            Some(value) => unsafe { std::env::set_var(ENV_MOTHER_ADDR, value) },
            None => unsafe { std::env::remove_var(ENV_MOTHER_ADDR) },
        }
        match old_legacy {
            Some(value) => unsafe { std::env::set_var(ENV_MOTHER_LEGACY, value) },
            None => unsafe { std::env::remove_var(ENV_MOTHER_LEGACY) },
        }

        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    #[test]
    fn resolve_control_plane_address_defaults_to_localhost() {
        with_env_vars_cleared(|| {
            assert_eq!(
                resolve_control_plane_address(),
                format!("localhost:{}", DEFAULT_PORT)
            );
        });
    }

    #[test]
    fn resolve_control_plane_address_prefers_primary_env() {
        with_env_vars_cleared(|| {
            unsafe {
                std::env::set_var(ENV_MOTHER_ADDR, "addr-alias:1111");
                std::env::set_var(ENV_MOTHER, "primary:2222");
            }
            assert_eq!(resolve_control_plane_address(), "primary:2222");
        });
    }
}
