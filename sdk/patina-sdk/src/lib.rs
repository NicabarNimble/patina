//! Patina SDK for backend-neutral WASI/component children.
//!
//! Child business contracts remain WIT + toys based. Orchestration backends
//! (Rivet, future alternates) are integrated at Mother adapter boundaries, not
//! inside child business code.

#[cfg(feature = "manifest")]
pub mod manifest;
pub mod toys;
pub mod types;

pub use types::*;

pub mod prelude {
    pub use crate::types::*;
}
