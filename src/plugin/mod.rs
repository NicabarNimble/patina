//! Plugin engine — shared wasmtime infrastructure for WASM plugins.
//!
//! PluginEngine holds the wasmtime Engine singleton and Linker.
//! Mother uses it for resident daemon children. CLI uses it for
//! one-shot command plugins. Same WASM loading, same capability
//! grants, same manifest format — different lifecycles.
//!
//! See: layer/surface/build/feat/plugin-system/SPEC.md

mod internal;
pub use internal::{PluginEngine, PluginManifest, PluginProvides};
