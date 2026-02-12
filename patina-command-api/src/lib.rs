//! Guest-side API for Patina WASM command plugins.
//!
//! Command plugins add subcommands to the `patina` CLI.
//! They run via CommandEngine directly — no Mother daemon needed.
//!
//! Provides the `CommandPlugin` trait and `register_command!` macro.
//! Pattern mirrors patina-plugin-api for mother-child plugins.
//!
//! See: layer/surface/build/feat/plugin-system/SPEC.md (Phase 2)

// Version embedded in every plugin binary — host reads before instantiation.
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
// Re-exports for plugin authors
// =========================================================================

/// Host logging — call from plugin code to log through the host.
pub mod host_log {
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

// =========================================================================
// Plugin trait
// =========================================================================

/// Trait for CLI command plugins.
///
/// Implement this trait and call `register_command!` to create a WASM command plugin.
/// The plugin type must also implement `Default`.
pub trait CommandPlugin {
    /// Command name — the subcommand added to `patina` CLI (e.g., "doctor").
    fn name(&self) -> String;

    /// Command description for help text.
    fn description(&self) -> String;

    /// Run the command with CLI arguments. Returns exit code.
    fn run(&mut self, args: &[String]) -> i32;
}

// =========================================================================
// Plugin singleton (WASM is single-threaded)
// =========================================================================

static mut PLUGIN: Option<Box<dyn CommandPlugin>> = None;

#[doc(hidden)]
pub fn __register_command(plugin: Box<dyn CommandPlugin>) {
    unsafe {
        PLUGIN = Some(plugin);
    }
}

fn plugin() -> &'static mut dyn CommandPlugin {
    #[allow(static_mut_refs)]
    unsafe {
        PLUGIN
            .as_deref_mut()
            .expect("command plugin not initialized — host must call init first")
    }
}

// =========================================================================
// Component — bridges Guest trait to CommandPlugin
// =========================================================================

struct Component;

impl Guest for Component {
    fn name() -> String {
        plugin().name()
    }

    fn description() -> String {
        plugin().description()
    }

    fn run(args: Vec<String>) -> i32 {
        plugin().run(&args)
    }
}

export!(Component);

// =========================================================================
// Registration macro
// =========================================================================

/// Register a type as a command plugin.
///
/// Generates the `init` WASM export that the host calls first.
///
/// # Example
///
/// ```ignore
/// use patina_command_api::{CommandPlugin, register_command};
///
/// #[derive(Default)]
/// struct DoctorPlugin;
///
/// impl CommandPlugin for DoctorPlugin {
///     fn name(&self) -> String { "doctor".into() }
///     fn description(&self) -> String { "Check project health".into() }
///     fn run(&mut self, args: &[String]) -> i32 { 0 }
/// }
///
/// register_command!(DoctorPlugin);
/// ```
#[macro_export]
macro_rules! register_command {
    ($plugin_type:ty) => {
        #[export_name = "init"]
        extern "C" fn __patina_command_init() {
            $crate::__register_command(Box::new(<$plugin_type>::default()));
        }
    };
}
