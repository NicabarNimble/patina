//! Canonical child-engine surface.

pub use crate::child::internal::{
    check_capabilities, ChildEngine, ChildKind, ChildManifest, ChildRole, CredentialMapping,
    FilesystemAccessMode, FilesystemPreopen, GrantedCapabilities, InjectionLocation,
    PipelineEngine, QueryDispatchFn, QueryScope, GUEST_PROJECT_ROOT,
};

pub type ChildProvides = crate::child::internal::ChildProvides;
