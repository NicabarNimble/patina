//! Patina SDK — build WASM children for the Patina ecosystem.
//!
//! Enable one feature to select your child world:
//!
//! ```toml
//! # Pipeline child (pure compute, log only)
//! patina-sdk = { version = "0.21", features = ["pipeline"] }
//!
//! # Knowledge-child (Mother/Child/Toy doctrine)
//! patina-sdk = { version = "0.21", features = ["knowledge-child"] }
//! ```
//!
//! Child world policy:
//! - Stabilization target: `knowledge-child`.
//! - Pure-compute lane: `pipeline`.
//!
//! Toy contract policy:
//! - A toy is a Mother-defined boundary opening in the WASM sandbox wall.
//! - Toys are granted by `child.toml` (`[needs].toys` + optional `[needs.scopes]`).
//! - Scope config shapes authority; it does not create new toy kinds.
//! - Litmus test: if a child can do it via pure compute, it is not a toy.

// =========================================================================
// Compiler-enforced world exclusion — [[compiler-enforced-safety]]
// =========================================================================

// Only enforce on wasm32 — workspace builds on native unify features across
// consumers (doctor=command) which is harmless on native
// but would break a WASM binary with conflicting export symbols.
#[cfg(all(
    target_arch = "wasm32",
    all(feature = "knowledge-child", feature = "pipeline")
))]
compile_error!("Enable exactly one patina-sdk world feature: knowledge-child or pipeline");

// =========================================================================
// Shared internals
// =========================================================================

mod wasm_cell;

// =========================================================================
// Feature-gated world modules
// =========================================================================

#[cfg(feature = "knowledge-child")]
pub mod helpers;
#[cfg(feature = "knowledge-child")]
pub mod knowledge_child;
#[cfg(feature = "knowledge-child")]
pub mod toys;
#[cfg(feature = "knowledge-child")]
pub use knowledge_child::{granted, substrate, KnowledgeChild, KnowledgeChildPlugin};

#[cfg(feature = "pipeline")]
pub mod pipeline;
#[cfg(feature = "pipeline")]
pub use pipeline::{parse_request, PipelineChild, PipelinePlugin};
