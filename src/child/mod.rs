//! Child runtime boundary surface.
//!
//! Phase-A facade that exposes child contracts through one canonical module
//! root while implementation remains in `mother`/`plugin` internals.

pub mod runtime {
    pub use crate::mother::{
        ChildHealth, ChildRequest, ChildResponse, KnowledgeChild, MotherChild, MotherHost,
        PendingEvent, TaskIntent, TaskIntentKind,
    };
}
