//! Internal implementation for spec command
//!
//! Follows dependable-rust pattern: private modules with curated re-exports.

mod archive;
mod create;
mod mutations;
mod packets;
mod queries;
mod queue;
mod split;

pub(crate) fn db_path() -> anyhow::Result<std::path::PathBuf> {
    patina::eventlog::patina_db_path()
}

// --- Re-exports for parent mod.rs ---
//
// Items re-exported at pub(crate) by spec/mod.rs need pub(crate) here.
// Items only called from spec/mod.rs function bodies use pub(super).

// Query types + functions re-exported pub(crate) by parent for session/MCP
pub(crate) use queries::{
    check_spec_value, get_all_specs, get_blocked_specs, get_ready_specs, history_spec_value,
    show_spec_value, ListFilters,
};

// Packet projection query functions re-exported pub(crate) for MCP
pub(crate) use packets::{handoff_spec_value, packet_spec_value, prompt_spec_value};

// Queue functions re-exported pub(crate) by parent for session/MCP
pub(crate) use queue::next_spec_value;

// Mutation _value() functions re-exported pub(crate) by parent for MCP
pub(crate) use mutations::{
    abandon_spec_value, block_spec_value, complete_spec_value, pause_spec_value,
    promote_spec_value, rename_spec_value, reopen_spec_value, resume_spec_value, set_spec_value,
};

#[allow(unused_imports)]
// Create _value() functions re-exported pub(crate) by parent for MCP
pub(crate) use create::{create_spec_value, create_spec_value_for_project};

// Split _value() function re-exported pub(crate) by parent for MCP
pub(crate) use split::split_spec_value;

#[allow(unused_imports)]
// Functions called from spec execute path
pub(crate) use archive::{archive_spec, archive_stale_specs};
