//! Session lifecycle management for Patina projects
//!
//! Replaces ~640 lines of bash (session-{start,update,note,end}.sh) with native
//! Rust commands. Sessions track development work with dual-write: markdown is the
//! collaboration artifact (LLM reads/writes), events are the structured query layer.
//!
//! This module follows the dependable-rust pattern:
//! - Public interface (this file): clean API + clap subcommands
//! - Internal implementation: all logic in internal.rs

mod internal;

pub(crate) use internal::{
    end_live_session_value, note_live_session, resolve_live_session, start_session_value,
    update_live_session_value, SessionStartRequest,
};
