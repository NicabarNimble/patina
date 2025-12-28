//! Internal implementation of scry command
//!
//! This module contains the implementation details hidden from the public API.
//! The external interface in `mod.rs` re-exports only what's needed.

pub mod enrichment;
pub mod hybrid;
pub mod logging;
pub mod query_prep;
pub mod routing;
pub mod search;
pub mod subcommands;
