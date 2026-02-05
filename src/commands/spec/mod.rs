//! Spec lifecycle management
//!
//! This module follows the dependable-rust pattern:
//! - Public interface (this file): clean API for spec operations
//! - Internal implementation: all logic in internal.rs

mod internal;

use anyhow::Result;

/// Spec CLI subcommands (used by main.rs via clap)
#[derive(Debug, Clone, clap::Subcommand)]
pub enum SpecCommands {
    /// Archive a completed spec (git tag + remove from tree)
    Archive {
        /// Spec ID to archive (e.g., "session-092-hardening")
        id: String,

        /// Dry run - show what would happen without executing
        #[arg(long)]
        dry_run: bool,
    },

    /// Show specs ready to work on (unblocked, status=ready/active)
    Ready {
        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Show specs blocked by incomplete dependencies
    Blocked {
        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Update a spec's status (draft → ready → active → complete)
    Status {
        /// Spec ID to update
        id: String,

        /// New status (draft, ready, active, complete, abandoned)
        status: String,
    },

    /// List all specs with optional filters
    List {
        /// Filter by status (draft, ready, active, complete, abandoned)
        #[arg(long)]
        status: Option<String>,

        /// Filter by target version (e.g., v0.12.0)
        #[arg(long)]
        target: Option<String>,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },
}

/// Archive a completed spec: tag, remove, update build.md, commit
pub fn archive(id: &str, dry_run: bool) -> Result<()> {
    internal::archive_spec(id, dry_run)
}

/// Show specs ready to work on
pub fn ready(json: bool) -> Result<()> {
    internal::show_ready_specs(json)
}

/// Show specs blocked by incomplete dependencies
pub fn blocked(json: bool) -> Result<()> {
    internal::show_blocked_specs(json)
}

/// Update a spec's status
pub fn status(id: &str, new_status: &str) -> Result<()> {
    internal::update_spec_status(id, new_status)
}

/// List all specs with optional filters
pub fn list(status: Option<String>, target: Option<String>, json: bool) -> Result<()> {
    let filters = internal::ListFilters { status, target };
    internal::show_spec_list(&filters, json)
}
