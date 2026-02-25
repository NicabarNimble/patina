use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::process::Command;

use serde::Serialize;

use patina::release::BumpType;
use patina::spec::{serialize_spec_file, SpecFrontmatter};

use super::archive::{
    archive_spec_inner, find_spec, load_spec, release_and_archive, resolve_spec_dir, LoadedSpec,
};
use super::queries::{get_all_specs, ListFilters};
use super::queue::tag_exists;
use super::DB_PATH;

/// Result of a spec mutation: pre/post frontmatter + file path.
pub(super) struct MutationOutput {
    pub file_path: String,
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
    let db_path = Path::new(DB_PATH);
    if db_path.exists() {
        if let Some(ref status) = post.status {
            let conn = Connection::open(db_path).context("Failed to open database")?;
            conn.execute(
                "UPDATE patterns SET status = ?1 WHERE id = ?2",
                rusqlite::params![status, pre.id],
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
    patina::git::add_paths(&[file_path])?;
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

/// Promote a spec: draft → ready, or ready → active.
/// When promoting to active, creates tag spec/<id>-start.
pub fn promote_spec(id: &str, json: bool) -> Result<()> {
    let result = promote_spec_value(id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let new_status = result["new_status"].as_str().unwrap_or("unknown");
        let file_path = result["file"].as_str().unwrap_or("");
        println!("Promoted: {} → {}", id, new_status);
        println!("  File: {}", file_path);
        if new_status == "active" {
            println!("  Tag: spec/{}-start", id);
        }
    }

    Ok(())
}

/// Promote a spec and return structured result (for MCP).
pub fn promote_spec_value(id: &str) -> Result<serde_json::Value> {
    let out = load_and_mutate(id, |fm| match fm.status.as_deref() {
        Some("draft") => {
            fm.status = Some("ready".to_string());
            Ok(())
        }
        Some("ready") => {
            fm.status = Some("active".to_string());
            Ok(())
        }
        Some(s) => anyhow::bail!(
            "Cannot promote '{}' — status is '{}'. Only draft and ready specs can be promoted.",
            id,
            s
        ),
        None => anyhow::bail!("Spec '{}' has no status", id),
    })?;

    let new_status = out.post.status.as_deref().unwrap_or("unknown");

    // If promoted to active, create start tag
    if new_status == "active" {
        let tag_name = format!("spec/{}-start", id);
        patina::git::create_tag_at(&tag_name, &format!("Spec {} activated", id), "HEAD")?;
    }

    // Git commit
    let commit_msg = format!("spec: promote {} to {}", id, new_status);
    git_stage_and_commit(&out.file_path, &commit_msg)?;

    Ok(serde_json::json!({
        "command": "promote",
        "spec_id": id,
        "new_status": new_status,
        "file": out.file_path,
    }))
}

/// Complete an active spec (release + archive + tag)
pub fn complete_spec(id: &str, major: bool, json: bool) -> Result<()> {
    let result = complete_spec_value(id, major)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let file_path = result["file"].as_str().unwrap_or("");
        println!("Completed: {} → complete", id);
        println!("  Archived: spec/{}", id);
        println!("  Recover: git show spec/{}:{}", id, file_path);
    }

    Ok(())
}

/// Complete an active spec and return structured result (for MCP).
pub fn complete_spec_value(id: &str, major: bool) -> Result<serde_json::Value> {
    // 1. Load spec and validate status
    let loaded = load_spec(id)?;
    match loaded.status.as_deref() {
        Some("active") => {}
        Some(s) => anyhow::bail!(
            "Cannot complete '{}' — status is '{}', expected 'active'",
            id,
            s
        ),
        None => anyhow::bail!("Spec '{}' has no status", id),
    }

    let title_str = loaded.title.as_deref().unwrap_or(id).to_string();

    // 2. Update status via mutate_spec (write + DB)
    let out = mutate_spec(loaded, |fm| {
        fm.status = Some("complete".to_string());
        Ok(())
    })?;

    // 3. Release + archive
    let bump = if major {
        Some(BumpType::Major)
    } else {
        BumpType::from_spec_type(&out.post.r#type)
    };
    release_and_archive(id, &out.file_path, &out.post, &title_str, bump)?;

    Ok(serde_json::json!({
        "command": "complete",
        "spec_id": id,
        "new_status": "complete",
        "archived": true,
        "tag": format!("spec/{}", id),
        "file": out.file_path,
    }))
}

/// Abandon a spec (archive + tag, no release)
pub fn abandon_spec(id: &str, reason: Option<&str>, json: bool) -> Result<()> {
    let result = abandon_spec_value(id, reason)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let file_path = result["file"].as_str().unwrap_or("");
        println!("Abandoned: {}", id);
        if let Some(r) = reason {
            println!("  Reason: {}", r);
        }
        println!("  Archived: spec/{}", id);
        println!("  Recover: git show spec/{}:{}", id, file_path);
    }

    Ok(())
}

/// Abandon a spec and return structured result (for MCP).
pub fn abandon_spec_value(id: &str, reason: Option<&str>) -> Result<serde_json::Value> {
    // 1. Load spec and validate status
    let loaded = load_spec(id)?;
    match loaded.status.as_deref() {
        Some("complete") => anyhow::bail!("Spec '{}' is already complete", id),
        Some("abandoned") => anyhow::bail!("Spec '{}' is already abandoned", id),
        None => anyhow::bail!("Spec '{}' has no status", id),
        _ => {} // draft, ready, active, paused, blocked — all can be abandoned
    }

    let title_str = loaded.title.as_deref().unwrap_or(id).to_string();

    // 2. Update status via mutate_spec (write + DB)
    let out = mutate_spec(loaded, |fm| {
        fm.status = Some("abandoned".to_string());
        Ok(())
    })?;

    // 3. Pre-check archive tag
    let tag_name = format!("spec/{}", id);
    if tag_exists(&tag_name)? {
        anyhow::bail!(
            "Tag '{}' already exists. Spec may have been archived previously.",
            tag_name
        );
    }

    // 4. Archive (tag + git rm + commit)
    let description = if let Some(r) = reason {
        format!("{} — {}", title_str, r)
    } else {
        title_str
    };
    let spec_dir = resolve_spec_dir(&out.file_path);
    archive_spec_inner(
        id,
        &out.file_path,
        "abandoned",
        &description,
        spec_dir.as_deref(),
    )?;

    Ok(serde_json::json!({
        "command": "abandon",
        "spec_id": id,
        "new_status": "abandoned",
        "reason": reason,
        "archived": true,
        "tag": format!("spec/{}", id),
        "file": out.file_path,
    }))
}

/// Pause an active spec with reason.
///
/// Enforces one-paused-spec rule, creates WIP commit if dirty,
/// tags with spec/<id>-paused-<N>, rolls back YAML on failure.
pub fn pause_spec(id: &str, reason: &str, json: bool) -> Result<()> {
    let result = pause_spec_value(id, reason)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let tag_name = result["tag"].as_str().unwrap_or("");
        println!("Paused: {} → paused", id);
        println!("  Reason: {}", reason);
        println!("  Tag: {}", tag_name);
        println!("  Resume: patina spec resume {}", id);
    }

    Ok(())
}

/// Pause an active spec and return structured result (for MCP).
pub fn pause_spec_value(id: &str, reason: &str) -> Result<serde_json::Value> {
    // 1. Load spec and validate status
    let loaded = load_spec(id)?;
    match loaded.status.as_deref() {
        Some("active") => {}
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
        .filter(|s| s.status.as_deref() == Some("paused") && s.id != id)
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
            fm.status = Some("paused".to_string());
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

    Ok(serde_json::json!({
        "command": "pause",
        "spec_id": id,
        "new_status": "paused",
        "reason": reason,
        "tag": tag_name,
        "paused_date": today,
    }))
}

/// Resume a paused or blocked spec.
///
/// For blocked specs: checks all blockers are complete (or --force).
/// Shows context diffs from paused_at_tag. Clears pause/block fields.
pub fn resume_spec(id: &str, force: bool, json: bool) -> Result<()> {
    let result = resume_spec_value(id, force)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let tag_name = result["tag"].as_str().unwrap_or("");
        let paused_at_tag = result["paused_at_tag"].as_str();
        println!("Resumed: {} → active", id);
        println!("  Tag: {}", tag_name);

        if let Some(pause_tag) = paused_at_tag {
            // What changed while away
            // `git diff --stat` for display — no patina::git helper for range diffs
            println!("\n--- Changes since pause ({}) ---", pause_tag);
            let output = Command::new("git")
                .args(["diff", "--stat", &format!("{}..HEAD", pause_tag)])
                .output();
            if let Ok(output) = output {
                let diff = String::from_utf8_lossy(&output.stdout);
                if diff.trim().is_empty() {
                    println!("  (no changes)");
                } else {
                    for line in diff.lines() {
                        println!("  {}", line);
                    }
                }
            }

            // What you accomplished before pausing
            let start_tag = format!("spec/{}-start", id);
            if patina::git::tag_exists(&start_tag).unwrap_or(false) {
                println!(
                    "\n--- Your work before pause ({}..{}) ---",
                    start_tag, pause_tag
                );
                let output = Command::new("git")
                    .args(["diff", "--stat", &format!("{}..{}", start_tag, pause_tag)])
                    .output();
                if let Ok(output) = output {
                    let diff = String::from_utf8_lossy(&output.stdout);
                    if diff.trim().is_empty() {
                        println!("  (no changes)");
                    } else {
                        for line in diff.lines() {
                            println!("  {}", line);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Resume a paused or blocked spec and return structured result (for MCP).
pub fn resume_spec_value(id: &str, force: bool) -> Result<serde_json::Value> {
    // 1. Load spec and validate status
    let loaded = load_spec(id)?;
    let status_str = loaded.status.as_deref().unwrap_or("").to_string();

    match status_str.as_str() {
        "paused" | "blocked" => {}
        s => anyhow::bail!(
            "Cannot resume '{}' — status is '{}', expected 'paused' or 'blocked'",
            id,
            s
        ),
    }

    // 2. Get paused_at_tag and check blockers from loaded frontmatter (no re-read)
    let paused_at_tag = loaded.frontmatter.paused_at_tag.clone();

    if status_str == "blocked" && !force {
        let incomplete: Vec<_> = loaded
            .frontmatter
            .blocked_by
            .iter()
            .filter_map(|blocker_id| match find_spec(blocker_id) {
                Ok(blocker) => {
                    let s = blocker.status.as_deref().unwrap_or("unknown");
                    if s != "complete" && s != "done" {
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
        fm.status = Some("active".to_string());
        fm.paused_reason = None;
        fm.paused_date = None;
        fm.paused_at_tag = None;
        fm.blocked_reason = None;
        fm.blocked_date = None;
        // Note: blocked_by list is NOT cleared — it's historical record
        Ok(())
    })?;

    // 4. Clear spec_deps if was blocked
    if status_str == "blocked" {
        let db_path = Path::new(DB_PATH);
        if db_path.exists() {
            let conn = Connection::open(db_path).context("Failed to open database")?;
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

    Ok(serde_json::json!({
        "command": "resume",
        "spec_id": id,
        "new_status": "active",
        "previous_status": status_str,
        "tag": tag_name,
        "paused_at_tag": paused_at_tag,
    }))
}

/// Block an active spec on another spec.
///
/// Appends to blocked_by list, updates spec_deps, creates annotated tag.
pub fn block_spec(id: &str, blocker: &str, reason: &str, json: bool) -> Result<()> {
    let result = block_spec_value(id, blocker, reason)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let tag_name = result["tag"].as_str().unwrap_or("");
        println!("Blocked: {} → blocked", id);
        println!("  Blocked by: {}", blocker);
        println!("  Reason: {}", reason);
        println!("  Tag: {}", tag_name);
        println!(
            "  Unblock: patina spec resume {} (when {} is complete)",
            id, blocker
        );
    }

    Ok(())
}

/// Block an active spec and return structured result (for MCP).
pub fn block_spec_value(id: &str, blocker: &str, reason: &str) -> Result<serde_json::Value> {
    // 1. Load spec and validate status
    let loaded = load_spec(id)?;
    match loaded.status.as_deref() {
        Some("active") => {}
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
            fm.status = Some("blocked".to_string());
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
        let db_path = Path::new(DB_PATH);
        if db_path.exists() {
            let conn = Connection::open(db_path).context("Failed to open database")?;
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

    Ok(serde_json::json!({
        "command": "block",
        "spec_id": id,
        "new_status": "blocked",
        "blocker": blocker,
        "reason": reason,
        "tag": tag_name,
    }))
}
