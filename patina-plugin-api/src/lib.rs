//! Guest-side API for Patina WASM plugins.
//!
//! Plugin authors depend on this crate to build Mother child plugins.
//! Provides the `MotherChildPlugin` trait and `register_plugin!` macro.
//!
//! Pattern: same as Zed's `zed_extension_api` — API crate owns bindgen,
//! Component, Guest impl, and export!. Plugin crates implement the trait
//! and call `register_plugin!` which generates the `init` WASM export.
//!
//! See: layer/surface/build/feat/plugin-system/SPEC.md

// Version embedded in every plugin binary — host reads before instantiation.
// Only included in WASM targets (Mach-O/ELF have different section formats).
#[cfg(target_arch = "wasm32")]
#[used]
#[link_section = ".patina_api_version"]
static API_VERSION: [u8; 3] = [0, 1, 0];

// =========================================================================
// WIT bindings (guest-side) — generated at crate root for path resolution
// =========================================================================

wit_bindgen::generate!({
    path: "wit",
    world: "mother-child",
    skip: ["init"],
    generate_all,
});

// =========================================================================
// Re-exports for plugin authors
// =========================================================================

// ChildHealth and Toy are generated at crate root by wit_bindgen
// LogLevel and log are under patina::host::log

/// Host logging — call from plugin code to log through the host.
pub mod host_log {
    pub use super::patina::host::log::LogLevel;

    /// Log a message to the host's structured logging.
    pub fn log(level: LogLevel, message: &str) {
        super::patina::host::log::log(level, message);
    }
}

// =========================================================================
// Plugin trait
// =========================================================================

/// Trait for Mother daemon child plugins.
///
/// Implement this trait and call `register_plugin!` to create a WASM plugin.
/// All methods except `name()` and `handle()` have default implementations.
///
/// The plugin type must also implement `Default` — the `register_plugin!`
/// macro uses it to create the instance. (Not a supertrait to keep dyn-compatible.)
pub trait MotherChildPlugin {
    /// Plugin identity — unique name used for request routing.
    fn name(&self) -> String;

    /// Called when Mother loads this child.
    fn on_load(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Called when Mother shuts down.
    fn on_unload(&mut self) {}

    /// Health check — Mother calls this on heartbeat.
    fn health(&self) -> ChildHealth {
        ChildHealth::Healthy
    }

    /// Handle a routed request. Action and payload are strings (payload is JSON).
    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String>;

    /// Heartbeat tick — return work requests for Mother to execute.
    fn tick(&mut self) -> Vec<Toy> {
        vec![]
    }
}

// =========================================================================
// Plugin singleton (WASM is single-threaded)
// =========================================================================

static mut PLUGIN: Option<Box<dyn MotherChildPlugin>> = None;

#[doc(hidden)]
pub fn __register_plugin(plugin: Box<dyn MotherChildPlugin>) {
    unsafe {
        PLUGIN = Some(plugin);
    }
}

fn plugin() -> &'static mut dyn MotherChildPlugin {
    // Safety: WASM is single-threaded. Only one reference exists at a time.
    #[allow(static_mut_refs)]
    unsafe {
        PLUGIN
            .as_deref_mut()
            .expect("plugin not initialized — host must call init first")
    }
}

// =========================================================================
// Component — bridges Guest trait to MotherChildPlugin
// =========================================================================

struct Component;

impl Guest for Component {
    fn name() -> String {
        plugin().name()
    }

    fn on_load() -> Result<(), String> {
        plugin().on_load()
    }

    fn on_unload() {
        plugin().on_unload()
    }

    fn health() -> ChildHealth {
        plugin().health()
    }

    fn handle(action: String, payload: String) -> Result<String, String> {
        plugin().handle(&action, &payload)
    }

    fn tick() -> Vec<Toy> {
        plugin().tick()
    }
}

export!(Component);

// =========================================================================
// Registration macro
// =========================================================================

/// Register a type as a Mother child plugin.
///
/// Generates the `init` WASM export that the host calls first.
/// The host calls `init` → plugin is created → all subsequent exports work.
///
/// The plugin type must implement both `MotherChildPlugin` and `Default`.
///
/// # Example
///
/// ```ignore
/// use patina_plugin_api::{MotherChildPlugin, ChildHealth, register_plugin};
///
/// #[derive(Default)]
/// struct MyPlugin;
///
/// impl MotherChildPlugin for MyPlugin {
///     fn name(&self) -> String { "my-plugin".into() }
///     fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
///         Ok("{}".into())
///     }
/// }
///
/// register_plugin!(MyPlugin);
/// ```
#[macro_export]
macro_rules! register_plugin {
    ($plugin_type:ty) => {
        #[export_name = "init"]
        extern "C" fn __patina_plugin_init() {
            $crate::__register_plugin(Box::new(<$plugin_type>::default()));
        }
    };
}
