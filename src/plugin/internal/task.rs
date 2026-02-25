//! Task world — bindgen + host functions + TaskEngine.
//!
//! On-demand action plugins: analyze AND act, then exit.
//! Structurally closest to command world (init, name, description, run)
//! plus toys (from mother-child) and HTTP (from host-http).

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::Store;

use super::mother_child::PluginEngine;
use super::{wasm_engine, GrantedCapabilities, PluginManifest};
use crate::mother::Toy;

use super::command::QueryDispatchFn;

// =========================================================================
// Task world — bindgen + host functions + TaskEngine
// =========================================================================

/// Bindgen for the task world (separate from command and mother-child).
/// Task plugins have ALL host capabilities: log, layer, query, types, http.
mod task_bindings {
    /// Host state for task plugins — union of command and HTTP fields.
    pub struct TaskHostState {
        pub plugin_name: String,
        pub wasi: wasmtime_wasi::WasiCtx,
        pub wasi_table: wasmtime::component::ResourceTable,
        /// Cached project root — computed once at store creation.
        pub project_root: Option<std::path::PathBuf>,
        /// Resolved capabilities for call-time gating.
        pub grants: super::GrantedCapabilities,
        /// Query dispatch — provided by binary crate, handles engine calls.
        /// None for plugins without query grants.
        pub query_fn: Option<super::QueryDispatchFn>,
        /// Pre-configured HTTP client with cross-domain redirect rejection.
        pub http_client: reqwest::blocking::Client,
    }

    impl wasmtime_wasi::WasiView for TaskHostState {
        fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
            wasmtime_wasi::WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.wasi_table,
            }
        }
    }

    wasmtime::component::bindgen!({
        path: "wit/task/",
        world: "task",
    });

    // patina:host/log — delegates to host_support
    impl patina::host::log::Host for TaskHostState {
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

    // patina:host/types — no functions, empty Host trait
    impl patina::host::types::Host for TaskHostState {}

    // patina:host/layer — delegates to host_support
    impl patina::host::layer::Host for TaskHostState {
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
    impl patina::host::query::Host for TaskHostState {
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

    // patina:host/http — delegates to host_support
    impl patina::host::http::Host for TaskHostState {
        fn http_post(
            &mut self,
            url: String,
            body: String,
            content_type: String,
        ) -> Result<patina::host::http::HttpResponse, String> {
            let r = super::super::host_support::http_post(
                &self.http_client,
                &self.grants,
                &self.plugin_name,
                &url,
                &body,
                &content_type,
            )?;
            Ok(patina::host::http::HttpResponse {
                status: r.status,
                body: r.body,
            })
        }

        fn http_get(&mut self, url: String) -> Result<patina::host::http::HttpResponse, String> {
            let r = super::super::host_support::http_get(
                &self.http_client,
                &self.grants,
                &self.plugin_name,
                &url,
            )?;
            Ok(patina::host::http::HttpResponse {
                status: r.status,
                body: r.body,
            })
        }
    }

    // patina:host/measure — delegates to host_support
    impl patina::host::measure::Host for TaskHostState {
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

/// Task plugin engine — loads and runs task world WASM plugins.
///
/// On-demand action plugins with full host access (log, layer, query,
/// types, HTTP) plus toy intents. CLI creates this for one-shot use.
pub struct TaskEngine {
    linker: Linker<task_bindings::TaskHostState>,
}

impl TaskEngine {
    /// Create a new TaskEngine for one-shot CLI plugin use.
    pub fn new() -> Result<Self> {
        let mut linker = Linker::new(wasm_engine());
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        task_bindings::Task::add_to_linker::<
            task_bindings::TaskHostState,
            wasmtime::component::HasSelf<task_bindings::TaskHostState>,
        >(&mut linker, |s| s)?;
        Ok(Self { linker })
    }

    /// Load a WASM component from bytes.
    pub fn load_component(&self, wasm: &[u8]) -> Result<Component> {
        Component::new(wasm_engine(), wasm)
    }

    /// Run a task plugin. Returns exit code and filtered toy list.
    ///
    /// Checks capabilities from the manifest before execution.
    /// After run(), calls toys() and filters through allowed_toy_commands.
    pub fn run_task(
        &self,
        component: &Component,
        manifest: &PluginManifest,
        args: &[String],
        query_fn: Option<QueryDispatchFn>,
    ) -> Result<(i32, Vec<Toy>)> {
        // Check capabilities before execution
        PluginEngine::check_capabilities(manifest)?;

        // Build HTTP client with cross-domain redirect rejection (G5: shared builder).
        let http_client = super::host_support::build_http_client()?;

        let wasi = wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .build();
        let project_root = crate::session::SessionManager::find_project_root().ok();
        let grants = manifest.granted_capabilities();
        let host_state = task_bindings::TaskHostState {
            plugin_name: manifest.name.clone(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
            project_root,
            grants,
            query_fn,
            http_client,
        };
        let mut store = Store::new(wasm_engine(), host_state);
        let instance = task_bindings::Task::instantiate(&mut store, component, &self.linker)?;

        // Initialize plugin
        instance.call_init(&mut store)?;

        // Run with args
        let exit_code = instance.call_run(&mut store, args)?;

        // Get toy intents and filter through allowed commands
        let wasm_toys = instance.call_toys(&mut store)?;
        let filtered_toys = wasm_toys
            .into_iter()
            .filter_map(|t| {
                let toy = Toy {
                    name: t.name,
                    command: t.command,
                    args: t.args,
                };
                if manifest.allowed_toy_commands.contains(&toy.command) {
                    Some(toy)
                } else {
                    eprintln!(
                        "[plugin:{}] toy '{}' denied: command '{}' not in allowed list {:?}",
                        manifest.name, toy.name, toy.command, manifest.allowed_toy_commands
                    );
                    None
                }
            })
            .collect();

        Ok((exit_code, filtered_toys))
    }

    /// Get the task name from a WASM plugin.
    pub fn get_task_name(&self, component: &Component) -> Result<String> {
        let host_state = Self::probe_host_state();
        let mut store = Store::new(wasm_engine(), host_state);
        let instance = task_bindings::Task::instantiate(&mut store, component, &self.linker)?;
        instance.call_init(&mut store)?;
        instance.call_name(&mut store)
    }

    /// Get the task description from a WASM plugin.
    pub fn get_task_description(&self, component: &Component) -> Result<String> {
        let host_state = Self::probe_host_state();
        let mut store = Store::new(wasm_engine(), host_state);
        let instance = task_bindings::Task::instantiate(&mut store, component, &self.linker)?;
        instance.call_init(&mut store)?;
        instance.call_description(&mut store)
    }

    /// Minimal host state for probing plugin metadata (name/description).
    fn probe_host_state() -> task_bindings::TaskHostState {
        let wasi = wasmtime_wasi::WasiCtxBuilder::new().build();
        let project_root = crate::session::SessionManager::find_project_root().ok();
        task_bindings::TaskHostState {
            plugin_name: "probe".to_string(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
            project_root,
            grants: GrantedCapabilities::default(),
            query_fn: None,
            http_client: reqwest::blocking::Client::new(),
        }
    }
}
