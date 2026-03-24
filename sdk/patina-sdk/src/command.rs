//! Guest-side API for Patina WASM command children.
//!
//! Command children add subcommands to the `patina` CLI.
//! They run via CommandEngine directly — no Mother daemon needed.
//!
//! Provides the `CommandChild` trait and `register_command_child!` macro.

use crate::wasm_cell::WasmCell;

// Version embedded in every child binary — host reads before instantiation.
#[cfg(target_arch = "wasm32")]
#[used]
#[link_section = ".patina_api_version"]
static API_VERSION: [u8; 3] = [0, 1, 0];

// =========================================================================
// WIT bindings (guest-side) — command world
// =========================================================================

wit_bindgen::generate!({
    path: "wit/command",
    world: "command",
    skip: ["init"],
    generate_all,
});

// =========================================================================
// Re-exports for child authors
// =========================================================================

/// Host logging — call from child code to log through the host.
pub mod log {
    pub use super::patina::host::log::LogLevel;

    /// Log a message to the host's structured logging.
    pub fn log(level: LogLevel, message: &str) {
        super::patina::host::log::log(level, message);
    }
}

/// Read-only access to project layer data.
pub mod layer {
    /// Find the project root directory.
    pub fn find_project_root() -> Option<String> {
        super::patina::host::layer::find_project_root()
    }

    /// Read project config as JSON string.
    pub fn read_config() -> Result<String, String> {
        super::patina::host::layer::read_config()
    }

    /// Detect current environment as JSON string.
    pub fn detect_environment() -> Result<String, String> {
        super::patina::host::layer::detect_environment()
    }

    /// Get stored environment tools from project config.
    pub fn get_stored_tools() -> Vec<String> {
        super::patina::host::layer::get_stored_tools()
    }

    /// Count markdown files in a layer subdirectory.
    pub fn count_layer_files(subdir: &str) -> u32 {
        super::patina::host::layer::count_layer_files(subdir)
    }

    /// Get project UID.
    pub fn get_project_uid() -> Option<String> {
        super::patina::host::layer::get_project_uid()
    }

    /// Check adapter for updates.
    pub fn check_adapter_version(adapter_name: &str) -> Result<Option<String>, String> {
        super::patina::host::layer::check_adapter_version(adapter_name)
    }
}

/// Measurement reporting — record metrics from child execution.
///
/// Requires the relevant toy grant in `child.toml` via `[needs].toys`
/// (and optional `[needs.scopes]` where applicable).
/// The host validates verb, checks metrics are numeric JSON, and
/// writes to eventlog with the child name as source.
pub mod measure {
    /// Record a measurement event.
    ///
    /// - `verb`: protocol verb (capture, index, search, believe, evolve)
    /// - `tool`: tool name (e.g., "doctor")
    /// - `mode`: sub-mode (e.g., "freshness-check")
    /// - `metrics_json`: JSON object with numeric values (e.g., `{"score": 0.95}`)
    pub fn record_measurement(
        verb: &str,
        tool: &str,
        mode: &str,
        metrics_json: &str,
    ) -> Result<(), String> {
        super::patina::host::measure::record_measurement(verb, tool, mode, metrics_json)
    }
}

/// Capability-gated access to Patina's query engines.
///
/// Requires a query-capable grant in `child.toml` (`[needs].toys` + optional scopes).
/// The host checks grants at both load time and call time (defense in depth).
/// Params and results are JSON strings — parse on the guest side.
pub mod query {
    /// Execute a query against a Patina engine.
    ///
    /// `kind` — one of "scry", "context", "assay".
    /// `params` — JSON object with kind-specific parameters.
    ///
    /// Returns the query result as a string, or an error message.
    /// The host will deny the call if the child manifest doesn't
    /// grant the requested kind.
    pub fn query(kind: &str, params: &str) -> Result<String, String> {
        super::patina::host::query::query(kind, params)
    }
}

// =========================================================================
// Child trait
// =========================================================================

/// Trait for CLI command children.
///
/// Implement this trait and call `register_command_child!` to create a WASM command child.
/// The child type must also implement `Default`.
pub trait CommandChild {
    /// Command name — the subcommand added to `patina` CLI (e.g., "doctor").
    fn name(&self) -> String;

    /// Command description for help text.
    fn description(&self) -> String;

    /// Run the command with CLI arguments. Returns exit code.
    fn run(&mut self, args: &[String]) -> i32;
}

// =========================================================================
// Child singleton
// =========================================================================

static PLUGIN: WasmCell<Option<Box<dyn CommandChild>>> = WasmCell(std::cell::UnsafeCell::new(None));

#[doc(hidden)]
pub fn __register_command_child(child: Box<dyn CommandChild>) {
    // Safety: called once from init export, WASM is single-threaded
    unsafe {
        *PLUGIN.0.get() = Some(child);
    }
}

#[doc(hidden)]
pub fn __register_command(plugin: Box<dyn CommandChild>) {
    __register_command_child(plugin);
}

#[cfg(target_arch = "wasm32")]
mod __wasm {
    use super::*;

    fn child() -> &'static mut dyn CommandChild {
        // Safety: WASM is single-threaded, no concurrent access
        unsafe {
            (*PLUGIN.0.get())
                .as_deref_mut()
                .expect("command child not initialized — host must call init first")
        }
    }

    struct Component;

    impl Guest for Component {
        fn name() -> String {
            child().name()
        }

        fn description() -> String {
            child().description()
        }

        fn run(args: Vec<String>) -> i32 {
            child().run(&args)
        }
    }

    export!(Component);
}

// =========================================================================
// Registration macro
// =========================================================================

/// Register a type as a command child.
///
/// Generates the `init` WASM export that the host calls first.
///
/// # Example
///
/// ```ignore
/// use patina_sdk::{CommandChild, register_command_child};
///
/// #[derive(Default)]
/// struct DoctorPlugin;
///
/// impl CommandChild for DoctorPlugin {
///     fn name(&self) -> String { "doctor".into() }
///     fn description(&self) -> String { "Check project health".into() }
///     fn run(&mut self, args: &[String]) -> i32 { 0 }
/// }
///
/// register_command_child!(DoctorPlugin);
/// ```
#[macro_export]
macro_rules! register_command_child {
    ($plugin_type:ty) => {
        #[export_name = "init"]
        extern "C" fn __patina_command_init() {
            $crate::command::__register_command_child(Box::new(<$plugin_type>::default()));
        }
    };
}

#[macro_export]
macro_rules! register_command {
    ($plugin_type:ty) => {
        $crate::register_command_child!($plugin_type);
    };
}

pub use CommandChild as CommandPlugin;
