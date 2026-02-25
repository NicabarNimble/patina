//! Mother-child world — bindgen, PluginEngine, WasmChild adapter.

use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::Store;

use super::{wasm_engine, GrantedCapabilities, PluginManifest};
use crate::mother::{ChildHealth, ChildRequest, ChildResponse, MotherHost, Toy};

use super::command::QueryDispatchFn;

// =========================================================================
// Bindgen — generates types from WIT definitions
// =========================================================================

/// Generated types + HostState live together so bindgen's HasData/Host
/// trait resolution works correctly. The generated MotherChild type
/// stays internal — WasmChild bridges to our crate::mother::MotherChild trait.
mod bindings {
    /// State passed to WASM plugins via Store<HostState>.
    /// Contains WASI context (wasm32-wasip2 components always import basic WASI),
    /// plugin name for log prefix, and HTTP client for domain-allowlisted access.
    pub struct HostState {
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

    // WasiView is required for wasmtime-wasi to satisfy WASI imports
    impl wasmtime_wasi::WasiView for HostState {
        fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
            wasmtime_wasi::WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.wasi_table,
            }
        }
    }

    wasmtime::component::bindgen!({
        path: "wit/mother-child/",
        world: "mother-child",
    });

    // patina:host/log — delegates to host_support
    impl patina::host::log::Host for HostState {
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
    impl patina::host::types::Host for HostState {}

    // patina:host/layer — delegates to host_support
    impl patina::host::layer::Host for HostState {
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
    impl patina::host::query::Host for HostState {
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
    impl patina::host::http::Host for HostState {
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
    impl patina::host::measure::Host for HostState {
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

use bindings::HostState;

// =========================================================================
// PluginEngine
// =========================================================================

/// Shared wasmtime infrastructure for loading and running WASM plugins.
pub struct PluginEngine {
    linker: Linker<HostState>,
}

impl PluginEngine {
    /// Create a new PluginEngine with host functions registered.
    ///
    /// Create once per process and reuse for all plugin loading. The
    /// underlying wasmtime::Engine is a process-wide singleton (OnceLock),
    /// but Linker setup (WASI + host functions) runs on each call.
    /// In daemon mode, daemon.rs creates one PluginEngine and passes it
    /// to load_wasm_child(). CLI command plugins (Phase 2) will need to
    /// decide whether to share the daemon's engine or create a fresh one.
    pub fn new() -> Result<Self> {
        let mut linker = Linker::new(wasm_engine());

        // Add WASI to linker — wasm32-wasip2 components always import basic WASI
        // (stdio, env, clocks) even for pure-computation code.
        // Using sync linker — no async runtime.
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;

        // Add our custom host functions (patina:host/log, etc.)
        bindings::MotherChild::add_to_linker::<HostState, wasmtime::component::HasSelf<HostState>>(
            &mut linker,
            |s| s,
        )?;
        Ok(Self { linker })
    }

    /// Load and parse a plugin manifest from plugin.toml.
    pub fn load_manifest(path: &Path) -> Result<PluginManifest> {
        PluginManifest::from_path(path)
    }

    /// Load a WASM component from bytes.
    pub fn load_component(&self, wasm: &[u8]) -> Result<Component> {
        PluginManifest::load_component(wasm)
    }

    /// Check that a plugin's requested capabilities are granted.
    ///
    /// Phase 1: host_log, host_layer are always granted. All others denied.
    /// Phase 2: host_query validated — kinds must be known.
    /// Future: reads from ~/.patina/plugin-config/grants.toml.
    pub fn check_capabilities(manifest: &PluginManifest) -> Result<()> {
        // F4: Per-world capability enforcement — reject capabilities
        // that the manifest's world doesn't support.
        let allowed = manifest.world.allowed_capabilities();
        let world_denied: Vec<&str> = manifest
            .capabilities
            .iter()
            .filter(|cap| !allowed.contains(&cap.as_str()))
            .map(|s| s.as_str())
            .collect();

        if !world_denied.is_empty() {
            anyhow::bail!(
                "plugin '{}' (world '{}') requests capabilities not allowed for this world: {}",
                manifest.name,
                manifest.world,
                world_denied.join(", ")
            );
        }

        // Boolean capabilities that are always granted (no config needed)
        let auto_granted = ["host_log", "host_layer", "host_measure"];

        let denied: Vec<&str> = manifest
            .capabilities
            .iter()
            .filter(|cap| !auto_granted.contains(&cap.as_str()))
            .map(|s| s.as_str())
            .collect();

        if !denied.is_empty() {
            anyhow::bail!(
                "plugin '{}' requests capabilities not granted: {}",
                manifest.name,
                denied.join(", ")
            );
        }

        // Load-time validation: host_query kinds must be known
        const KNOWN_QUERY_KINDS: &[&str] = &["scry", "context", "assay"];
        let unknown: Vec<&str> = manifest
            .host_query_kinds
            .iter()
            .filter(|k| !KNOWN_QUERY_KINDS.contains(&k.as_str()))
            .map(|s| s.as_str())
            .collect();

        if !unknown.is_empty() {
            anyhow::bail!(
                "plugin '{}' requests unknown query kinds: {}",
                manifest.name,
                unknown.join(", ")
            );
        }

        // Load-time validation: host_http domains must be valid
        for domain in &manifest.host_http_domains {
            if domain.is_empty() {
                anyhow::bail!(
                    "plugin '{}' has empty HTTP domain in host_http",
                    manifest.name
                );
            }
            if !domain.is_ascii() {
                anyhow::bail!(
                    "plugin '{}' has non-ASCII HTTP domain '{}' in host_http",
                    manifest.name,
                    domain
                );
            }
            if domain.contains('/') {
                anyhow::bail!(
                    "plugin '{}' has path component in HTTP domain '{}' in host_http",
                    manifest.name,
                    domain
                );
            }
        }

        // Load-time validation: host_secrets domains must be in host_http
        let http_set: std::collections::HashSet<&str> = manifest
            .host_http_domains
            .iter()
            .map(|s| s.as_str())
            .collect();
        for (domain, mapping) in &manifest.host_secrets {
            if !http_set.contains(domain.as_str()) {
                anyhow::bail!(
                    "plugin '{}': domain '{}' in host_secrets but not in host_http",
                    manifest.name,
                    domain
                );
            }
            // Load-time vault probe: warn if secret missing, don't block load
            match crate::secrets::get_global_secret(&mapping.secret_name) {
                Ok(Some(_)) => {} // secret exists, all good
                Ok(None) => {
                    eprintln!(
                        "[plugin:{}] warning: secret '{}' not found in vault (domain '{}')",
                        manifest.name, mapping.secret_name, domain
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[plugin:{}] warning: could not probe secret '{}': {}",
                        manifest.name, mapping.secret_name, e
                    );
                }
            }
        }

        Ok(())
    }

    /// Instantiate a MotherChild from a WASM component + manifest.
    /// Returns Box<dyn MotherChild> for ChildRegistry compatibility.
    ///
    /// `query_fn`: Optional query dispatch provided by the binary crate.
    /// Required if the plugin has host_query capabilities. The host impl
    /// handles gating; this function handles actual engine dispatch.
    pub fn instantiate_child(
        &self,
        component: &Component,
        manifest: &PluginManifest,
        query_fn: Option<QueryDispatchFn>,
    ) -> Result<Box<dyn crate::mother::MotherChild>> {
        // Check capabilities before instantiation
        Self::check_capabilities(manifest)?;

        // Build resolved capabilities for call-time gating
        let grants = manifest.granted_capabilities();

        // Build HTTP client with cross-domain redirect rejection (G5: shared builder).
        let http_client = super::host_support::build_http_client()?;

        // Minimal WASI context — no filesystem access, no env inheritance.
        let wasi = wasmtime_wasi::WasiCtxBuilder::new().build();
        let project_root = crate::session::SessionManager::find_project_root().ok();
        let host_state = HostState {
            plugin_name: manifest.name.clone(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
            project_root,
            grants,
            query_fn,
            http_client,
        };
        let mut store = Store::new(wasm_engine(), host_state);

        let instance = bindings::MotherChild::instantiate(&mut store, component, &self.linker)?;

        // Initialize the plugin (must be called before any other export)
        instance.call_init(&mut store)?;

        // Get the child name from the WASM module
        let name = instance.call_name(&mut store)?;

        Ok(Box::new(WasmChild {
            name,
            allowed_toy_commands: manifest.allowed_toy_commands.clone(),
            inner: Mutex::new(WasmChildInner { store, instance }),
        }))
    }
}

// =========================================================================
// WasmChild adapter — wraps WASM instance as native MotherChild
// =========================================================================

/// Adapter: wraps a WASM component instance as a MotherChild.
///
/// Both store and instance live behind a single Mutex. This is the WASM
/// isolation boundary — no `unsafe` needed. Mutex<T> is Sync when T is Send,
/// and WasmChildInner is Send because both Store<HostState> and
/// bindings::MotherChild are Send. We already acquire the lock on every
/// call, so there's zero performance cost vs the previous layout.
struct WasmChild {
    name: String,
    allowed_toy_commands: Vec<String>,
    inner: Mutex<WasmChildInner>,
}

/// Interior state behind the Mutex — store and instance together.
struct WasmChildInner {
    store: Store<HostState>,
    instance: bindings::MotherChild,
}

impl crate::mother::MotherChild for WasmChild {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
        // Host capabilities come through WASM imports (patina:host/log),
        // not the Rust MotherHost reference.
        let mut inner = self.inner.lock().unwrap_or_else(|e| {
            eprintln!(
                "[plugin:{}] WARN: mutex was poisoned, recovering. Previous call may have panicked.",
                self.name
            );
            e.into_inner()
        });
        let WasmChildInner { store, instance } = &mut *inner;
        match instance.call_on_load(store)? {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("WASM on_load failed: {}", e)),
        }
    }

    fn on_unload(&mut self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| {
            eprintln!(
                "[plugin:{}] WARN: mutex was poisoned, recovering. Previous call may have panicked.",
                self.name
            );
            e.into_inner()
        });
        let WasmChildInner { store, instance } = &mut *inner;
        let _ = instance.call_on_unload(store);
    }

    fn health(&self) -> ChildHealth {
        let mut inner = self.inner.lock().unwrap_or_else(|e| {
            eprintln!(
                "[plugin:{}] WARN: mutex was poisoned, recovering. Previous call may have panicked.",
                self.name
            );
            e.into_inner()
        });
        let WasmChildInner { store, instance } = &mut *inner;
        match instance.call_health(store) {
            Ok(h) => {
                let reason = h.reason.unwrap_or_default();
                match h.status {
                    bindings::patina::host::types::HealthStatus::Healthy => ChildHealth::Healthy,
                    bindings::patina::host::types::HealthStatus::Degraded => {
                        ChildHealth::Degraded(if reason.is_empty() {
                            "degraded".into()
                        } else {
                            reason
                        })
                    }
                    bindings::patina::host::types::HealthStatus::Unhealthy => {
                        ChildHealth::Unhealthy(if reason.is_empty() {
                            "unhealthy".into()
                        } else {
                            reason
                        })
                    }
                }
            }
            Err(e) => ChildHealth::Unhealthy(format!("WASM call failed: {}", e)),
        }
    }

    fn handle(&self, request: &ChildRequest) -> Result<ChildResponse> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| {
            eprintln!(
                "[plugin:{}] WARN: mutex was poisoned, recovering. Previous call may have panicked.",
                self.name
            );
            e.into_inner()
        });
        let WasmChildInner { store, instance } = &mut *inner;
        let payload_json = serde_json::to_string(&request.payload)?;
        let result = instance.call_handle(store, &request.action, &payload_json)?;
        match result {
            Ok(json) => Ok(ChildResponse {
                payload: serde_json::from_str(&json)?,
            }),
            Err(e) => Err(anyhow::anyhow!("{}", e)),
        }
    }

    fn tick(&mut self) -> Vec<Toy> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| {
            eprintln!(
                "[plugin:{}] WARN: mutex was poisoned, recovering. Previous call may have panicked.",
                self.name
            );
            e.into_inner()
        });
        let WasmChildInner { store, instance } = &mut *inner;
        match instance.call_tick(store) {
            Ok(wasm_toys) => wasm_toys
                .into_iter()
                .filter_map(|t| {
                    let toy = Toy {
                        name: t.name,
                        command: t.command,
                        args: t.args,
                    };
                    if self.allowed_toy_commands.contains(&toy.command) {
                        Some(toy)
                    } else {
                        eprintln!(
                            "[plugin:{}] toy '{}' denied: command '{}' not in allowed list {:?}",
                            self.name, toy.name, toy.command, self.allowed_toy_commands
                        );
                        None
                    }
                })
                .collect(),
            Err(e) => {
                eprintln!("[plugin:{}] tick failed: {}", self.name, e);
                vec![]
            }
        }
    }
}
