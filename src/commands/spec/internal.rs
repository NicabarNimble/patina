use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

use patina::git;
use patina::spec::{parse_spec_file, serialize_spec_file};

const DB_PATH: &str = ".patina/local/data/patina.db";

// ============================================================================
// Version Rules (spec type → version impact)
// ============================================================================

/// Determine version bump type from spec type
/// Returns: "patch", "minor", or "none"
fn version_bump_for_spec_type(spec_type: &str) -> &'static str {
    match spec_type {
        "fix" | "refactor" => "patch",
        "feat" => "minor",
        "explore" => "none",
        _ => "none", // unknown types don't bump
    }
}

/// Read current version from Cargo.toml
fn read_cargo_version() -> Result<String> {
    let content = fs::read_to_string("Cargo.toml")?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version") && trimmed.contains('=') {
            if let Some(version) = trimmed.split('=').nth(1) {
                let version = version.trim().trim_matches('"').trim_matches('\'');
                return Ok(version.to_string());
            }
        }
    }
    anyhow::bail!("Could not find version in Cargo.toml")
}

/// Compute next version based on bump type
fn next_version(current: &str, bump: &str) -> Result<String> {
    let parts: Vec<u32> = current
        .split('.')
        .map(|s| s.parse::<u32>().context("Invalid version component"))
        .collect::<Result<Vec<_>>>()?;

    if parts.len() != 3 {
        anyhow::bail!("Expected semver format (x.y.z), got '{}'", current);
    }

    Ok(match bump {
        "patch" => format!("{}.{}.{}", parts[0], parts[1], parts[2] + 1),
        "minor" => format!("{}.{}.0", parts[0], parts[1] + 1),
        "major" => format!("{}.0.0", parts[0] + 1),
        _ => current.to_string(),
    })
}

/// Update version in Cargo.toml
fn update_cargo_version(new_version: &str) -> Result<()> {
    let path = Path::new("Cargo.toml");
    let content = fs::read_to_string(path)?;

    let mut in_package_section = false;
    let mut version_updated = false;
    let mut new_content = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package_section = trimmed == "[package]";
        }
        if in_package_section && !version_updated && trimmed.starts_with("version") {
            new_content.push_str(&format!("version = \"{}\"\n", new_version));
            version_updated = true;
        } else {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    if !version_updated {
        anyhow::bail!("Could not find version field in [package] section");
    }

    fs::write(path, new_content)?;
    Ok(())
}

/// Perform release: update Cargo.toml, commit, tag
fn do_release(version: &str, spec_title: &str, spec_path: &str) -> Result<()> {
    // 1. Update Cargo.toml
    update_cargo_version(version)?;

    // 2. Stage and commit
    let commit_msg = format!("release: v{} — {}", version, spec_title);
    let output = Command::new("git")
        .args(["add", "Cargo.toml", spec_path])
        .output()
        .context("Failed to stage files")?;
    if !output.status.success() {
        anyhow::bail!("git add failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let output = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .output()
        .context("Failed to commit")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("nothing to commit") {
            anyhow::bail!("git commit failed: {}", stderr);
        }
    }

    // 3. Create tag
    let tag_name = format!("v{}", version);
    git::create_tag(&tag_name, spec_title)?;

    Ok(())
}

// ============================================================================
// Ready Queue (spec-as-work-item Phase 2)
// ============================================================================

/// A spec ready to work on (status=ready/active, all blockers complete)
#[derive(Debug, Clone, Serialize)]
pub struct ReadySpec {
    pub id: String,
    pub status: String,
    pub target: Option<String>,
    pub title: String,
}

/// Query specs ready to work on
///
/// Returns specs where:
/// - File is in layer/surface/build/ (actual specs, not beliefs)
/// - status IN ('ready', 'active')
/// - All blocked_by specs have status 'complete' or 'done'
pub fn get_ready_specs() -> Result<Vec<ReadySpec>> {
    let db_path = Path::new(DB_PATH);
    if !db_path.exists() {
        anyhow::bail!("Knowledge database not found. Run 'patina scrape' first.");
    }

    let conn = Connection::open(db_path).context("Failed to open database")?;

    let mut stmt = conn.prepare(
        r#"
        SELECT p.id, p.status, p.target, p.title
        FROM patterns p
        WHERE p.file_path LIKE 'layer/surface/build/%'
          AND p.status IN ('ready', 'active')
          AND NOT EXISTS (
            SELECT 1 FROM spec_deps d
            JOIN patterns blocker ON d.depends_on = blocker.id
            WHERE d.spec_id = p.id
              AND blocker.status NOT IN ('complete', 'done')
          )
        ORDER BY p.target, p.id
        "#,
    )?;

    let specs = stmt
        .query_map([], |row| {
            Ok(ReadySpec {
                id: row.get(0)?,
                status: row.get(1)?,
                target: row.get(2)?,
                title: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(specs)
}

/// Display ready specs (human-readable or JSON)
pub fn show_ready_specs(json: bool) -> Result<()> {
    let specs = get_ready_specs()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&specs)?);
        return Ok(());
    }

    if specs.is_empty() {
        println!("No specs ready to work on.");
        println!("\nHint: Specs need status 'ready' or 'active' with all blockers complete.");
        return Ok(());
    }

    // Group by status for display
    let ready: Vec<_> = specs.iter().filter(|s| s.status == "ready").collect();
    let active: Vec<_> = specs.iter().filter(|s| s.status == "active").collect();

    if !ready.is_empty() {
        println!("READY (can start now):");
        for spec in &ready {
            let target = spec.target.as_deref().unwrap_or("-");
            println!("  {:<28} {:<10} {}", spec.id, target, spec.title);
        }
    }

    if !active.is_empty() {
        if !ready.is_empty() {
            println!();
        }
        println!("ACTIVE (in progress):");
        for spec in &active {
            let target = spec.target.as_deref().unwrap_or("-");
            println!("  {:<28} {:<10} {}", spec.id, target, spec.title);
        }
    }

    Ok(())
}

// ============================================================================
// Blocked View (spec-as-work-item Phase 3)
// ============================================================================

/// A blocker preventing a spec from being ready
#[derive(Debug, Clone, Serialize)]
pub struct Blocker {
    pub id: String,
    pub status: String,
}

/// A spec that is blocked by incomplete dependencies
#[derive(Debug, Clone, Serialize)]
pub struct BlockedSpec {
    pub id: String,
    pub status: String,
    pub target: Option<String>,
    pub title: String,
    pub blocked_by: Vec<Blocker>,
}

/// Query specs that are blocked by incomplete dependencies
///
/// Returns specs where:
/// - File is in layer/surface/build/ (actual specs, not beliefs)
/// - Has at least one blocker with status not in ('complete', 'done')
pub fn get_blocked_specs() -> Result<Vec<BlockedSpec>> {
    let db_path = Path::new(DB_PATH);
    if !db_path.exists() {
        anyhow::bail!("Knowledge database not found. Run 'patina scrape' first.");
    }

    let conn = Connection::open(db_path).context("Failed to open database")?;

    // Get all specs with incomplete blockers
    let mut stmt = conn.prepare(
        r#"
        SELECT p.id, p.status, p.target, p.title, d.depends_on, b.status
        FROM patterns p
        JOIN spec_deps d ON d.spec_id = p.id
        JOIN patterns b ON d.depends_on = b.id
        WHERE p.file_path LIKE 'layer/surface/build/%'
          AND b.status NOT IN ('complete', 'done')
        ORDER BY p.id, d.depends_on
        "#,
    )?;

    // Group by spec
    let mut specs: Vec<BlockedSpec> = Vec::new();
    let mut current_id: Option<String> = None;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,         // spec id
            row.get::<_, String>(1)?,         // spec status
            row.get::<_, Option<String>>(2)?, // spec target
            row.get::<_, String>(3)?,         // spec title
            row.get::<_, String>(4)?,         // blocker id
            row.get::<_, String>(5)?,         // blocker status
        ))
    })?;

    for row in rows {
        let (id, status, target, title, blocker_id, blocker_status) = row?;

        if current_id.as_ref() != Some(&id) {
            // New spec
            specs.push(BlockedSpec {
                id: id.clone(),
                status,
                target,
                title,
                blocked_by: vec![Blocker {
                    id: blocker_id,
                    status: blocker_status,
                }],
            });
            current_id = Some(id);
        } else {
            // Add blocker to current spec
            if let Some(spec) = specs.last_mut() {
                spec.blocked_by.push(Blocker {
                    id: blocker_id,
                    status: blocker_status,
                });
            }
        }
    }

    Ok(specs)
}

/// Display blocked specs (human-readable or JSON)
pub fn show_blocked_specs(json: bool) -> Result<()> {
    let specs = get_blocked_specs()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&specs)?);
        return Ok(());
    }

    if specs.is_empty() {
        println!("No blocked specs.");
        return Ok(());
    }

    println!("BLOCKED:");
    for spec in &specs {
        let target = spec.target.as_deref().unwrap_or("-");
        print!("  {:<28} {:<10}", spec.id, target);

        // Print blockers
        for (i, blocker) in spec.blocked_by.iter().enumerate() {
            if i == 0 {
                println!(" blocked by: {} ({})", blocker.id, blocker.status);
            } else {
                println!(
                    "  {:<28} {:<10}             {} ({})",
                    "", "", blocker.id, blocker.status
                );
            }
        }
    }

    Ok(())
}

// ============================================================================
// Spec List (spec-as-work-item v0.13.0)
// ============================================================================

/// Spec info for list display
#[derive(Debug, Clone, Serialize)]
pub struct SpecInfo {
    pub id: String,
    pub status: Option<String>,
    pub target: Option<String>,
    pub title: String,
}

/// Filter options for spec list
#[derive(Debug, Clone, Default)]
pub struct ListFilters {
    pub status: Option<String>,
    pub target: Option<String>,
}

/// Query all specs with optional filters
pub fn get_all_specs(filters: &ListFilters) -> Result<Vec<SpecInfo>> {
    let db_path = Path::new(DB_PATH);
    if !db_path.exists() {
        anyhow::bail!("Knowledge database not found. Run 'patina scrape' first.");
    }

    let conn = Connection::open(db_path).context("Failed to open database")?;

    // Build query with optional filters
    let mut sql = String::from(
        "SELECT p.id, p.status, p.target, p.title
         FROM patterns p
         WHERE p.file_path LIKE 'layer/surface/build/%'",
    );

    let mut params: Vec<String> = Vec::new();

    if let Some(status) = &filters.status {
        sql.push_str(" AND p.status = ?");
        params.push(status.clone());
    }

    if let Some(target) = &filters.target {
        sql.push_str(" AND p.target = ?");
        params.push(target.clone());
    }

    sql.push_str(" ORDER BY p.status, p.target, p.id");

    let mut stmt = conn.prepare(&sql)?;

    // Convert params to references for rusqlite
    let param_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    let specs = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(SpecInfo {
                id: row.get(0)?,
                status: row.get::<_, Option<String>>(1)?,
                target: row.get(2)?,
                title: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(specs)
}

/// Display spec list (human-readable or JSON)
pub fn show_spec_list(filters: &ListFilters, json: bool) -> Result<()> {
    let specs = get_all_specs(filters)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&specs)?);
        return Ok(());
    }

    if specs.is_empty() {
        println!("No specs found.");
        if filters.status.is_some() || filters.target.is_some() {
            println!("  (with current filters)");
        }
        return Ok(());
    }

    // Header
    println!("{:<28} {:<10} {:<10} TITLE", "ID", "STATUS", "TARGET");
    println!("{:-<80}", "");

    for spec in &specs {
        let status = spec.status.as_deref().unwrap_or("-");
        let target = spec.target.as_deref().unwrap_or("-");
        println!(
            "{:<28} {:<10} {:<10} {}",
            spec.id, status, target, spec.title
        );
    }

    println!("\n{} spec(s)", specs.len());

    Ok(())
}

// ============================================================================
// Status Update (spec-as-work-item Phase 4)
// ============================================================================

/// Valid spec statuses (state machine: draft → ready → active → complete)
const VALID_STATUSES: &[&str] = &["draft", "ready", "active", "complete", "abandoned"];

/// Update a spec's status in both file and database
pub fn update_spec_status(id: &str, new_status: &str) -> Result<()> {
    // 1. Validate new status
    if !VALID_STATUSES.contains(&new_status) {
        anyhow::bail!(
            "Invalid status '{}'. Valid statuses: {}",
            new_status,
            VALID_STATUSES.join(", ")
        );
    }

    // 2. Find spec file
    let (file_path, old_status, title) = find_spec(id)?;
    let old_status_str = old_status.as_deref().unwrap_or("");

    if old_status_str == new_status {
        println!("Spec '{}' already has status '{}'", id, new_status);
        return Ok(());
    }

    // 3. Read, parse, update, serialize (serde-based, deterministic)
    let content = std::fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read {}", file_path))?;

    let (mut frontmatter, body) = parse_spec_file(&content)
        .with_context(|| format!("Failed to parse frontmatter in {}", file_path))?;

    // Update status
    frontmatter.status = Some(new_status.to_string());

    // 4. Write file back (deterministic YAML output)
    let new_content = serialize_spec_file(&frontmatter, &body)?;
    std::fs::write(&file_path, &new_content)
        .with_context(|| format!("Failed to write {}", file_path))?;

    // 5. Update database directly (faster than full scrape)
    let db_path = Path::new(DB_PATH);
    if db_path.exists() {
        let conn = Connection::open(db_path).context("Failed to open database")?;
        conn.execute(
            "UPDATE patterns SET status = ?1 WHERE id = ?2",
            rusqlite::params![new_status, id],
        )?;
    }

    // 6. Report success
    let title_str = title.as_deref().unwrap_or(id);
    println!("Updated: {} → {}", title_str, new_status);
    println!("  File: {}", file_path);

    // 7. If completing, trigger version bump based on spec type
    if new_status == "complete" {
        let spec_type = &frontmatter.r#type;
        let bump_type = version_bump_for_spec_type(spec_type);

        if bump_type != "none" {
            let current_version = read_cargo_version()?;
            let new_version = next_version(&current_version, bump_type)?;

            println!("\n  Spec type '{}' → {} bump", spec_type, bump_type);
            println!("  Version: {} → {}", current_version, new_version);

            do_release(&new_version, title_str, &file_path)?;

            println!("  Tagged: v{}", new_version);
        } else {
            println!("\n  Spec type '{}' → no version bump", spec_type);
        }
    }

    Ok(())
}

/// Archive a completed spec: create spec/<id> tag, remove file, update build.md, commit
pub fn archive_spec(id: &str, dry_run: bool) -> Result<()> {
    // 1. Find spec in patterns table by id
    let (file_path, status, title) = find_spec(id)?;
    let status_str = status.as_deref().unwrap_or("");

    // 2. Validate status is complete
    if status_str != "complete" {
        anyhow::bail!(
            "Spec '{}' has status '{}', expected 'complete'\n\
             Only completed specs can be archived.",
            id,
            status_str
        );
    }

    let tag_name = format!("spec/{}", id);

    // 3. Check tag doesn't already exist
    if tag_exists(&tag_name)? {
        anyhow::bail!(
            "Tag '{}' already exists. Spec may have been archived previously.\n\
             View with: git show {}:{}",
            tag_name,
            tag_name,
            file_path
        );
    }

    // Resolve spec directory (parent of SPEC.md)
    let spec_file = Path::new(&file_path);
    let spec_dir = spec_file
        .parent()
        .filter(|p| p.file_name().is_some())
        .map(|p| p.to_path_buf());

    if dry_run {
        println!("Dry run — would perform these changes:\n");
        println!("  Tag:    {} (preserves spec content)", tag_name);
        if let Some(dir) = &spec_dir {
            println!("  Remove: {}/", dir.display());
        } else {
            println!("  Remove: {}", file_path);
        }
        println!("  Update: layer/core/build.md (add to Archived section)");
        println!("  Commit: docs: archive {} (complete)", tag_name);
        println!("\nRecover with: git show {}:{}", tag_name, file_path);
        return Ok(());
    }

    // 4. Check working tree is clean (only for actual execution, not dry-run)
    if !is_tree_clean()? {
        anyhow::bail!(
            "Working tree has uncommitted changes.\n\
             Commit or stash your changes before archiving."
        );
    }

    // 5. Create annotated tag
    println!("Creating tag: {}", tag_name);
    let desc = title.as_deref().unwrap_or(id);
    let output = Command::new("git")
        .args([
            "tag",
            "-a",
            &tag_name,
            "-m",
            &format!("Archived spec: {}", desc),
        ])
        .output()
        .context("Failed to create git tag")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git tag failed: {}", stderr);
    }

    // 6. Remove spec file/directory from tree
    let remove_target = if let Some(dir) = &spec_dir {
        // Check if directory contains only SPEC.md (or SPEC.md + nothing else interesting)
        dir.to_str().unwrap_or(&file_path).to_string()
    } else {
        file_path.clone()
    };
    println!("Removing: {}", remove_target);
    let output = Command::new("git")
        .args(["rm", "-r", &remove_target])
        .output()
        .context("Failed to remove spec from tree")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git rm failed: {}", stderr);
    }

    // 7. Update build.md Archives section
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let archive_entry = format!("- `{}` - {} ({})", tag_name, desc, today);
    if let Err(e) = update_build_md(&archive_entry) {
        eprintln!("Warning: failed to update build.md: {}", e);
        eprintln!("  You may want to add this entry manually:");
        eprintln!("  {}", archive_entry);
    }

    // 8. Commit
    let commit_msg = format!(
        "docs: archive {} (complete)\n\nSpec preserved via git tag: {}\nRecover with: git show {}:{}",
        tag_name, tag_name, tag_name, file_path
    );
    println!("Committing archive");

    // Stage build.md too
    let _ = Command::new("git")
        .args(["add", "layer/core/build.md"])
        .output();

    let output = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .output()
        .context("Failed to commit archive")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git commit failed: {}", stderr);
    }

    println!(
        "\n✓ Archived: {}\n  Tag: {}\n  Recover: git show {}:{}",
        id, tag_name, tag_name, file_path
    );

    Ok(())
}

/// Find a spec by its frontmatter id in the patterns table
fn find_spec(id: &str) -> Result<(String, Option<String>, Option<String>)> {
    let db_path = Path::new(".patina/local/data/patina.db");
    if !db_path.exists() {
        anyhow::bail!("Knowledge database not found. Run 'patina scrape' first.");
    }

    let conn = Connection::open(db_path).context("Failed to open database")?;

    let result = conn.query_row(
        "SELECT file_path, status, title FROM patterns WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        },
    );

    match result {
        Ok(row) => Ok(row),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            anyhow::bail!(
                "Spec '{}' not found in patterns table.\n\
                 Run 'patina scrape' to index specs, or check the id.",
                id
            );
        }
        Err(e) => Err(e).context("Failed to query patterns table"),
    }
}

/// Check if a git tag exists
fn tag_exists(tag: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["tag", "-l", tag])
        .output()
        .context("Failed to list git tags")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(!stdout.trim().is_empty())
}

/// Check if working tree is clean (no uncommitted tracked changes)
fn is_tree_clean() -> Result<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain", "-uno"])
        .output()
        .context("Failed to check git status")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().is_empty())
}

/// Update build.md to add an entry to the "Archived (git tags)" section
fn update_build_md(entry: &str) -> Result<()> {
    let build_path = "layer/core/build.md";
    let content = std::fs::read_to_string(build_path)
        .with_context(|| format!("Failed to read {}", build_path))?;

    // Find the "Full list:" line that ends the archived section and insert before it
    let marker = "Full list: `git tag -l 'spec/*'`";
    if let Some(pos) = content.find(marker) {
        let new_content = format!("{}{}\n{}", &content[..pos], entry, &content[pos..]);
        std::fs::write(build_path, &new_content)
            .with_context(|| format!("Failed to write {}", build_path))?;

        // Also update the tag count
        update_tag_count(&new_content, build_path)?;

        Ok(())
    } else {
        anyhow::bail!("Could not find '{}' marker in {}", marker, build_path);
    }
}

/// Update the "(N archived specs)" count in build.md
fn update_tag_count(content: &str, path: &str) -> Result<()> {
    // Match pattern like "(46 archived specs)"
    if let Some(start) = content.find("archived specs)") {
        // Walk backwards to find the opening paren and number
        let prefix = &content[..start];
        if let Some(paren_pos) = prefix.rfind('(') {
            let num_str = prefix[paren_pos + 1..].trim();
            if let Ok(count) = num_str.parse::<u32>() {
                let old = format!("({} archived specs)", count);
                let new = format!("({} archived specs)", count + 1);
                let updated = content.replace(&old, &new);
                std::fs::write(path, updated)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_tag_name_format() {
        let id = "session-092-hardening";
        let tag = format!("spec/{}", id);
        assert_eq!(tag, "spec/session-092-hardening");
    }
}
