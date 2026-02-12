//! Plugin engine — shared wasmtime infrastructure for WASM plugins.
//!
//! Two engines for two worlds:
//! - `PluginEngine` — mother-child world (daemon resident children)
//! - `CommandEngine` — command world (one-shot CLI plugins, no daemon)
//!
//! Both share the process-wide wasmtime::Engine singleton.
//!
//! See: layer/surface/build/feat/plugin-system/SPEC.md

mod internal;
pub use internal::{CommandEngine, PluginEngine, PluginManifest, PluginProvides};
