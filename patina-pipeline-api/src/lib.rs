//! Guest-side API for Patina WASM pipeline plugins.
//!
//! Pipeline plugins are host-invoked pure-compute plugins: grammar parsers,
//! chunkers, tokenizers. Log is the only host import — no query, no layer,
//! no HTTP, no toys. The simplest world.
//!
//! Provides the `PipelinePlugin` trait, `register_pipeline!` macro, and
//! typed `PipelineOp` enum with envelope helpers.
//!
//! See: layer/surface/build/feat/plugin-pipeline-world/SPEC.md

// Version embedded in every plugin binary — host reads before instantiation.
#[cfg(target_arch = "wasm32")]
#[used]
#[link_section = ".patina_api_version"]
static API_VERSION: [u8; 3] = [0, 1, 0];

// =========================================================================
// WIT bindings (guest-side) — pipeline world
// =========================================================================

wit_bindgen::generate!({
    path: "wit/pipeline",
    world: "pipeline",
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

// =========================================================================
// Versioned envelope types
// =========================================================================

/// Pipeline operation types. Declares which operations a plugin handles.
///
/// Maps to `[provides].pipeline_ops` in the plugin manifest.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineOp {
    Parse,
    Chunk,
    Tokenize,
}

impl PipelineOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            PipelineOp::Parse => "parse",
            PipelineOp::Chunk => "chunk",
            PipelineOp::Tokenize => "tokenize",
        }
    }
}

/// Versioned request envelope for pipeline dispatch.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PipelineRequest {
    pub op: String,
    pub version: String,
    pub payload: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

/// Build a parse request envelope.
///
/// `source` — base64-encoded file content.
/// `language` — file extension / language identifier.
pub fn build_parse_request(source: &str, language: &str) -> String {
    let request = PipelineRequest {
        op: "parse".to_string(),
        version: "1".to_string(),
        payload: serde_json::json!({
            "source": source,
            "language": language
        }),
        trace_id: None,
    };
    serde_json::to_string(&request).expect("serialize pipeline request")
}

/// Build a chunk request envelope.
pub fn build_chunk_request(source: &str, language: &str) -> String {
    let request = PipelineRequest {
        op: "chunk".to_string(),
        version: "1".to_string(),
        payload: serde_json::json!({
            "source": source,
            "language": language
        }),
        trace_id: None,
    };
    serde_json::to_string(&request).expect("serialize pipeline request")
}

/// Build a tokenize request envelope.
pub fn build_tokenize_request(source: &str, language: &str) -> String {
    let request = PipelineRequest {
        op: "tokenize".to_string(),
        version: "1".to_string(),
        payload: serde_json::json!({
            "source": source,
            "language": language
        }),
        trace_id: None,
    };
    serde_json::to_string(&request).expect("serialize pipeline request")
}

/// Parse a request envelope from JSON (for use inside plugin handle()).
pub fn parse_request(json: &str) -> Result<PipelineRequest, String> {
    serde_json::from_str(json).map_err(|e| format!("invalid pipeline request: {}", e))
}

// =========================================================================
// Plugin trait
// =========================================================================

/// Trait for pure-compute pipeline plugins.
///
/// Implement this trait and call `register_pipeline!` to create a WASM
/// pipeline plugin. The plugin type must also implement `Default`.
pub trait PipelinePlugin {
    /// Plugin name — used for dispatch and logging.
    fn name(&self) -> String;

    /// Handle a request envelope (JSON). Returns JSON response or error.
    fn handle(&mut self, request: &str) -> Result<String, String>;
}

// =========================================================================
// Plugin singleton (WASM is single-threaded)
// =========================================================================

use std::cell::UnsafeCell;

/// Single-threaded mutable global for WASM plugin singleton.
///
/// Safety: WASM is single-threaded (wasm32-wasip2 has no threads).
/// No concurrent access is possible.
struct WasmCell<T>(UnsafeCell<T>);
unsafe impl<T> Sync for WasmCell<T> {}

static PLUGIN: WasmCell<Option<Box<dyn PipelinePlugin>>> = WasmCell(UnsafeCell::new(None));

#[doc(hidden)]
pub fn __register_pipeline(plugin: Box<dyn PipelinePlugin>) {
    // Safety: called once from init export, WASM is single-threaded
    unsafe {
        *PLUGIN.0.get() = Some(plugin);
    }
}

fn plugin() -> &'static mut dyn PipelinePlugin {
    // Safety: WASM is single-threaded, no concurrent access
    unsafe {
        (*PLUGIN.0.get())
            .as_deref_mut()
            .expect("pipeline plugin not initialized — host must call init first")
    }
}

// =========================================================================
// Component — bridges Guest trait to PipelinePlugin
// =========================================================================

struct Component;

impl Guest for Component {
    fn name() -> String {
        plugin().name()
    }

    fn handle(request: String) -> Result<String, String> {
        plugin().handle(&request)
    }
}

export!(Component);

// =========================================================================
// Registration macro
// =========================================================================

/// Register a type as a pipeline plugin.
///
/// Generates the `init` WASM export that the host calls first.
///
/// # Example
///
/// ```ignore
/// use patina_pipeline_api::{PipelinePlugin, register_pipeline};
///
/// #[derive(Default)]
/// struct ZigParser;
///
/// impl PipelinePlugin for ZigParser {
///     fn name(&self) -> String { "zig-grammar".into() }
///     fn handle(&mut self, request: &str) -> Result<String, String> {
///         // Parse envelope, dispatch by op
///         Ok("{}".into())
///     }
/// }
///
/// register_pipeline!(ZigParser);
/// ```
#[macro_export]
macro_rules! register_pipeline {
    ($plugin_type:ty) => {
        #[export_name = "init"]
        extern "C" fn __patina_pipeline_init() {
            $crate::__register_pipeline(Box::new(<$plugin_type>::default()));
        }
    };
}
