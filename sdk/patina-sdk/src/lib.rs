//! Patina SDK for backend-neutral WASI/component children.
//!
//! Child business contracts remain WIT + toys based. Orchestration backends
//! (Rivet, future alternates) are integrated at Mother adapter boundaries, not
//! inside child business code.

#[cfg(feature = "manifest")]
pub mod manifest;
#[cfg(feature = "types")]
pub mod toys;
#[cfg(feature = "types")]
pub mod types;

#[cfg(feature = "types")]
pub use types::*;

pub mod prelude {
    #[cfg(feature = "types")]
    pub use crate::types::*;
}
