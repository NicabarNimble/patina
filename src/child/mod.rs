//! Child runtime boundary surface.

pub mod engine;
pub(crate) mod internal;
pub mod runtime;
pub mod scaffold;
pub(crate) mod toy_host;

// Re-exports for integration testing. Not a public API commitment.
#[doc(hidden)]
pub mod testing {
    pub use super::internal::{
        check_capabilities, ChildEngine, ChildIngressMode, ChildKind, ChildManifest, ChildProvides,
        FilesystemAccessMode, FilesystemPreopen, PipelineEngine,
    };
    pub use super::toy_host::v2::{events_subscribe, EventRecord};
}
