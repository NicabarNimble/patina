//! MotherChild trait and supporting types
//!
//! Defines the plugin interface for Mother's children.
//! Children are black boxes that own state and handle requests.
//! Mother iterates children, routes to them, doesn't know their internals.
//!
//! Today these are native Rust traits (dependable-rust pattern).
//! The trait maps to a WIT world — when patina-platform lands,
//! children become WASM plugins. Same shape at both stages.
//!
//! See: layer/surface/build/feat/mother-architecture/SPEC.md

use anyhow::Result;
use std::fmt;

// =========================================================================
// Core Trait
// =========================================================================

/// A child plugin running in Mother's daemon context.
///
/// Children are black boxes: they own some state and handle requests.
/// Mother iterates children, routes to them by name, doesn't know their
/// internals. Children never spawn processes directly — they request
/// toys via `tick()` and Mother runs them.
///
/// # Concurrency
///
/// `handle()` takes `&self` because the daemon serves requests concurrently.
/// Children needing mutable state in handlers use interior mutability (Mutex).
/// `on_load()`, `on_unload()`, and `tick()` take `&mut self` — they run
/// single-threaded in the daemon's lifecycle/heartbeat loop.
pub trait MotherChild: Send + Sync {
    /// Identity — unique name used for request routing
    fn name(&self) -> &str;

    /// Lifecycle — called when Mother loads this child
    fn on_load(&mut self, host: &dyn MotherHost) -> Result<()>;

    /// Lifecycle — called when Mother shuts down
    fn on_unload(&mut self) {}

    /// Health check — Mother calls this on heartbeat
    fn health(&self) -> ChildHealth;

    /// Handle a request routed by Mother
    fn handle(&self, request: &ChildRequest) -> Result<ChildResponse>;

    /// Heartbeat tick — child checks its own state, may request toys.
    ///
    /// Takes &mut self (not &self like handle) because the heartbeat loop
    /// is single-threaded and compiled-in children benefit from direct
    /// mutation without interior mutability overhead. WASM children pay an
    /// adapter cost (Mutex) but that's inherent to wasmtime's &mut Store
    /// requirement, not a flaw in the trait design.
    ///
    /// Zed has no tick/heartbeat equivalent — their extensions are purely
    /// event-driven. Our daemon heartbeat model justifies this split.
    ///
    /// Default: no-op (children that don't need periodic checks skip this).
    fn tick(&mut self) -> Vec<Toy> {
        vec![]
    }
}

// =========================================================================
// Health
// =========================================================================

/// Child health status, returned by health checks.
#[derive(Debug, Clone)]
pub enum ChildHealth {
    /// Child is operating normally
    Healthy,
    /// Child is running but with issues
    Degraded(String),
    /// Child is not functional
    Unhealthy(String),
}

impl fmt::Display for ChildHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChildHealth::Healthy => write!(f, "healthy"),
            ChildHealth::Degraded(reason) => write!(f, "degraded: {}", reason),
            ChildHealth::Unhealthy(reason) => write!(f, "unhealthy: {}", reason),
        }
    }
}

// =========================================================================
// Request / Response
// =========================================================================

/// A request routed to a child by Mother.
///
/// Children receive actions as strings and deserialize the payload
/// into their own types. This maps cleanly to a future WIT interface
/// where actions are string-dispatched.
#[derive(Debug, Clone)]
pub struct ChildRequest {
    /// Action name (child-specific, e.g. "embed_query", "cache")
    pub action: String,
    /// Payload (child deserializes into its own types)
    pub payload: serde_json::Value,
}

/// A response from a child.
#[derive(Debug, Clone)]
pub struct ChildResponse {
    /// Response payload (child serializes from its own types)
    pub payload: serde_json::Value,
}

// =========================================================================
// Toys
// =========================================================================

/// Work that a child asks Mother to run.
///
/// The child decides *what*. Mother handles *how*.
/// Children never spawn processes directly — they return toys
/// from `tick()` and Mother spawns and monitors them.
#[derive(Debug, Clone)]
pub struct Toy {
    /// Descriptive name for logging and status display
    pub name: String,
    /// Shell command to execute
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
}

// =========================================================================
// Host Capabilities
// =========================================================================

/// Capabilities Mother provides to children.
///
/// A child only sees what the host exposes. It cannot reach into
/// other children's state or touch project internals.
///
/// Note: Exact API emerges from implementing children.
/// Methods are added as needs become concrete.
pub trait MotherHost: Send + Sync {
    /// Structured logging on behalf of a child
    fn log(&self, child: &str, message: &str);
}
