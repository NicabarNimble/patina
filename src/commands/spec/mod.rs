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
}

/// Archive a completed spec: tag, remove, update build.md, commit
pub fn archive(id: &str, dry_run: bool) -> Result<()> {
    internal::archive_spec(id, dry_run)
}
