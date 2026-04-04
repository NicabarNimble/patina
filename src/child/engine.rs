//! Canonical child-engine surface.

#[deprecated(since = "0.46.0", note = "use ChildEngine")]
pub use crate::child::internal::KnowledgeChildEngine;
pub use crate::child::internal::{
    check_capabilities, ChildEngine, ChildKind, ChildManifest, ChildRole, CredentialMapping,
    GrantedCapabilities, InjectionLocation, PipelineEngine, QueryDispatchFn, QueryScope,
};

pub type ChildProvides = crate::child::internal::ChildProvides;
