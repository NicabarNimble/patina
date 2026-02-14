//! Task world — bindgen + host functions + TaskEngine.
//!
//! On-demand action plugins: analyze AND act, then exit.
//! Structurally closest to command world (init, name, description, run)
//! plus toys (from mother-child) and HTTP (from host-http).

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::Store;

use super::mother_child::PluginEngine;
use super::{wasm_engine, GrantedCapabilities, PluginManifest, QueryScope};
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

    // patina:host/log — same implementation as command and mother-child
    impl patina::host::log::Host for TaskHostState {
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

    // patina:host/types — only defines types (no functions), empty Host trait
    impl patina::host::types::Host for TaskHostState {}

    // patina:host/layer — read-only project data access (copied from command world)
    impl patina::host::layer::Host for TaskHostState {
        fn find_project_root(&mut self) -> Option<String> {
            self.project_root
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
        }

        fn read_config(&mut self) -> Result<String, String> {
            let root = self
                .project_root
                .as_ref()
                .ok_or_else(|| "no project root".to_string())?;
            let config = crate::project::load_with_migration(root)
                .map_err(|e| format!("load config: {}", e))?;
            serde_json::to_string(&config).map_err(|e| format!("serialize config: {}", e))
        }

        fn detect_environment(&mut self) -> Result<String, String> {
            let env = crate::environment::Environment::detect()
                .map_err(|e| format!("detect env: {}", e))?;
            serde_json::to_string(&env).map_err(|e| format!("serialize env: {}", e))
        }

        fn get_stored_tools(&mut self) -> Vec<String> {
            let root = match self.project_root.as_ref() {
                Some(r) => r,
                None => return vec![],
            };
            let config = match crate::project::load_with_migration(root) {
                Ok(c) => c,
                Err(_) => return vec![],
            };
            config
                .environment
                .map(|e| e.detected_tools)
                .unwrap_or_default()
        }

        fn count_layer_files(&mut self, subdir: String) -> u32 {
            let root = match self.project_root.as_ref() {
                Some(r) => r,
                None => return 0,
            };
            let path = root.join("layer").join(&subdir);
            if let Ok(entries) = std::fs::read_dir(path) {
                entries
                    .filter_map(Result::ok)
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                    .count() as u32
            } else {
                0
            }
        }

        fn get_project_uid(&mut self) -> Option<String> {
            let root = self.project_root.as_ref()?;
            crate::project::get_uid(root)
        }

        fn check_adapter_version(
            &mut self,
            adapter_name: String,
        ) -> Result<Option<String>, String> {
            let root = self
                .project_root
                .as_ref()
                .ok_or_else(|| "no project root".to_string())?;
            let adapter = crate::adapters::get_adapter(&adapter_name);
            adapter
                .check_for_updates(root)
                .map(|opt| opt.map(|(current, _)| current))
                .map_err(|e| format!("adapter check: {}", e))
        }
    }

    // patina:host/query — capability-gated query dispatch (copied from command world)
    impl patina::host::query::Host for TaskHostState {
        fn query(&mut self, kind: String, params: String) -> Result<String, String> {
            // Call-time gating: kind must be in granted set
            if !self.grants.query_kinds.contains(&kind) {
                return Err(format!(
                    "query kind '{}' not granted for plugin '{}'",
                    kind, self.plugin_name
                ));
            }

            // Scope enforcement: deny all_repos explicitly, then sanitize
            if let Ok(args) = serde_json::from_str::<serde_json::Value>(&params) {
                let all_repos = args
                    .get("all_repos")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if all_repos && !matches!(self.grants.query_scope, super::QueryScope::AllRepos) {
                    return Err(
                        "all_repos not allowed: plugin query_scope is current_project".to_string(),
                    );
                }
                if all_repos {
                    eprintln!(
                        "[plugin:{}] query: all_repos=true (audit)",
                        self.plugin_name
                    );
                }
            }

            // Sanitize: strip scope-reserved keys
            let sanitized_params =
                super::super::command::sanitize_query_params(&params, &self.grants.query_scope);

            // Delegate to binary-provided dispatch function
            let query_fn = self
                .query_fn
                .as_mut()
                .ok_or_else(|| "query dispatch not configured".to_string())?;
            query_fn(&kind, &sanitized_params)
        }
    }

    // patina:host/http — domain-allowlisted HTTP access (copied from mother-child)
    impl patina::host::http::Host for TaskHostState {
        fn http_post(
            &mut self,
            url: String,
            body: String,
            content_type: String,
        ) -> Result<patina::host::http::HttpResponse, String> {
            let domain = super::super::mother_child::validate_http_url(&url)?;
            if !self.grants.http_domains.contains(&domain) {
                return Err(format!(
                    "domain '{}' not in allowlist for plugin '{}'",
                    domain, self.plugin_name
                ));
            }
            let response = self
                .http_client
                .post(&url)
                .header("Content-Type", &content_type)
                .body(body)
                .send()
                .map_err(|e| format!("HTTP POST failed: {}", e))?;
            let status = response.status().as_u16();
            let resp_body = response.text().map_err(|e| format!("read body: {}", e))?;
            Ok(patina::host::http::HttpResponse {
                status,
                body: resp_body,
            })
        }

        fn http_get(&mut self, url: String) -> Result<patina::host::http::HttpResponse, String> {
            let domain = super::super::mother_child::validate_http_url(&url)?;
            if !self.grants.http_domains.contains(&domain) {
                return Err(format!(
                    "domain '{}' not in allowlist for plugin '{}'",
                    domain, self.plugin_name
                ));
            }
            let response = self
                .http_client
                .get(&url)
                .send()
                .map_err(|e| format!("HTTP GET failed: {}", e))?;
            let status = response.status().as_u16();
            let resp_body = response.text().map_err(|e| format!("read body: {}", e))?;
            Ok(patina::host::http::HttpResponse {
                status,
                body: resp_body,
            })
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

        // Build HTTP client with cross-domain redirect rejection
        let http_client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.url().host_str() != attempt.previous().last().and_then(|u| u.host_str())
                {
                    attempt.stop()
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .map_err(|e| anyhow::anyhow!("build HTTP client: {}", e))?;

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
