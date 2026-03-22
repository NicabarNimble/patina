//! Canonical child-engine surface.

pub use crate::child::internal::{
    ChildEngine, ChildKind, ChildManifest, ChildRole, CommandEngine, CredentialMapping,
    GrantedCapabilities, InjectionLocation, KnowledgeChildEngine, PipelineEngine, QueryDispatchFn,
    QueryScope, TaskEngine,
};

pub type MotherChildEngine = crate::child::internal::MotherChildEngine;
pub type ChildProvides = crate::child::internal::ChildProvides;
