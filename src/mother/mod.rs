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

mod graph;
mod internal;

use anyhow::Result;

// Graph exports
pub use graph::{
    Edge, EdgeType, EdgeUsageStats, Graph, Node, NodeType, WeightChange, WeightLearningReport,
    DEFAULT_ALPHA, MIN_SAMPLES, WEIGHT_MAX, WEIGHT_MIN,
};

// Client exports
pub use internal::{Client, ScryRequest, ScryResponse, ScryResultJson};

/// Default port for mother daemon
pub const DEFAULT_PORT: u16 = 50051;

/// Environment variable for mother address
pub const ENV_MOTHER: &str = "PATINA_MOTHER";

/// Legacy environment variable (deprecated, use PATINA_MOTHER)
const ENV_MOTHER_LEGACY: &str = "PATINA_MOTHERSHIP";

/// Check if mother is configured via environment
pub fn is_configured() -> bool {
    // Warn if using legacy env var
    if std::env::var(ENV_MOTHER_LEGACY).is_ok() && std::env::var(ENV_MOTHER).is_err() {
        eprintln!("⚠️  PATINA_MOTHERSHIP is deprecated, use PATINA_MOTHER instead");
    }
    std::env::var(ENV_MOTHER).is_ok() || std::env::var(ENV_MOTHER_LEGACY).is_ok()
}

/// Get the mother address from environment
/// Returns None if not configured
pub fn get_address() -> Option<String> {
    std::env::var(ENV_MOTHER)
        .or_else(|_| std::env::var(ENV_MOTHER_LEGACY))
        .ok()
}

/// Create a client connected to the configured mother
/// Returns None if PATINA_MOTHER is not set
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
