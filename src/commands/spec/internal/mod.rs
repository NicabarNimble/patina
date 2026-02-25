//! Internal implementation for spec command
//!
//! Follows dependable-rust pattern: private modules with curated re-exports.

mod archive;
mod create;
mod mutations;
mod queries;
mod queue;
mod split;

pub(crate) const DB_PATH: &str = ".patina/local/data/patina.db";

// --- Re-exports for parent mod.rs ---
//
// Items re-exported at pub(crate) by spec/mod.rs need pub(crate) here.
// Items only called from spec/mod.rs function bodies use pub(super).

// Query types + functions re-exported pub(crate) by parent for session/MCP
pub(crate) use queries::{get_all_specs, get_blocked_specs, get_ready_specs, show_spec_value, ListFilters};

// Queue functions re-exported pub(crate) by parent for session/MCP
pub(crate) use queue::{load_dep_counts, next_spec_value, spec_age_days_from_list};

// Mutation _value() functions re-exported pub(crate) by parent for MCP
pub(crate) use mutations::{
    abandon_spec_value, block_spec_value, complete_spec_value, pause_spec_value,
    promote_spec_value, resume_spec_value,
};

// Create _value() function re-exported pub(crate) by parent for MCP
pub(crate) use create::create_spec_value;

// Split _value() function re-exported pub(crate) by parent for MCP
pub(crate) use split::split_spec_value;

// Functions called only from spec/mod.rs function bodies — pub(super) suffices
pub(super) use archive::{archive_spec, archive_stale_specs};
pub(super) use create::create_spec;
pub(super) use mutations::{
    abandon_spec, block_spec, complete_spec, pause_spec, promote_spec, resume_spec,
};
pub(super) use queries::{show_blocked_specs, show_ready_specs, show_spec, show_spec_list};
pub(super) use queue::next_spec;
pub(super) use split::split_spec;
