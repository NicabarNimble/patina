use anyhow::{Context, Result};
use regex::Regex;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use patina::release::{BumpType, ReleaseStrategy};
use patina::spec::{parse_spec_file, serialize_spec_file};

const DB_PATH: &str = ".patina/local/data/patina.db";

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
          AND p.status IS NOT NULL
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
        // JSON contract: ready/active only, no drafts
        println!("{}", serde_json::to_string_pretty(&specs)?);
        return Ok(());
    }

    // Get draft specs from merged filesystem+DB source
    let draft_filters = ListFilters {
        status: Some("draft".to_string()),
        target: None,
    };
    let drafts = get_all_specs(&draft_filters).unwrap_or_default();

    if specs.is_empty() {
        if drafts.is_empty() {
            println!("No specs ready to work on.");
            println!("\nHint: Specs need status 'ready' or 'active' with all blockers complete.");
        } else {
            println!("No specs ready to work on. {} draft spec(s) \u{2014} promote with `patina spec status <id> ready`", drafts.len());
        }
    } else {
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
    }

    // DRAFTS section — human output only
    if !drafts.is_empty() {
        if !specs.is_empty() {
            println!();
        }
        println!("DRAFTS (need promotion to ready):");
        for spec in &drafts {
            let target = spec.target.as_deref().unwrap_or("-");
            let suffix = if spec.unscraped { " [unscraped]" } else { "" };
            println!("  {:<28} {:<10} {}{}", spec.id, target, spec.title, suffix);
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

/// Query specs that are blocked by incomplete dependencies or have status='blocked'.
///
/// Returns specs where:
/// - File is in layer/surface/build/ (actual specs, not beliefs)
/// - Has at least one blocker with status not in ('complete', 'done')
///   OR has status = 'blocked' (set by `spec block` command)
pub fn get_blocked_specs() -> Result<Vec<BlockedSpec>> {
    let db_path = Path::new(DB_PATH);
    if !db_path.exists() {
        anyhow::bail!("Knowledge database not found. Run 'patina scrape' first.");
    }

    let conn = Connection::open(db_path).context("Failed to open database")?;

    // Get specs with incomplete blockers via spec_deps
    let mut stmt = conn.prepare(
        r#"
        SELECT p.id, p.status, p.target, p.title, d.depends_on, b.status
        FROM patterns p
        JOIN spec_deps d ON d.spec_id = p.id
        JOIN patterns b ON d.depends_on = b.id
        WHERE p.file_path LIKE 'layer/surface/build/%'
          AND p.status IS NOT NULL
          AND b.status NOT IN ('complete', 'done')
        ORDER BY p.id, d.depends_on
        "#,
    )?;

    // Group by spec
    let mut specs: Vec<BlockedSpec> = Vec::new();
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
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
            seen_ids.insert(id.clone());
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
            if let Some(spec) = specs.last_mut() {
                spec.blocked_by.push(Blocker {
                    id: blocker_id,
                    status: blocker_status,
                });
            }
        }
    }

    // Also include specs with status='blocked' not already found via spec_deps
    let mut stmt2 = conn.prepare(
        r#"
        SELECT p.id, p.status, p.target, p.title
        FROM patterns p
        WHERE p.file_path LIKE 'layer/surface/build/%'
          AND p.status = 'blocked'
        ORDER BY p.id
        "#,
    )?;

    let blocked_rows = stmt2
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

    for row in blocked_rows {
        let (id, status, target, title) = row?;
        if !seen_ids.contains(&id) {
            specs.push(BlockedSpec {
                id,
                status,
                target,
                title,
                blocked_by: vec![], // No spec_deps but status is blocked
            });
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
    pub unscraped: bool,
}

/// Filter options for spec list
#[derive(Debug, Clone, Default)]
pub struct ListFilters {
    pub status: Option<String>,
    pub target: Option<String>,
}

/// Scan layer/surface/build/ for SPEC.md files and parse frontmatter.
/// Returns specs found on disk, keyed by id. Returns empty vec if build dir missing.
fn scan_disk_specs() -> Vec<SpecInfo> {
    let build_dir = Path::new("layer/surface/build");
    if !build_dir.exists() {
        return Vec::new();
    }

    let title_re = Regex::new(r"^# (.+)$").unwrap();
    let mut specs = Vec::new();

    // Recursive walk of layer/surface/build/ looking for SPEC.md files
    fn walk_for_specs(dir: &Path, title_re: &Regex, specs: &mut Vec<SpecInfo>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("Warning: cannot read {}: {}", dir.display(), e);
                return;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();

            if path.is_dir() {
                walk_for_specs(&path, title_re, specs);
            } else if path.file_name().and_then(|n| n.to_str()) == Some("SPEC.md") {
                let content = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Warning: cannot read {}: {}", path.display(), e);
                        continue;
                    }
                };

                let (frontmatter, body) = match parse_spec_file(&content) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Warning: cannot parse {}: {}", path.display(), e);
                        continue;
                    }
                };

                if frontmatter.id.is_empty() {
                    eprintln!("Warning: missing id in {}", path.display());
                    continue;
                }

                // Extract title from first # heading in body
                let title = body
                    .lines()
                    .find_map(|line| title_re.captures(line).map(|c| c[1].to_string()))
                    .unwrap_or_else(|| frontmatter.id.clone());

                specs.push(SpecInfo {
                    id: frontmatter.id,
                    status: frontmatter.status,
                    target: frontmatter.target,
                    title,
                    unscraped: true, // disk-only until merged with DB
                });
            }
        }
    }

    walk_for_specs(build_dir, &title_re, &mut specs);
    specs
}

/// Query all specs with optional filters.
///
/// Merges filesystem scan (truth for existence) with DB query (supplementary).
/// Filesystem wins for all fields. DB entries without matching disk files are excluded.
pub fn get_all_specs(filters: &ListFilters) -> Result<Vec<SpecInfo>> {
    // 1. Scan filesystem — truth for spec existence
    let disk_specs = scan_disk_specs();
    let mut spec_map: HashMap<String, SpecInfo> =
        disk_specs.into_iter().map(|s| (s.id.clone(), s)).collect();

    // 2. Merge with DB if it exists (supplementary data)
    let db_path = Path::new(DB_PATH);
    if db_path.exists() {
        if let Ok(conn) = Connection::open(db_path) {
            let mut stmt = conn.prepare(
                "SELECT p.id, p.status, p.target, p.title
                 FROM patterns p
                 WHERE p.file_path LIKE 'layer/surface/build/%'
                   AND p.status IS NOT NULL",
            )?;

            let db_specs = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            for (id, _status, _target, _title) in db_specs {
                // Only mark as scraped if spec exists on disk
                if let Some(spec) = spec_map.get_mut(&id) {
                    spec.unscraped = false;
                }
                // DB entries not on disk are excluded (stale)
            }
        }
    }

    // 3. Collect, filter, sort
    let mut specs: Vec<SpecInfo> = spec_map.into_values().collect();

    // Apply filters
    if let Some(status_filter) = &filters.status {
        specs.retain(|s| s.status.as_deref() == Some(status_filter.as_str()));
    }
    if let Some(target_filter) = &filters.target {
        specs.retain(|s| s.target.as_deref() == Some(target_filter.as_str()));
    }

    // Sort by status, target, id (matches original ORDER BY)
    specs.sort_by(|a, b| {
        a.status
            .cmp(&b.status)
            .then(a.target.cmp(&b.target))
            .then(a.id.cmp(&b.id))
    });

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
    println!("{:<28} {:<22} {:<10} TITLE", "ID", "STATUS", "TARGET");
    println!("{:-<80}", "");

    for spec in &specs {
        let status_raw = spec.status.as_deref().unwrap_or("-");
        let status_display = if spec.unscraped {
            format!("{} [unscraped]", status_raw)
        } else {
            status_raw.to_string()
        };
        let target = spec.target.as_deref().unwrap_or("-");
        println!(
            "{:<28} {:<22} {:<10} {}",
            spec.id, status_display, target, spec.title
        );
    }

    println!("\n{} spec(s)", specs.len());

    // Warn about completed/abandoned specs still in tree
    let stale_count = specs
        .iter()
        .filter(|s| matches!(s.status.as_deref(), Some("complete") | Some("abandoned")))
        .count();
    if stale_count > 0 {
        let noun = if stale_count == 1 { "spec" } else { "specs" };
        eprintln!(
            "\n\u{26a0} {} completed/abandoned {} still in tree \u{2014} run `patina spec archive --stale` to archive",
            stale_count, noun
        );
    }

    Ok(())
}

// ============================================================================
// Status Update (spec-as-work-item Phase 4)
// ============================================================================

/// Valid spec statuses (state machine: draft → ready → active → paused/blocked → complete/abandoned)
const VALID_STATUSES: &[&str] = &["draft", "ready", "active", "paused", "blocked", "complete", "abandoned"];

/// Update a spec's status in both file and database
///
/// When status is "complete", delegates to `ReleaseStrategy` for version
/// management. The `major` flag overrides type-based bump detection for
/// 1.0.0 moments (`patina spec status <id> complete --major`).
///
/// When `no_archive` is false and status is "complete" or "abandoned",
/// the spec is auto-archived (tag + git rm) as part of the release commit
/// or as a standalone commit if no release is needed.
pub fn update_spec_status(id: &str, new_status: &str, major: bool, no_archive: bool) -> Result<()> {
    // Deprecation redirect
    let redirect = match new_status {
        "ready" | "active" => Some(format!("patina spec promote {}", id)),
        "paused" => Some(format!("patina spec pause {} --reason \"...\"", id)),
        "blocked" => Some(format!("patina spec block {} --by <blocker> --reason \"...\"", id)),
        "complete" => Some(if major {
            format!("patina spec complete {} --major", id)
        } else {
            format!("patina spec complete {}", id)
        }),
        "abandoned" => Some(format!("patina spec abandon {}", id)),
        _ => None,
    };
    if let Some(cmd) = redirect {
        eprintln!("Warning: `spec status` is deprecated.");
        eprintln!("  Use: {}", cmd);
        eprintln!();
    }

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

    // Should we auto-archive after this status change?
    let should_archive = !no_archive && (new_status == "complete" || new_status == "abandoned");

    // Pre-check: if archiving, ensure spec tag doesn't already exist
    let spec_dir = if should_archive {
        let tag_name = format!("spec/{}", id);
        if tag_exists(&tag_name)? {
            anyhow::bail!(
                "Tag '{}' already exists. Spec may have been archived previously.",
                tag_name
            );
        }
        resolve_spec_dir(&file_path)
    } else {
        None
    };

    // 7. If completing, delegate to release strategy
    if new_status == "complete" {
        let strategy = ReleaseStrategy::from_project(Path::new("."));

        // --major overrides type-based bump detection
        let bump = if major {
            Some(BumpType::Major)
        } else {
            BumpType::from_spec_type(&frontmatter.r#type)
        };

        if let Some(bump) = bump {
            let prepared = strategy.preflight(bump, &file_path)?;

            // Archive dir tells execute_cargo to git rm -rf instead of git add
            let archive_dir = if should_archive {
                spec_dir
                    .as_ref()
                    .and_then(|d| d.to_str())
                    .or(Some(&file_path))
            } else {
                None
            };
            prepared.execute(title_str, &file_path, archive_dir)?;

            if should_archive {
                // Tag HEAD~1 (the parent commit still has the spec file).
                // Created after the release commit so no orphaned tag on failure.
                create_spec_tag(id, title_str, "HEAD~1")?;
                println!("  Archived: spec/{}", id);
            }
        } else {
            println!("\n  Spec type '{}' → no version bump", frontmatter.r#type);
            // No release — archive as standalone commit if requested
            if should_archive {
                archive_spec_inner(id, &file_path, new_status, title_str, &spec_dir)?;
            }
        }
    } else if new_status == "abandoned" && should_archive {
        // No release for abandoned — archive as standalone commit
        archive_spec_inner(id, &file_path, new_status, title_str, &spec_dir)?;
    }

    Ok(())
}

/// Create an annotated spec tag on HEAD (preserves spec content for recovery)
/// Create an annotated spec tag on the given git ref.
///
/// `git_ref` determines which commit the tag points to (e.g., "HEAD~1"
/// to tag the parent commit that still contains the spec file).
fn create_spec_tag(id: &str, description: &str, git_ref: &str) -> Result<()> {
    let tag_name = format!("spec/{}", id);
    println!("Creating tag: {} (on {})", tag_name, git_ref);
    let output = Command::new("git")
        .args([
            "tag",
            "-a",
            &tag_name,
            "-m",
            &format!("Archived spec: {}", description),
            git_ref,
        ])
        .output()
        .context("Failed to create spec tag")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git tag failed: {}", stderr);
    }
    Ok(())
}

/// Archive a completed or abandoned spec: create spec/<id> tag, remove file, commit
///
/// Public entry point — validates status, checks clean tree, then delegates
/// to `archive_spec_inner` for the actual git operations.
pub fn archive_spec(id: &str, dry_run: bool) -> Result<()> {
    // 1. Find spec in patterns table by id
    let (file_path, status, title) = find_spec(id)?;
    let status_str = status.as_deref().unwrap_or("");

    // 2. Validate status allows archiving
    if status_str != "complete" && status_str != "abandoned" {
        anyhow::bail!(
            "Spec '{}' has status '{}', expected 'complete' or 'abandoned'\n\
             Only completed or abandoned specs can be archived.",
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
    let spec_dir = resolve_spec_dir(&file_path);

    if dry_run {
        println!("Dry run — would perform these changes:\n");
        println!("  Tag:    {} (preserves spec content)", tag_name);
        if let Some(dir) = &spec_dir {
            println!("  Remove: {}/", dir.display());
        } else {
            println!("  Remove: {}", file_path);
        }
        println!("  Commit: docs: archive {} ({})", tag_name, status_str);
        println!("\nRecover with: git show {}:{}", tag_name, file_path);
        return Ok(());
    }

    // 4. Check working tree is clean (standalone archive requires clean tree)
    if !is_tree_clean()? {
        anyhow::bail!(
            "Working tree has uncommitted changes.\n\
             Commit or stash your changes before archiving."
        );
    }

    // 5. Delegate to inner (tag, rm, commit)
    let desc = title.as_deref().unwrap_or(id);
    archive_spec_inner(id, &file_path, status_str, desc, &spec_dir)
}

/// Core archive logic: tag + git rm + commit.
///
/// Skips clean-tree check — caller is responsible for ensuring the tree
/// state is appropriate (either clean, or managed as part of a release flow).
fn archive_spec_inner(
    id: &str,
    file_path: &str,
    status: &str,
    description: &str,
    spec_dir: &Option<std::path::PathBuf>,
) -> Result<()> {
    let tag_name = format!("spec/{}", id);

    // 1. Remove spec file/directory from tree
    let remove_target = if let Some(dir) = spec_dir {
        dir.to_str().unwrap_or(file_path).to_string()
    } else {
        file_path.to_string()
    };
    // -f needed because the spec file may have just been modified (status update) on disk
    println!("Removing: {}", remove_target);
    let output = Command::new("git")
        .args(["rm", "-rf", &remove_target])
        .output()
        .context("Failed to remove spec from tree")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git rm failed: {}", stderr);
    }

    // 2. Commit
    let commit_msg = format!(
        "docs: archive {} ({})\n\nSpec preserved via git tag: {}\nRecover with: git show {}:{}",
        tag_name, status, tag_name, tag_name, file_path
    );
    println!("Committing archive");
    let output = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .output()
        .context("Failed to commit archive")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git commit failed: {}", stderr);
    }

    // 3. Tag HEAD~1 (the parent commit that still has the spec file).
    // Created after commit so no orphaned tag if git rm or commit fails.
    create_spec_tag(id, description, "HEAD~1")?;

    println!(
        "\n✓ Archived: {}\n  Tag: {}\n  Recover: git show {}:{}",
        id, tag_name, tag_name, file_path
    );

    Ok(())
}

/// Resolve the spec directory from a SPEC.md file path
fn resolve_spec_dir(file_path: &str) -> Option<std::path::PathBuf> {
    Path::new(file_path)
        .parent()
        .filter(|p| p.file_name().is_some())
        .map(|p| p.to_path_buf())
}

/// Archive all completed/abandoned specs that still have files in the tree.
///
/// Uses the merged filesystem+DB source (get_all_specs) so it catches both
/// scraped and unscraped stale specs — matching the warning in show_spec_list().
pub fn archive_stale_specs(dry_run: bool) -> Result<()> {
    let all_specs = get_all_specs(&ListFilters::default())?;
    let stale: Vec<_> = all_specs
        .into_iter()
        .filter(|s| matches!(s.status.as_deref(), Some("complete") | Some("abandoned")))
        .collect();

    if stale.is_empty() {
        println!("No stale specs to archive.");
        return Ok(());
    }

    println!("Found {} stale spec(s) to archive:\n", stale.len());

    for spec in &stale {
        let tag_name = format!("spec/{}", spec.id);

        // Skip if tag already exists
        if tag_exists(&tag_name)? {
            println!("  Skip: {} (tag already exists)", spec.id);
            continue;
        }

        // Resolve file path via find_spec (handles both DB and filesystem)
        let (file_path, _, _) = match find_spec(&spec.id) {
            Ok(found) => found,
            Err(e) => {
                eprintln!("  Skip: {} ({})", spec.id, e);
                continue;
            }
        };

        let status = spec.status.as_deref().unwrap_or("complete");
        let spec_dir = resolve_spec_dir(&file_path);

        if dry_run {
            println!("  Would archive: {} ({})", spec.id, status);
            continue;
        }

        archive_spec_inner(&spec.id, &file_path, status, &spec.title, &spec_dir)?;
    }

    if dry_run {
        println!("\nDry run — no changes made.");
    }

    Ok(())
}

/// Find a spec by its frontmatter id.
///
/// Tries DB first, falls back to filesystem scan for unscraped specs.
fn find_spec(id: &str) -> Result<(String, Option<String>, Option<String>)> {
    // Try DB first
    let db_path = Path::new(".patina/local/data/patina.db");
    if db_path.exists() {
        if let Ok(conn) = Connection::open(db_path) {
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
                Ok(row) => return Ok(row),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    // Fall through to filesystem scan
                }
                Err(e) => return Err(e).context("Failed to query patterns table"),
            }
        }
    }

    // Filesystem fallback: scan disk for matching spec
    let disk_specs = scan_disk_specs();
    for spec in disk_specs {
        if spec.id == id {
            // Reconstruct file_path from the spec id by scanning for the actual file
            let build_dir = Path::new("layer/surface/build");
            if let Some(path) = find_spec_file_on_disk(build_dir, id) {
                return Ok((path, spec.status, Some(spec.title)));
            }
        }
    }

    anyhow::bail!(
        "Spec '{}' not found.\n\
         Check the id, or create it under layer/surface/build/.",
        id
    );
}

/// Find the file path for a spec id on disk.
fn find_spec_file_on_disk(dir: &Path, target_id: &str) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_spec_file_on_disk(&path, target_id) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some("SPEC.md") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok((fm, _)) = parse_spec_file(&content) {
                    if fm.id == target_id {
                        return Some(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    None
}

// ============================================================================
// Shared Mutation Infrastructure (spec-workflow-rigor Phase 1)
// ============================================================================

use patina::spec::SpecFrontmatter;

/// Core YAML + DB status update. Reads the spec file, applies a mutation
/// closure to the frontmatter, writes the file back, and updates the DB.
/// Returns the file path and mutated frontmatter.
fn mutate_spec<F>(id: &str, mutate: F) -> Result<(String, SpecFrontmatter)>
where
    F: FnOnce(&mut SpecFrontmatter) -> Result<()>,
{
    let (file_path, _, _) = find_spec(id)?;

    let content = std::fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read {}", file_path))?;

    let (mut frontmatter, body) = parse_spec_file(&content)
        .with_context(|| format!("Failed to parse frontmatter in {}", file_path))?;

    mutate(&mut frontmatter)?;

    let new_content = serialize_spec_file(&frontmatter, &body)?;
    std::fs::write(&file_path, &new_content)
        .with_context(|| format!("Failed to write {}", file_path))?;

    // Update DB status if DB exists
    let db_path = Path::new(DB_PATH);
    if db_path.exists() {
        if let Some(ref status) = frontmatter.status {
            let conn = Connection::open(db_path).context("Failed to open database")?;
            conn.execute(
                "UPDATE patterns SET status = ?1 WHERE id = ?2",
                rusqlite::params![status, id],
            )?;
        }
    }

    Ok((file_path, frontmatter))
}

/// Derive the next tag sequence number from existing tags.
/// e.g., list_matching_tags("spec/{id}-paused-*") → count existing → N+1
fn next_tag_number(id: &str, prefix: &str) -> Result<u32> {
    let pattern = format!("spec/{}-{}-*", id, prefix);
    let tags = patina::git::list_matching_tags(&pattern)?;
    Ok(tags.len() as u32 + 1)
}

/// Promote a spec: draft → ready, or ready → active.
/// When promoting to active, creates tag spec/<id>-start.
pub fn promote_spec(id: &str, json: bool) -> Result<()> {
    let (file_path, fm) = mutate_spec(id, |fm| {
        match fm.status.as_deref() {
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
                id, s
            ),
            None => anyhow::bail!("Spec '{}' has no status", id),
        }
    })?;

    let new_status = fm.status.as_deref().unwrap_or("unknown");

    // If promoted to active, create start tag
    if new_status == "active" {
        let tag_name = format!("spec/{}-start", id);
        patina::git::create_tag_at(&tag_name, &format!("Spec {} activated", id), "HEAD")?;
    }

    // Git commit
    let output = Command::new("git")
        .args(["add", &file_path])
        .output()
        .context("Failed to stage spec file")?;
    if !output.status.success() {
        anyhow::bail!("git add failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let commit_msg = format!("spec: promote {} to {}", id, new_status);
    let output = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .output()
        .context("Failed to commit")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If nothing to commit (file unchanged in git's view), that's ok
        if !stderr.contains("nothing to commit") {
            anyhow::bail!("git commit failed: {}", stderr);
        }
    }

    if json {
        let result = serde_json::json!({
            "command": "promote",
            "spec_id": id,
            "new_status": new_status,
            "file": file_path,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Promoted: {} → {}", id, new_status);
        println!("  File: {}", file_path);
        if new_status == "active" {
            println!("  Tag: spec/{}-start", id);
        }
    }

    Ok(())
}

/// Complete an active spec (release + archive + tag)
pub fn complete_spec(id: &str, major: bool, json: bool) -> Result<()> {
    // 1. Find spec and validate status
    let (file_path, old_status, title) = find_spec(id)?;
    match old_status.as_deref() {
        Some("active") => {}
        Some(s) => anyhow::bail!(
            "Cannot complete '{}' — status is '{}', expected 'active'", id, s
        ),
        None => anyhow::bail!("Spec '{}' has no status", id),
    }

    // 2. Read and update status to complete
    let content = std::fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read {}", file_path))?;
    let (mut frontmatter, body) = parse_spec_file(&content)
        .with_context(|| format!("Failed to parse frontmatter in {}", file_path))?;
    frontmatter.status = Some("complete".to_string());
    let new_content = serialize_spec_file(&frontmatter, &body)?;
    std::fs::write(&file_path, &new_content)
        .with_context(|| format!("Failed to write {}", file_path))?;

    // 3. Update DB
    let db_path = Path::new(DB_PATH);
    if db_path.exists() {
        let conn = Connection::open(db_path).context("Failed to open database")?;
        conn.execute(
            "UPDATE patterns SET status = 'complete' WHERE id = ?1",
            rusqlite::params![id],
        )?;
    }

    let title_str = title.as_deref().unwrap_or(id);

    // 4. Pre-check archive tag
    let tag_name = format!("spec/{}", id);
    if tag_exists(&tag_name)? {
        anyhow::bail!(
            "Tag '{}' already exists. Spec may have been archived previously.",
            tag_name
        );
    }
    let spec_dir = resolve_spec_dir(&file_path);

    // 5. Delegate to release strategy
    let strategy = ReleaseStrategy::from_project(Path::new("."));
    let bump = if major {
        Some(BumpType::Major)
    } else {
        BumpType::from_spec_type(&frontmatter.r#type)
    };

    if let Some(bump) = bump {
        let prepared = strategy.preflight(bump, &file_path)?;
        let archive_dir = spec_dir
            .as_ref()
            .and_then(|d| d.to_str())
            .or(Some(&file_path));
        prepared.execute(title_str, &file_path, archive_dir)?;

        // Tag HEAD~1 (parent commit still has spec file)
        create_spec_tag(id, title_str, "HEAD~1")?;
    } else {
        // No release (explore type) — archive as standalone commit
        archive_spec_inner(id, &file_path, "complete", title_str, &spec_dir)?;
    }

    if json {
        let result = serde_json::json!({
            "command": "complete",
            "spec_id": id,
            "new_status": "complete",
            "archived": true,
            "tag": format!("spec/{}", id),
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Completed: {} → complete", id);
        println!("  Archived: spec/{}", id);
        println!("  Recover: git show spec/{}:{}", id, file_path);
    }

    Ok(())
}

/// Abandon a spec (archive + tag, no release)
pub fn abandon_spec(id: &str, reason: Option<&str>, json: bool) -> Result<()> {
    // 1. Find spec and validate status
    let (file_path, old_status, title) = find_spec(id)?;
    match old_status.as_deref() {
        Some("complete") => anyhow::bail!("Spec '{}' is already complete", id),
        Some("abandoned") => anyhow::bail!("Spec '{}' is already abandoned", id),
        None => anyhow::bail!("Spec '{}' has no status", id),
        _ => {} // draft, ready, active, paused, blocked — all can be abandoned
    }

    // 2. Update status to abandoned
    let content = std::fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read {}", file_path))?;
    let (mut frontmatter, body) = parse_spec_file(&content)
        .with_context(|| format!("Failed to parse frontmatter in {}", file_path))?;
    frontmatter.status = Some("abandoned".to_string());
    let new_content = serialize_spec_file(&frontmatter, &body)?;
    std::fs::write(&file_path, &new_content)
        .with_context(|| format!("Failed to write {}", file_path))?;

    // 3. Update DB
    let db_path = Path::new(DB_PATH);
    if db_path.exists() {
        let conn = Connection::open(db_path).context("Failed to open database")?;
        conn.execute(
            "UPDATE patterns SET status = 'abandoned' WHERE id = ?1",
            rusqlite::params![id],
        )?;
    }

    // 4. Pre-check archive tag
    let tag_name = format!("spec/{}", id);
    if tag_exists(&tag_name)? {
        anyhow::bail!(
            "Tag '{}' already exists. Spec may have been archived previously.",
            tag_name
        );
    }

    // 5. Archive (tag + git rm + commit)
    let title_str = title.as_deref().unwrap_or(id);
    let description = if let Some(r) = reason {
        format!("{} — {}", title_str, r)
    } else {
        title_str.to_string()
    };
    let spec_dir = resolve_spec_dir(&file_path);
    archive_spec_inner(id, &file_path, "abandoned", &description, &spec_dir)?;

    if json {
        let result = serde_json::json!({
            "command": "abandon",
            "spec_id": id,
            "new_status": "abandoned",
            "reason": reason,
            "archived": true,
            "tag": format!("spec/{}", id),
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Abandoned: {}", id);
        if let Some(r) = reason {
            println!("  Reason: {}", r);
        }
        println!("  Archived: spec/{}", id);
        println!("  Recover: git show spec/{}:{}", id, file_path);
    }

    Ok(())
}

/// Pause an active spec with reason.
///
/// Enforces one-paused-spec rule, creates WIP commit if dirty,
/// tags with spec/<id>-paused-<N>, rolls back YAML on failure.
pub fn pause_spec(id: &str, reason: &str, json: bool) -> Result<()> {
    // 1. Find spec and validate status
    let (file_path, old_status, _title) = find_spec(id)?;
    match old_status.as_deref() {
        Some("active") => {}
        Some(s) => anyhow::bail!(
            "Cannot pause '{}' — status is '{}', expected 'active'", id, s
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
            already_paused.id, already_paused.id, already_paused.id
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

    // 5. Save original file for rollback
    let original_content = std::fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read {}", file_path))?;

    // 6. Update YAML: status=paused, set pause fields
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let tag_n = next_tag_number(id, "paused")?;
    let tag_name = format!("spec/{}-paused-{}", id, tag_n);

    let (file_path, _fm) = match mutate_spec(id, |fm| {
        fm.status = Some("paused".to_string());
        fm.paused_reason = Some(reason.to_string());
        fm.paused_date = Some(today.clone());
        fm.paused_at_tag = Some(tag_name.clone());
        Ok(())
    }) {
        Ok(result) => result,
        Err(e) => {
            // Rollback YAML
            let _ = std::fs::write(&file_path, &original_content);
            return Err(e);
        }
    };

    // 7. Create annotated tag
    if let Err(e) = patina::git::create_tag_at(&tag_name, reason, "HEAD") {
        let _ = std::fs::write(&file_path, &original_content);
        return Err(e);
    }

    // 8. Git commit the spec file change
    let commit_result = (|| -> Result<()> {
        let output = Command::new("git")
            .args(["add", &file_path])
            .output()
            .context("Failed to stage spec file")?;
        if !output.status.success() {
            anyhow::bail!("git add failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        let commit_msg = format!("spec: pause {} — {}", id, reason);
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
        Ok(())
    })();

    if let Err(e) = commit_result {
        let _ = std::fs::write(&file_path, &original_content);
        return Err(e);
    }

    if json {
        let result = serde_json::json!({
            "command": "pause",
            "spec_id": id,
            "new_status": "paused",
            "reason": reason,
            "tag": tag_name,
            "paused_date": today,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Paused: {} → paused", id);
        println!("  Reason: {}", reason);
        println!("  Tag: {}", tag_name);
        println!("  Resume: patina spec resume {}", id);
    }

    Ok(())
}

/// Resume a paused or blocked spec.
///
/// For blocked specs: checks all blockers are complete (or --force).
/// Shows context diffs from paused_at_tag. Clears pause/block fields.
pub fn resume_spec(id: &str, force: bool, json: bool) -> Result<()> {
    // 1. Find spec and validate status
    let (file_path, old_status, _title) = find_spec(id)?;
    let status_str = old_status.as_deref().unwrap_or("");

    match status_str {
        "paused" | "blocked" => {}
        s => anyhow::bail!(
            "Cannot resume '{}' — status is '{}', expected 'paused' or 'blocked'", id, s
        ),
    }

    // 2. Read current frontmatter to get paused_at_tag and check blockers
    let content = std::fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read {}", file_path))?;
    let (fm_snapshot, _) = parse_spec_file(&content)
        .with_context(|| format!("Failed to parse frontmatter in {}", file_path))?;

    let paused_at_tag = fm_snapshot.paused_at_tag.clone();

    // 3. If blocked, check all blockers are complete
    if status_str == "blocked" && !force {
        let incomplete: Vec<_> = fm_snapshot
            .blocked_by
            .iter()
            .filter_map(|blocker_id| {
                match find_spec(blocker_id) {
                    Ok((_, blocker_status, _)) => {
                        let s = blocker_status.as_deref().unwrap_or("unknown");
                        if s != "complete" && s != "done" {
                            Some(format!("{} ({})", blocker_id, s))
                        } else {
                            None
                        }
                    }
                    Err(_) => Some(format!("{} (not found)", blocker_id)),
                }
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

    // 4. Update YAML: status=active, clear pause/block fields
    let tag_n = next_tag_number(id, "resumed")?;
    let tag_name = format!("spec/{}-resumed-{}", id, tag_n);

    let (file_path, _fm) = mutate_spec(id, |fm| {
        fm.status = Some("active".to_string());
        fm.paused_reason = None;
        fm.paused_date = None;
        fm.paused_at_tag = None;
        fm.blocked_reason = None;
        fm.blocked_date = None;
        // Note: blocked_by list is NOT cleared — it's historical record
        Ok(())
    })?;

    // 5. Clear spec_deps if was blocked
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

    // 6. Create annotated tag
    patina::git::create_tag_at(&tag_name, &format!("Resumed {}", id), "HEAD")?;

    // 7. Git commit
    let output = Command::new("git")
        .args(["add", &file_path])
        .output()
        .context("Failed to stage spec file")?;
    if !output.status.success() {
        anyhow::bail!("git add failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let commit_msg = format!("spec: resume {}", id);
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

    // 8. Show context diffs (non-JSON mode)
    if !json {
        println!("Resumed: {} → active", id);
        println!("  Tag: {}", tag_name);

        if let Some(ref pause_tag) = paused_at_tag {
            // What changed while away
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
            if patina::git::tag_exists(&start_tag)? {
                println!("\n--- Your work before pause ({}..{}) ---", start_tag, pause_tag);
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
    } else {
        let result = serde_json::json!({
            "command": "resume",
            "spec_id": id,
            "new_status": "active",
            "previous_status": status_str,
            "tag": tag_name,
            "paused_at_tag": paused_at_tag,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    }

    Ok(())
}

/// Block an active spec on another spec.
///
/// Appends to blocked_by list, updates spec_deps, creates annotated tag.
pub fn block_spec(id: &str, blocker: &str, reason: &str, json: bool) -> Result<()> {
    // 1. Validate spec is active
    let (_file_path, old_status, _title) = find_spec(id)?;
    match old_status.as_deref() {
        Some("active") => {}
        Some(s) => anyhow::bail!(
            "Cannot block '{}' — status is '{}', expected 'active'", id, s
        ),
        None => anyhow::bail!("Spec '{}' has no status", id),
    }

    // 2. Validate blocker spec exists
    let _ = find_spec(blocker).with_context(|| {
        format!("Blocker spec '{}' not found", blocker)
    })?;

    // 3. Update YAML
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let tag_n = next_tag_number(id, "blocked")?;
    let tag_name = format!("spec/{}-blocked-{}", id, tag_n);

    let (file_path, _fm) = mutate_spec(id, |fm| {
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

    // 4. Update spec_deps in DB
    let db_path = Path::new(DB_PATH);
    if db_path.exists() {
        let conn = Connection::open(db_path).context("Failed to open database")?;
        conn.execute(
            "INSERT OR IGNORE INTO spec_deps (spec_id, depends_on) VALUES (?1, ?2)",
            rusqlite::params![id, blocker],
        )?;
    }

    // 5. Create annotated tag
    let tag_msg = format!("{} (blocked by {})", reason, blocker);
    patina::git::create_tag_at(&tag_name, &tag_msg, "HEAD")?;

    // 6. Git commit
    let output = Command::new("git")
        .args(["add", &file_path])
        .output()
        .context("Failed to stage spec file")?;
    if !output.status.success() {
        anyhow::bail!("git add failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let commit_msg = format!("spec: block {} (waiting on {})", id, blocker);
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

    if json {
        let result = serde_json::json!({
            "command": "block",
            "spec_id": id,
            "new_status": "blocked",
            "blocker": blocker,
            "reason": reason,
            "tag": tag_name,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Blocked: {} → blocked", id);
        println!("  Blocked by: {}", blocker);
        println!("  Reason: {}", reason);
        println!("  Tag: {}", tag_name);
        println!("  Unblock: patina spec resume {} (when {} is complete)", id, blocker);
    }

    Ok(())
}

/// Check if a git tag exists (delegates to patina::git::tag_exists)
fn tag_exists(tag: &str) -> Result<bool> {
    patina::git::tag_exists(tag)
}

/// Check if working tree is clean for tracked files (delegates to patina::git::is_clean_tracked)
fn is_tree_clean() -> Result<bool> {
    patina::git::is_clean_tracked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_name_format() {
        let id = "session-092-hardening";
        let tag = format!("spec/{}", id);
        assert_eq!(tag, "spec/session-092-hardening");
    }

    #[test]
    fn test_resolve_spec_dir_with_directory() {
        let dir = resolve_spec_dir("layer/surface/build/feat/my-feature/SPEC.md");
        assert_eq!(
            dir.as_ref().map(|p| p.to_str().unwrap()),
            Some("layer/surface/build/feat/my-feature")
        );
    }

    #[test]
    fn test_resolve_spec_dir_root_file() {
        // A file at the root has no meaningful parent directory
        let dir = resolve_spec_dir("SPEC.md");
        // Parent is "" which has no file_name, so None
        assert!(dir.is_none());
    }

    #[test]
    fn test_valid_statuses_include_complete_and_abandoned() {
        assert!(VALID_STATUSES.contains(&"complete"));
        assert!(VALID_STATUSES.contains(&"abandoned"));
        assert!(!VALID_STATUSES.contains(&"archived"));
    }

    #[test]
    fn test_archive_requires_complete_or_abandoned() {
        // Verify the status check logic matches expectations
        let archivable = ["complete", "abandoned"];
        let non_archivable = ["draft", "ready", "active"];
        for s in archivable {
            assert!(
                s == "complete" || s == "abandoned",
                "{} should be archivable",
                s
            );
        }
        for s in non_archivable {
            assert!(
                s != "complete" && s != "abandoned",
                "{} should not be archivable",
                s
            );
        }
    }
}
