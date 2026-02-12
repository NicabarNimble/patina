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
    /// Toy commands this plugin is allowed to request (from [capabilities.toys].commands).
    /// Empty means no toys allowed.
    pub allowed_toy_commands: Vec<String>,
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
        let cap_table = table.get("capabilities").and_then(|v| v.as_table());
        let capabilities = cap_table
            .map(|cap| {
                cap.iter()
                    .filter(|(_, v)| v.as_bool() == Some(true))
                    .map(|(k, _)| k.clone())
                    .collect()
            })
            .unwrap_or_default();

        // Parse [capabilities.toys].commands — allowed toy commands
        let allowed_toy_commands = cap_table
            .and_then(|cap| cap.get("toys"))
            .and_then(|v| v.as_table())
            .and_then(|toys| toys.get("commands"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
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
            allowed_toy_commands,
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
        Component::new(wasm_engine(), wasm)
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

// =========================================================================
// Command world — bindgen + host functions + CommandEngine
// =========================================================================

/// Bindgen for the command world (separate from mother-child).
/// Each world needs its own host state because command plugins
/// import patina:host/layer (project data access) while
/// mother-child plugins do not.
mod command_bindings {
    /// Host state for command plugins — includes layer access.
    pub struct CommandHostState {
        pub plugin_name: String,
        pub wasi: wasmtime_wasi::WasiCtx,
        pub wasi_table: wasmtime::component::ResourceTable,
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

    // patina:host/log — same implementation as mother-child
    impl patina::host::log::Host for CommandHostState {
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

    // patina:host/layer — read-only project data access.
    //
    // Re-entrancy invariant: these implementations MUST NOT acquire the
    // store Mutex or call WASM methods on the same instance.
    // All calls go to the Patina core library, never back into WASM.
    impl patina::host::layer::Host for CommandHostState {
        fn find_project_root(&mut self) -> Option<String> {
            crate::session::SessionManager::find_project_root()
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        }

        fn read_config(&mut self) -> Result<String, String> {
            let root = crate::session::SessionManager::find_project_root()
                .map_err(|e| format!("project root: {}", e))?;
            let config = crate::project::load_with_migration(&root)
                .map_err(|e| format!("load config: {}", e))?;
            serde_json::to_string(&config).map_err(|e| format!("serialize config: {}", e))
        }

        fn detect_environment(&mut self) -> Result<String, String> {
            let env = crate::environment::Environment::detect()
                .map_err(|e| format!("detect env: {}", e))?;
            serde_json::to_string(&env).map_err(|e| format!("serialize env: {}", e))
        }

        fn get_stored_tools(&mut self) -> Vec<String> {
            let root = match crate::session::SessionManager::find_project_root() {
                Ok(r) => r,
                Err(_) => return vec![],
            };
            let config = match crate::project::load_with_migration(&root) {
                Ok(c) => c,
                Err(_) => return vec![],
            };
            config
                .environment
                .map(|e| e.detected_tools)
                .unwrap_or_default()
        }

        fn count_layer_files(&mut self, subdir: String) -> u32 {
            let root = match crate::session::SessionManager::find_project_root() {
                Ok(r) => r,
                Err(_) => return 0,
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
            let root = crate::session::SessionManager::find_project_root().ok()?;
            crate::project::get_uid(&root)
        }

        fn check_adapter_version(
            &mut self,
            adapter_name: String,
        ) -> Result<Option<String>, String> {
            let root = crate::session::SessionManager::find_project_root()
                .map_err(|e| format!("project root: {}", e))?;
            let adapter = crate::adapters::get_adapter(&adapter_name);
            adapter
                .check_for_updates(&root)
                .map(|opt| opt.map(|(current, _)| current))
                .map_err(|e| format!("adapter check: {}", e))
        }
    }
}

/// Command plugin engine — loads and runs command world WASM plugins.
///
/// Separate from PluginEngine because command plugins use a different
/// WIT world with different imports (patina:host/layer for project
/// data access). CLI creates this for one-shot use without the daemon.
pub struct CommandEngine {
    linker: Linker<command_bindings::CommandHostState>,
}

impl CommandEngine {
    /// Create a new CommandEngine for one-shot CLI plugin use.
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

    /// Run a command plugin. Returns exit code.
    pub fn run_command(&self, component: &Component, name: &str, args: &[String]) -> Result<i32> {
        let wasi = wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .build();
        let host_state = command_bindings::CommandHostState {
            plugin_name: name.to_string(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
        };
        let mut store = Store::new(wasm_engine(), host_state);
        let instance = command_bindings::Command::instantiate(&mut store, component, &self.linker)?;

        // Initialize plugin
        instance.call_init(&mut store)?;

        // Run with args
        instance.call_run(&mut store, args)
    }

    /// Get the command name from a WASM plugin.
    pub fn get_command_name(&self, component: &Component) -> Result<String> {
        let wasi = wasmtime_wasi::WasiCtxBuilder::new().build();
        let host_state = command_bindings::CommandHostState {
            plugin_name: "probe".to_string(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
        };
        let mut store = Store::new(wasm_engine(), host_state);
        let instance = command_bindings::Command::instantiate(&mut store, component, &self.linker)?;
        instance.call_init(&mut store)?;
        instance.call_name(&mut store)
    }

    /// Get the command description from a WASM plugin.
    pub fn get_command_description(&self, component: &Component) -> Result<String> {
        let wasi = wasmtime_wasi::WasiCtxBuilder::new().build();
        let host_state = command_bindings::CommandHostState {
            plugin_name: "probe".to_string(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
        };
        let mut store = Store::new(wasm_engine(), host_state);
        let instance = command_bindings::Command::instantiate(&mut store, component, &self.linker)?;
        instance.call_init(&mut store)?;
        instance.call_description(&mut store)
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
    fn manifest_parses_toy_commands() {
        let f = write_temp_manifest(
            r#"
[plugin]
name = "test-plugin"
world = "mother-child"

[capabilities]
host_log = true

[capabilities.toys]
commands = ["git", "patina"]

[provides]
child = "test"
"#,
        );
        let m = PluginManifest::from_path(f.path()).unwrap();
        assert_eq!(m.allowed_toy_commands, vec!["git", "patina"]);
    }

    #[test]
    fn manifest_no_toy_commands_defaults_empty() {
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
        assert!(m.allowed_toy_commands.is_empty());
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
            allowed_toy_commands: vec![],
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
            allowed_toy_commands: vec![],
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
            allowed_toy_commands: vec![],
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
            allowed_toy_commands: vec![],
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
            allowed_toy_commands: vec![],
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
    // WASM integration — load repos.wasm, test toy system end-to-end
    // =====================================================================

    /// Helper: load repos.wasm fixture and instantiate child.
    fn load_repos_child() -> Option<Box<dyn crate::mother::MotherChild>> {
        let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/patina_plugin_repos.wasm");
        if !wasm_path.exists() {
            return None;
        }

        let engine = PluginEngine::new().unwrap();
        let wasm_bytes = std::fs::read(&wasm_path).unwrap();
        let component = engine.load_component(&wasm_bytes).unwrap();
        let manifest = PluginManifest {
            name: "patina-repos".into(),
            version: "0.1.0".into(),
            description: "test".into(),
            world: "mother-child".into(),
            patina_min: "0.0.0".into(),
            capabilities: vec!["host_log".into()],
            allowed_toy_commands: vec!["git".into(), "patina".into()],
            provides: PluginProvides {
                child: Some("repos".into()),
                commands: vec![],
            },
        };

        Some(engine.instantiate_child(&component, &manifest).unwrap())
    }

    /// Repos child: report_repo + check_freshness handle() round-trip.
    #[test]
    fn wasm_repos_child_handle_roundtrip() {
        let child = match load_repos_child() {
            Some(c) => c,
            None => {
                panic!(
                    "test fixture missing: tests/fixtures/patina_plugin_repos.wasm\n\
                     Build: cargo build --release -p patina-plugin-repos --target wasm32-wasip2\n\
                     Copy: cp target/wasm32-wasip2/release/patina_plugin_repos.wasm tests/fixtures/"
                );
            }
        };

        assert_eq!(child.name(), "repos");

        // Report a repo
        let request = crate::mother::ChildRequest {
            action: "report_repo".into(),
            payload: serde_json::json!({
                "name": "test-repo",
                "path": "/tmp/repos/test-repo",
                "last_indexed": 0
            }),
        };
        let response = child.handle(&request).expect("report_repo failed");
        assert_eq!(
            response.payload.get("status").and_then(|v| v.as_str()),
            Some("registered")
        );
        assert_eq!(
            response.payload.get("total_repos").and_then(|v| v.as_u64()),
            Some(1)
        );

        // Check freshness
        let request = crate::mother::ChildRequest {
            action: "check_freshness".into(),
            payload: serde_json::json!({}),
        };
        let response = child.handle(&request).expect("check_freshness failed");
        let stale_count = response.payload.get("stale_count").and_then(|v| v.as_u64());
        assert_eq!(
            stale_count,
            Some(1),
            "repo with last_indexed=0 should be stale"
        );
    }

    /// Repos child: tick() returns toys for stale repos — end-to-end toy system proof.
    #[test]
    fn wasm_repos_child_tick_returns_toys() {
        let mut child = match load_repos_child() {
            Some(c) => c,
            None => return, // Skip if fixture not available
        };

        // Report a stale repo (last_indexed = 0 means it's ancient)
        let request = crate::mother::ChildRequest {
            action: "report_repo".into(),
            payload: serde_json::json!({
                "name": "stale-repo",
                "path": "/home/user/.patina/cache/repos/stale-repo",
                "last_indexed": 0
            }),
        };
        child.handle(&request).expect("report_repo failed");

        // tick() should return toys for the stale repo
        let toys = child.tick();
        assert!(
            toys.len() >= 2,
            "expected at least 2 toys (pull + scrape), got {}",
            toys.len()
        );

        // Verify pull toy
        let pull_toy = toys.iter().find(|t| t.name.contains("pull"));
        assert!(pull_toy.is_some(), "expected a pull toy, got: {:?}", toys);
        let pull = pull_toy.unwrap();
        assert_eq!(pull.command, "git");
        assert!(
            pull.args.contains(&"-C".to_string()),
            "pull toy should use -C flag"
        );

        // Verify scrape toy
        let scrape_toy = toys.iter().find(|t| t.name.contains("scrape"));
        assert!(
            scrape_toy.is_some(),
            "expected a scrape toy, got: {:?}",
            toys
        );
        let scrape = scrape_toy.unwrap();
        assert_eq!(scrape.command, "patina");
        assert!(
            scrape.args.contains(&"scrape".to_string()),
            "scrape toy should include 'scrape' arg"
        );
    }

    /// Repos child: fresh repo produces no toys.
    #[test]
    fn wasm_repos_child_fresh_repo_no_toys() {
        let mut child = match load_repos_child() {
            Some(c) => c,
            None => return,
        };

        // Report a fresh repo (last_indexed = now)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let request = crate::mother::ChildRequest {
            action: "report_repo".into(),
            payload: serde_json::json!({
                "name": "fresh-repo",
                "path": "/tmp/repos/fresh-repo",
                "last_indexed": now
            }),
        };
        child.handle(&request).expect("report_repo failed");

        // tick() should return no toys — repo is fresh
        let toys = child.tick();
        assert!(
            toys.is_empty(),
            "expected no toys for fresh repo, got: {:?}",
            toys
        );
    }

    /// Repos child: health is Healthy when no repos, Degraded when stale.
    #[test]
    fn wasm_repos_child_health_reflects_staleness() {
        let child = match load_repos_child() {
            Some(c) => c,
            None => return,
        };

        // No repos → Healthy
        match child.health() {
            crate::mother::ChildHealth::Healthy => {}
            other => panic!("expected Healthy with no repos, got: {:?}", other),
        }

        // Add stale repo → Degraded
        let request = crate::mother::ChildRequest {
            action: "report_repo".into(),
            payload: serde_json::json!({
                "name": "old-repo",
                "path": "/tmp/repos/old-repo",
                "last_indexed": 0
            }),
        };
        child.handle(&request).expect("report_repo failed");

        match child.health() {
            crate::mother::ChildHealth::Degraded(_) => {} // expected
            other => panic!("expected Degraded with stale repo, got: {:?}", other),
        }
    }

    // =====================================================================
    // Toy capability gating (F4)
    // =====================================================================

    /// Repos child with restricted manifest: only "patina" allowed, not "git".
    /// Verifies that unauthorized toy commands are filtered out by WasmChild.
    #[test]
    fn wasm_repos_child_toy_capability_gating() {
        let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/patina_plugin_repos.wasm");
        if !wasm_path.exists() {
            return; // Skip if fixture not available
        }

        let engine = PluginEngine::new().unwrap();
        let wasm_bytes = std::fs::read(&wasm_path).unwrap();
        let component = engine.load_component(&wasm_bytes).unwrap();

        // Manifest only allows "patina", NOT "git"
        let manifest = PluginManifest {
            name: "patina-repos".into(),
            version: "0.1.0".into(),
            description: "test".into(),
            world: "mother-child".into(),
            patina_min: "0.0.0".into(),
            capabilities: vec!["host_log".into()],
            allowed_toy_commands: vec!["patina".into()], // git excluded
            provides: PluginProvides {
                child: Some("repos".into()),
                commands: vec![],
            },
        };

        let mut child = engine.instantiate_child(&component, &manifest).unwrap();

        // Report a stale repo — tick() will return both git pull and patina scrape toys
        let request = crate::mother::ChildRequest {
            action: "report_repo".into(),
            payload: serde_json::json!({
                "name": "gated-repo",
                "path": "/tmp/repos/gated-repo",
                "last_indexed": 0
            }),
        };
        child.handle(&request).expect("report_repo failed");

        let toys = child.tick();

        // Only "patina" toys should pass — "git" toys should be filtered
        for toy in &toys {
            assert_eq!(
                toy.command, "patina",
                "expected only 'patina' toys, got command '{}' in toy '{}'",
                toy.command, toy.name
            );
        }
        assert!(
            !toys.is_empty(),
            "expected at least one patina toy to pass the filter"
        );
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
            allowed_toy_commands: vec![],
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

    // =====================================================================
    // CommandEngine — doctor.wasm integration tests
    // =====================================================================

    fn load_doctor_component() -> Option<(CommandEngine, wasmtime::component::Component)> {
        let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/patina_doctor.wasm");
        if !wasm_path.exists() {
            return None;
        }
        let engine = CommandEngine::new().expect("CommandEngine::new() failed");
        let wasm_bytes = std::fs::read(&wasm_path).expect("failed to read doctor wasm");
        let component = engine
            .load_component(&wasm_bytes)
            .expect("load_component failed");
        Some((engine, component))
    }

    #[test]
    fn command_doctor_name() {
        let (engine, component) = match load_doctor_component() {
            Some(ec) => ec,
            None => {
                panic!(
                    "test fixture missing: tests/fixtures/patina_doctor.wasm\n\
                     Build: cargo build --release -p patina-doctor --target wasm32-wasip2\n\
                     Copy: cp target/wasm32-wasip2/release/patina_doctor.wasm tests/fixtures/"
                );
            }
        };

        let name = engine
            .get_command_name(&component)
            .expect("get_command_name failed");
        assert_eq!(name, "doctor");
    }

    #[test]
    fn command_doctor_description() {
        let (engine, component) = match load_doctor_component() {
            Some(ec) => ec,
            None => return,
        };

        let desc = engine
            .get_command_description(&component)
            .expect("get_command_description failed");
        assert!(
            desc.contains("health"),
            "expected description to mention 'health', got: {}",
            desc
        );
    }

    #[test]
    fn command_doctor_run() {
        let (engine, component) = match load_doctor_component() {
            Some(ec) => ec,
            None => return,
        };

        // Run with --json to avoid terminal output dependencies.
        // Exit code depends on project state — just verify it doesn't panic.
        let args = vec!["--json".to_string()];
        let exit_code = engine
            .run_command(&component, "doctor", &args)
            .expect("run_command failed");
        // doctor returns 0 (healthy), 1 (error), 2 (warning), or 3 (critical)
        assert!(
            [0, 1, 2, 3].contains(&exit_code),
            "unexpected exit code: {}",
            exit_code
        );
    }
}
