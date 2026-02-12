//! Command world — bindgen + host functions + CommandEngine.

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::Store;

use super::wasm_engine;

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
        /// Cached project root — computed once at store creation.
        pub project_root: Option<std::path::PathBuf>,
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
        let project_root = crate::session::SessionManager::find_project_root().ok();
        let host_state = command_bindings::CommandHostState {
            plugin_name: name.to_string(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
            project_root,
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
        let project_root = crate::session::SessionManager::find_project_root().ok();
        let host_state = command_bindings::CommandHostState {
            plugin_name: "probe".to_string(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
            project_root,
        };
        let mut store = Store::new(wasm_engine(), host_state);
        let instance = command_bindings::Command::instantiate(&mut store, component, &self.linker)?;
        instance.call_init(&mut store)?;
        instance.call_name(&mut store)
    }

    /// Get the command description from a WASM plugin.
    pub fn get_command_description(&self, component: &Component) -> Result<String> {
        let wasi = wasmtime_wasi::WasiCtxBuilder::new().build();
        let project_root = crate::session::SessionManager::find_project_root().ok();
        let host_state = command_bindings::CommandHostState {
            plugin_name: "probe".to_string(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
            project_root,
        };
        let mut store = Store::new(wasm_engine(), host_state);
        let instance = command_bindings::Command::instantiate(&mut store, component, &self.linker)?;
        instance.call_init(&mut store)?;
        instance.call_description(&mut store)
    }
}
