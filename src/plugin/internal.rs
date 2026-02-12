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
        Component::new(wasm_engine(), wasm)
    }

    /// Check that a plugin's requested capabilities are granted.
    ///
    /// Phase 1: host_log is always granted. All others are denied.
    /// Future: reads from ~/.patina/plugin-config/grants.toml.
    pub fn check_capabilities(manifest: &PluginManifest) -> Result<()> {
        // Capabilities that are always granted (no config needed)
        let auto_granted = ["host_log"];

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

// Safety: bindings::MotherChild is Send + !Sync. Its call_*() methods
// take &self (immutable) and require &mut Store (mutable). The Mutex
// on store serializes all WASM calls, preventing concurrent access.
// The instance is effectively immutable between calls.
unsafe impl Sync for WasmChild {}

impl crate::mother::MotherChild for WasmChild {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
        // Host capabilities come through WASM imports (patina:host/log),
        // not the Rust MotherHost reference.
        let mut store = self.store.lock().unwrap_or_else(|e| e.into_inner());
        match self.instance.call_on_load(&mut *store)? {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("WASM on_load failed: {}", e)),
        }
    }

    fn on_unload(&mut self) {
        let mut store = self.store.lock().unwrap_or_else(|e| e.into_inner());
        let _ = self.instance.call_on_unload(&mut *store);
    }

    fn health(&self) -> ChildHealth {
        let mut store = self.store.lock().unwrap_or_else(|e| e.into_inner());
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
        let mut store = self.store.lock().unwrap_or_else(|e| e.into_inner());
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
        let mut store = self.store.lock().unwrap_or_else(|e| e.into_inner());
        match self.instance.call_tick(&mut *store) {
            Ok(wasm_toys) => wasm_toys
                .into_iter()
                .map(|t| Toy {
                    name: t.name,
                    command: t.command,
                    args: t.args,
                })
                .collect(),
            Err(e) => {
                eprintln!("[plugin:{}] tick failed: {}", self.name, e);
                vec![]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_manifest(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    // =====================================================================
    // PluginManifest::from_path
    // =====================================================================

    #[test]
    fn manifest_valid_minimal() {
        let f = write_temp_manifest(
            r#"
[plugin]
name = "test-plugin"
world = "mother-child"

[capabilities]
host_log = true

[provides]
child = "test"
"#,
        );
        let m = PluginManifest::from_path(f.path()).unwrap();
        assert_eq!(m.name, "test-plugin");
        assert_eq!(m.world, "mother-child");
        assert_eq!(m.version, "0.0.0"); // default
        assert_eq!(m.capabilities, vec!["host_log"]);
        assert_eq!(m.provides.child.as_deref(), Some("test"));
    }

    #[test]
    fn manifest_valid_full() {
        let f = write_temp_manifest(
            r#"
[plugin]
name = "full-plugin"
version = "1.2.3"
description = "A full manifest"
world = "mother-child"
patina_min = "0.17.0"

[capabilities]
host_log = true
filesystem = false

[provides]
child = "full"
commands = ["cmd1", "cmd2"]
"#,
        );
        let m = PluginManifest::from_path(f.path()).unwrap();
        assert_eq!(m.name, "full-plugin");
        assert_eq!(m.version, "1.2.3");
        assert_eq!(m.description, "A full manifest");
        assert_eq!(m.patina_min, "0.17.0");
        // filesystem = false should NOT be in capabilities
        assert_eq!(m.capabilities, vec!["host_log"]);
        assert_eq!(m.provides.commands, vec!["cmd1", "cmd2"]);
    }

    #[test]
    fn manifest_missing_plugin_section() {
        let f = write_temp_manifest("[other]\nfoo = 1\n");
        let err = PluginManifest::from_path(f.path()).unwrap_err();
        assert!(
            err.to_string().contains("missing [plugin] section"),
            "got: {}",
            err
        );
    }

    #[test]
    fn manifest_missing_name() {
        let f = write_temp_manifest("[plugin]\nworld = \"mother-child\"\n");
        let err = PluginManifest::from_path(f.path()).unwrap_err();
        assert!(
            err.to_string().contains("missing plugin.name"),
            "got: {}",
            err
        );
    }

    #[test]
    fn manifest_missing_world() {
        let f = write_temp_manifest("[plugin]\nname = \"test\"\n");
        let err = PluginManifest::from_path(f.path()).unwrap_err();
        assert!(
            err.to_string().contains("missing plugin.world"),
            "got: {}",
            err
        );
    }

    #[test]
    fn manifest_invalid_toml() {
        let f = write_temp_manifest("this is not valid toml {{{}}}");
        assert!(PluginManifest::from_path(f.path()).is_err());
    }

    // =====================================================================
    // check_capabilities
    // =====================================================================

    #[test]
    fn capabilities_all_granted() {
        let m = PluginManifest {
            name: "test".into(),
            version: "0.1.0".into(),
            description: String::new(),
            world: "mother-child".into(),
            patina_min: "0.0.0".into(),
            capabilities: vec!["host_log".into()],
            provides: PluginProvides {
                child: None,
                commands: vec![],
            },
        };
        assert!(PluginEngine::check_capabilities(&m).is_ok());
    }

    #[test]
    fn capabilities_empty() {
        let m = PluginManifest {
            name: "test".into(),
            version: "0.1.0".into(),
            description: String::new(),
            world: "mother-child".into(),
            patina_min: "0.0.0".into(),
            capabilities: vec![],
            provides: PluginProvides {
                child: None,
                commands: vec![],
            },
        };
        assert!(PluginEngine::check_capabilities(&m).is_ok());
    }

    #[test]
    fn capabilities_denied() {
        let m = PluginManifest {
            name: "test".into(),
            version: "0.1.0".into(),
            description: String::new(),
            world: "mother-child".into(),
            patina_min: "0.0.0".into(),
            capabilities: vec!["host_log".into(), "filesystem".into(), "network".into()],
            provides: PluginProvides {
                child: None,
                commands: vec![],
            },
        };
        let err = PluginEngine::check_capabilities(&m).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("filesystem"), "got: {}", msg);
        assert!(msg.contains("network"), "got: {}", msg);
        assert!(
            !msg.contains("host_log"),
            "host_log should be granted: {}",
            msg
        );
    }

    // =====================================================================
    // WASM integration — load models.wasm, call handle()
    // =====================================================================

    /// Load the pre-compiled models.wasm fixture, instantiate it,
    /// and verify the full handle() round-trip works.
    #[test]
    fn wasm_models_child_handle_roundtrip() {
        let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/patina_plugin_models.wasm");
        if !wasm_path.exists() {
            panic!(
                "test fixture missing: {}\n\
                 Build it with: cargo build --release -p patina-plugin-models --target wasm32-wasip2\n\
                 Then: cp target/wasm32-wasip2/release/patina_plugin_models.wasm tests/fixtures/",
                wasm_path.display()
            );
        }

        let engine = PluginEngine::new().expect("PluginEngine::new() failed");
        let wasm_bytes = std::fs::read(&wasm_path).expect("failed to read .wasm fixture");
        let component = engine
            .load_component(&wasm_bytes)
            .expect("load_component failed");

        // Use a manifest matching models plugin
        let manifest = PluginManifest {
            name: "patina-models".into(),
            version: "0.1.0".into(),
            description: "test".into(),
            world: "mother-child".into(),
            patina_min: "0.0.0".into(),
            capabilities: vec!["host_log".into()],
            provides: PluginProvides {
                child: Some("models".into()),
                commands: vec![],
            },
        };

        let child = engine
            .instantiate_child(&component, &manifest)
            .expect("instantiate_child failed");

        // Verify identity
        assert_eq!(child.name(), "models");

        // Test handle() round-trip: resolve_model action
        let request = crate::mother::ChildRequest {
            action: "resolve_model".into(),
            payload: serde_json::json!({"name": "e5-small"}),
        };
        let response = child.handle(&request).expect("handle() failed");

        // Verify response contains expected path
        let path = response.payload.get("path").and_then(|v| v.as_str());
        assert!(
            path.is_some_and(|p| p.contains("e5-small")),
            "expected path containing 'e5-small', got: {:?}",
            response.payload
        );
    }

    /// Verify that health() works on a WASM child.
    #[test]
    fn wasm_models_child_health() {
        let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/patina_plugin_models.wasm");
        if !wasm_path.exists() {
            return; // Skip if fixture not available
        }

        let engine = PluginEngine::new().unwrap();
        let wasm_bytes = std::fs::read(&wasm_path).unwrap();
        let component = engine.load_component(&wasm_bytes).unwrap();
        let manifest = PluginManifest {
            name: "patina-models".into(),
            version: "0.1.0".into(),
            description: "test".into(),
            world: "mother-child".into(),
            patina_min: "0.0.0".into(),
            capabilities: vec!["host_log".into()],
            provides: PluginProvides {
                child: Some("models".into()),
                commands: vec![],
            },
        };

        let child = engine.instantiate_child(&component, &manifest).unwrap();
        match child.health() {
            crate::mother::ChildHealth::Healthy => {} // expected
            other => panic!("expected Healthy, got: {:?}", other),
        }
    }

    // =====================================================================
    // Benchmarks (C2) — Instant::now() instrumentation
    // =====================================================================

    /// Measure PluginEngine::new(), Component::new(), instantiate_child(),
    /// and handle() round-trip. Run with `cargo test -- --nocapture benchmark`.
    #[test]
    fn benchmark_plugin_performance() {
        use std::time::Instant;

        let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/patina_plugin_models.wasm");
        if !wasm_path.exists() {
            return;
        }

        // 1. PluginEngine::new() — spec threshold: <100ms
        let t0 = Instant::now();
        let engine = PluginEngine::new().unwrap();
        let engine_ms = t0.elapsed().as_secs_f64() * 1000.0;

        // 2. Component::new() — document compilation time
        let wasm_bytes = std::fs::read(&wasm_path).unwrap();
        let t1 = Instant::now();
        let component = engine.load_component(&wasm_bytes).unwrap();
        let component_ms = t1.elapsed().as_secs_f64() * 1000.0;

        // 3. instantiate_child() total — Component + WasiCtx + Store + init + name
        let manifest = PluginManifest {
            name: "patina-models".into(),
            version: "0.1.0".into(),
            description: "bench".into(),
            world: "mother-child".into(),
            patina_min: "0.0.0".into(),
            capabilities: vec!["host_log".into()],
            provides: PluginProvides {
                child: Some("models".into()),
                commands: vec![],
            },
        };
        let t2 = Instant::now();
        let child = engine.instantiate_child(&component, &manifest).unwrap();
        let instantiate_ms = t2.elapsed().as_secs_f64() * 1000.0;

        // 4. handle() round-trip — spec threshold: <1ms
        let request = crate::mother::ChildRequest {
            action: "resolve_model".into(),
            payload: serde_json::json!({"name": "e5-small"}),
        };
        // Warm up
        let _ = child.handle(&request).unwrap();
        // Measure 10 iterations
        let t3 = Instant::now();
        let iterations = 10;
        for _ in 0..iterations {
            let _ = child.handle(&request).unwrap();
        }
        let handle_avg_ms = t3.elapsed().as_secs_f64() * 1000.0 / iterations as f64;

        eprintln!();
        eprintln!("=== Plugin System Benchmarks (C2) ===");
        eprintln!(
            "  PluginEngine::new():     {:.2}ms (threshold: <100ms) {}",
            engine_ms,
            if engine_ms < 100.0 { "PASS" } else { "FAIL" }
        );
        eprintln!(
            "  Component::new():        {:.2}ms (156KB WASM cranelift JIT)",
            component_ms
        );
        eprintln!(
            "  instantiate_child():     {:.2}ms (WasiCtx + Store + init + name)",
            instantiate_ms
        );
        eprintln!(
            "  handle() round-trip:     {:.3}ms avg over {} calls (threshold: <1ms) {}",
            handle_avg_ms,
            iterations,
            if handle_avg_ms < 1.0 { "PASS" } else { "FAIL" }
        );
        eprintln!("=====================================");

        // Assert thresholds
        assert!(
            engine_ms < 100.0,
            "PluginEngine::new() took {:.2}ms, threshold is 100ms",
            engine_ms
        );
        assert!(
            handle_avg_ms < 1.0,
            "handle() avg took {:.3}ms, threshold is 1ms",
            handle_avg_ms
        );
    }
}
