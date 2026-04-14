//! Canonical spec frontmatter format
//!
//! This module defines the contract for spec files. All commands that read/write
//! spec frontmatter should use these types and functions.
//!
//! Design rationale:
//! - [[system-owns-format]]: Rust struct owns the format, deterministic output
//! - [[milestones-in-specs]]: Data lives in specs, derive indexes
//!
//! # Example
//!
//! ```ignore
//! use patina::spec::{parse_spec_file, serialize_spec_file};
//!
//! let content = std::fs::read_to_string("SPEC.md")?;
//! let (mut frontmatter, body) = parse_spec_file(&content)?;
//! frontmatter.status = Some("ready".to_string());
//! let new_content = serialize_spec_file(&frontmatter, &body)?;
//! ```

use crate::commands::spec::internal;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, clap::Subcommand, serde::Serialize, serde::Deserialize)]
pub enum SpecCommands {
    Create {
        r#type: String,
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        blocked_by: Vec<String>,
        #[arg(long)]
        related: Vec<String>,
        /// Target Patina project path or project UID (8-hex) where the spec should live
        #[arg(long)]
        project: Option<String>,
        /// Allow creating in another project even when that target has no active session
        #[arg(long, default_value_t = false)]
        #[serde(default)]
        force_cross_project: bool,
        /// Internal: source project root used for cross-project provenance linking
        #[arg(long, hide = true)]
        #[serde(default)]
        origin_project: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Archive {
        id: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        stale: bool,
    },
    Ready {
        #[arg(long)]
        json: bool,
    },
    Blocked {
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Promote {
        id: String,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    Complete {
        id: String,
        #[arg(long)]
        major: bool,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    Abandon {
        id: String,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Pause {
        id: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        json: bool,
    },
    Resume {
        id: String,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    Block {
        id: String,
        #[arg(long)]
        by: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        json: bool,
    },
    Split {
        id: String,
        #[arg(long)]
        new_id: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Show {
        id: String,
        #[arg(long)]
        handoff: bool,
        #[arg(long)]
        json: bool,
    },
    Prompt {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Handoff {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Packet {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Check {
        id: String,
        #[arg(long)]
        json: bool,
    },
    History {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Rename {
        id: String,
        new_id: String,
        #[arg(long)]
        json: bool,
    },
    Reopen {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Set {
        id: String,
        field: String,
        #[arg(allow_hyphen_values = true)]
        value: String,
        #[arg(long)]
        json: bool,
    },
    Next {
        #[arg(long)]
        json: bool,
    },
}

impl SpecCommands {
    pub fn wants_json(&self) -> bool {
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

#[derive(Debug, Clone, Default)]
struct SpecRouteContext {
    project: Option<String>,
    origin_project: Option<String>,
}

thread_local! {
    static SPEC_ROUTE_CONTEXT: RefCell<Option<SpecRouteContext>> = const { RefCell::new(None) };
}

pub fn execute_command_value_with_route(
    command: SpecCommands,
    project: Option<String>,
    origin_project: Option<String>,
) -> Result<Value> {
    let previous = SPEC_ROUTE_CONTEXT.with(|slot| {
        slot.replace(Some(SpecRouteContext {
            project,
            origin_project,
        }))
    });
    let result = execute_command_value(command);
    SPEC_ROUTE_CONTEXT.with(|slot| {
        slot.replace(previous);
    });
    result
}

fn current_route_context() -> SpecRouteContext {
    SPEC_ROUTE_CONTEXT
        .with(|slot| slot.borrow().clone())
        .unwrap_or_default()
}

fn looks_like_project_uid(value: &str) -> bool {
    value.len() == 8
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn canonical_project_root(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    absolute
        .canonicalize()
        .with_context(|| format!("Project path not found: {}", absolute.display()))
}

fn resolve_project_uid_to_path(uid: &str) -> Result<PathBuf> {
    let store = crate::mother::MotherRuntimeStore::default();
    let projects = store.list_registered_projects()?;
    let Some(project) = projects.into_iter().find(|entry| entry.project_uid == uid) else {
        anyhow::bail!(
            "Unknown project uid '{}'. Register it first by running `patina init .` in that project.",
            uid
        );
    };
    canonical_project_root(Path::new(&project.project_path))
}

fn ensure_patina_project(path: &Path) -> Result<()> {
    if !crate::project::is_patina_project(path) {
        anyhow::bail!(
            "Target is not a Patina project: {}\nRun `patina init .` in that directory first.",
            path.display()
        );
    }
    Ok(())
}

fn resolve_target_project_root(selector: Option<&str>) -> Result<(PathBuf, String)> {
    let root = match selector.map(str::trim).filter(|value| !value.is_empty()) {
        None => std::env::current_dir()?,
        Some(value) if looks_like_project_uid(value) && !Path::new(value).exists() => {
            resolve_project_uid_to_path(value)?
        }
        Some(value) => canonical_project_root(Path::new(value))?,
    };

    ensure_patina_project(&root)?;
    let uid = crate::project::register_with_mother(&root)?;
    Ok((root, uid))
}

fn resolve_origin_project_root(origin_project: Option<&str>) -> Result<PathBuf> {
    let root = match origin_project
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => canonical_project_root(Path::new(value))?,
        None => std::env::current_dir()?,
    };
    ensure_patina_project(&root)?;
    Ok(root)
}

fn strip_routing_from_command(command: &SpecCommands) -> SpecCommands {
    match command.clone() {
        SpecCommands::Create {
            r#type,
            id,
            title,
            description,
            blocked_by,
            related,
            json,
            ..
        } => SpecCommands::Create {
            r#type,
            id,
            title,
            description,
            blocked_by,
            related,
            project: None,
            force_cross_project: false,
            origin_project: None,
            json,
        },
        other => other,
    }
}

fn execute_spec_command_in_project(project_root: &Path, command: &SpecCommands) -> Result<Value> {
    let exe = std::env::current_exe().context("Failed to resolve patina executable path")?;
    let serialized = serde_json::to_string(&strip_routing_from_command(command))?;

    let output = Command::new(exe)
        .current_dir(project_root)
        .env("PATINA_SPEC_DIRECT", "1")
        .env("PATINA_SPEC_DIRECT_COMMAND_JSON", serialized)
        .args(["spec", "next", "--json"])
        .output()
        .with_context(|| {
            format!(
                "Failed to execute spec command in project {}",
                project_root.display()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        anyhow::bail!(
            "Cross-project spec dispatch failed in {}: {}",
            project_root.display(),
            detail
        );
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| anyhow::anyhow!("Failed to decode subprocess output: {}", e))?;
    serde_json::from_str(stdout.trim())
        .map_err(|e| anyhow::anyhow!("Failed to parse subprocess JSON output: {}", e))
}

pub fn execute_command_value(command: SpecCommands) -> Result<Value> {
    let route = current_route_context();

    if route.project.is_some() && !matches!(&command, SpecCommands::Create { .. }) {
        let (target_root, _) = resolve_target_project_root(route.project.as_deref())?;
        return execute_spec_command_in_project(&target_root, &command);
    }

    let json_mode = command.wants_json();
    let (text, data) = match command {
        SpecCommands::Create {
            r#type,
            id,
            title,
            description,
            blocked_by,
            mut related,
            project,
            force_cross_project,
            origin_project,
            ..
        } => {
            let selected_project = project.or(route.project.clone());
            let selected_origin = origin_project.or(route.origin_project.clone());

            let origin_root = resolve_origin_project_root(selected_origin.as_deref())?;
            let origin_uid = crate::project::register_with_mother(&origin_root)?;
            let (target_root, target_uid) =
                resolve_target_project_root(selected_project.as_deref())?;
            let cross_project = origin_uid != target_uid;

            if cross_project {
                let origin_link = format!("origin-project:{}", origin_uid);
                if !related.iter().any(|entry| entry == &origin_link) {
                    related.push(origin_link);
                }

                if !force_cross_project
                    && crate::session::current_session_file_id(&target_root)?.is_none()
                {
                    anyhow::bail!(
                        "Cross-project spec create denied: target project '{}' has no active session.\n\
                         Remediation: run `patina ai pi --path {} --title \"spec-init\"` first,\n\
                         or re-run with --force-cross-project.",
                        target_uid,
                        target_root.display()
                    );
                }
            }

            let mut created = internal::create_spec_value_for_project(
                &target_root,
                Some(&target_uid),
                &r#type,
                &id,
                title.as_deref(),
                description.as_deref(),
                blocked_by,
                related,
            )?;
            created.cross_project = cross_project;
            if cross_project {
                created.origin_project_uid = Some(origin_uid);
            }

            (
                Some(format!(
                    "Created spec '{}'{}",
                    id,
                    if cross_project {
                        " (cross-project)"
                    } else {
                        ""
                    }
                )),
                serde_json::to_value(created).ok(),
            )
        }
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
        SpecCommands::Ready { .. } => (
            None,
            serde_json::to_value(internal::get_ready_specs()?).ok(),
        ),
        SpecCommands::Blocked { .. } => (
            None,
            serde_json::to_value(internal::get_blocked_specs()?).ok(),
        ),
        SpecCommands::List { status, target, .. } => {
            let parsed_status = status
                .as_deref()
                .map(|s| s.parse::<SpecStatus>())
                .transpose()?;
            let filters = internal::ListFilters {
                status: parsed_status,
                target,
            };
            (
                None,
                serde_json::to_value(internal::get_all_specs(&filters)?).ok(),
            )
        }
        SpecCommands::Promote { id, force, .. } => (
            None,
            serde_json::to_value(internal::promote_spec_value(&id, force)?).ok(),
        ),
        SpecCommands::Complete {
            id, major, force, ..
        } => (
            None,
            serde_json::to_value(internal::complete_spec_value(&id, major, force)?).ok(),
        ),
        SpecCommands::Abandon { id, reason, .. } => (
            None,
            serde_json::to_value(internal::abandon_spec_value(&id, reason.as_deref())?).ok(),
        ),
        SpecCommands::Pause { id, reason, .. } => (
            None,
            serde_json::to_value(internal::pause_spec_value(&id, &reason)?).ok(),
        ),
        SpecCommands::Resume { id, force, .. } => (
            None,
            serde_json::to_value(internal::resume_spec_value(&id, force)?).ok(),
        ),
        SpecCommands::Block { id, by, reason, .. } => (
            None,
            serde_json::to_value(internal::block_spec_value(&id, &by, &reason)?).ok(),
        ),
        SpecCommands::Split {
            id,
            new_id,
            description,
            ..
        } => (
            None,
            serde_json::to_value(internal::split_spec_value(
                &id,
                new_id.as_deref(),
                description.as_deref(),
            )?)
            .ok(),
        ),
        SpecCommands::Show { id, .. } => (
            None,
            serde_json::to_value(internal::show_spec_value(&id)?).ok(),
        ),
        SpecCommands::Prompt { id, .. } => (
            None,
            serde_json::to_value(internal::prompt_spec_value(&id)?).ok(),
        ),
        SpecCommands::Handoff { id, .. } => (
            None,
            serde_json::to_value(internal::handoff_spec_value(&id)?).ok(),
        ),
        SpecCommands::Packet { id, .. } => (
            None,
            serde_json::to_value(internal::packet_spec_value(&id)?).ok(),
        ),
        SpecCommands::Set {
            id, field, value, ..
        } => (
            None,
            serde_json::to_value(internal::set_spec_value(&id, &field, &value)?).ok(),
        ),
        SpecCommands::Next { .. } => (
            None,
            serde_json::to_value(internal::next_spec_value()?).ok(),
        ),
        SpecCommands::Check { id, .. } => (
            None,
            serde_json::to_value(internal::check_spec_value(&id)?).ok(),
        ),
        SpecCommands::History { id, .. } => (
            None,
            serde_json::to_value(internal::history_spec_value(&id)?).ok(),
        ),
        SpecCommands::Rename { id, new_id, .. } => (
            None,
            serde_json::to_value(internal::rename_spec_value(&id, &new_id)?).ok(),
        ),
        SpecCommands::Reopen { id, .. } => (
            None,
            serde_json::to_value(internal::reopen_spec_value(&id)?).ok(),
        ),
    };

    Ok(json!({
        "child": "spec-manager",
        "json": json_mode,
        "text": text,
        "data": data.unwrap_or(serde_json::Value::Null)
    }))
}

// ============================================================================
// Spec Frontmatter Types
// ============================================================================

/// Sessions can be either a simple list or a structured object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Sessions {
    /// Simple list of session IDs: [20260108-200725, ...]
    List(Vec<String>),
    /// Structured with origin and work: { origin: ..., work: [...] }
    Structured {
        #[serde(skip_serializing_if = "Option::is_none")]
        origin: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        work: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        updated: Option<String>,
    },
}

/// Milestone in spec frontmatter
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpecMilestoneEntry {
    pub version: String,
    pub name: String,
    pub status: String,
}

/// Structured exit criterion — machine-readable contract for spec completion.
///
/// Each criterion has a stable id for programmatic reference, human text,
/// checked state, and an optional verify command/instruction.
///
/// Accepts both string shorthand and full struct form in YAML:
///
/// ```yaml
/// exit_criteria:
/// - All tests pass                              # string shorthand
/// - id: rollback-db                             # full struct
///   text: DB rolls back on failure
///   checked: false
/// ```
///
/// String shorthand auto-generates id by slugifying the text.
/// Follows the same pattern as `Sessions` (untagged enum for flexible input).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ExitCriterion {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub checked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify: Option<String>,
}

impl<'de> serde::Deserialize<'de> for ExitCriterion {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        struct ExitCriterionVisitor;

        impl<'de> de::Visitor<'de> for ExitCriterionVisitor {
            type Value = ExitCriterion;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a string or an ExitCriterion object")
            }

            fn visit_str<E: de::Error>(self, text: &str) -> std::result::Result<Self::Value, E> {
                Ok(ExitCriterion {
                    id: slugify(text),
                    text: text.to_string(),
                    checked: false,
                    verify: None,
                })
            }

            fn visit_map<M: de::MapAccess<'de>>(
                self,
                map: M,
            ) -> std::result::Result<Self::Value, M::Error> {
                #[derive(Deserialize)]
                struct Inner {
                    id: String,
                    text: String,
                    #[serde(default)]
                    checked: bool,
                    #[serde(default)]
                    verify: Option<String>,
                }
                let inner = Inner::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(ExitCriterion {
                    id: inner.id,
                    text: inner.text,
                    checked: inner.checked,
                    verify: inner.verify,
                })
            }
        }

        deserializer.deserialize_any(ExitCriterionVisitor)
    }
}

/// Slugify text into a kebab-case id.
///
/// "All tests pass" → "all-tests-pass"
/// "DB rolls back on failure" → "db-rolls-back-on-failure"
fn slugify(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Complete spec frontmatter - the canonical contract for spec files
///
/// All fields except `r#type` and `id` are optional to handle legacy specs.
/// Use `#[serde(skip_serializing_if)]` to avoid writing empty fields.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpecFrontmatter {
    /// Spec type: feat, fix, refactor, explore, etc.
    #[serde(default)]
    pub r#type: String,

    /// Unique identifier (matches filename convention)
    #[serde(default)]
    pub id: String,

    /// Status: draft, ready, active, paused, blocked, complete, abandoned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<SpecStatus>,

    /// Creation date (YYYY-MM-DD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,

    /// Last update date (YYYY-MM-DD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,

    /// Target version (e.g., "v0.12.0") — spec-as-work-item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// Specs that block this one — spec-as-work-item
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<String>,

    /// Specs that this one blocks — spec-as-work-item
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<String>,

    /// Session references
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions: Option<Sessions>,

    /// Related specs/files
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<String>,

    /// Belief references
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beliefs: Vec<String>,

    /// Schema references (fact types this spec introduces)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub schemas: Vec<String>,

    /// External references
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,

    /// Structured exit criteria — machine-readable completion contract.
    /// Always serialized (even when empty) so the field is visible in YAML
    /// as a prompt to define criteria. Unlike optional metadata fields,
    /// exit criteria are contractual and should always be explicit.
    #[serde(default)]
    pub exit_criteria: Vec<ExitCriterion>,

    /// Version milestones
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub milestones: Vec<SpecMilestoneEntry>,

    /// Current milestone being worked on
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_milestone: Option<String>,

    /// Why this spec was paused (required on pause)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused_reason: Option<String>,

    /// When paused (ISO 8601 date, UTC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused_date: Option<String>,

    /// Tag ref for resume diffs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused_at_tag: Option<String>,

    /// Why this spec was blocked
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,

    /// When blocked (ISO 8601 date, UTC)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_date: Option<String>,

    /// Parent spec ID (set by split)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split_from: Option<String>,

    /// Commit SHA this spec was last validated against for freshness checks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validated_against_commit: Option<String>,

    /// Last freshness validation date/time (ISO date or RFC3339).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_freshness_check: Option<String>,

    /// Paths/contracts validated during the freshness pass.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub freshness_scope: Vec<String>,
}

// ============================================================================
// Spec Status Enum
// ============================================================================

/// Canonical list of valid spec statuses (for error messages, help text, tests).
pub const SPEC_STATUSES: &[&str] = &[
    "draft",
    "ready",
    "active",
    "paused",
    "blocked",
    "complete",
    "abandoned",
];

/// Typed spec status — the spec lifecycle state machine.
///
/// Replaces `status: String` with compiler-enforced exhaustive matching.
/// Follows [[enum-not-string-for-finite-states]]: 7 variants, no stringly-typed
/// comparisons in control flow.
///
/// `#[serde(alias = "done")]` on Complete is defensive — no spec files on disk
/// carry `status: done`, but historical DB/tag data might. See ADR-2 in DESIGN.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecStatus {
    Draft,
    Ready,
    Active,
    Paused,
    Blocked,
    #[serde(alias = "done")]
    Complete,
    Abandoned,
}

/// Error when parsing an invalid spec status string.
#[derive(Debug)]
pub struct SpecStatusError {
    pub got: String,
}

impl std::fmt::Display for SpecStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid spec status \"{}\" (expected one of: {})",
            self.got,
            SPEC_STATUSES.join(", ")
        )
    }
}

impl std::error::Error for SpecStatusError {}

impl std::str::FromStr for SpecStatus {
    type Err = SpecStatusError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "draft" => Ok(SpecStatus::Draft),
            "ready" => Ok(SpecStatus::Ready),
            "active" => Ok(SpecStatus::Active),
            "paused" => Ok(SpecStatus::Paused),
            "blocked" => Ok(SpecStatus::Blocked),
            "complete" | "completed" | "done" => Ok(SpecStatus::Complete),
            "abandoned" => Ok(SpecStatus::Abandoned),
            _ => Err(SpecStatusError { got: s.to_string() }),
        }
    }
}

impl std::fmt::Display for SpecStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl SpecStatus {
    /// Canonical string form (matches YAML frontmatter values).
    pub fn as_str(&self) -> &'static str {
        match self {
            SpecStatus::Draft => "draft",
            SpecStatus::Ready => "ready",
            SpecStatus::Active => "active",
            SpecStatus::Paused => "paused",
            SpecStatus::Blocked => "blocked",
            SpecStatus::Complete => "complete",
            SpecStatus::Abandoned => "abandoned",
        }
    }

    /// Whether this status represents a terminal state (complete or abandoned).
    ///
    /// Replaces the `== "complete" || == "done"` pattern across 6 call sites.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete | Self::Abandoned)
    }
}

// ============================================================================
// Spec Type Enum
// ============================================================================

/// Canonical list of valid spec types (for error messages, help text, tests).
pub const SPEC_TYPES: &[&str] = &["feat", "fix", "refactor", "explore"];

/// Typed spec type — parse from string at boundaries, match internally.
///
/// Follows [[boundary-string-internal-enum]]: SpecFrontmatter.r#type stays
/// String for serde compatibility; this enum is used for validation and
/// exhaustive matching in new code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecType {
    Feat,
    Fix,
    Refactor,
    Explore,
}

/// Error when parsing an invalid spec type string.
#[derive(Debug)]
pub struct SpecTypeError {
    pub got: String,
}

impl std::fmt::Display for SpecTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid spec type \"{}\" (expected one of: {})",
            self.got,
            SPEC_TYPES.join(", ")
        )
    }
}

impl std::error::Error for SpecTypeError {}

impl std::str::FromStr for SpecType {
    type Err = SpecTypeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "feat" => Ok(SpecType::Feat),
            "fix" => Ok(SpecType::Fix),
            "refactor" => Ok(SpecType::Refactor),
            "explore" => Ok(SpecType::Explore),
            _ => Err(SpecTypeError { got: s.to_string() }),
        }
    }
}

impl SpecType {
    /// Canonical string form (matches YAML frontmatter values).
    pub fn as_str(&self) -> &'static str {
        match self {
            SpecType::Feat => "feat",
            SpecType::Fix => "fix",
            SpecType::Refactor => "refactor",
            SpecType::Explore => "explore",
        }
    }
}

// ============================================================================
// Parse / Serialize
// ============================================================================

/// Parse spec file into frontmatter and body
///
/// Expects file to start with `---` YAML frontmatter delimiter.
pub fn parse_spec_file(content: &str) -> Result<(SpecFrontmatter, String)> {
    // Extract frontmatter between --- markers
    let content = content
        .strip_prefix("---")
        .ok_or_else(|| anyhow::anyhow!("Spec file must start with '---' frontmatter delimiter"))?;

    // Handle both \n--- and \r\n--- line endings
    let end = content.find("\n---").ok_or_else(|| {
        anyhow::anyhow!("Spec file must have closing '---' frontmatter delimiter")
    })?;

    let frontmatter_str = &content[..end];
    let body = &content[end + 4..]; // Skip "\n---"

    let frontmatter: SpecFrontmatter = serde_yaml::from_str(frontmatter_str)
        .with_context(|| format!("Failed to parse frontmatter:\n{}", frontmatter_str))?;

    Ok((frontmatter, body.to_string()))
}

/// Serialize spec back to file content
///
/// Produces deterministic YAML output with consistent field ordering.
pub fn serialize_spec_file(frontmatter: &SpecFrontmatter, body: &str) -> Result<String> {
    let yaml = serde_yaml::to_string(frontmatter)?;
    Ok(format!("---\n{}---{}", yaml, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_roundtrip() {
        let content = r#"---
type: feat
id: test-spec
status: ready
target: v0.12.0
blocked_by:
  - other-spec
blocks: []
---

# Test Spec

Body content here.
"#;

        let (frontmatter, body) = parse_spec_file(content).expect("should parse");
        assert_eq!(frontmatter.id, "test-spec");
        assert_eq!(frontmatter.status, Some(SpecStatus::Ready));
        assert_eq!(frontmatter.target, Some("v0.12.0".to_string()));
        assert_eq!(frontmatter.blocked_by, vec!["other-spec"]);
        assert!(body.contains("# Test Spec"));

        let output = serialize_spec_file(&frontmatter, &body).expect("should serialize");
        let (fm2, _) = parse_spec_file(&output).expect("should re-parse");
        assert_eq!(fm2.id, frontmatter.id);
        assert_eq!(fm2.status, frontmatter.status);
    }

    #[test]
    fn test_spec_type_roundtrip() {
        for &name in SPEC_TYPES {
            let t: SpecType = name.parse().expect(name);
            assert_eq!(t.as_str(), name);
        }
    }

    #[test]
    fn test_spec_type_invalid() {
        let err = "unknown".parse::<SpecType>().unwrap_err();
        assert!(err.to_string().contains("unknown"));
        assert!(err.to_string().contains("feat"));
    }

    #[test]
    fn test_exit_criteria_roundtrip() {
        let content = r#"---
type: fix
id: test-exit
status: active
exit_criteria:
  - id: rollback-db
    text: "complete_spec_value rolls back DB status on failure"
    checked: false
  - id: simulated-failure
    text: "Simulated failure leaves DB status unchanged"
    checked: true
    verify: "patina spec complete <id> with dirty tree; check DB"
---

# Test exit criteria
"#;

        let (frontmatter, body) = parse_spec_file(content).expect("should parse");
        assert_eq!(frontmatter.exit_criteria.len(), 2);

        let c0 = &frontmatter.exit_criteria[0];
        assert_eq!(c0.id, "rollback-db");
        assert!(!c0.checked);
        assert!(c0.verify.is_none());

        let c1 = &frontmatter.exit_criteria[1];
        assert_eq!(c1.id, "simulated-failure");
        assert!(c1.checked);
        assert!(c1.verify.is_some());

        // Round-trip: serialize then re-parse
        let output = serialize_spec_file(&frontmatter, &body).expect("should serialize");
        let (fm2, _) = parse_spec_file(&output).expect("should re-parse");
        assert_eq!(fm2.exit_criteria, frontmatter.exit_criteria);
    }

    #[test]
    fn test_exit_criteria_string_shorthand() {
        let content = r#"---
type: fix
id: test-shorthand
status: draft
exit_criteria:
  - All tests pass
  - .git directory under 400 MB after cleanup
  - id: explicit-id
    text: "Explicit struct form works too"
    checked: true
---

# Test string shorthand
"#;

        let (frontmatter, _) = parse_spec_file(content).expect("should parse mixed formats");
        assert_eq!(frontmatter.exit_criteria.len(), 3);

        // String shorthand: auto-generated id, unchecked
        let c0 = &frontmatter.exit_criteria[0];
        assert_eq!(c0.id, "all-tests-pass");
        assert_eq!(c0.text, "All tests pass");
        assert!(!c0.checked);
        assert!(c0.verify.is_none());

        // String with special chars: slugified
        let c1 = &frontmatter.exit_criteria[1];
        assert_eq!(c1.id, "git-directory-under-400-mb-after-cleanup");
        assert_eq!(c1.text, ".git directory under 400 MB after cleanup");
        assert!(!c1.checked);

        // Full struct form still works
        let c2 = &frontmatter.exit_criteria[2];
        assert_eq!(c2.id, "explicit-id");
        assert!(c2.checked);
    }

    #[test]
    fn test_optional_fields() {
        let content = r#"---
type: explore
id: minimal
---

# Minimal spec
"#;

        let (frontmatter, _) = parse_spec_file(content).expect("should parse minimal");
        assert_eq!(frontmatter.id, "minimal");
        assert_eq!(frontmatter.status, None);
        assert!(frontmatter.blocked_by.is_empty());
    }

    #[test]
    fn test_spec_status_roundtrip() {
        for &name in SPEC_STATUSES {
            let s: SpecStatus = name.parse().expect(name);
            assert_eq!(s.as_str(), name);
            assert_eq!(s.to_string(), name);
        }
    }

    #[test]
    fn test_spec_status_done_alias() {
        let s: SpecStatus = "done".parse().expect("done should parse");
        assert_eq!(s, SpecStatus::Complete);
        assert_eq!(s.as_str(), "complete");
    }

    #[test]
    fn test_spec_status_invalid() {
        let err = "actve".parse::<SpecStatus>().unwrap_err();
        assert!(err.to_string().contains("actve"));
        assert!(err.to_string().contains("draft"));
    }

    #[test]
    fn test_spec_status_is_terminal() {
        assert!(SpecStatus::Complete.is_terminal());
        assert!(SpecStatus::Abandoned.is_terminal());
        assert!(!SpecStatus::Draft.is_terminal());
        assert!(!SpecStatus::Active.is_terminal());
        assert!(!SpecStatus::Paused.is_terminal());
    }

    #[test]
    fn test_spec_status_serde_roundtrip() {
        // Serialize to YAML and back
        let status = SpecStatus::Active;
        let yaml = serde_yaml::to_string(&status).expect("serialize");
        assert_eq!(yaml.trim(), "active");
        let back: SpecStatus = serde_yaml::from_str(&yaml).expect("deserialize");
        assert_eq!(back, status);
    }

    #[test]
    fn test_spec_status_serde_done_alias() {
        // "done" in YAML should deserialize to Complete
        let back: SpecStatus = serde_yaml::from_str("done").expect("deserialize done");
        assert_eq!(back, SpecStatus::Complete);
    }
}
