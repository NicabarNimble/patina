//! Spec lifecycle management
//!
//! This module follows the dependable-rust pattern:
//! - Public interface (this file): clean API for spec operations
//! - Internal implementation: all logic in internal/

mod internal;

// Data types and functions re-exported for session integration (Phase 5)
pub(crate) use internal::{get_all_specs, get_blocked_specs, ListFilters};

// Query data functions re-exported for MCP (Phase 6)
pub(crate) use internal::{
    check_spec_value, get_ready_specs, handoff_spec_value, history_spec_value, next_spec_value,
    packet_spec_value, prompt_spec_value, show_spec_value,
};

// Mutation _value() functions re-exported for MCP (Phase 6)
pub(crate) use internal::{
    abandon_spec_value, block_spec_value, complete_spec_value, create_spec_value, pause_spec_value,
    promote_spec_value, rename_spec_value, reopen_spec_value, resume_spec_value, set_spec_value,
    split_spec_value,
};

use anyhow::Result;
use patina::spec::SpecStatus;
use serde_json::{json, Value};

/// Spec CLI subcommands (used by main.rs via clap)
#[derive(Debug, Clone, clap::Subcommand, serde::Serialize, serde::Deserialize)]
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

        /// Override readiness lint when promoting draft -> ready
        #[arg(long)]
        force: bool,

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

        /// Show compact implementation handoff view
        #[arg(long)]
        handoff: bool,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Generate build-ready execution prompt packet from spec/design
    Prompt {
        /// Spec ID to generate prompt for
        id: String,

        /// Output as JSON packet
        #[arg(long)]
        json: bool,
    },

    /// Generate continuation handoff packet for next agent
    Handoff {
        /// Spec ID to generate handoff for
        id: String,

        /// Output as JSON packet
        #[arg(long)]
        json: bool,
    },

    /// Generate combined prompt + handoff packet
    Packet {
        /// Spec ID to generate packet for
        id: String,

        /// Output as JSON packet bundle
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

    /// Show lifecycle history from git tags
    History {
        /// Spec ID to show history for
        id: String,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Rename a spec id (moves spec directory)
    Rename {
        /// Current spec ID
        id: String,

        /// New spec ID
        new_id: String,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Reopen a completed or abandoned spec back to active
    Reopen {
        /// Spec ID to reopen
        id: String,

        /// Output as JSON (for agent use)
        #[arg(long)]
        json: bool,
    },

    /// Set a metadata field on a spec
    Set {
        /// Spec ID
        id: String,

        /// Field to set (beliefs, related, references, blocked_by, target)
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

impl SpecCommands {
    fn wants_json(&self) -> bool {
        match self {
            Self::Create { json, .. }
            | Self::Ready { json }
            | Self::Blocked { json }
            | Self::List { json, .. }
            | Self::Promote { json, .. }
            | Self::Complete { json, .. }
            | Self::Abandon { json, .. }
            | Self::Pause { json, .. }
            | Self::Resume { json, .. }
            | Self::Block { json, .. }
            | Self::Split { json, .. }
            | Self::Show { json, .. }
            | Self::Prompt { json, .. }
            | Self::Handoff { json, .. }
            | Self::Packet { json, .. }
            | Self::Check { json, .. }
            | Self::History { json, .. }
            | Self::Set { json, .. }
            | Self::Next { json }
            | Self::Rename { json, .. }
            | Self::Reopen { json, .. } => *json,
            Self::Archive { .. } => false,
        }
    }
}

pub fn execute(command: SpecCommands) -> Result<()> {
    if !command.wants_json() {
        match &command {
            SpecCommands::Complete { id, .. } => {
                if !confirm(&format!("Complete spec '{}' now?", id))? {
                    println!("Aborted.");
                    return Ok(());
                }
            }
            SpecCommands::Abandon { id, .. } => {
                if !confirm(&format!("Abandon spec '{}' now?", id))? {
                    println!("Aborted.");
                    return Ok(());
                }
            }
            _ => {}
        }
    }

    let payload = json!({ "command": command });
    let client = patina::mother::Client::new("localhost:50051".to_string());
    let response = client
        .child_action("spec-manager", "dispatch", &payload)
        .map_err(|e| {
            anyhow::anyhow!(
                "spec-manager unavailable via Mother (start with `patina mother start`): {}",
                e
            )
        })?;

    if let Some(text) = response.get("text").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            println!("{}", text);
        }
    }

    if let Some(data) = response.get("data") {
        if response
            .get("json")
            .and_then(|v| v.as_bool())
            .unwrap_or_else(|| command.wants_json())
            || response.get("text").is_none()
        {
            println!("{}", serde_json::to_string_pretty(data)?);
        }
    }

    Ok(())
}

pub(crate) fn execute_value(command: SpecCommands) -> Result<Value> {
    let json_mode = command.wants_json();
    let (text, data) = match command {
        SpecCommands::Create {
            r#type,
            id,
            title,
            description,
            blocked_by,
            related,
            ..
        } => (
            Some(format!("Created spec '{}'", id)),
            serde_json::to_value(create_spec_value(
                &r#type,
                &id,
                title.as_deref(),
                description.as_deref(),
                blocked_by,
                related,
            )?)
            .ok(),
        ),
        SpecCommands::Archive { id, dry_run, stale } => {
            if stale {
                internal::archive_stale_specs(dry_run)?;
                (
                    Some("Archived stale specs".to_string()),
                    Some(json!({"stale": true, "dry_run": dry_run})),
                )
            } else if let Some(id) = id {
                internal::archive_spec(&id, dry_run)?;
                (
                    Some(format!("Archived spec '{}'", id)),
                    Some(json!({"id": id, "dry_run": dry_run})),
                )
            } else {
                anyhow::bail!("Spec ID required. Use `patina spec archive <id>` or --stale");
            }
        }
        SpecCommands::Ready { .. } => (None, serde_json::to_value(get_ready_specs()?).ok()),
        SpecCommands::Blocked { .. } => (None, serde_json::to_value(get_blocked_specs()?).ok()),
        SpecCommands::List { status, target, .. } => {
            let parsed_status = status
                .as_deref()
                .map(|s| s.parse::<SpecStatus>())
                .transpose()?;
            let filters = ListFilters {
                status: parsed_status,
                target,
            };
            (None, serde_json::to_value(get_all_specs(&filters)?).ok())
        }
        SpecCommands::Promote { id, force, .. } => (
            None,
            serde_json::to_value(promote_spec_value(&id, force)?).ok(),
        ),
        SpecCommands::Complete {
            id, major, force, ..
        } => (
            None,
            serde_json::to_value(complete_spec_value(&id, major, force)?).ok(),
        ),
        SpecCommands::Abandon { id, reason, .. } => (
            None,
            serde_json::to_value(abandon_spec_value(&id, reason.as_deref())?).ok(),
        ),
        SpecCommands::Pause { id, reason, .. } => (
            None,
            serde_json::to_value(pause_spec_value(&id, &reason)?).ok(),
        ),
        SpecCommands::Resume { id, force, .. } => (
            None,
            serde_json::to_value(resume_spec_value(&id, force)?).ok(),
        ),
        SpecCommands::Block { id, by, reason, .. } => (
            None,
            serde_json::to_value(block_spec_value(&id, &by, &reason)?).ok(),
        ),
        SpecCommands::Split {
            id,
            new_id,
            description,
            ..
        } => (
            None,
            serde_json::to_value(split_spec_value(
                &id,
                new_id.as_deref(),
                description.as_deref(),
            )?)
            .ok(),
        ),
        SpecCommands::Show { id, .. } => (None, serde_json::to_value(show_spec_value(&id)?).ok()),
        SpecCommands::Prompt { id, .. } => {
            (None, serde_json::to_value(prompt_spec_value(&id)?).ok())
        }
        SpecCommands::Handoff { id, .. } => {
            (None, serde_json::to_value(handoff_spec_value(&id)?).ok())
        }
        SpecCommands::Packet { id, .. } => {
            (None, serde_json::to_value(packet_spec_value(&id)?).ok())
        }
        SpecCommands::Set {
            id, field, value, ..
        } => (
            None,
            serde_json::to_value(set_spec_value(&id, &field, &value)?).ok(),
        ),
        SpecCommands::Next { .. } => (None, serde_json::to_value(next_spec_value()?).ok()),
        SpecCommands::Check { id, .. } => (None, serde_json::to_value(check_spec_value(&id)?).ok()),
        SpecCommands::History { id, .. } => {
            (None, serde_json::to_value(history_spec_value(&id)?).ok())
        }
        SpecCommands::Rename { id, new_id, .. } => (
            None,
            serde_json::to_value(rename_spec_value(&id, &new_id)?).ok(),
        ),
        SpecCommands::Reopen { id, .. } => {
            (None, serde_json::to_value(reopen_spec_value(&id)?).ok())
        }
    };

    Ok(json!({
        "child": "spec-manager",
        "json": json_mode,
        "text": text,
        "data": data.unwrap_or(serde_json::Value::Null)
    }))
}

fn confirm(prompt: &str) -> Result<bool> {
    use std::io::{stdin, stdout, Write};

    print!("{} [y/N] ", prompt);
    stdout().flush()?;
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    Ok(trimmed.eq_ignore_ascii_case("y") || trimmed.eq_ignore_ascii_case("yes"))
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
