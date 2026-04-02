//! Guest-side API for Patina WASM pipeline children.
//!
//! Pipeline children are host-invoked pure-compute children: grammar parsers,
//! chunkers, tokenizers. Log is the only host import — no query, no layer,
//! no HTTP, no toys. The simplest world.
//!
//! Provides the `PipelineChild` trait, `register_pipeline_child!` macro, and
//! typed `PipelineOp` enum with envelope helpers.

use crate::wasm_cell::WasmCell;

// Version embedded in every child binary — host reads before instantiation.
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
// Re-exports for child authors
// =========================================================================

/// Host logging — call from child code to log through the host.
pub mod log {
    pub use super::wasi::logging::logging::Level;

    /// Log a message to the host's structured logging.
    pub fn log(level: Level, message: &str) {
        super::wasi::logging::logging::log(level, "", message);
    }
}

// =========================================================================
// Versioned envelope types
// =========================================================================

/// Pipeline operation types. Declares which operations a child handles.
///
/// Maps to `[provides].pipeline_ops` in the child manifest.
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

/// Parse a request envelope from JSON (for use inside child handle()).
pub fn parse_request(json: &str) -> Result<PipelineRequest, String> {
    serde_json::from_str(json).map_err(|e| format!("invalid pipeline request: {}", e))
}

// =========================================================================
// Child trait
// =========================================================================

/// Trait for pure-compute pipeline children.
///
/// Implement this trait and call `register_pipeline_child!` to create a WASM
/// pipeline child. The child type must also implement `Default`.
pub trait PipelineChild {
    /// Child name — used for dispatch and logging.
    fn name(&self) -> String;

    /// Handle a request envelope (JSON). Returns JSON response or error.
    fn handle(&mut self, request: &str) -> Result<String, String>;
}

// =========================================================================
// Child singleton
// =========================================================================

static PLUGIN: WasmCell<Option<Box<dyn PipelineChild>>> =
    WasmCell(std::cell::UnsafeCell::new(None));

#[doc(hidden)]
pub fn __register_pipeline_child(child: Box<dyn PipelineChild>) {
    // Safety: called once from init export, WASM is single-threaded
    unsafe {
        *PLUGIN.0.get() = Some(child);
    }
}

#[doc(hidden)]
pub fn __register_pipeline(plugin: Box<dyn PipelineChild>) {
    __register_pipeline_child(plugin);
}

#[cfg(target_arch = "wasm32")]
mod __wasm {
    use super::*;

    fn child() -> &'static mut dyn PipelineChild {
        // Safety: WASM is single-threaded, no concurrent access
        unsafe {
            (*PLUGIN.0.get())
                .as_deref_mut()
                .expect("pipeline child not initialized — host must call init first")
        }
    }

    struct Component;

    impl Guest for Component {
        fn name() -> String {
            child().name()
        }

        fn handle(request: String) -> Result<String, String> {
            child().handle(&request)
        }
    }

    export!(Component);
}

// =========================================================================
// Registration macro
// =========================================================================

/// Register a type as a pipeline child.
///
/// Generates the `init` WASM export that the host calls first.
///
/// # Example
///
/// ```ignore
/// use patina_sdk::{PipelineChild, register_pipeline_child};
///
/// #[derive(Default)]
/// struct ZigParser;
///
/// impl PipelineChild for ZigParser {
///     fn name(&self) -> String { "zig-grammar".into() }
///     fn handle(&mut self, request: &str) -> Result<String, String> {
///         // Parse envelope, dispatch by op
///         Ok("{}".into())
///     }
/// }
///
/// register_pipeline_child!(ZigParser);
/// ```
#[macro_export]
macro_rules! register_pipeline_child {
    ($plugin_type:ty) => {
        #[export_name = "init"]
        extern "C" fn __patina_pipeline_init() {
            $crate::pipeline::__register_pipeline_child(Box::new(<$plugin_type>::default()));
        }
    };
}

#[macro_export]
macro_rules! register_pipeline {
    ($plugin_type:ty) => {
        $crate::register_pipeline_child!($plugin_type);
    };
}

pub use PipelineChild as PipelinePlugin;
