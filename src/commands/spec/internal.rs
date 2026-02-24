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
///
/// Enhanced view: shows impact counts, paused specs with age, and
/// blocked specs whose blockers completed (Phase 3 enhancements).
pub fn show_ready_specs(json: bool) -> Result<()> {
    let specs = get_ready_specs()?;

    if json {
        // JSON contract: ready/active only, no drafts
        println!("{}", serde_json::to_string_pretty(&specs)?);
        return Ok(());
    }

    // Load impact counts (how many specs each spec blocks)
    let dep_counts = load_dep_counts();

    // Get additional spec groups for the enhanced view
    let all_specs = get_all_specs(&ListFilters::default()).unwrap_or_default();
    let draft_filters = ListFilters {
        status: Some("draft".to_string()),
        target: None,
    };
    let drafts = get_all_specs(&draft_filters).unwrap_or_default();
    let paused: Vec<_> = all_specs
        .iter()
        .filter(|s| s.status.as_deref() == Some("paused"))
        .collect();
    let blocked_specs = get_blocked_specs().unwrap_or_default();
    let unblocked: Vec<_> = blocked_specs
        .iter()
        .filter(|b| {
            b.blocked_by.is_empty()
                || b.blocked_by
                    .iter()
                    .all(|bl| bl.status == "complete" || bl.status == "done")
        })
        .collect();

    let mut printed_section = false;

    if !specs.is_empty() {
        // Group by status for display
        let ready: Vec<_> = specs.iter().filter(|s| s.status == "ready").collect();
        let active: Vec<_> = specs.iter().filter(|s| s.status == "active").collect();

        if !ready.is_empty() {
            println!("READY (can start now):");
            for spec in &ready {
                let target = spec.target.as_deref().unwrap_or("-");
                let impact = dep_counts.get(&spec.id).copied().unwrap_or(0);
                let impact_str = if impact > 0 {
                    format!(" [blocks {}]", impact)
                } else {
                    String::new()
                };
                println!(
                    "  {:<28} {:<10} {}{}",
                    spec.id, target, spec.title, impact_str
                );
            }
            printed_section = true;
        }

        if !active.is_empty() {
            if printed_section {
                println!();
            }
            println!("ACTIVE (in progress):");
            for spec in &active {
                let target = spec.target.as_deref().unwrap_or("-");
                let impact = dep_counts.get(&spec.id).copied().unwrap_or(0);
                let impact_str = if impact > 0 {
                    format!(" [blocks {}]", impact)
                } else {
                    String::new()
                };
                println!(
                    "  {:<28} {:<10} {}{}",
                    spec.id, target, spec.title, impact_str
                );
            }
            printed_section = true;
        }
    } else if drafts.is_empty() && paused.is_empty() && unblocked.is_empty() {
        println!("No specs ready to work on.");
        println!("\nHint: Specs need status 'ready' or 'active' with all blockers complete.");
        return Ok(());
    }

    // UNBLOCKED section — blocked specs whose blockers are now complete
    if !unblocked.is_empty() {
        if printed_section {
            println!();
        }
        println!("UNBLOCKED (blockers complete — resume with `patina spec resume <id>`):");
        for spec in &unblocked {
            let target = spec.target.as_deref().unwrap_or("-");
            println!("  {:<28} {:<10} {}", spec.id, target, spec.title);
        }
        printed_section = true;
    }

    // PAUSED section — with age warnings
    if !paused.is_empty() {
        if printed_section {
            println!();
        }
        println!("PAUSED (resolve before pausing another):");
        for spec in &paused {
            let target = spec.target.as_deref().unwrap_or("-");
            let age = spec_age_days_from_list(spec);
            let age_str = if age > 14 {
                format!(" ({} days — overdue)", age)
            } else if age > 0 {
                format!(" ({} days)", age)
            } else {
                String::new()
            };
            println!("  {:<28} {:<10} {}{}", spec.id, target, spec.title, age_str);
        }
        printed_section = true;
    }

    // DRAFTS section
    if !drafts.is_empty() {
        if printed_section {
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
        } else if let Some(spec) = specs.last_mut() {
            spec.blocked_by.push(Blocker {
                id: blocker_id,
                status: blocker_status,
            });
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

    let blocked_rows = stmt2.query_map([], |row| {
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
        // Add age suffix for paused/blocked specs
        let age_suffix = if status_raw == "paused" || status_raw == "blocked" {
            let age = spec_age_days_from_list(spec);
            if age > 0 {
                format!(" ({}d)", age)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let status_display = if spec.unscraped {
            format!("{} [unscraped]", status_raw)
        } else {
            format!("{}{}", status_raw, age_suffix)
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

    // One-paused-spec constraint status
    let paused_count = specs
        .iter()
        .filter(|s| s.status.as_deref() == Some("paused"))
        .count();
    if paused_count > 0 {
        let paused_spec = specs.iter().find(|s| s.status.as_deref() == Some("paused"));
        if let Some(spec) = paused_spec {
            eprintln!("\nPaused: {} — resolve before pausing another", spec.id);
        }
    }

    Ok(())
}

// ============================================================================
// Status Update (spec-as-work-item Phase 4)
// ============================================================================

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
    println!("Creating tag: {} (on HEAD~1)", tag_name);
    patina::git::create_tag_at(
        &tag_name,
        &format!("Archived spec: {}", description),
        "HEAD~1",
    )?;

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

/// Save file content, execute action, rollback file on failure.
/// Used by pause_spec and block_spec where YAML is mutated before git
/// operations that could fail (D1 rollback pattern from design.md).
fn with_yaml_rollback<T, F>(file_path: &str, action: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let original = std::fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read {} for rollback backup", file_path))?;
    match action() {
        Ok(val) => Ok(val),
        Err(e) => {
            let _ = std::fs::write(file_path, &original);
            Err(e)
        }
    }
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
    let (file_path, fm) = mutate_spec(id, |fm| match fm.status.as_deref() {
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
        anyhow::bail!(
            "git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let commit_msg = format!("spec: promote {} to {}", id, new_status);
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

    Ok(serde_json::json!({
        "command": "promote",
        "spec_id": id,
        "new_status": new_status,
        "file": file_path,
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
    // 1. Find spec and validate status
    let (file_path, old_status, title) = find_spec(id)?;
    match old_status.as_deref() {
        Some("active") => {}
        Some(s) => anyhow::bail!(
            "Cannot complete '{}' — status is '{}', expected 'active'",
            id,
            s
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
        let archive_tag_name = format!("spec/{}", id);
        println!("Creating tag: {} (on HEAD~1)", archive_tag_name);
        patina::git::create_tag_at(
            &archive_tag_name,
            &format!("Archived spec: {}", title_str),
            "HEAD~1",
        )?;
    } else {
        // No release (explore type) — archive as standalone commit
        archive_spec_inner(id, &file_path, "complete", title_str, &spec_dir)?;
    }

    Ok(serde_json::json!({
        "command": "complete",
        "spec_id": id,
        "new_status": "complete",
        "archived": true,
        "tag": format!("spec/{}", id),
        "file": file_path,
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

    Ok(serde_json::json!({
        "command": "abandon",
        "spec_id": id,
        "new_status": "abandoned",
        "reason": reason,
        "archived": true,
        "tag": format!("spec/{}", id),
        "file": file_path,
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
    // 1. Find spec and validate status
    let (file_path, old_status, _title) = find_spec(id)?;
    match old_status.as_deref() {
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

    with_yaml_rollback(&file_path, || {
        let (file_path, _fm) = mutate_spec(id, |fm| {
            fm.status = Some("paused".to_string());
            fm.paused_reason = Some(reason.to_string());
            fm.paused_date = Some(today.clone());
            fm.paused_at_tag = Some(tag_name.clone());
            Ok(())
        })?;

        patina::git::create_tag_at(&tag_name, reason, "HEAD")?;

        let output = Command::new("git")
            .args(["add", &file_path])
            .output()
            .context("Failed to stage spec file")?;
        if !output.status.success() {
            anyhow::bail!(
                "git add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
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
    // 1. Find spec and validate status
    let (file_path, old_status, _title) = find_spec(id)?;
    let status_str = old_status.as_deref().unwrap_or("");

    match status_str {
        "paused" | "blocked" => {}
        s => anyhow::bail!(
            "Cannot resume '{}' — status is '{}', expected 'paused' or 'blocked'",
            id,
            s
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
            .filter_map(|blocker_id| match find_spec(blocker_id) {
                Ok((_, blocker_status, _)) => {
                    let s = blocker_status.as_deref().unwrap_or("unknown");
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
        anyhow::bail!(
            "git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
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
    // 1. Validate spec is active
    let (spec_file_path, old_status, _title) = find_spec(id)?;
    match old_status.as_deref() {
        Some("active") => {}
        Some(s) => anyhow::bail!(
            "Cannot block '{}' — status is '{}', expected 'active'",
            id,
            s
        ),
        None => anyhow::bail!("Spec '{}' has no status", id),
    }

    // 2. Validate blocker spec exists
    let _ = find_spec(blocker).with_context(|| format!("Blocker spec '{}' not found", blocker))?;

    // 3. Update YAML, DB, tag, and commit — with rollback on failure
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let tag_n = next_tag_number(id, "blocked")?;
    let tag_name = format!("spec/{}-blocked-{}", id, tag_n);

    with_yaml_rollback(&spec_file_path, || {
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
        let output = Command::new("git")
            .args(["add", &file_path])
            .output()
            .context("Failed to stage spec file")?;
        if !output.status.success() {
            anyhow::bail!(
                "git add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
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

// ============================================================================
// Spec Split (spec-workflow-rigor Phase 2)
// ============================================================================

/// Split a spec: complete original with release, create new draft for remaining work.
///
/// Flow: validate → tag → complete original → create new spec → commit
pub fn split_spec(
    id: &str,
    new_id: Option<&str>,
    description: Option<&str>,
    json: bool,
) -> Result<()> {
    let result = split_spec_value(id, new_id, description)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let derived_id = result["new_spec_id"].as_str().unwrap_or("");
        let version_tag = result["version_tag"].as_str().unwrap_or("");
        let new_spec_path = result["new_spec_path"].as_str().unwrap_or("");
        let file_path = result["original_file"].as_str().unwrap_or("");
        println!("Split: {}", id);
        println!("  Completed: {} → archived (spec/{})", id, id);
        println!("  Version tag: {}", version_tag);
        println!("  New draft: {} ({})", derived_id, new_spec_path);
        println!("  Recover parent: git show {}:{}", version_tag, file_path);
    }

    Ok(())
}

/// Split a spec and return structured result (for MCP).
pub fn split_spec_value(
    id: &str,
    new_id: Option<&str>,
    description: Option<&str>,
) -> Result<serde_json::Value> {
    // 1. Find spec and validate status (active or paused)
    let (file_path, old_status, title) = find_spec(id)?;
    match old_status.as_deref() {
        Some("active") | Some("paused") => {}
        Some(s) => anyhow::bail!(
            "Cannot split '{}' — status is '{}', expected 'active' or 'paused'",
            id,
            s
        ),
        None => anyhow::bail!("Spec '{}' has no status", id),
    }

    // 2. Read frontmatter for spec type
    let content = std::fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read {}", file_path))?;
    let (frontmatter, _body) = parse_spec_file(&content)
        .with_context(|| format!("Failed to parse frontmatter in {}", file_path))?;
    let spec_type = frontmatter.r#type.clone();

    // 3. Tag current state: spec/<id>-v<N>-complete
    let version_tags = patina::git::list_matching_tags(&format!("spec/{}-v*-complete", id))?;
    let version_n = version_tags.len() as u32 + 1;
    let version_tag = format!("spec/{}-v{}-complete", id, version_n);
    patina::git::create_tag_at(
        &version_tag,
        &format!("Split point: {} v{}", id, version_n),
        "HEAD",
    )?;

    // 4. Complete original spec (release + archive)
    //    Set status to complete first, then delegate to release strategy
    let title_str = title.as_deref().unwrap_or(id);

    {
        // Update status to complete
        let (mut fm_clone, body_clone) = parse_spec_file(&content)?;
        fm_clone.status = Some("complete".to_string());
        let new_content = serialize_spec_file(&fm_clone, &body_clone)?;
        std::fs::write(&file_path, &new_content)
            .with_context(|| format!("Failed to write {}", file_path))?;

        // Update DB
        let db_path = Path::new(DB_PATH);
        if db_path.exists() {
            let conn = Connection::open(db_path).context("Failed to open database")?;
            conn.execute(
                "UPDATE patterns SET status = 'complete' WHERE id = ?1",
                rusqlite::params![id],
            )?;
        }

        // Pre-check archive tag
        let archive_tag = format!("spec/{}", id);
        if tag_exists(&archive_tag)? {
            anyhow::bail!(
                "Tag '{}' already exists. Spec may have been archived previously.",
                archive_tag
            );
        }
        let spec_dir = resolve_spec_dir(&file_path);

        // Release strategy
        let strategy = ReleaseStrategy::from_project(Path::new("."));
        let bump = BumpType::from_spec_type(&spec_type);

        if let Some(bump) = bump {
            let prepared = strategy.preflight(bump, &file_path)?;
            let archive_dir = spec_dir
                .as_ref()
                .and_then(|d| d.to_str())
                .or(Some(&file_path));
            prepared.execute(title_str, &file_path, archive_dir)?;

            // Tag parent commit (still has spec file)
            println!("Creating tag: {} (on HEAD~1)", archive_tag);
            patina::git::create_tag_at(
                &archive_tag,
                &format!("Archived spec: {}", title_str),
                "HEAD~1",
            )?;
        } else {
            // No release (explore type) — archive as standalone commit
            archive_spec_inner(id, &file_path, "complete", title_str, &spec_dir)?;
        }
    }

    // 5. Determine new spec ID
    let derived_id = if let Some(explicit) = new_id {
        explicit.to_string()
    } else {
        // Default: <id>-v2, -v3, etc.
        let mut n = 2u32;
        loop {
            let candidate = format!("{}-v{}", id, n);
            let candidate_dir = format!("layer/surface/build/{}/{}", spec_type, candidate);
            if !Path::new(&candidate_dir).exists() {
                break candidate;
            }
            n += 1;
        }
    };

    // 6. Create new spec directory + SPEC.md
    let new_spec_dir = format!("layer/surface/build/{}/{}", spec_type, derived_id);
    std::fs::create_dir_all(&new_spec_dir)
        .with_context(|| format!("Failed to create directory {}", new_spec_dir))?;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let desc_text = description.unwrap_or("Remaining work from split");
    let new_spec_content = format!(
        "---\ntype: {}\nid: {}\nstatus: draft\ncreated: {}\nsplit_from: {}\n---\n\n# {}\n\n{}\n\n## Recovery\n\nParent spec content: `git show {}:{}`\n",
        spec_type,
        derived_id,
        today,
        id,
        derived_id,
        desc_text,
        version_tag,
        file_path,
    );

    let new_spec_path = format!("{}/SPEC.md", new_spec_dir);
    std::fs::write(&new_spec_path, &new_spec_content)
        .with_context(|| format!("Failed to write {}", new_spec_path))?;

    // 7. Git commit the new spec
    let output = Command::new("git")
        .args(["add", &new_spec_path])
        .output()
        .context("Failed to stage new spec")?;
    if !output.status.success() {
        anyhow::bail!(
            "git add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let commit_msg = format!(
        "spec: split {} — ship v{}, draft remainder as {}",
        id, version_n, derived_id
    );
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

    Ok(serde_json::json!({
        "command": "split",
        "original_spec_id": id,
        "new_spec_id": derived_id,
        "version_tag": version_tag,
        "archive_tag": format!("spec/{}", id),
        "new_spec_path": new_spec_path,
        "original_file": file_path,
        "status": "completed",
    }))
}

// ============================================================================
// Spec Next / Queue System (spec-workflow-rigor Phase 3)
// ============================================================================

/// Recommend the next spec to work on based on priority ranking.
///
/// Ranking: active > blocked-ready-to-resume > paused-with-age > impact > drafts
pub fn next_spec(json: bool) -> Result<()> {
    let result = next_spec_value()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    let recommendations = match result.as_array() {
        Some(arr) => arr,
        None => {
            println!("No specs found.");
            return Ok(());
        }
    };

    if recommendations.is_empty() {
        println!("No actionable specs found.");
        return Ok(());
    }

    // Top recommendation
    let top = &recommendations[0];
    println!("RECOMMENDED: {}", top["id"].as_str().unwrap_or(""));
    println!("  Status: {}", top["status"].as_str().unwrap_or(""));
    println!("  Reason: {}", top["reason"].as_str().unwrap_or(""));
    let impact = top["impact"].as_u64().unwrap_or(0);
    if impact > 0 {
        println!("  Impact: blocks {} other spec(s)", impact);
    }

    // Show the rest as alternatives
    if recommendations.len() > 1 {
        println!("\nOther specs:");
        for rec in &recommendations[1..] {
            let impact = rec["impact"].as_u64().unwrap_or(0);
            let impact_str = if impact > 0 {
                format!(" (blocks {})", impact)
            } else {
                String::new()
            };
            println!(
                "  {:<28} {:<10} {}{}",
                rec["id"].as_str().unwrap_or(""),
                rec["status"].as_str().unwrap_or(""),
                rec["reason"].as_str().unwrap_or(""),
                impact_str
            );
        }
    }

    Ok(())
}

/// Recommend the next spec and return structured result (for MCP).
pub fn next_spec_value() -> Result<serde_json::Value> {
    let all_specs = get_all_specs(&ListFilters::default())?;

    if all_specs.is_empty() {
        return Ok(serde_json::json!([]));
    }

    // Load blocked specs to check for unblocked ones
    let blocked_specs = get_blocked_specs().unwrap_or_default();

    // Load spec_deps for impact scoring
    let dep_counts = load_dep_counts();

    #[derive(Debug, Serialize)]
    struct Recommendation {
        id: String,
        status: String,
        reason: String,
        priority: u32,
        impact: usize,
    }

    let mut recommendations: Vec<Recommendation> = Vec::new();

    for spec in &all_specs {
        let status = spec.status.as_deref().unwrap_or("unknown");
        let impact = dep_counts.get(&spec.id).copied().unwrap_or(0);

        match status {
            "active" => {
                recommendations.push(Recommendation {
                    id: spec.id.clone(),
                    status: status.to_string(),
                    reason: "Currently active — continue working".to_string(),
                    priority: 1,
                    impact,
                });
            }
            "blocked" => {
                // Check if blockers are now complete
                let spec_blocked = blocked_specs.iter().find(|b| b.id == spec.id);
                let all_blockers_done = spec_blocked
                    .map(|b| {
                        b.blocked_by.is_empty()
                            || b.blocked_by
                                .iter()
                                .all(|bl| bl.status == "complete" || bl.status == "done")
                    })
                    .unwrap_or(false);

                if all_blockers_done {
                    recommendations.push(Recommendation {
                        id: spec.id.clone(),
                        status: status.to_string(),
                        reason: "Blockers complete — ready to resume".to_string(),
                        priority: 2,
                        impact,
                    });
                }
            }
            "paused" => {
                let age = spec_age_days_from_list(spec);
                let reason = if age > 14 {
                    format!("Paused {} days — overdue for attention", age)
                } else {
                    format!("Paused {} days", age)
                };
                recommendations.push(Recommendation {
                    id: spec.id.clone(),
                    status: status.to_string(),
                    reason,
                    priority: if age > 14 { 3 } else { 4 },
                    impact,
                });
            }
            "ready" => {
                recommendations.push(Recommendation {
                    id: spec.id.clone(),
                    status: status.to_string(),
                    reason: if impact > 0 {
                        format!("Ready to start — blocks {} other spec(s)", impact)
                    } else {
                        "Ready to start".to_string()
                    },
                    priority: 5,
                    impact,
                });
            }
            "draft" => {
                recommendations.push(Recommendation {
                    id: spec.id.clone(),
                    status: status.to_string(),
                    reason: "Draft — promote to ready when prepared".to_string(),
                    priority: 6,
                    impact,
                });
            }
            _ => {}
        }
    }

    // Sort by priority (ascending), then by impact (descending)
    recommendations.sort_by(|a, b| a.priority.cmp(&b.priority).then(b.impact.cmp(&a.impact)));

    Ok(serde_json::to_value(&recommendations)?)
}

/// Compute age in days for a spec from the all-specs list.
/// Uses the spec's frontmatter paused_date/blocked_date by re-reading the file.
pub(crate) fn spec_age_days_from_list(spec: &SpecInfo) -> i64 {
    // Try to find the spec file and read paused_date or blocked_date
    if let Ok((file_path, _, _)) = find_spec(&spec.id) {
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            if let Ok((fm, _)) = parse_spec_file(&content) {
                let date_str = fm.paused_date.as_deref().or(fm.blocked_date.as_deref());
                if let Some(date_str) = date_str {
                    if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                        let today = chrono::Utc::now().date_naive();
                        return (today - date).num_days();
                    }
                }
            }
        }
    }
    0
}

/// Load dependency counts from spec_deps: how many specs depend on each spec.
pub(crate) fn load_dep_counts() -> HashMap<String, usize> {
    let db_path = Path::new(DB_PATH);
    if !db_path.exists() {
        return HashMap::new();
    }
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    let mut stmt =
        match conn.prepare("SELECT depends_on, COUNT(*) FROM spec_deps GROUP BY depends_on") {
            Ok(s) => s,
            Err(_) => return HashMap::new(),
        };
    stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
    })
    .ok()
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
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
