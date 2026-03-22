//! Child engine bridge — shared wasmtime infrastructure for WASM runtimes.
//!
//! Four engines for four worlds:
//! - `MotherChildEngine` — mother-child world (daemon resident children)
//! - `CommandEngine` — command world (one-shot CLI plugins, no daemon)
//! - `TaskEngine` — task world (on-demand action plugins, CLI-invoked)
//! - `PipelineEngine` — pipeline world (host-invoked pure compute, log-only)
//!
//! All share the process-wide wasmtime::Engine singleton.
//!
//! See: layer/surface/build/feat/plugin-system/SPEC.md

pub(crate) mod internal;
pub mod scaffold;
pub use internal::{
    ChildEngine, ChildKind, ChildManifest, ChildProvides, ChildRole, CommandEngine,
    CredentialMapping, GrantedCapabilities, InjectionLocation, KnowledgeChildEngine,
    PipelineEngine, QueryDispatchFn, QueryScope, TaskEngine,
};

pub use internal::PluginEngine as MotherChildEngine;

// Legacy plugin-era aliases kept during CV6 migration.
pub type PluginEngine = MotherChildEngine;
pub type PluginManifest = ChildManifest;
pub type PluginRole = ChildRole;
pub type PluginWorld = ChildKind;
pub type PluginProvides = ChildProvides;
