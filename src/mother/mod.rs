//! Mother - Cross-project awareness layer
//!
//! This module consolidates mothership functionality:
//! - **Client**: HTTP client for communicating with `patina serve` daemon
//! - **Graph**: Local SQLite storage for project relationships
//!
//! # Client Usage (containers → mothership)
//!
//! Set `PATINA_MOTHERSHIP=host:port` to enable remote queries.
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

/// Default port for mothership daemon
pub const DEFAULT_PORT: u16 = 50051;

/// Environment variable for mothership address
pub const ENV_MOTHERSHIP: &str = "PATINA_MOTHERSHIP";

/// Check if mothership is configured via environment
pub fn is_configured() -> bool {
    std::env::var(ENV_MOTHERSHIP).is_ok()
}

/// Get the mothership address from environment
/// Returns None if not configured
pub fn get_address() -> Option<String> {
    std::env::var(ENV_MOTHERSHIP).ok()
}

/// Create a client connected to the configured mothership
/// Returns None if PATINA_MOTHERSHIP is not set
pub fn connect() -> Option<Client> {
    get_address().map(Client::new)
}

/// Check if the mothership is reachable (health check)
pub fn is_available() -> bool {
    if let Some(client) = connect() {
        client.health().is_ok()
    } else {
        false
    }
}

/// Query the mothership with scry
/// Returns Err if mothership is not configured or unreachable
pub fn scry(request: ScryRequest) -> Result<ScryResponse> {
    let client = connect().ok_or_else(|| anyhow::anyhow!("PATINA_MOTHERSHIP not set"))?;
    client.scry(request)
}
