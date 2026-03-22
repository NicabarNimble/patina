//! Canonical child-engine surface.
//!
//! Runtime implementation still lives behind the plugin-era module during CV6
//! migration; this module is the child-native entry point for callers.

pub use crate::plugin::{
    ChildEngine, ChildKind, ChildManifest, ChildRole, CommandEngine, CredentialMapping,
    GrantedCapabilities, InjectionLocation, KnowledgeChildEngine, PipelineEngine, QueryDispatchFn,
    QueryScope, TaskEngine,
};

pub type MotherChildEngine = crate::plugin::PluginEngine;
