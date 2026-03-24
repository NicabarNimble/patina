//! Guest-side API for Patina WASM mother-child children.
//!
//! Mother-child children are daemon-resident: long-lived, heartbeat, toys.
//! They run inside the Mother daemon and handle routed requests.
//!
//! Provides the `MotherChild` trait and `register_mother_child!` macro.

use crate::wasm_cell::WasmCell;

// Version embedded in every child binary — host reads before instantiation.
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
// Re-exports for child authors
// =========================================================================

// ChildHealth, Toy are generated at crate root by wit_bindgen (via world's use statement).
// HealthStatus is generated under patina::host::types — re-export for ergonomic access.
pub use patina::host::types::HealthStatus;

/// Host logging — call from child code to log through the host.
pub mod log {
    pub use super::patina::host::log::LogLevel;

    /// Log a message to the host's structured logging.
    pub fn log(level: LogLevel, message: &str) {
        super::patina::host::log::log(level, message);
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

/// Host HTTP — domain-allowlisted HTTP access for children.
///
/// The host controls domain enforcement, TLS, and credential injection.
/// Child code calls these functions; the host validates URLs against
/// the domain allowlist from child manifest toy grants/scopes.
pub mod fetch {
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

/// Host emit — publish facts to the eventlog via schema-validated emission.
///
/// Requires the relevant toy grant in `child.toml` via `[needs].toys`
/// (and optional `[needs.scopes]` where applicable), plus a `[schemas.<name>]` entry.
/// The host validates the schema and fact type at load time (zero disk I/O
/// at emit time). Facts are written with provenance="external".
pub mod emit {
    /// Emit a fact to the eventlog.
    ///
    /// - `schema`: schema name (e.g., "forge") — must match a `[schemas.<name>]` entry
    /// - `fact_type`: fact name within the schema (e.g., "issue")
    /// - `data`: JSON payload matching the schema's WIT record type
    ///
    /// Returns the event sequence number on success.
    pub fn emit_fact(schema: &str, fact_type: &str, data: &str) -> Result<u64, String> {
        super::patina::host::emit::emit_fact(schema, fact_type, data)
    }
}

// =========================================================================
// Child trait
// =========================================================================

/// Trait for Mother daemon children.
///
/// Implement this trait and call `register_mother_child!` to create a WASM child.
/// All methods except `name()` and `handle()` have default implementations.
///
/// The child type must also implement `Default` — the `register_mother_child!`
/// macro uses it to create the instance. (Not a supertrait to keep dyn-compatible.)
pub trait MotherChild {
    /// Child identity — unique name used for request routing.
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
// Child singleton
// =========================================================================

static PLUGIN: WasmCell<Option<Box<dyn MotherChild>>> = WasmCell(std::cell::UnsafeCell::new(None));

#[doc(hidden)]
pub fn __register_mother_child(child: Box<dyn MotherChild>) {
    // Safety: called once from init export, WASM is single-threaded
    unsafe {
        *PLUGIN.0.get() = Some(child);
    }
}

#[doc(hidden)]
pub fn __register_plugin(plugin: Box<dyn MotherChild>) {
    __register_mother_child(plugin);
}

#[cfg(target_arch = "wasm32")]
mod __wasm {
    use super::*;

    fn child() -> &'static mut dyn MotherChild {
        // Safety: WASM is single-threaded, no concurrent access
        unsafe {
            (*PLUGIN.0.get())
                .as_deref_mut()
                .expect("child not initialized — host must call init first")
        }
    }

    struct Component;

    impl Guest for Component {
        fn name() -> String {
            child().name()
        }

        fn on_load() -> Result<(), String> {
            child().on_load()
        }

        fn on_unload() {
            child().on_unload()
        }

        fn health() -> ChildHealth {
            child().health()
        }

        fn handle(action: String, payload: String) -> Result<String, String> {
            child().handle(&action, &payload)
        }

        fn tick() -> Vec<Toy> {
            child().tick()
        }
    }

    export!(Component);
}

// =========================================================================
// Registration macro
// =========================================================================

/// Register a type as a Mother child.
///
/// Generates the `init` WASM export that the host calls first.
/// The host calls `init` -> child is created -> all subsequent exports work.
///
/// The child type must implement both `MotherChild` and `Default`.
///
/// # Example
///
/// ```ignore
/// use patina_sdk::{MotherChild, ChildHealth, HealthStatus, register_mother_child};
///
/// #[derive(Default)]
/// struct MyPlugin;
///
/// impl MotherChild for MyPlugin {
///     fn name(&self) -> String { "my-plugin".into() }
///     fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
///         Ok("{}".into())
///     }
/// }
///
/// register_mother_child!(MyPlugin);
/// ```
#[macro_export]
macro_rules! register_mother_child {
    ($plugin_type:ty) => {
        #[export_name = "init"]
        extern "C" fn __patina_plugin_init() {
            $crate::mother_child::__register_mother_child(Box::new(<$plugin_type>::default()));
        }
    };
}

#[macro_export]
macro_rules! register_plugin {
    ($plugin_type:ty) => {
        $crate::register_mother_child!($plugin_type);
    };
}

pub use MotherChild as MotherChildPlugin;
