//! Patina SDK — build WASM children for the Patina ecosystem.
//!
//! Enable one feature to select your child world:
//!
//! ```toml
//! # Task child (actions + toys, full host access)
//! patina-sdk = { version = "0.21", features = ["task"] }
//!
//! # Command child (CLI subcommands, read-only)
//! patina-sdk = { version = "0.21", features = ["command"] }
//!
//! # Pipeline child (pure compute, log only)
//! patina-sdk = { version = "0.21", features = ["pipeline"] }
//!
//! # Knowledge-child (Mother/Child/Toy doctrine)
//! patina-sdk = { version = "0.21", features = ["knowledge-child"] }
//! ```
//!
//! M5 classification policy:
//! - Stabilization target: `knowledge-child`.
//! - Migration scaffolds: `task`, `command`.
//! - Experimental lane: `pipeline`.
//!
//! Removal-gate policy for shim lanes:
//! - Shim worlds remain available until compatibility matrix + scaffold parity stay green.
//! - Shim removal must be rollback-safe and explicitly spec-authorized.
//! - Child-first names stay canonical; legacy aliases are compatibility-only.
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
    any(
        all(feature = "task", feature = "command"),
        all(feature = "task", feature = "knowledge-child"),
        all(feature = "task", feature = "pipeline"),
        all(feature = "command", feature = "knowledge-child"),
        all(feature = "command", feature = "pipeline"),
        all(feature = "knowledge-child", feature = "pipeline"),
    )
))]
compile_error!(
    "Enable exactly one patina-sdk world feature: task, command, knowledge-child, or pipeline"
);

// =========================================================================
// Shared internals
// =========================================================================

mod wasm_cell;

// =========================================================================
// Feature-gated world modules
// =========================================================================

#[cfg(feature = "task")]
pub mod task;
#[cfg(feature = "task")]
pub use task::{TaskChild, TaskPlugin, Toy};

#[cfg(feature = "command")]
pub mod command;
#[cfg(feature = "command")]
pub use command::{CommandChild, CommandPlugin};

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
