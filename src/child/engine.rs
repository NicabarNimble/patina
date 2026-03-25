//! Canonical child-engine surface.

pub use crate::child::internal::{
    check_capabilities, ChildEngine, ChildKind, ChildManifest, ChildRole, CommandEngine,
    CredentialMapping, GrantedCapabilities, InjectionLocation, KnowledgeChildEngine,
    PipelineEngine, QueryDispatchFn, QueryScope, TaskEngine,
};

pub type ChildProvides = crate::child::internal::ChildProvides;
