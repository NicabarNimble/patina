use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::process::Command;

use serde::Serialize;

use patina::release::BumpType;
use patina::spec::{serialize_spec_file, SpecFrontmatter, SpecStatus};

use super::archive::{
    archive_spec_inner, find_spec, load_spec, release_and_archive, resolve_spec_dir, LoadedSpec,
};
use super::db_path;
use super::queries::{check_spec_value, extract_section_items, get_all_specs, ListFilters};
use super::queue::tag_exists;

const FRESHNESS_MAX_GLOBAL_COMMITS: u64 = 200;
const FRESHNESS_MAX_RELATED_COMMITS: u64 = 40;
const FRESHNESS_MAX_CONTRACT_COMMITS: u64 = 20;

const FRESHNESS_CONTRACT_PATHS: &[&str] = &[
    "wit/",
    "sdk/patina-sdk/",
    "src/child/internal/mod.rs",
    "mother/src/toys.rs",
    "resources/templates/child/",
    "sdk/template/child.toml",
];

struct CanonicalToyMapping {
    manifest: &'static str,
    runtime: &'static str,
    aliases: &'static [&'static str],
}

const CANONICAL_TOY_MAPPINGS: &[CanonicalToyMapping] = &[
    CanonicalToyMapping {
        manifest: "log",
        runtime: "host_log",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "state",
        runtime: "host_keyvalue",
        aliases: &["keyvalue"],
    },
    CanonicalToyMapping {
        manifest: "filesystem",
        runtime: "host_filesystem",
        aliases: &["layer", "layer-fs", "fs"],
    },
    CanonicalToyMapping {
        manifest: "sql",
        runtime: "host_sql",
        aliases: &["store"],
    },
    CanonicalToyMapping {
        manifest: "git",
        runtime: "git_toy",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "peer",
        runtime: "peer_toy",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "task",
        runtime: "task_intents",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "lake",
        runtime: "lake_names",
        aliases: &["store"],
    },
    CanonicalToyMapping {
        manifest: "checkpoint",
        runtime: "checkpoint_streams",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "measure",
        runtime: "host_measure",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "github",
        runtime: "toys.github",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "connector",
        runtime: "toys.connector",
        aliases: &["connect"],
    },
    CanonicalToyMapping {
        manifest: "query",
        runtime: "host_query",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "session",
        runtime: "toys.session",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "events",
        runtime: "subscribed_streams",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "messaging",
        runtime: "toys.messaging",
        aliases: &["emit"],
    },
    CanonicalToyMapping {
        manifest: "ingress",
        runtime: "ingress_sources",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "http",
        runtime: "host_http",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "belief",
        runtime: "toys.belief",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "graph",
        runtime: "toys.graph",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "fetch",
        runtime: "toys.fetch",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "schema",
        runtime: "schema_facts",
        aliases: &[],
    },
    CanonicalToyMapping {
        manifest: "types",
        runtime: "support_contract",
        aliases: &[],
    },
];

fn normalize_toy_name(name: &str) -> Option<&'static str> {
    for mapping in CANONICAL_TOY_MAPPINGS {
        if mapping.manifest == name {
            return Some(mapping.manifest);
        }
        if mapping.aliases.contains(&name) {
            return Some(mapping.manifest);
        }
    }
    None
}

fn canonical_toy_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = CANONICAL_TOY_MAPPINGS.iter().map(|m| m.manifest).collect();
    names.sort_unstable();
    names
}

fn canonical_toy_mapping_rows() -> Vec<String> {
    CANONICAL_TOY_MAPPINGS
        .iter()
        .map(|mapping| {
            if mapping.aliases.is_empty() {
                format!("{} -> {}", mapping.manifest, mapping.runtime)
            } else {
                format!(
                    "{} (aliases: {}) -> {}",
                    mapping.manifest,
                    mapping.aliases.join(", "),
                    mapping.runtime
                )
            }
        })
        .collect()
}

fn extract_inline_code_tokens(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut inside = false;
    let mut current = String::new();

    for ch in line.chars() {
        if ch == '`' {
            if inside && !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            inside = !inside;
            continue;
        }
        if inside {
            current.push(ch);
        }
    }

    tokens
}

fn validate_recipe_toy_names(spec_id: &str, body: &str) -> Result<()> {
    let has_recipe_surface = body.contains("## Objective Recipe Format")
        || body.contains("required_toys_by_role")
        || body.contains("Required toys by role");
    if !has_recipe_surface {
        return Ok(());
    }

    let mut in_toy_section = false;
    let mut found_toys = Vec::new();
    let mut unknown = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("## ") && !trimmed.contains("Required toys by role") {
            in_toy_section = false;
        }

        if trimmed == "Required toys by role:" || trimmed == "### Required toys by role" {
            in_toy_section = true;
            continue;
        }

        if !in_toy_section {
            continue;
        }

        for token in extract_inline_code_tokens(trimmed) {
            if token.contains(' ') || token.contains('=') {
                continue;
            }
            found_toys.push(token.clone());
            if normalize_toy_name(&token).is_none() {
                unknown.push(token);
            }
        }
    }

    if found_toys.is_empty() {
        anyhow::bail!(
            "Spec '{}' is not ready: objective recipe declares toy roles but no machine-parseable toy names were found. Use backtick toy tokens (for example `log`, `connector`, `lake`).",
            spec_id
        );
    }

    if !unknown.is_empty() {
        let mut uniq = unknown;
        uniq.sort();
        uniq.dedup();
        anyhow::bail!(
            "Spec '{}' is not ready: objective recipe uses unknown toy names: {}.\nKnown manifest toy names: {}\nCanonical mapping: {}",
            spec_id,
            uniq.join(", "),
            canonical_toy_names().join(", "),
            canonical_toy_mapping_rows().join("; ")
        );
    }

    Ok(())
}

fn git_commit_drift(validated_commit: &str) -> Result<u64> {
    let output = Command::new("git")
        .args([
            "rev-list",
            "--count",
            &format!("{}..HEAD", validated_commit),
        ])
        .output()
        .context("Failed to evaluate freshness commit drift")?;
    if !output.status.success() {
        anyhow::bail!(
            "failed to compute commit drift from '{}': {}",
            validated_commit,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let drift = text
        .parse::<u64>()
        .with_context(|| format!("unexpected git rev-list count output '{}'", text))?;
    Ok(drift)
}

fn git_commit_drift_for_paths(validated_commit: &str, paths: &[String]) -> Result<u64> {
    if paths.is_empty() {
        return Ok(0);
    }

    let mut args = vec!["rev-list".to_string(), "--count".to_string()];
    args.push(format!("{}..HEAD", validated_commit));
    args.push("--".to_string());
    args.extend(paths.iter().cloned());

    let output = Command::new("git")
        .args(args.iter().map(String::as_str))
        .output()
        .context("Failed to evaluate freshness path drift")?;
    if !output.status.success() {
        anyhow::bail!(
            "failed to compute path drift from '{}': {}",
            validated_commit,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let drift = text
        .parse::<u64>()
        .with_context(|| format!("unexpected git rev-list count output '{}'", text))?;
    Ok(drift)
}

fn looks_like_repo_path(entry: &str) -> bool {
    let v = entry.trim();
    if v.is_empty() || v.starts_with("[[") || v.starts_with("http://") || v.starts_with("https://")
    {
        return false;
    }
    v.contains('/') || v.ends_with(".rs") || v.ends_with(".md") || v.ends_with(".toml")
}

fn collect_related_paths(loaded: &LoadedSpec) -> Vec<String> {
    let mut paths = Vec::new();
    for entry in loaded
        .frontmatter
        .related
        .iter()
        .chain(loaded.frontmatter.freshness_scope.iter())
    {
        let trimmed = entry.trim().trim_matches('`').to_string();
        if looks_like_repo_path(&trimmed) {
            paths.push(trimmed);
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn enforce_freshness_preflight(loaded: &LoadedSpec) -> Result<()> {
    let fm = &loaded.frontmatter;
    let id = &fm.id;

    let commit = fm
        .validated_against_commit
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Cannot promote '{}' to active: missing frontmatter `validated_against_commit` for freshness gate.",
                id
            )
        })?;

    let freshness_raw = fm
        .last_freshness_check
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Cannot promote '{}' to active: missing frontmatter `last_freshness_check` for freshness gate.",
                id
            )
        })?;

    if fm.freshness_scope.is_empty() {
        anyhow::bail!(
            "Cannot promote '{}' to active: missing frontmatter `freshness_scope` entries for freshness gate.",
            id
        );
    }

    if chrono::DateTime::parse_from_rfc3339(freshness_raw).is_err()
        && chrono::NaiveDate::parse_from_str(freshness_raw, "%Y-%m-%d").is_err()
    {
        anyhow::bail!(
            "Cannot promote '{}' to active: invalid `last_freshness_check` '{}'. Expected RFC3339 or YYYY-MM-DD.",
            id,
            freshness_raw
        );
    }

    let global_drift = git_commit_drift(commit)?;
    if global_drift > FRESHNESS_MAX_GLOBAL_COMMITS {
        anyhow::bail!(
            "Cannot promote '{}' to active: global commit drift since validated commit is {} (limit {}). Re-run freshness pass and update frontmatter.",
            id,
            global_drift,
            FRESHNESS_MAX_GLOBAL_COMMITS
        );
    }

    let related_paths = collect_related_paths(loaded);
    if related_paths.is_empty() {
        anyhow::bail!(
            "Cannot promote '{}' to active: no path-like entries found in related/freshness_scope for commit-velocity freshness checks.",
            id
        );
    }

    let related_drift = git_commit_drift_for_paths(commit, &related_paths)?;
    if related_drift > FRESHNESS_MAX_RELATED_COMMITS {
        anyhow::bail!(
            "Cannot promote '{}' to active: related-path commit drift is {} (limit {}). Related paths: {}",
            id,
            related_drift,
            FRESHNESS_MAX_RELATED_COMMITS,
            related_paths.join(", ")
        );
    }

    let contract_paths: Vec<String> = FRESHNESS_CONTRACT_PATHS
        .iter()
        .map(|p| p.to_string())
        .collect();
    let contract_drift = git_commit_drift_for_paths(commit, &contract_paths)?;
    if contract_drift > FRESHNESS_MAX_CONTRACT_COMMITS {
        anyhow::bail!(
            "Cannot promote '{}' to active: contract-surface commit drift is {} (limit {}). Contract paths: {}",
            id,
            contract_drift,
            FRESHNESS_MAX_CONTRACT_COMMITS,
            contract_paths.join(", ")
        );
    }

    Ok(())
}

/// Result of a spec mutation: pre/post frontmatter + file path.
pub(super) struct MutationOutput {
    pub file_path: String,
    #[allow(dead_code)] // structurally correct; available for future assertions
    pub pre: SpecFrontmatter,
    pub post: SpecFrontmatter,
}

/// Typed result for spec mutation commands (replaces serde_json::Value).
#[derive(Debug, Serialize)]
pub struct MutationResult {
    pub command: &'static str,
    pub spec_id: String,
    pub new_status: String,
    #[serde(flatten)]
    pub detail: MutationDetail,
}

/// Command-specific fields for each mutation type.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum MutationDetail {
    Promote {
        file: String,
    },
    Complete {
        file: String,
        tag: String,
        archived: bool,
    },
    Abandon {
        file: String,
        tag: String,
        archived: bool,
        reason: Option<String>,
    },
    Pause {
        tag: String,
        reason: String,
        paused_date: String,
    },
    Resume {
        tag: String,
        previous_status: String,
        paused_at_tag: Option<String>,
    },
    Block {
        tag: String,
        blocker: String,
        reason: String,
    },
    Set {
        file: String,
        field: String,
        action: String,
        value: String,
    },
    Rename {
        old_id: String,
        new_id: String,
        file: String,
    },
    Reopen {
        file: String,
        previous_status: String,
    },
}

fn extract_section_paragraph(text: &str, heading: &str) -> Option<String> {
    let mut in_section = false;
    let mut lines = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == heading {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with("## ") {
            break;
        }
        if in_section && !trimmed.is_empty() && !trimmed.starts_with('-') {
            lines.push(trimmed.to_string());
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join(" "))
    }
}

fn section_is_captured(text: &str, heading: &str) -> bool {
    extract_section_paragraph(text, heading)
        .map(|value| {
            let lower = value.to_ascii_lowercase();
            !lower.contains("not captured") && !lower.contains("todo") && !lower.trim().is_empty()
        })
        .unwrap_or(false)
        || !extract_section_items(text, heading).is_empty()
}

fn has_blockquote(text: &str) -> bool {
    text.lines().any(|line| line.trim_start().starts_with("> "))
}

fn validate_slate_intent_readiness(loaded: &LoadedSpec) -> Result<()> {
    let spec_type = loaded.frontmatter.r#type.as_str();
    if !matches!(spec_type, "feat" | "fix" | "refactor") {
        return Ok(());
    }

    let mut missing = Vec::new();
    if !section_is_captured(&loaded.body, "## Human Request") && !has_blockquote(&loaded.body) {
        missing.push("## Human Request");
    }
    if !section_is_captured(&loaded.body, "## Allium Intent") {
        missing.push("## Allium Intent");
    }
    if !section_is_captured(&loaded.body, "## User Alignment") {
        missing.push("## User Alignment");
    }
    if !section_is_captured(&loaded.body, "## Proof Plan")
        && extract_section_items(&loaded.body, "## Verification").is_empty()
    {
        missing.push("## Proof Plan or ## Verification");
    }

    if !missing.is_empty() {
        anyhow::bail!(
            "Spec '{}' is not ready for Slate: missing intent/proof sections: {}. Capture HITL-aligned Allium intent, or state why no Allium change is needed for refactor work.",
            loaded.frontmatter.id,
            missing.join(", ")
        );
    }

    let allium_text =
        extract_section_paragraph(&loaded.body, "## Allium Intent").unwrap_or_default();
    let allium_lower = allium_text.to_ascii_lowercase();
    if allium_lower.contains("ambiguous")
        || allium_lower.contains("unclear")
        || allium_lower.contains("disputed")
    {
        anyhow::bail!(
            "Spec '{}' is not ready for Slate: Allium intent is marked ambiguous/disputed. Resolve HITL alignment before promotion.",
            loaded.frontmatter.id
        );
    }

    if spec_type == "refactor" {
        let says_no_behavior_change = allium_lower.contains("no behavior")
            || allium_lower.contains("no allium")
            || allium_lower.contains("unchanged behavior");
        let has_allium_anchor = loaded
            .frontmatter
            .related
            .iter()
            .chain(loaded.frontmatter.references.iter())
            .any(|value| value.contains("layer/allium") || value.ends_with(".allium"));
        if !says_no_behavior_change && !has_allium_anchor {
            anyhow::bail!(
                "Spec '{}' is not ready for Slate: refactor work must either anchor Allium intent or explicitly state that behavior remains unchanged.",
                loaded.frontmatter.id
            );
        }
    }

    Ok(())
}

fn lint_ready_spec(loaded: &LoadedSpec) -> Result<()> {
    let required_headings = [
        "## Problem",
        "## Goal",
        "## Status",
        "## Non-Goals",
        "## Solution",
        "## Implementation Order",
        "## Resolved Decisions",
        "## Verification",
        "## Build Readiness",
    ];

    let mut missing = Vec::new();
    for heading in required_headings {
        if !loaded.body.contains(heading) {
            missing.push(heading);
        }
    }

    if !missing.is_empty() {
        anyhow::bail!(
            "Spec '{}' is not ready: missing required sections: {}",
            loaded.frontmatter.id,
            missing.join(", ")
        );
    }

    let design_path = Path::new(&loaded.file_path)
        .parent()
        .map(|dir| dir.join("DESIGN.md"))
        .filter(|p| p.exists())
        .context("Spec is not ready: missing DESIGN.md")?;
    let design = std::fs::read_to_string(&design_path)
        .with_context(|| format!("Failed to read {}", design_path.display()))?;

    let required_design_headings = [
        "## Why This Design",
        "## Build Target",
        "## Resolved Decisions",
        "## Commits",
        "## Direct Code Targets",
        "## Verification Plan",
        "## Build Readiness",
    ];
    let mut missing_design = Vec::new();
    for heading in required_design_headings {
        if !design.contains(heading) {
            missing_design.push(heading);
        }
    }
    if !missing_design.is_empty() {
        anyhow::bail!(
            "Spec '{}' is not ready: DESIGN.md missing required sections: {}",
            loaded.frontmatter.id,
            missing_design.join(", ")
        );
    }

    let body_lower = loaded.body.to_ascii_lowercase();
    let design_lower = design.to_ascii_lowercase();
    let ambiguous = [
        "maybe",
        "consider ",
        "if needed",
        "one possible",
        "could be",
    ];
    let mut hits = Vec::new();
    for token in ambiguous {
        if body_lower.contains(token) || design_lower.contains(token) {
            hits.push(token.trim());
        }
    }
    if !hits.is_empty() {
        anyhow::bail!(
            "Spec '{}' is not ready: ambiguous architecture language present ({}) — revise or use --force",
            loaded.frontmatter.id,
            hits.join(", ")
        );
    }

    let has_code_targets = design.contains("`src/")
        || design.contains("`crates/")
        || design.contains("`sdk/")
        || design.contains("`children/")
        || design.contains("`wit/")
        || design.contains("`plugins/")
        || design.contains("`layer/");
    if !has_code_targets {
        anyhow::bail!(
            "Spec '{}' is not ready: DESIGN.md needs direct code targets",
            loaded.frontmatter.id
        );
    }

    validate_recipe_toy_names(&loaded.frontmatter.id, &loaded.body)?;
    validate_slate_intent_readiness(loaded)?;

    Ok(())
}

/// Core YAML + DB status update. Takes a fully loaded spec, applies a mutation
/// closure to the frontmatter, writes the file back, and updates the DB.
pub(super) fn mutate_spec<F>(loaded: LoadedSpec, mutate: F) -> Result<MutationOutput>
where
    F: FnOnce(&mut SpecFrontmatter) -> Result<()>,
{
    let pre = loaded.frontmatter.clone();
    let mut post = loaded.frontmatter;

    mutate(&mut post)?;

    let new_content = serialize_spec_file(&post, &loaded.body)?;
    std::fs::write(&loaded.file_path, &new_content)
        .with_context(|| format!("Failed to write {}", loaded.file_path))?;

    // Update DB status if DB exists
    let db_path = db_path()?;
    if db_path.exists() {
        if let Some(status) = post.status {
            let conn = Connection::open(&db_path).context("Failed to open database")?;
            conn.execute(
                "UPDATE patterns SET status = ?1 WHERE id = ?2",
                rusqlite::params![status.as_str(), pre.id],
            )?;
        }
    }

    Ok(MutationOutput {
        file_path: loaded.file_path,
        pre,
        post,
    })
}

/// Convenience: load + mutate in one call. For callers that don't need
/// to inspect the loaded spec before mutating (e.g., promote_spec_value).
pub(super) fn load_and_mutate<F>(id: &str, mutate: F) -> Result<MutationOutput>
where
    F: FnOnce(&mut SpecFrontmatter) -> Result<()>,
{
    let loaded = load_spec(id)?;
    mutate_spec(loaded, mutate)
}

/// Execute action with file rollback on failure. Takes explicit backup
/// content instead of re-reading the file.
///
/// Used by pause_spec and block_spec where YAML is mutated before git
/// operations that could fail (D1 rollback pattern from design.md).
pub(super) fn with_content_rollback<T, F>(file_path: &str, backup: &str, action: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    match action() {
        Ok(val) => Ok(val),
        Err(e) => {
            let _ = std::fs::write(file_path, backup);
            Err(e)
        }
    }
}

/// Stage a file and commit. Tolerates "nothing to commit" (idempotent).
///
/// Used by spec mutations that update YAML and need a git commit.
/// Unlike archive_spec_inner (which uses git rm and strict commit),
/// this handles the case where the file hasn't actually changed.
pub(super) fn git_stage_and_commit(file_path: &str, message: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["add", "-f", file_path])
        .output()
        .context(format!("Failed to run git add -f {}", file_path))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to stage {}: {}", file_path, stderr.trim());
    }

    if patina::git::has_staged_changes()? {
        patina::git::commit(message)?;
    }
    Ok(())
}

/// Derive the next tag sequence number from existing tags.
/// e.g., list_matching_tags("spec/{id}-paused-*") → count existing → N+1
pub(super) fn next_tag_number(id: &str, prefix: &str) -> Result<u32> {
    let pattern = format!("spec/{}-{}-*", id, prefix);
    let tags = patina::git::list_matching_tags(&pattern)?;
    Ok(tags.len() as u32 + 1)
}

/// Promote a spec and return structured result (for MCP).
pub fn promote_spec_value(id: &str, force: bool) -> Result<MutationResult> {
    let loaded = load_spec(id)?;
    if loaded.frontmatter.status == Some(SpecStatus::Draft) && !force {
        lint_ready_spec(&loaded)?;
    }
    if loaded.frontmatter.status == Some(SpecStatus::Ready) && !force {
        enforce_freshness_preflight(&loaded)?;
    }

    let out = mutate_spec(loaded, |fm| match fm.status {
        Some(SpecStatus::Draft) => {
            fm.status = Some(SpecStatus::Ready);
            Ok(())
        }
        Some(SpecStatus::Ready) => {
            fm.status = Some(SpecStatus::Active);
            Ok(())
        }
        Some(s) => anyhow::bail!(
            "Cannot promote '{}' — status is '{}'. Only draft and ready specs can be promoted.",
            id,
            s
        ),
        None => anyhow::bail!("Spec '{}' has no status", id),
    })?;

    let new_status = out.post.status.map(|s| s.as_str()).unwrap_or("unknown");

    // If promoted to active, create start tag
    if out.post.status == Some(SpecStatus::Active) {
        let tag_name = format!("spec/{}-start", id);
        patina::git::create_tag_at(&tag_name, &format!("Spec {} activated", id), "HEAD")?;
    }

    // Git commit
    let commit_msg = format!("spec: promote {} to {}", id, new_status);
    git_stage_and_commit(&out.file_path, &commit_msg)?;

    Ok(MutationResult {
        command: "promote",
        spec_id: id.to_string(),
        new_status: new_status.to_string(),
        detail: MutationDetail::Promote {
            file: out.file_path,
        },
    })
}

/// Apply add/remove to a Vec<String> field.
fn apply_list_mutation(list: &mut Vec<String>, action: &str, value: &str) {
    if action == "added" {
        if !list.iter().any(|v| v == value) {
            list.push(value.to_string());
        }
    } else {
        list.retain(|v| v != value);
    }
}

/// Set a metadata field and return structured result (for MCP).
pub fn set_spec_value(id: &str, field: &str, value: &str) -> Result<MutationResult> {
    const VEC_FIELDS: &[&str] = &[
        "beliefs",
        "related",
        "references",
        "blocked_by",
        "freshness_scope",
    ];
    const SCALAR_FIELDS: &[&str] = &[
        "target",
        "release_bump",
        "validated_against_commit",
        "last_freshness_check",
    ];

    let is_vec = VEC_FIELDS.contains(&field);
    let is_scalar = SCALAR_FIELDS.contains(&field);

    if !is_vec && !is_scalar {
        anyhow::bail!(
            "Cannot set field '{}'. Supported fields: beliefs, related, references, blocked_by, freshness_scope, target, release_bump, validated_against_commit, last_freshness_check",
            field
        );
    }

    let (action, clean_value): (&str, String) = if is_vec {
        if let Some(v) = value.strip_prefix('+') {
            ("added", v.to_string())
        } else if let Some(v) = value.strip_prefix('-') {
            ("removed", v.to_string())
        } else {
            anyhow::bail!(
                "List field '{}' requires +value (append) or -value (remove) prefix",
                field
            );
        }
    } else if value.is_empty() {
        ("cleared", String::new())
    } else {
        ("set", value.to_string())
    };

    let result_value = clean_value.clone();

    let out = load_and_mutate(id, |fm| {
        match field {
            "beliefs" => apply_list_mutation(&mut fm.beliefs, action, &clean_value),
            "related" => apply_list_mutation(&mut fm.related, action, &clean_value),
            "references" => apply_list_mutation(&mut fm.references, action, &clean_value),
            "blocked_by" => apply_list_mutation(&mut fm.blocked_by, action, &clean_value),
            "freshness_scope" => apply_list_mutation(&mut fm.freshness_scope, action, &clean_value),
            "target" => {
                fm.target = if clean_value.is_empty() {
                    None
                } else {
                    Some(clean_value)
                };
            }
            "release_bump" => {
                fm.release_bump = if clean_value.is_empty() {
                    None
                } else {
                    Some(clean_value)
                };
            }
            "validated_against_commit" => {
                fm.validated_against_commit = if clean_value.is_empty() {
                    None
                } else {
                    Some(clean_value)
                };
            }
            "last_freshness_check" => {
                fm.last_freshness_check = if clean_value.is_empty() {
                    None
                } else {
                    Some(clean_value)
                };
            }
            _ => unreachable!(),
        }
        Ok(())
    })?;

    let new_status = out
        .post
        .status
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let commit_msg = format!("spec: set {} on {}", field, id);
    git_stage_and_commit(&out.file_path, &commit_msg)?;

    Ok(MutationResult {
        command: "set",
        spec_id: id.to_string(),
        new_status,
        detail: MutationDetail::Set {
            file: out.file_path,
            field: field.to_string(),
            action: action.to_string(),
            value: result_value,
        },
    })
}

fn bump_from_release_bump(value: &str) -> Result<Option<BumpType>> {
    match value {
        "patch" => Ok(Some(BumpType::Patch)),
        "minor" => Ok(Some(BumpType::Minor)),
        "major" => Ok(Some(BumpType::Major)),
        "none" | "skip" => Ok(None),
        other => anyhow::bail!(
            "Invalid release_bump '{}'. Expected one of: patch, minor, major, none",
            other
        ),
    }
}

/// Complete an active spec and return structured result (for MCP).
pub fn complete_spec_value(
    id: &str,
    major: bool,
    patch: bool,
    force: bool,
) -> Result<MutationResult> {
    if major && patch {
        anyhow::bail!("Cannot use --major and --patch together");
    }
    // 1. Load spec and validate status
    let loaded = load_spec(id)?;
    match loaded.status {
        Some(SpecStatus::Active) => {}
        Some(s) => anyhow::bail!(
            "Cannot complete '{}' — status is '{}', expected 'active'",
            id,
            s
        ),
        None => anyhow::bail!("Spec '{}' has no status", id),
    }

    // 2. Check exit criteria gate (skipped with --force)
    let check = check_spec_value(id)?;
    if !check.passed && !force {
        let unchecked_list: Vec<String> = check
            .unchecked
            .iter()
            .map(|c| format!("  \u{2717} {} \u{2014} {}", c.id, c.text))
            .collect();
        let noun = if check.unchecked.len() == 1 {
            "criterion"
        } else {
            "criteria"
        };
        anyhow::bail!(
            "Cannot complete '{}' — {} unchecked exit {}:\n{}\n\n  \
             Use --force to bypass.",
            id,
            check.unchecked.len(),
            noun,
            unchecked_list.join("\n")
        );
    }

    let title_str = loaded.title.as_deref().unwrap_or(id).to_string();
    let pre_status = loaded.status.map(|s| s.to_string()).unwrap_or_default();
    let backup = loaded.content.clone();
    let file_path = loaded.file_path.clone();

    // Compute bump type before mutation (type doesn't change)
    let bump = if major {
        Some(BumpType::Major)
    } else if patch {
        Some(BumpType::Patch)
    } else if let Some(release_bump) = loaded.frontmatter.release_bump.as_deref() {
        bump_from_release_bump(release_bump)?
    } else {
        BumpType::from_spec_type(&loaded.frontmatter.r#type)
    };

    // 2. Mutate + release + archive — with rollback on failure
    let result = with_content_rollback(&file_path, &backup, || {
        let out = mutate_spec(loaded, |fm| {
            fm.status = Some(SpecStatus::Complete);
            Ok(())
        })?;

        release_and_archive(id, &out.file_path, &out.post, &title_str, bump)?;
        Ok(())
    });

    match result {
        Ok(()) => Ok(MutationResult {
            command: "complete",
            spec_id: id.to_string(),
            new_status: SpecStatus::Complete.to_string(),
            detail: MutationDetail::Complete {
                file: file_path,
                tag: format!("spec/{}", id),
                archived: true,
            },
        }),
        Err(e) => {
            // with_content_rollback already restored the YAML file.
            // Also restore DB status to pre-mutation value.
            let db_path = db_path()?;
            if db_path.exists() {
                if let Ok(conn) = Connection::open(&db_path) {
                    let _ = conn.execute(
                        "UPDATE patterns SET status = ?1 WHERE id = ?2",
                        rusqlite::params![pre_status, id],
                    );
                }
            }
            Err(e)
        }
    }
}

/// Abandon a spec and return structured result (for MCP).
pub fn abandon_spec_value(id: &str, reason: Option<&str>) -> Result<MutationResult> {
    // 1. Load spec and validate status
    let loaded = load_spec(id)?;
    match loaded.status {
        Some(SpecStatus::Complete) => anyhow::bail!("Spec '{}' is already complete", id),
        Some(SpecStatus::Abandoned) => anyhow::bail!("Spec '{}' is already abandoned", id),
        None => anyhow::bail!("Spec '{}' has no status", id),
        _ => {} // draft, ready, active, paused, blocked — all can be abandoned
    }

    let title_str = loaded.title.as_deref().unwrap_or(id).to_string();
    let pre_status = loaded.status.map(|s| s.to_string()).unwrap_or_default();
    let backup = loaded.content.clone();
    let file_path = loaded.file_path.clone();

    // 2. Mutate + archive — with rollback on failure
    let result = with_content_rollback(&file_path, &backup, || {
        let out = mutate_spec(loaded, |fm| {
            fm.status = Some(SpecStatus::Abandoned);
            Ok(())
        })?;

        // Pre-check archive tag
        let tag_name = format!("spec/{}", id);
        if tag_exists(&tag_name)? {
            anyhow::bail!(
                "Tag '{}' already exists. Spec may have been archived previously.",
                tag_name
            );
        }

        // Archive (tag + git rm + commit)
        let description = if let Some(r) = reason {
            format!("{} — {}", title_str, r)
        } else {
            title_str.clone()
        };
        let spec_dir = resolve_spec_dir(&out.file_path);
        archive_spec_inner(
            id,
            &out.file_path,
            "abandoned",
            &description,
            spec_dir.as_deref(),
        )?;

        Ok(())
    });

    match result {
        Ok(()) => Ok(MutationResult {
            command: "abandon",
            spec_id: id.to_string(),
            new_status: SpecStatus::Abandoned.to_string(),
            detail: MutationDetail::Abandon {
                file: file_path,
                tag: format!("spec/{}", id),
                archived: true,
                reason: reason.map(|s| s.to_string()),
            },
        }),
        Err(e) => {
            // with_content_rollback already restored the YAML file.
            // Also restore DB status to pre-mutation value.
            let db_path = db_path()?;
            if db_path.exists() {
                if let Ok(conn) = Connection::open(&db_path) {
                    let _ = conn.execute(
                        "UPDATE patterns SET status = ?1 WHERE id = ?2",
                        rusqlite::params![pre_status, id],
                    );
                }
            }
            Err(e)
        }
    }
}

/// Pause an active spec and return structured result (for MCP).
pub fn pause_spec_value(id: &str, reason: &str) -> Result<MutationResult> {
    // 1. Load spec and validate status
    let loaded = load_spec(id)?;
    match loaded.status {
        Some(SpecStatus::Active) => {}
        Some(s) => anyhow::bail!(
            "Cannot pause '{}' — status is '{}', expected 'active'",
            id,
            s
        ),
        None => anyhow::bail!("Spec '{}' has no status", id),
    }

    // 2. One-paused-spec rule: check no other spec is already paused
    let all_specs = get_all_specs(&ListFilters::default())?;
    let paused: Vec<_> = all_specs
        .iter()
        .filter(|s| s.status == Some(SpecStatus::Paused) && s.id != id)
        .collect();
    if let Some(already_paused) = paused.first() {
        anyhow::bail!(
            "{} is already paused.\n  Resume, split, or abandon it first:\n    \
             patina spec resume {}\n    \
             patina spec abandon {}",
            already_paused.id,
            already_paused.id,
            already_paused.id
        );
    }

    // 3. Check no merge conflicts
    if patina::git::has_merge_conflicts()? {
        anyhow::bail!(
            "Cannot pause — unresolved merge conflicts exist.\n  \
             Resolve conflicts before pausing."
        );
    }

    // 4. WIP commit if tracked files are dirty
    if !patina::git::is_clean_tracked()? {
        let wip_msg = format!("WIP: {} paused — {}", id, reason);
        patina::git::add_all()?;
        patina::git::commit(&wip_msg)?;
    }

    // 5. Update YAML, tag, and commit — with rollback on failure
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let tag_n = next_tag_number(id, "paused")?;
    let tag_name = format!("spec/{}-paused-{}", id, tag_n);
    let backup = loaded.content.clone();
    let file_path_for_rollback = loaded.file_path.clone();

    with_content_rollback(&file_path_for_rollback, &backup, || {
        let out = mutate_spec(loaded, |fm| {
            fm.status = Some(SpecStatus::Paused);
            fm.paused_reason = Some(reason.to_string());
            fm.paused_date = Some(today.clone());
            fm.paused_at_tag = Some(tag_name.clone());
            Ok(())
        })?;

        patina::git::create_tag_at(&tag_name, reason, "HEAD")?;

        let commit_msg = format!("spec: pause {} — {}", id, reason);
        git_stage_and_commit(&out.file_path, &commit_msg)?;

        Ok(())
    })?;

    Ok(MutationResult {
        command: "pause",
        spec_id: id.to_string(),
        new_status: SpecStatus::Paused.to_string(),
        detail: MutationDetail::Pause {
            tag: tag_name,
            reason: reason.to_string(),
            paused_date: today,
        },
    })
}

/// Resume a paused or blocked spec and return structured result (for MCP).
pub fn resume_spec_value(id: &str, force: bool) -> Result<MutationResult> {
    // 1. Load spec and validate status
    let loaded = load_spec(id)?;
    let loaded_status = loaded.status.unwrap_or(SpecStatus::Draft);

    match loaded_status {
        SpecStatus::Paused | SpecStatus::Blocked => {}
        s => anyhow::bail!(
            "Cannot resume '{}' — status is '{}', expected 'paused' or 'blocked'",
            id,
            s
        ),
    }

    // 2. Get paused_at_tag and check blockers from loaded frontmatter (no re-read)
    let paused_at_tag = loaded.frontmatter.paused_at_tag.clone();

    if loaded_status == SpecStatus::Blocked && !force {
        let incomplete: Vec<_> = loaded
            .frontmatter
            .blocked_by
            .iter()
            .filter_map(|blocker_id| match find_spec(blocker_id) {
                Ok(blocker) => {
                    if !blocker.status.is_some_and(|s| s.is_terminal()) {
                        let s = blocker.status.map_or("unknown", |s| s.as_str());
                        Some(format!("{} ({})", blocker_id, s))
                    } else {
                        None
                    }
                }
                Err(_) => Some(format!("{} (not found)", blocker_id)),
            })
            .collect();

        if !incomplete.is_empty() {
            anyhow::bail!(
                "Cannot resume '{}' — still blocked by:\n  {}\n\n  \
                 Use --force to override.",
                id,
                incomplete.join("\n  ")
            );
        }
    }

    // 3. Update YAML: status=active, clear pause/block fields
    let tag_n = next_tag_number(id, "resumed")?;
    let tag_name = format!("spec/{}-resumed-{}", id, tag_n);

    let out = mutate_spec(loaded, |fm| {
        fm.status = Some(SpecStatus::Active);
        fm.paused_reason = None;
        fm.paused_date = None;
        fm.paused_at_tag = None;
        fm.blocked_reason = None;
        fm.blocked_date = None;
        // Note: blocked_by list is NOT cleared — it's historical record
        Ok(())
    })?;

    // 4. Clear spec_deps if was blocked
    if loaded_status == SpecStatus::Blocked {
        let db_path = db_path()?;
        if db_path.exists() {
            let conn = Connection::open(&db_path).context("Failed to open database")?;
            conn.execute(
                "DELETE FROM spec_deps WHERE spec_id = ?1",
                rusqlite::params![id],
            )?;
        }
    }

    // 5. Create annotated tag
    patina::git::create_tag_at(&tag_name, &format!("Resumed {}", id), "HEAD")?;

    // 6. Git commit
    let commit_msg = format!("spec: resume {}", id);
    git_stage_and_commit(&out.file_path, &commit_msg)?;

    Ok(MutationResult {
        command: "resume",
        spec_id: id.to_string(),
        new_status: SpecStatus::Active.to_string(),
        detail: MutationDetail::Resume {
            tag: tag_name,
            previous_status: loaded_status.to_string(),
            paused_at_tag,
        },
    })
}

/// Block an active spec and return structured result (for MCP).
pub fn block_spec_value(id: &str, blocker: &str, reason: &str) -> Result<MutationResult> {
    // 1. Load spec and validate status
    let loaded = load_spec(id)?;
    match loaded.status {
        Some(SpecStatus::Active) => {}
        Some(s) => anyhow::bail!(
            "Cannot block '{}' — status is '{}', expected 'active'",
            id,
            s
        ),
        None => anyhow::bail!("Spec '{}' has no status", id),
    }

    // 2. Validate blocker spec exists (lightweight find_spec, no load)
    let _ = find_spec(blocker).with_context(|| format!("Blocker spec '{}' not found", blocker))?;

    // 3. Update YAML, DB, tag, and commit — with rollback on failure
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let tag_n = next_tag_number(id, "blocked")?;
    let tag_name = format!("spec/{}-blocked-{}", id, tag_n);
    let backup = loaded.content.clone();
    let file_path_for_rollback = loaded.file_path.clone();

    with_content_rollback(&file_path_for_rollback, &backup, || {
        let out = mutate_spec(loaded, |fm| {
            fm.status = Some(SpecStatus::Blocked);
            // Append blocker to blocked_by list (D3: don't overwrite)
            if !fm.blocked_by.contains(&blocker.to_string()) {
                fm.blocked_by.push(blocker.to_string());
            }
            fm.blocked_reason = Some(reason.to_string());
            fm.blocked_date = Some(today.clone());
            fm.paused_at_tag = Some(tag_name.clone());
            Ok(())
        })?;

        // Update spec_deps in DB
        let db_path = db_path()?;
        if db_path.exists() {
            let conn = Connection::open(&db_path).context("Failed to open database")?;
            conn.execute(
                "INSERT OR IGNORE INTO spec_deps (spec_id, depends_on) VALUES (?1, ?2)",
                rusqlite::params![id, blocker],
            )?;
        }

        // Create annotated tag
        let tag_msg = format!("{} (blocked by {})", reason, blocker);
        patina::git::create_tag_at(&tag_name, &tag_msg, "HEAD")?;

        // Git commit
        let commit_msg = format!("spec: block {} (waiting on {})", id, blocker);
        git_stage_and_commit(&out.file_path, &commit_msg)?;

        Ok(())
    })?;

    Ok(MutationResult {
        command: "block",
        spec_id: id.to_string(),
        new_status: SpecStatus::Blocked.to_string(),
        detail: MutationDetail::Block {
            tag: tag_name,
            blocker: blocker.to_string(),
            reason: reason.to_string(),
        },
    })
}

pub fn rename_spec_value(id: &str, new_id: &str) -> Result<MutationResult> {
    if new_id.is_empty()
        || !new_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        anyhow::bail!("invalid new spec id '{}': use kebab-case", new_id);
    }
    if find_spec(new_id).is_ok() {
        anyhow::bail!("spec '{}' already exists", new_id);
    }

    let loaded = load_spec(id)?;
    let old_file = loaded.file_path.clone();
    let old_dir = Path::new(&old_file)
        .parent()
        .context("spec has no parent directory")?
        .to_path_buf();
    let parent_dir = old_dir
        .parent()
        .context("spec directory has no parent")?
        .to_path_buf();
    let new_dir = parent_dir.join(new_id);
    if new_dir.exists() {
        anyhow::bail!("target directory already exists: {}", new_dir.display());
    }

    let mut frontmatter = loaded.frontmatter.clone();
    frontmatter.id = new_id.to_string();
    let content = serialize_spec_file(&frontmatter, &loaded.body)?;
    std::fs::write(&old_file, content)?;

    std::fs::rename(&old_dir, &new_dir).with_context(|| {
        format!(
            "failed to rename {} -> {}",
            old_dir.display(),
            new_dir.display()
        )
    })?;

    let new_file = new_dir.join("SPEC.md");
    let new_file_str = new_file.to_string_lossy().to_string();

    let db_path = db_path()?;
    if db_path.exists() {
        let conn = Connection::open(&db_path).context("Failed to open database")?;
        conn.execute(
            "UPDATE patterns SET id = ?1, file_path = ?2 WHERE id = ?3",
            rusqlite::params![new_id, new_file_str, id],
        )?;
    }

    let stage_output = Command::new("git")
        .args([
            "add",
            "-A",
            old_dir.to_string_lossy().as_ref(),
            new_dir.to_string_lossy().as_ref(),
        ])
        .output()
        .context("Failed to stage rename")?;
    if !stage_output.status.success() {
        anyhow::bail!(
            "failed to stage rename: {}",
            String::from_utf8_lossy(&stage_output.stderr)
        );
    }

    let commit_msg = format!("spec: rename {} to {}", id, new_id);
    if patina::git::has_staged_changes()? {
        patina::git::commit(&commit_msg)?;
    }

    Ok(MutationResult {
        command: "rename",
        spec_id: new_id.to_string(),
        new_status: frontmatter
            .status
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        detail: MutationDetail::Rename {
            old_id: id.to_string(),
            new_id: new_id.to_string(),
            file: new_file_str,
        },
    })
}

pub fn reopen_spec_value(id: &str) -> Result<MutationResult> {
    let loaded = load_spec(id)?;
    let status = loaded
        .status
        .ok_or_else(|| anyhow::anyhow!("Spec '{}' has no status", id))?;
    if !status.is_terminal() {
        anyhow::bail!(
            "Cannot reopen '{}' — status is '{}', expected complete or abandoned",
            id,
            status
        );
    }

    let out = mutate_spec(loaded, |fm| {
        fm.status = Some(SpecStatus::Active);
        Ok(())
    })?;
    git_stage_and_commit(&out.file_path, &format!("spec: reopen {}", id))?;

    Ok(MutationResult {
        command: "reopen",
        spec_id: id.to_string(),
        new_status: SpecStatus::Active.to_string(),
        detail: MutationDetail::Reopen {
            file: out.file_path,
            previous_status: status.to_string(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_bump_frontmatter_values_map_to_bump_types() {
        assert_eq!(
            bump_from_release_bump("patch").unwrap(),
            Some(BumpType::Patch)
        );
        assert_eq!(
            bump_from_release_bump("minor").unwrap(),
            Some(BumpType::Minor)
        );
        assert_eq!(
            bump_from_release_bump("major").unwrap(),
            Some(BumpType::Major)
        );
        assert_eq!(bump_from_release_bump("none").unwrap(), None);
        assert_eq!(bump_from_release_bump("skip").unwrap(), None);
        assert!(bump_from_release_bump("feature").is_err());
    }
}
