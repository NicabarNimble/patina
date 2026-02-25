//! Guest-side API for Patina WASM mother-child plugins.
//!
//! Mother-child plugins are daemon-resident: long-lived, heartbeat, toys.
//! They run inside the Mother daemon and handle routed requests.
//!
//! Provides the `MotherChildPlugin` trait and `register_plugin!` macro.

use crate::wasm_cell::WasmCell;

// Version embedded in every plugin binary — host reads before instantiation.
#[cfg(target_arch = "wasm32")]
#[used]
#[link_section = ".patina_api_version"]
static API_VERSION: [u8; 3] = [0, 1, 0];

// =========================================================================
// WIT bindings (guest-side) — mother-child world
// =========================================================================

wit_bindgen::generate!({
    path: "wit/mother-child",
    world: "mother-child",
    skip: ["init"],
    generate_all,
});

// =========================================================================
// Re-exports for plugin authors
// =========================================================================

// ChildHealth, Toy are generated at crate root by wit_bindgen (via world's use statement).
// HealthStatus is generated under patina::host::types — re-export for ergonomic access.
pub use patina::host::types::HealthStatus;

/// Host logging — call from plugin code to log through the host.
pub mod host_log {
    pub use super::patina::host::log::LogLevel;

    /// Log a message to the host's structured logging.
    pub fn log(level: LogLevel, message: &str) {
        super::patina::host::log::log(level, message);
    }
}

/// Measurement reporting — record metrics from plugin execution.
///
/// Requires `host_measure = true` in plugin.toml capabilities.
/// The host validates verb, checks metrics are numeric JSON, and
/// writes to eventlog with the plugin name as source.
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

/// Host HTTP — domain-allowlisted HTTP access for plugins.
///
/// The host controls domain enforcement, TLS, and credential injection.
/// Plugin code calls these functions; the host validates URLs against
/// the domain allowlist from `[capabilities].host_http` in plugin.toml.
pub mod host_http {
    pub use super::patina::host::http::HttpResponse;

    /// HTTP GET from an allowed domain.
    pub fn get(url: &str) -> Result<HttpResponse, String> {
        super::patina::host::http::http_get(url)
    }

    /// HTTP POST to an allowed domain.
    pub fn post(url: &str, body: &str, content_type: &str) -> Result<HttpResponse, String> {
        super::patina::host::http::http_post(url, body, content_type)
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
        ChildHealth {
            status: HealthStatus::Healthy,
            reason: None,
        }
    }

    /// Handle a routed request. Action and payload are strings (payload is JSON).
    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String>;

    /// Heartbeat tick — return work requests for Mother to execute.
    fn tick(&mut self) -> Vec<Toy> {
        vec![]
    }
}

// =========================================================================
// Plugin singleton
// =========================================================================

static PLUGIN: WasmCell<Option<Box<dyn MotherChildPlugin>>> =
    WasmCell(std::cell::UnsafeCell::new(None));

#[doc(hidden)]
pub fn __register_plugin(plugin: Box<dyn MotherChildPlugin>) {
    // Safety: called once from init export, WASM is single-threaded
    unsafe {
        *PLUGIN.0.get() = Some(plugin);
    }
}

#[cfg(target_arch = "wasm32")]
mod __wasm {
    use super::*;

    fn plugin() -> &'static mut dyn MotherChildPlugin {
        // Safety: WASM is single-threaded, no concurrent access
        unsafe {
            (*PLUGIN.0.get())
                .as_deref_mut()
                .expect("plugin not initialized — host must call init first")
        }
    }

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
}

// =========================================================================
// Registration macro
// =========================================================================

/// Register a type as a Mother child plugin.
///
/// Generates the `init` WASM export that the host calls first.
/// The host calls `init` -> plugin is created -> all subsequent exports work.
///
/// The plugin type must implement both `MotherChildPlugin` and `Default`.
///
/// # Example
///
/// ```ignore
/// use patina_sdk::{MotherChildPlugin, ChildHealth, HealthStatus, register_plugin};
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
            $crate::mother_child::__register_plugin(Box::new(<$plugin_type>::default()));
        }
    };
}
