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
        /// Spec ID to archive (required unless --stale)
        id: Option<String>,

        /// Dry run - show what would happen without executing
        #[arg(long)]
        dry_run: bool,

        /// Archive all completed/abandoned specs still in tree
        #[arg(long)]
        stale: bool,
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

        /// Force major version bump on complete (for 1.0.0 moments)
        #[arg(long)]
        major: bool,

        /// Skip auto-archive on complete/abandoned (preserve spec in tree)
        #[arg(long)]
        no_archive: bool,
    },

    /// List all specs with optional filters
    List {
        /// Filter by status (draft, ready, active, paused, blocked, complete, abandoned)
        #[arg(long)]
        status: Option<String>,

        /// Filter by target version (e.g., v0.12.0)
        #[arg(long)]
        target: Option<String>,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Promote a spec: draft → ready, or ready → active
    Promote {
        /// Spec ID to promote
        id: String,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Complete an active spec (release + archive)
    Complete {
        /// Spec ID to complete
        id: String,

        /// Force major version bump (for 1.0.0 moments)
        #[arg(long)]
        major: bool,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Abandon a spec (archive, no release)
    Abandon {
        /// Spec ID to abandon
        id: String,

        /// Reason for abandoning
        #[arg(long)]
        reason: Option<String>,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Pause an active spec with reason
    Pause {
        /// Spec ID to pause
        id: String,

        /// Why this spec is being paused (required)
        #[arg(long)]
        reason: String,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Resume a paused or blocked spec
    Resume {
        /// Spec ID to resume
        id: String,

        /// Force resume even if blockers aren't complete
        #[arg(long)]
        force: bool,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Block an active spec on another spec
    Block {
        /// Spec ID to block
        id: String,

        /// Blocking spec ID
        #[arg(long)]
        by: String,

        /// Reason for blocking
        #[arg(long)]
        reason: String,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },
}

/// Archive a completed spec: tag, remove, commit
pub fn archive(id: &str, dry_run: bool) -> Result<()> {
    internal::archive_spec(id, dry_run)
}

/// Archive all completed/abandoned specs still in tree
pub fn archive_stale(dry_run: bool) -> Result<()> {
    internal::archive_stale_specs(dry_run)
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
pub fn status(id: &str, new_status: &str, major: bool, no_archive: bool) -> Result<()> {
    internal::update_spec_status(id, new_status, major, no_archive)
}

/// List all specs with optional filters
pub fn list(status: Option<String>, target: Option<String>, json: bool) -> Result<()> {
    let filters = internal::ListFilters { status, target };
    internal::show_spec_list(&filters, json)
}

/// Promote a spec: draft → ready, ready → active
pub fn promote(id: &str, json: bool) -> Result<()> {
    internal::promote_spec(id, json)
}

/// Complete an active spec (release + archive)
pub fn complete(id: &str, major: bool, json: bool) -> Result<()> {
    internal::complete_spec(id, major, json)
}

/// Abandon a spec (archive, no release)
pub fn abandon(id: &str, reason: Option<&str>, json: bool) -> Result<()> {
    internal::abandon_spec(id, reason, json)
}

/// Pause an active spec with reason
pub fn pause(id: &str, reason: &str, json: bool) -> Result<()> {
    internal::pause_spec(id, reason, json)
}

/// Resume a paused or blocked spec
pub fn resume(id: &str, force: bool, json: bool) -> Result<()> {
    internal::resume_spec(id, force, json)
}

/// Block an active spec on another spec
pub fn block(id: &str, by: &str, reason: &str, json: bool) -> Result<()> {
    internal::block_spec(id, by, reason, json)
}

#[cfg(test)]
mod tests {
    use super::SpecCommands;
    use clap::Parser;

    // Minimal CLI struct for testing SpecCommands parsing
    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        command: SpecCommands,
    }

    fn parse(args: &[&str]) -> Result<SpecCommands, clap::Error> {
        TestCli::try_parse_from(std::iter::once("patina-spec").chain(args.iter().copied()))
            .map(|cli| cli.command)
    }

    #[test]
    fn status_with_no_archive_flag() {
        let cmd = parse(&["status", "my-spec", "complete", "--no-archive"]).unwrap();
        match cmd {
            SpecCommands::Status {
                id,
                status,
                no_archive,
                major,
            } => {
                assert_eq!(id, "my-spec");
                assert_eq!(status, "complete");
                assert!(no_archive);
                assert!(!major);
            }
            _ => panic!("expected Status"),
        }
    }

    #[test]
    fn status_defaults_archive_on() {
        let cmd = parse(&["status", "my-spec", "complete"]).unwrap();
        match cmd {
            SpecCommands::Status { no_archive, .. } => {
                assert!(!no_archive, "--no-archive should default to false");
            }
            _ => panic!("expected Status"),
        }
    }

    #[test]
    fn archive_with_id() {
        let cmd = parse(&["archive", "my-spec"]).unwrap();
        match cmd {
            SpecCommands::Archive { id, stale, dry_run } => {
                assert_eq!(id.as_deref(), Some("my-spec"));
                assert!(!stale);
                assert!(!dry_run);
            }
            _ => panic!("expected Archive"),
        }
    }

    #[test]
    fn archive_stale_no_id() {
        let cmd = parse(&["archive", "--stale"]).unwrap();
        match cmd {
            SpecCommands::Archive { id, stale, .. } => {
                assert!(id.is_none());
                assert!(stale);
            }
            _ => panic!("expected Archive"),
        }
    }

    #[test]
    fn archive_stale_dry_run() {
        let cmd = parse(&["archive", "--stale", "--dry-run"]).unwrap();
        match cmd {
            SpecCommands::Archive { id, stale, dry_run } => {
                assert!(id.is_none());
                assert!(stale);
                assert!(dry_run);
            }
            _ => panic!("expected Archive"),
        }
    }

    #[test]
    fn archive_no_id_no_stale_still_parses() {
        // clap accepts this — validation happens at dispatch time in main.rs
        let cmd = parse(&["archive"]).unwrap();
        match cmd {
            SpecCommands::Archive { id, stale, .. } => {
                assert!(id.is_none());
                assert!(!stale);
            }
            _ => panic!("expected Archive"),
        }
    }
}
