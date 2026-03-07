//! Native transport binding for Patina pipe protocol.
//!
//! Provides the `Child` trait and `run()` entry point for native children —
//! normal Rust binaries that speak JSON-RPC 2.0 over stdio.

pub mod emitter;
pub mod protocol;

pub use emitter::FactEmitter;
pub use patina_pipe_types::*;
