//! Pipeline world — bindgen + host functions + PipelineEngine.
//!
//! Host-invoked pure-compute plugins. Grammar parsers, chunkers, tokenizers.
//! The simplest world: log-only import, no query, no layer, no HTTP, no toys.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};

use super::{wasm_engine, ChildKind, ChildManifest};

// =========================================================================
// Pipeline world — bindgen + host functions + PipelineEngine
// =========================================================================

/// Bindgen for the pipeline world (simplest world — log-only import).
mod pipeline_bindings {
    /// Host state for pipeline plugins — minimal (no grants, no HTTP, no query).
    pub struct PipelineHostState {
        pub plugin_name: String,
        pub wasi: wasmtime_wasi::WasiCtx,
        pub wasi_table: wasmtime::component::ResourceTable,
    }

    impl wasmtime_wasi::WasiView for PipelineHostState {
        fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
            wasmtime_wasi::WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.wasi_table,
            }
        }
    }

    wasmtime::component::bindgen!({
        path: "wit/pipeline/",
        world: "pipeline",
    });

    // patina:host/log — delegates to host_support
    impl patina::host::log::Host for PipelineHostState {
        fn log(&mut self, level: patina::host::log::LogLevel, message: String) {
            let level_str = match level {
                patina::host::log::LogLevel::Debug => "DEBUG",
                patina::host::log::LogLevel::Info => "INFO",
                patina::host::log::LogLevel::Warn => "WARN",
                patina::host::log::LogLevel::Error => "ERROR",
            };
            super::super::host_support::log(&self.plugin_name, level_str, &message);
        }
    }
}

/// Pipeline plugin engine — loads and runs pipeline world WASM plugins.
///
/// Host-invoked pure-compute plugins for grammar parsing, chunking, etc.
/// Simplest engine: log-only import, no capabilities to gate.
pub struct PipelineEngine {
    linker: Linker<pipeline_bindings::PipelineHostState>,
}

impl PipelineEngine {
    /// Create a new PipelineEngine.
    pub fn new() -> Result<Self> {
        let mut linker = Linker::new(wasm_engine());
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        pipeline_bindings::Pipeline::add_to_linker::<
            pipeline_bindings::PipelineHostState,
            wasmtime::component::HasSelf<pipeline_bindings::PipelineHostState>,
        >(&mut linker, |s| s)?;
        Ok(Self { linker })
    }

    /// Load a WASM component from bytes (no caching).
    pub fn load_component(&self, wasm: &[u8]) -> Result<Component> {
        Component::new(wasm_engine(), wasm)
    }

    /// Load a WASM component with AOT cache.
    ///
    /// If a `.cwasm` file exists alongside the `.wasm` and is newer, loads the
    /// pre-compiled native code via mmap (sub-millisecond). Otherwise compiles
    /// from WASM source (~120ms) and writes the `.cwasm` cache for next time.
    ///
    /// Cache invalidation: the `.cwasm` file includes a wasmtime version marker.
    /// If the marker doesn't match the current engine, the cache is rebuilt.
    ///
    /// See [[scrape-diff-driven]] EC5: aot-module-cache.
    pub fn load_component_cached(&self, wasm_path: &Path) -> Result<Component> {
        let cwasm_path = wasm_path.with_extension("cwasm");
        let engine = wasm_engine();

        // Check if cached AOT artifact exists and is fresh
        if cwasm_path.exists() {
            let wasm_mtime = std::fs::metadata(wasm_path).and_then(|m| m.modified()).ok();
            let cwasm_mtime = std::fs::metadata(&cwasm_path)
                .and_then(|m| m.modified())
                .ok();

            if let (Some(wasm_t), Some(cwasm_t)) = (wasm_mtime, cwasm_mtime) {
                if cwasm_t > wasm_t {
                    // SAFETY: The .cwasm file was produced by this same wasmtime
                    // engine version (checked via Engine::detect_precompiled_file).
                    // We control the file contents — it was written by a previous
                    // invocation of this function. The wasmtime engine configuration
                    // (wasm_component_model=true, sync mode) is identical because
                    // it comes from the same OnceLock singleton.
                    if Engine::detect_precompiled_file(&cwasm_path)?.is_some() {
                        match unsafe { Component::deserialize_file(engine, &cwasm_path) } {
                            Ok(component) => return Ok(component),
                            Err(e) => {
                                // Cache corrupted — fall through to recompile
                                eprintln!(
                                    "[pipeline] AOT cache invalid for {}: {} — recompiling",
                                    cwasm_path.display(),
                                    e
                                );
                            }
                        }
                    } else {
                        // Engine version mismatch — delete stale cache
                        let _ = std::fs::remove_file(&cwasm_path);
                    }
                }
            }
        }

        // Cold path: compile from WASM source
        let wasm_bytes = std::fs::read(wasm_path)?;
        let component = Component::new(engine, &wasm_bytes)?;

        // Write AOT cache for next load
        match component.serialize() {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(&cwasm_path, &bytes) {
                    eprintln!(
                        "[pipeline] failed to write AOT cache {}: {}",
                        cwasm_path.display(),
                        e
                    );
                }
            }
            Err(e) => {
                eprintln!("[pipeline] failed to serialize component: {}", e);
            }
        }

        Ok(component)
    }

    /// Invoke a pipeline plugin with a request envelope.
    /// Returns the JSON response or error string.
    pub fn handle(
        &self,
        component: &Component,
        manifest: &ChildManifest,
        request: &str,
    ) -> Result<String> {
        let wasi = wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stderr()
            .build();
        let host_state = pipeline_bindings::PipelineHostState {
            plugin_name: manifest.name.clone(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
        };
        let mut store = Store::new(wasm_engine(), host_state);
        let instance =
            pipeline_bindings::Pipeline::instantiate(&mut store, component, &self.linker)?;

        // Initialize plugin
        instance.call_init(&mut store)?;

        // Invoke handle with the request envelope
        match instance.call_handle(&mut store, request)? {
            Ok(response) => Ok(response),
            Err(e) => Err(anyhow::anyhow!("pipeline plugin error: {}", e)),
        }
    }

    /// Get the pipeline plugin name.
    pub fn get_name(&self, component: &Component) -> Result<String> {
        let host_state = Self::probe_host_state();
        let mut store = Store::new(wasm_engine(), host_state);
        let instance =
            pipeline_bindings::Pipeline::instantiate(&mut store, component, &self.linker)?;
        instance.call_init(&mut store)?;
        instance.call_name(&mut store)
    }

    /// Minimal host state for probing plugin metadata.
    fn probe_host_state() -> pipeline_bindings::PipelineHostState {
        let wasi = wasmtime_wasi::WasiCtxBuilder::new().build();
        pipeline_bindings::PipelineHostState {
            plugin_name: "probe".to_string(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
        }
    }

    /// Discover pipeline plugins from ~/.patina/pipeline/.
    ///
    /// Scans for pipeline manifests, loads WASM components, and builds
    /// a language→(component, manifest) map for dispatch.
    pub fn discover(&self, pipeline_dir: &Path) -> HashMap<String, (Component, ChildManifest)> {
        let mut plugins: HashMap<String, (Component, ChildManifest)> = HashMap::new();

        if !pipeline_dir.is_dir() {
            return plugins;
        }

        let entries = match std::fs::read_dir(pipeline_dir) {
            Ok(e) => e,
            Err(_) => return plugins,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let manifest_path = path.join("plugin.toml");
            let wasm_path = path.join("plugin.wasm");

            if !manifest_path.exists() || !wasm_path.exists() {
                continue;
            }

            let manifest = match ChildManifest::from_path(&manifest_path) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!(
                        "[pipeline] failed to load manifest {}: {}",
                        manifest_path.display(),
                        e
                    );
                    continue;
                }
            };

            if manifest.world != ChildKind::Pipeline {
                continue;
            }

            let component = match self.load_component_cached(&wasm_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[pipeline] failed to load {}: {}", wasm_path.display(), e);
                    continue;
                }
            };

            // Map each claimed language to this plugin
            for lang in &manifest.provides.languages {
                if plugins.contains_key(lang) {
                    eprintln!(
                        "[pipeline] language '{}' already claimed, skipping plugin '{}'",
                        lang, manifest.name
                    );
                    continue;
                }
                eprintln!("[pipeline] {} claims language '{}'", manifest.name, lang);
                // Clone component for each language mapping.
                // Component is cheap to clone (Arc internally).
                plugins.insert(lang.clone(), (component.clone(), manifest.clone()));
            }
        }

        plugins
    }
}
