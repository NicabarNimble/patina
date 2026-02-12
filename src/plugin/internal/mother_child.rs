//! Mother-child world — bindgen, PluginEngine, WasmChild adapter.

use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::Store;

use super::{wasm_engine, PluginManifest};
use crate::mother::{ChildHealth, ChildRequest, ChildResponse, MotherHost, Toy};

// =========================================================================
// Bindgen — generates types from WIT definitions
// =========================================================================

/// Generated types + HostState live together so bindgen's HasData/Host
/// trait resolution works correctly. The generated MotherChild type
/// stays internal — WasmChild bridges to our crate::mother::MotherChild trait.
mod bindings {
    /// State passed to WASM plugins via Store<HostState>.
    /// Contains WASI context (wasm32-wasip2 components always import basic WASI)
    /// and plugin name for log prefix.
    pub struct HostState {
        pub plugin_name: String,
        pub wasi: wasmtime_wasi::WasiCtx,
        pub wasi_table: wasmtime::component::ResourceTable,
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

    // Implement the generated Host trait for patina:host/log
    impl patina::host::log::Host for HostState {
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

    // patina:host/types only defines types (no functions) — empty Host trait
    impl patina::host::types::Host for HostState {}
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
    /// Phase 1: host_log is always granted. All others are denied.
    /// Future: reads from ~/.patina/plugin-config/grants.toml.
    pub fn check_capabilities(manifest: &PluginManifest) -> Result<()> {
        // Capabilities that are always granted (no config needed)
        let auto_granted = ["host_log", "host_layer"];

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
        Ok(())
    }

    /// Instantiate a MotherChild from a WASM component + manifest.
    /// Returns Box<dyn MotherChild> for ChildRegistry compatibility.
    pub fn instantiate_child(
        &self,
        component: &Component,
        manifest: &PluginManifest,
    ) -> Result<Box<dyn crate::mother::MotherChild>> {
        // Check capabilities before instantiation
        Self::check_capabilities(manifest)?;

        // Minimal WASI context — no filesystem access, no env inheritance.
        // Phase 1: plugins are sandboxed to pure computation + host log.
        let wasi = wasmtime_wasi::WasiCtxBuilder::new().build();
        let host_state = HostState {
            plugin_name: manifest.name.clone(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
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
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmChildInner { store, instance } = &mut *inner;
        match instance.call_on_load(store)? {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("WASM on_load failed: {}", e)),
        }
    }

    fn on_unload(&mut self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmChildInner { store, instance } = &mut *inner;
        let _ = instance.call_on_unload(store);
    }

    fn health(&self) -> ChildHealth {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
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
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
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
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
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
