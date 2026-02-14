//! Plugin engine — shared wasmtime infrastructure for WASM plugins.
//!
//! Three engines for three worlds:
//! - `PluginEngine` — mother-child world (daemon resident children)
//! - `CommandEngine` — command world (one-shot CLI plugins, no daemon)
//! - `TaskEngine` — task world (on-demand action plugins, CLI-invoked)
//!
//! All share the process-wide wasmtime::Engine singleton.
//!
//! See: layer/surface/build/feat/plugin-system/SPEC.md

mod internal;
pub use internal::{
    CommandEngine, GrantedCapabilities, PluginEngine, PluginManifest, PluginProvides,
    QueryDispatchFn, QueryScope, TaskEngine,
};
