//! Guest-side API for Patina WASM task plugins.
//!
//! Task plugins are on-demand action plugins: analyze AND act, then exit.
//! They have full host access (log, layer, query, HTTP) plus toy intents.
//!
//! Provides the `TaskPlugin` trait and `register_task!` macro.
//! Pattern mirrors patina-command-api for command plugins.
//!
//! See: layer/surface/build/feat/plugin-task-world/SPEC.md

// Version embedded in every plugin binary — host reads before instantiation.
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
// Re-exports for plugin authors
// =========================================================================

// Toy is generated at crate root by wit_bindgen (via world's use statement).
// Re-export for ergonomic access.

/// Host logging — call from plugin code to log through the host.
pub mod host_log {
    pub use super::patina::host::log::LogLevel;

    /// Log a message to the host's structured logging.
    pub fn log(level: LogLevel, message: &str) {
        super::patina::host::log::log(level, message);
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
/// Requires `host_query = ["scry"]` (or "context", "assay") in plugin.toml.
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
// Plugin trait
// =========================================================================

/// Trait for on-demand task plugins.
///
/// Implement this trait and call `register_task!` to create a WASM task plugin.
/// The plugin type must also implement `Default`.
pub trait TaskPlugin {
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
// Plugin singleton (WASM is single-threaded)
// =========================================================================

use std::cell::UnsafeCell;

/// Single-threaded mutable global for WASM plugin singleton.
///
/// Safety: WASM is single-threaded (wasm32-wasip2 has no threads).
/// No concurrent access is possible. This replaces `static mut` which
/// is deprecated in edition 2024. The `unsafe impl Sync` is required
/// because `static` items must be `Sync`, but WASM's single-threaded
/// execution model makes this sound.
struct WasmCell<T>(UnsafeCell<T>);
#[cfg(not(target_feature = "atomics"))]
unsafe impl<T> Sync for WasmCell<T> {}

#[cfg(target_feature = "atomics")]
compile_error!("WasmCell assumes single-threaded WASM. Use thread_local! with atomics.");

static PLUGIN: WasmCell<Option<Box<dyn TaskPlugin>>> = WasmCell(UnsafeCell::new(None));

#[doc(hidden)]
pub fn __register_task(plugin: Box<dyn TaskPlugin>) {
    // Safety: called once from init export, WASM is single-threaded
    unsafe {
        *PLUGIN.0.get() = Some(plugin);
    }
}

fn plugin() -> &'static mut dyn TaskPlugin {
    // Safety: WASM is single-threaded, no concurrent access
    unsafe {
        (*PLUGIN.0.get())
            .as_deref_mut()
            .expect("task plugin not initialized — host must call init first")
    }
}

// =========================================================================
// Component — bridges Guest trait to TaskPlugin
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

    fn toys() -> Vec<Toy> {
        plugin().toys()
    }
}

export!(Component);

// =========================================================================
// Registration macro
// =========================================================================

/// Register a type as a task plugin.
///
/// Generates the `init` WASM export that the host calls first.
///
/// # Example
///
/// ```ignore
/// use patina_task_api::{TaskPlugin, Toy, register_task};
///
/// #[derive(Default)]
/// struct ReviewPlugin;
///
/// impl TaskPlugin for ReviewPlugin {
///     fn name(&self) -> String { "review-pr".into() }
///     fn description(&self) -> String { "Review pull requests".into() }
///     fn run(&mut self, args: &[String]) -> i32 { 0 }
///     fn toys(&self) -> Vec<Toy> { vec![] }
/// }
///
/// register_task!(ReviewPlugin);
/// ```
#[macro_export]
macro_rules! register_task {
    ($plugin_type:ty) => {
        #[export_name = "init"]
        extern "C" fn __patina_task_init() {
            $crate::__register_task(Box::new(<$plugin_type>::default()));
        }
    };
}
