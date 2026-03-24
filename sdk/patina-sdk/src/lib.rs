//! Patina SDK — build WASM plugins for the Patina ecosystem.
//!
//! Enable one feature to select your plugin world:
//!
//! ```toml
//! # Task plugin (actions + toys, full host access)
//! patina-sdk = { version = "0.21", features = ["task"] }
//!
//! # Command plugin (CLI subcommands, read-only)
//! patina-sdk = { version = "0.21", features = ["command"] }
//!
//! # Pipeline plugin (pure compute, log only)
//! patina-sdk = { version = "0.21", features = ["pipeline"] }
//!
//! # Mother-child plugin (daemon-resident, full access)
//! patina-sdk = { version = "0.21", features = ["mother-child"] }
//!
//! # Knowledge-child (Mother/Child/Toy doctrine)
//! patina-sdk = { version = "0.21", features = ["knowledge-child"] }
//! ```
//!
//! M5 classification policy:
//! - Stabilization target: `knowledge-child` + tier crates.
//! - Migration scaffolds: `task`, `command`, `mother-child`.
//! - Experimental lane: `pipeline`.

// =========================================================================
// Compiler-enforced world exclusion — [[compiler-enforced-safety]]
// =========================================================================

// Only enforce on wasm32 — workspace builds on native unify features across
// consumers (doctor=command, models=mother-child) which is harmless on native
// but would break a WASM binary with conflicting export symbols.
#[cfg(all(
    target_arch = "wasm32",
    any(
        all(feature = "task", feature = "command"),
        all(feature = "task", feature = "mother-child"),
        all(feature = "task", feature = "knowledge-child"),
        all(feature = "task", feature = "pipeline"),
        all(feature = "command", feature = "mother-child"),
        all(feature = "command", feature = "knowledge-child"),
        all(feature = "command", feature = "pipeline"),
        all(feature = "mother-child", feature = "knowledge-child"),
        all(feature = "mother-child", feature = "pipeline"),
        all(feature = "knowledge-child", feature = "pipeline"),
    )
))]
compile_error!(
    "Enable exactly one patina-sdk world feature: task, command, mother-child, knowledge-child, or pipeline"
);

// =========================================================================
// Shared internals
// =========================================================================

mod wasm_cell;

#[cfg(feature = "knowledge-child")]
pub use patina_sdk_agent as agent;
#[cfg(feature = "knowledge-child")]
pub use patina_sdk_core as core;
#[cfg(feature = "knowledge-child")]
pub use patina_sdk_data as data;

// =========================================================================
// Feature-gated world modules
// =========================================================================

#[cfg(feature = "task")]
pub mod task;
#[cfg(feature = "task")]
pub use task::{TaskPlugin, Toy};

#[cfg(feature = "command")]
pub mod command;
#[cfg(feature = "command")]
pub use command::CommandPlugin;

#[cfg(feature = "mother-child")]
pub mod mother_child;
#[cfg(feature = "mother-child")]
pub use mother_child::{ChildHealth, HealthStatus, MotherChildPlugin, Toy};

#[cfg(feature = "knowledge-child")]
pub mod knowledge_child;
#[cfg(feature = "knowledge-child")]
pub mod toys;
#[cfg(feature = "knowledge-child")]
pub use knowledge_child::{granted, substrate};

#[cfg(feature = "pipeline")]
pub mod pipeline;
#[cfg(feature = "pipeline")]
pub use pipeline::{parse_request, PipelinePlugin};
