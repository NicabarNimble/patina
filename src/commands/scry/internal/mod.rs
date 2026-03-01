//! Internal implementation of scry command
//!
//! This module contains the implementation details hidden from the public API.
//! The external interface in `mod.rs` re-exports only what's needed.

pub mod enrichment;
pub mod logging;
pub mod routing;
pub mod search;
pub mod semantic;
pub mod subcommands;

// Re-export _json() functions for MCP handler delegation
pub use subcommands::{orient_json, recent_json};
