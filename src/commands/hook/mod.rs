//! Git hook handlers for automatic incremental scrape
//!
//! Shell shims in `resources/git/` call `patina hook <event>`.
//! All logic lives in Rust — the shell scripts are 2-line guards.
//!
//! This module follows the dependable-rust pattern:
//! - Public interface (this file): clean API + clap subcommands
//! - Internal implementation: all logic in internal.rs

mod internal;

use anyhow::Result;

/// Hook CLI subcommands (used by main.rs via clap)
#[derive(Debug, Clone, clap::Subcommand)]
pub enum HookCommands {
    /// Handle post-commit hook (fork scrape to background)
    PostCommit,

    /// Handle post-merge hook (fork scrape to background)
    PostMerge,
}

/// Execute a hook subcommand
pub fn execute(command: HookCommands) -> Result<()> {
    match command {
        HookCommands::PostCommit => internal::handle_post_commit(),
        HookCommands::PostMerge => internal::handle_post_merge(),
    }
}
