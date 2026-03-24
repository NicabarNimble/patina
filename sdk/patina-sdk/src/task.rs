//! Guest-side API for Patina WASM task children.
//!
//! Task children are on-demand action children: analyze AND act, then exit.
//! They have full host access (log, layer, query, HTTP) plus toy intents.
//!
//! Provides the `TaskChild` trait and `register_task_child!` macro.

use crate::wasm_cell::WasmCell;

// Version embedded in every child binary — host reads before instantiation.
#[cfg(target_arch = "wasm32")]
#[used]
#[link_section = ".patina_api_version"]
static API_VERSION: [u8; 3] = [0, 1, 0];

// =========================================================================
// WIT bindings (guest-side) — task world
// =========================================================================

wit_bindgen::generate!({
    path: "wit/task",
    world: "task",
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
    pub fn query(kind: &str, params: &str) -> Result<String, String> {
        super::patina::host::query::query(kind, params)
    }
}

// =========================================================================
// Child trait
// =========================================================================

/// Trait for on-demand task children.
///
/// Implement this trait and call `register_task_child!` to create a WASM task child.
/// The child type must also implement `Default`.
pub trait TaskChild {
    /// Task name — used for dispatch and logging.
    fn name(&self) -> String;

    /// Task description for help text.
    fn description(&self) -> String;

    /// Run the task with CLI arguments. Returns exit code.
    fn run(&mut self, args: &[String]) -> i32;

    /// Return toy intents for host-side execution.
    /// Host filters through allowed_toy_commands before executing.
    fn toys(&self) -> Vec<Toy> {
        vec![]
    }
}

// =========================================================================
// Child singleton
// =========================================================================

static PLUGIN: WasmCell<Option<Box<dyn TaskChild>>> = WasmCell(std::cell::UnsafeCell::new(None));

#[doc(hidden)]
pub fn __register_task_child(child: Box<dyn TaskChild>) {
    // Safety: called once from init export, WASM is single-threaded
    unsafe {
        *PLUGIN.0.get() = Some(child);
    }
}

#[doc(hidden)]
pub fn __register_task(plugin: Box<dyn TaskChild>) {
    __register_task_child(plugin);
}

// Component bridge and export! — wasm32 only.
// Native workspace builds unify features across consumers, which would
// cause symbol conflicts between worlds that export the same names.
#[cfg(target_arch = "wasm32")]
mod __wasm {
    use super::*;

    fn child() -> &'static mut dyn TaskChild {
        // Safety: WASM is single-threaded, no concurrent access
        unsafe {
            (*PLUGIN.0.get())
                .as_deref_mut()
                .expect("task child not initialized — host must call init first")
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

        fn toys() -> Vec<Toy> {
            child().toys()
        }
    }

    export!(Component);
}

// =========================================================================
// Registration macro
// =========================================================================

/// Register a type as a task child.
///
/// Generates the `init` WASM export that the host calls first.
///
/// # Example
///
/// ```ignore
/// use patina_sdk::{TaskChild, Toy, register_task_child};
///
/// #[derive(Default)]
/// struct ReviewPlugin;
///
/// impl TaskChild for ReviewPlugin {
///     fn name(&self) -> String { "review-pr".into() }
///     fn description(&self) -> String { "Review pull requests".into() }
///     fn run(&mut self, args: &[String]) -> i32 { 0 }
///     fn toys(&self) -> Vec<Toy> { vec![] }
/// }
///
/// register_task_child!(ReviewPlugin);
/// ```
#[macro_export]
macro_rules! register_task_child {
    ($plugin_type:ty) => {
        #[export_name = "init"]
        extern "C" fn __patina_task_init() {
            $crate::task::__register_task_child(Box::new(<$plugin_type>::default()));
        }
    };
}

#[macro_export]
macro_rules! register_task {
    ($plugin_type:ty) => {
        $crate::register_task_child!($plugin_type);
    };
}

pub use TaskChild as TaskPlugin;
