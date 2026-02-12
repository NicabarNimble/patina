//! wasmtime guts — bindgen, Engine singleton, WasmChild adapter.

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};

use crate::mother::{ChildHealth, ChildRequest, ChildResponse, MotherHost, Toy};

// =========================================================================
// Bindgen — generates types from WIT definitions
// =========================================================================

/// Generated types + HostState live together so bindgen's HasData/Host
/// trait resolution works correctly. The generated MotherChild type
/// stays internal — WasmChild bridges to our crate::mother::MotherChild trait.
mod bindings {
    /// State passed to WASM plugins via Store<HostState>.
    /// Phase 1: just plugin name for log prefix.
    pub struct HostState {
        pub plugin_name: String,
    }

    wasmtime::component::bindgen!({
        path: "wit/",
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
}

use bindings::HostState;

// =========================================================================
// Engine singleton (OnceLock pattern from Zed)
// =========================================================================

/// Shared wasmtime engine — singleton per process.
fn wasm_engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config.wasm_component_model(true);
        // NO config.async_support(true) — sync-first
        Engine::new(&config).expect("failed to create wasmtime engine")
    })
}

// =========================================================================
// Plugin manifest (plugin.toml)
// =========================================================================

/// Parsed plugin manifest from plugin.toml.
#[derive(Debug)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub world: String,
    pub patina_min: String,
    pub capabilities: Vec<String>,
    pub provides: PluginProvides,
}

/// What the plugin provides to the system.
#[derive(Debug)]
pub struct PluginProvides {
    pub child: Option<String>,
    pub commands: Vec<String>,
}

impl PluginManifest {
    /// Parse a plugin manifest from a TOML file.
    fn from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let table: toml::Table = content.parse()?;

        let plugin = table
            .get("plugin")
            .and_then(|v| v.as_table())
            .ok_or_else(|| anyhow::anyhow!("missing [plugin] section"))?;

        let name = plugin
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing plugin.name"))?
            .to_string();

        let version = plugin
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        let description = plugin
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let world = plugin
            .get("world")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing plugin.world"))?
            .to_string();

        let patina_min = plugin
            .get("patina_min")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        // Parse capabilities
        let capabilities = table
            .get("capabilities")
            .and_then(|v| v.as_table())
            .map(|cap| {
                cap.iter()
                    .filter(|(_, v)| v.as_bool() == Some(true))
                    .map(|(k, _)| k.clone())
                    .collect()
            })
            .unwrap_or_default();

        // Parse provides
        let provides_table = table.get("provides").and_then(|v| v.as_table());
        let child = provides_table
            .and_then(|p| p.get("child"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let commands = provides_table
            .and_then(|p| p.get("commands"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            name,
            version,
            description,
            world,
            patina_min,
            capabilities,
            provides: PluginProvides { child, commands },
        })
    }
}

// =========================================================================
// PluginEngine
// =========================================================================

/// Shared wasmtime infrastructure for loading and running WASM plugins.
pub struct PluginEngine {
    linker: Linker<HostState>,
}

impl PluginEngine {
    /// Create a new PluginEngine with host functions registered.
    pub fn new() -> Result<Self> {
        let mut linker = Linker::new(wasm_engine());
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
        Component::new(wasm_engine(), wasm)
    }

    /// Instantiate a MotherChild from a WASM component + manifest.
    /// Returns Box<dyn MotherChild> for ChildRegistry compatibility.
    pub fn instantiate_child(
        &self,
        component: &Component,
        manifest: &PluginManifest,
    ) -> Result<Box<dyn crate::mother::MotherChild>> {
        let host_state = HostState {
            plugin_name: manifest.name.clone(),
        };
        let mut store = Store::new(wasm_engine(), host_state);

        let instance = bindings::MotherChild::instantiate(&mut store, component, &self.linker)?;

        // Get the child name from the WASM module
        let name = instance.call_name(&mut store)?;

        Ok(Box::new(WasmChild {
            name,
            store: Mutex::new(store),
            instance,
        }))
    }
}

// =========================================================================
// WasmChild adapter — wraps WASM instance as native MotherChild
// =========================================================================

/// Adapter: wraps a WASM component instance as a MotherChild.
///
/// The store is behind a Mutex because MotherChild::handle() and health()
/// take &self (for concurrent reads via RwLock in ChildRegistry) but
/// wasmtime calls always need &mut Store.
struct WasmChild {
    name: String,
    store: Mutex<Store<HostState>>,
    instance: bindings::MotherChild,
}

// Safety: Store<HostState> is Send (HostState is Send).
// Mutex provides Sync. The instance is only accessed through the Mutex-guarded store.
unsafe impl Sync for WasmChild {}

impl crate::mother::MotherChild for WasmChild {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
        // Host capabilities come through WASM imports (patina:host/log),
        // not the Rust MotherHost reference.
        let mut store = self.store.lock().unwrap();
        match self.instance.call_on_load(&mut *store)? {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("WASM on_load failed: {}", e)),
        }
    }

    fn on_unload(&mut self) {
        let mut store = self.store.lock().unwrap();
        let _ = self.instance.call_on_unload(&mut *store);
    }

    fn health(&self) -> ChildHealth {
        let mut store = self.store.lock().unwrap();
        match self.instance.call_health(&mut *store) {
            Ok(h) => match h {
                bindings::ChildHealth::Healthy => ChildHealth::Healthy,
                bindings::ChildHealth::Degraded => ChildHealth::Degraded("degraded".into()),
                bindings::ChildHealth::Unhealthy => ChildHealth::Unhealthy("unhealthy".into()),
            },
            Err(e) => ChildHealth::Unhealthy(format!("WASM call failed: {}", e)),
        }
    }

    fn handle(&self, request: &ChildRequest) -> Result<ChildResponse> {
        let mut store = self.store.lock().unwrap();
        let payload_json = serde_json::to_string(&request.payload)?;
        let result = self
            .instance
            .call_handle(&mut *store, &request.action, &payload_json)?;
        match result {
            Ok(json) => Ok(ChildResponse {
                payload: serde_json::from_str(&json)?,
            }),
            Err(e) => Err(anyhow::anyhow!("{}", e)),
        }
    }

    fn tick(&mut self) -> Vec<Toy> {
        let mut store = self.store.lock().unwrap();
        match self.instance.call_tick(&mut *store) {
            Ok(wasm_toys) => wasm_toys
                .into_iter()
                .map(|t| Toy {
                    name: t.name,
                    command: t.command,
                    args: t.args,
                })
                .collect(),
            Err(_) => vec![],
        }
    }
}
