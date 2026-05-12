//! Canonical child-engine surface.

pub use crate::child::internal::{
    check_capabilities, ChildEngine, ChildKind, ChildManifest, ChildRole, CredentialMapping,
    FilesystemAccessMode, GrantedCapabilities, InjectionLocation, PipelineEngine, QueryDispatchFn,
    QueryScope,
};

pub type ChildProvides = crate::child::internal::ChildProvides;
