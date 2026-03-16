//! Mother Child runtime traits and supporting types.
//!
//! `KnowledgeChild` is the target WASM-first runtime contract.
//! `MotherChild` + `Toy` remain only as an explicitly isolated legacy bridge
//! for older shell-toy children while the knowledge-child platform takes over.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

// =========================================================================
// Core Trait
// =========================================================================

/// Knowledge child plugin running in Mother's daemon context.
///
/// Mother owns runtime state, event offsets, and task leases.
/// The child owns workflow and business policy. Typed task intents replace
/// shell command recipes.
pub trait KnowledgeChild: Send + Sync {
    fn name(&self) -> &str;

    fn on_load(&mut self, host: &dyn MotherHost) -> Result<()>;

    fn on_unload(&mut self) {}

    fn health(&self) -> ChildHealth;

    fn handle(&self, request: &ChildRequest) -> Result<ChildResponse>;

    fn drain(&mut self, _limit: u32) -> Result<Vec<PendingEvent>> {
        Ok(vec![])
    }

    fn tick(&mut self) -> Vec<TaskIntent> {
        vec![]
    }
}

/// Legacy shell-toy child plugin.
///
/// Migration-only bridge for the old `mother-child` world. New knowledge
/// children must target `KnowledgeChild`.
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
// Knowledge runtime types
// =========================================================================

/// Task intents are Mother's runtime-substrate request language.
///
/// Children can ask Mother to enqueue work through these typed intents, but the
/// intents are not ordinary domain toys and should not read like a peer
/// capability bundle next to granted toys such as lake or belief.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskIntentKind {
    FetchSource,
    RunQuery,
    EmitFacts,
    MaterializeIndex,
    VerifyBelief,
    SyncGraph,
    RefreshCredential,
    NativeJob,
}

impl TaskIntentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FetchSource => "fetch-source",
            Self::RunQuery => "run-query",
            Self::EmitFacts => "emit-facts",
            Self::MaterializeIndex => "materialize-index",
            Self::VerifyBelief => "verify-belief",
            Self::SyncGraph => "sync-graph",
            Self::RefreshCredential => "refresh-credential",
            Self::NativeJob => "native-job",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "fetch-source" => Some(Self::FetchSource),
            "run-query" => Some(Self::RunQuery),
            "emit-facts" => Some(Self::EmitFacts),
            "materialize-index" => Some(Self::MaterializeIndex),
            "verify-belief" => Some(Self::VerifyBelief),
            "sync-graph" => Some(Self::SyncGraph),
            "refresh-credential" => Some(Self::RefreshCredential),
            "native-job" => Some(Self::NativeJob),
            _ => None,
        }
    }
}

impl fmt::Display for TaskIntentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskIntent {
    pub kind: TaskIntentKind,
    pub payload: serde_json::Value,
    pub dedupe_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingEvent {
    pub stream: String,
    pub offset: u64,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub occurred_at: String,
}

// =========================================================================
// Legacy toys
// =========================================================================

#[derive(Debug, Clone)]
pub struct Toy {
    pub name: String,
    pub command: String,
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
