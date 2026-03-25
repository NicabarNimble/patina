//! Command world — bindgen + host functions + CommandEngine.

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::Store;

use super::{check_capabilities, wasm_engine, ChildManifest, GrantedCapabilities};

/// Query dispatch function type.
///
/// Provided by the binary crate at runtime since query engines
/// (retrieval, commands) live in the binary, not the library.
/// The host impl handles gating; this function handles dispatch.
pub type QueryDispatchFn = Box<dyn FnMut(&str, &str) -> Result<String, String> + Send>;

// =========================================================================
// Command world — bindgen + host functions + CommandEngine
// =========================================================================

/// Bindgen for the command world (separate from knowledge-child).
/// Each world needs its own host state because command children
/// import patina:host/layer (project data access) while
/// knowledge-child children do not.
mod command_bindings {
    /// Host state for command children — includes layer and query access.
    pub struct CommandHostState {
        pub plugin_name: String,
        pub wasi: wasmtime_wasi::WasiCtx,
        pub wasi_table: wasmtime::component::ResourceTable,
        /// Cached project root — computed once at store creation.
        pub project_root: Option<std::path::PathBuf>,
        /// Resolved capabilities for call-time gating.
        pub grants: super::GrantedCapabilities,
        /// Query dispatch — provided by binary crate, handles engine calls.
        /// None for children without query grants (probe, etc).
        pub query_fn: Option<super::QueryDispatchFn>,
    }

    impl wasmtime_wasi::WasiView for CommandHostState {
        fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
            wasmtime_wasi::WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.wasi_table,
            }
        }
    }

    wasmtime::component::bindgen!({
        path: "wit/command/",
        world: "command",
    });

    // patina:host/log — delegates to host_support
    impl patina::host::log::Host for CommandHostState {
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

    // patina:host/layer — delegates to host_support
    impl patina::host::layer::Host for CommandHostState {
        fn find_project_root(&mut self) -> Option<String> {
            super::super::host_support::find_project_root(&self.project_root)
        }
        fn read_config(&mut self) -> Result<String, String> {
            super::super::host_support::read_config(&self.project_root)
        }
        fn detect_environment(&mut self) -> Result<String, String> {
            super::super::host_support::detect_environment()
        }
        fn get_stored_tools(&mut self) -> Vec<String> {
            super::super::host_support::get_stored_tools(&self.project_root)
        }
        fn count_layer_files(&mut self, subdir: String) -> u32 {
            super::super::host_support::count_layer_files(&self.project_root, &subdir)
        }
        fn get_project_uid(&mut self) -> Option<String> {
            super::super::host_support::get_project_uid(&self.project_root)
        }
        fn check_adapter_version(
            &mut self,
            adapter_name: String,
        ) -> Result<Option<String>, String> {
            super::super::host_support::check_adapter_version(&self.project_root, &adapter_name)
        }
    }

    // patina:host/query — delegates to host_support
    impl patina::host::query::Host for CommandHostState {
        fn query(&mut self, kind: String, params: String) -> Result<String, String> {
            super::super::host_support::query(
                &self.plugin_name,
                &self.grants,
                &mut self.query_fn,
                &kind,
                &params,
            )
        }
    }

    // patina:host/measure — delegates to host_support
    impl patina::host::measure::Host for CommandHostState {
        fn record_measurement(
            &mut self,
            verb: String,
            tool: String,
            mode: String,
            metrics_json: String,
        ) -> Result<(), String> {
            super::super::host_support::record_measurement(
                &self.project_root,
                &self.plugin_name,
                &verb,
                &tool,
                &mode,
                &metrics_json,
            )
        }
    }
}

/// Command child engine — loads and runs command world WASM children.
///
/// Separate from knowledge-child engine because command children use a different
/// WIT world with different imports (patina:host/layer for project
/// data access). CLI creates this for one-shot use without the daemon.
pub struct CommandEngine {
    linker: Linker<command_bindings::CommandHostState>,
}

impl CommandEngine {
    /// Create a new CommandEngine for one-shot CLI child use.
    pub fn new() -> Result<Self> {
        let mut linker = Linker::new(wasm_engine());
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        command_bindings::Command::add_to_linker::<
            command_bindings::CommandHostState,
            wasmtime::component::HasSelf<command_bindings::CommandHostState>,
        >(&mut linker, |s| s)?;
        Ok(Self { linker })
    }

    /// Load a WASM component from bytes.
    pub fn load_component(&self, wasm: &[u8]) -> Result<Component> {
        Component::new(wasm_engine(), wasm)
    }

    /// Run a command child. Returns exit code.
    ///
    /// Checks capabilities from the manifest before execution — matches
    /// Shared manifest capability validation pattern.
    ///
    /// `query_fn`: Optional query dispatch provided by the binary crate.
    /// Required if the child has host_query capabilities. The host impl
    /// handles gating; this function handles actual engine dispatch.
    pub fn run_command(
        &self,
        component: &Component,
        manifest: &ChildManifest,
        args: &[String],
        query_fn: Option<QueryDispatchFn>,
    ) -> Result<i32> {
        check_capabilities(manifest)?;

        let wasi = wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .build();
        let project_root = crate::session::SessionManager::find_project_root().ok();
        let grants = manifest.granted_capabilities();
        let host_state = command_bindings::CommandHostState {
            plugin_name: manifest.name.clone(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
            project_root,
            grants,
            query_fn,
        };
        let mut store = Store::new(wasm_engine(), host_state);
        let instance = command_bindings::Command::instantiate(&mut store, component, &self.linker)?;

        // Initialize child
        instance.call_init(&mut store)?;

        // Run with args
        instance.call_run(&mut store, args)
    }

    /// Get the command name from a WASM child.
    pub fn get_command_name(&self, component: &Component) -> Result<String> {
        let host_state = Self::probe_host_state();
        let mut store = Store::new(wasm_engine(), host_state);
        let instance = command_bindings::Command::instantiate(&mut store, component, &self.linker)?;
        instance.call_init(&mut store)?;
        instance.call_name(&mut store)
    }

    /// Get the command description from a WASM child.
    pub fn get_command_description(&self, component: &Component) -> Result<String> {
        let host_state = Self::probe_host_state();
        let mut store = Store::new(wasm_engine(), host_state);
        let instance = command_bindings::Command::instantiate(&mut store, component, &self.linker)?;
        instance.call_init(&mut store)?;
        instance.call_description(&mut store)
    }

    /// Minimal host state for probing child metadata (name/description).
    fn probe_host_state() -> command_bindings::CommandHostState {
        let wasi = wasmtime_wasi::WasiCtxBuilder::new().build();
        let project_root = crate::session::SessionManager::find_project_root().ok();
        command_bindings::CommandHostState {
            plugin_name: "probe".to_string(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
            project_root,
            grants: GrantedCapabilities::default(),
            query_fn: None,
        }
    }
}
