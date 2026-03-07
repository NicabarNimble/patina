//! Plugin engine — shared wasmtime infrastructure for WASM plugins.
//!
//! Four engines for four worlds:
//! - `PluginEngine` — mother-child world (daemon resident children)
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
    CommandEngine, CredentialMapping, GrantedCapabilities, InjectionLocation, PipelineEngine,
    PluginEngine, PluginManifest, PluginProvides, PluginRole, PluginWorld, QueryDispatchFn,
    QueryScope, TaskEngine,
};
