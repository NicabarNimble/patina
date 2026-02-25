//! Spec lifecycle management
//!
//! This module follows the dependable-rust pattern:
//! - Public interface (this file): clean API for spec operations
//! - Internal implementation: all logic in internal/

mod internal;

// Data types and functions re-exported for session integration (Phase 5)
pub(crate) use internal::{
    get_all_specs, get_blocked_specs, load_dep_counts, spec_age_days_from_list, ListFilters,
};

// Query data functions re-exported for MCP (Phase 6)
pub(crate) use internal::{check_spec_value, get_ready_specs, next_spec_value, show_spec_value};

// Mutation _value() functions re-exported for MCP (Phase 6)
pub(crate) use internal::{
    abandon_spec_value, block_spec_value, complete_spec_value, create_spec_value, pause_spec_value,
    promote_spec_value, resume_spec_value, set_spec_value, split_spec_value,
};

use anyhow::Result;

/// Spec CLI subcommands (used by main.rs via clap)
#[derive(Debug, Clone, clap::Subcommand)]
pub enum SpecCommands {
    /// Create a new spec draft
    Create {
        /// Spec type: feat, fix, refactor, explore
        r#type: String,

        /// Spec identifier (kebab-case)
        id: String,

        /// Human title (defaults to "<type>: <id>")
        #[arg(long)]
        title: Option<String>,

        /// One-line problem statement for the blockquote
        #[arg(long)]
        description: Option<String>,

        /// Spec IDs this is blocked by
        #[arg(long)]
        blocked_by: Vec<String>,

        /// Related file paths
        #[arg(long)]
        related: Vec<String>,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

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

        /// Bypass exit criteria check
        #[arg(long)]
        force: bool,

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

    /// Split a spec: ship done work, draft remainder as new spec
    Split {
        /// Spec ID to split
        id: String,

        /// Override new spec ID (defaults to <id>-v2, -v3, etc.)
        #[arg(long)]
        new_id: Option<String>,

        /// Description for the new spec's remaining work
        #[arg(long)]
        description: Option<String>,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Show full spec context (body, design, key files)
    Show {
        /// Spec ID to show
        id: String,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Check exit criteria status for a spec
    Check {
        /// Spec ID to check
        id: String,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Set a metadata field on a spec
    Set {
        /// Spec ID
        id: String,

        /// Field to set (beliefs, related, references, target)
        field: String,

        /// Value (+value to add, -value to remove for lists; value for scalars)
        #[arg(allow_hyphen_values = true)]
        value: String,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Recommend the next spec to work on
    Next {
        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },
}

/// Create a new spec draft
pub fn create(
    spec_type: &str,
    id: &str,
    title: Option<&str>,
    description: Option<&str>,
    blocked_by: Vec<String>,
    related: Vec<String>,
    json: bool,
) -> Result<()> {
    internal::create_spec(spec_type, id, title, description, blocked_by, related, json)
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
pub fn complete(id: &str, major: bool, force: bool, json: bool) -> Result<()> {
    internal::complete_spec(id, major, force, json)
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

/// Split a spec: ship done work, draft remainder as new spec
pub fn split(id: &str, new_id: Option<&str>, description: Option<&str>, json: bool) -> Result<()> {
    internal::split_spec(id, new_id, description, json)
}

/// Set a metadata field on a spec
pub fn set(id: &str, field: &str, value: &str, json: bool) -> Result<()> {
    internal::set_spec(id, field, value, json)
}

/// Show full spec context (body, design, key files)
pub fn show(id: &str, json: bool) -> Result<()> {
    internal::show_spec(id, json)
}

/// Check exit criteria status for a spec
pub fn check(id: &str, json: bool) -> Result<()> {
    internal::check_spec(id, json)
}

/// Recommend the next spec to work on
pub fn next(json: bool) -> Result<()> {
    internal::next_spec(json)
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
    fn create_basic() {
        let cmd = parse(&["create", "feat", "my-feature"]).unwrap();
        match cmd {
            SpecCommands::Create {
                r#type,
                id,
                title,
                json,
                ..
            } => {
                assert_eq!(r#type, "feat");
                assert_eq!(id, "my-feature");
                assert!(title.is_none());
                assert!(!json);
            }
            _ => panic!("expected Create"),
        }
    }

    #[test]
    fn create_with_options() {
        let cmd = parse(&[
            "create",
            "fix",
            "my-bug",
            "--title",
            "Fix the bug",
            "--blocked-by",
            "other-spec",
            "--json",
        ])
        .unwrap();
        match cmd {
            SpecCommands::Create {
                r#type,
                id,
                title,
                blocked_by,
                json,
                ..
            } => {
                assert_eq!(r#type, "fix");
                assert_eq!(id, "my-bug");
                assert_eq!(title.as_deref(), Some("Fix the bug"));
                assert_eq!(blocked_by, vec!["other-spec"]);
                assert!(json);
            }
            _ => panic!("expected Create"),
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

    #[test]
    fn set_basic() {
        let cmd = parse(&["set", "my-spec", "beliefs", "+some-belief"]).unwrap();
        match cmd {
            SpecCommands::Set {
                id,
                field,
                value,
                json,
            } => {
                assert_eq!(id, "my-spec");
                assert_eq!(field, "beliefs");
                assert_eq!(value, "+some-belief");
                assert!(!json);
            }
            _ => panic!("expected Set"),
        }
    }

    #[test]
    fn set_with_json() {
        let cmd = parse(&["set", "my-spec", "target", "v0.33.0", "--json"]).unwrap();
        match cmd {
            SpecCommands::Set {
                id,
                field,
                value,
                json,
            } => {
                assert_eq!(id, "my-spec");
                assert_eq!(field, "target");
                assert_eq!(value, "v0.33.0");
                assert!(json);
            }
            _ => panic!("expected Set"),
        }
    }
}
