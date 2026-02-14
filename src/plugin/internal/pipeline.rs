//! Pipeline world — bindgen + host functions + PipelineEngine.
//!
//! Host-invoked pure-compute plugins. Grammar parsers, chunkers, tokenizers.
//! The simplest world: log-only import, no query, no layer, no HTTP, no toys.

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::Store;

use super::{wasm_engine, PluginManifest};

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

    // patina:host/log — same implementation as all other worlds
    impl patina::host::log::Host for PipelineHostState {
        fn log(&mut self, level: patina::host::log::LogLevel, message: String) {
            let level_str = match level {
                patina::host::log::LogLevel::Debug => "DEBUG",
                patina::host::log::LogLevel::Info => "INFO",
                patina::host::log::LogLevel::Warn => "WARN",
                patina::host::log::LogLevel::Error => "ERROR",
            };
            eprintln!("[plugin:{}] {}: {}", self.plugin_name, level_str, message);
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

    /// Load a WASM component from bytes.
    pub fn load_component(&self, wasm: &[u8]) -> Result<Component> {
        Component::new(wasm_engine(), wasm)
    }

    /// Invoke a pipeline plugin with a request envelope.
    /// Returns the JSON response or error string.
    pub fn handle(
        &self,
        component: &Component,
        manifest: &PluginManifest,
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
}
