use anyhow::{Context, Result};
use regex::Regex;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

use patina::spec::{parse_spec_file, SpecFrontmatter};

use super::archive::load_spec;
use super::queue::{load_dep_counts, spec_age_days_from_list};
use super::DB_PATH;

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
pub(super) fn scan_disk_specs() -> Vec<SpecInfo> {
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

/// Full spec context for a single spec
#[derive(Debug, Clone, Serialize)]
pub struct ShowResult {
    pub id: String,
    pub frontmatter: SpecFrontmatter,
    pub body: String,
    pub design: Option<String>,
    pub files: Vec<String>,
}

/// Load full spec context: frontmatter + body + DESIGN.md + key files
pub fn show_spec_value(id: &str) -> Result<ShowResult> {
    let loaded = load_spec(id)?;

    // Check for DESIGN.md in the same directory as SPEC.md
    let design = Path::new(&loaded.file_path)
        .parent()
        .map(|dir| dir.join("DESIGN.md"))
        .filter(|p| p.exists())
        .and_then(|p| std::fs::read_to_string(p).ok());

    // Extract key files from ## Key Files section
    let files = extract_key_files(&loaded.body);

    Ok(ShowResult {
        id: loaded.frontmatter.id.clone(),
        frontmatter: loaded.frontmatter,
        body: loaded.body,
        design,
        files,
    })
}

/// Display full spec context (human-readable or JSON)
pub fn show_spec(id: &str, json: bool) -> Result<()> {
    let result = show_spec_value(id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Human-readable output
    let status = result.frontmatter.status.as_deref().unwrap_or("unknown");
    println!("{} [{}]", result.id, status);
    println!();
    println!("{}", result.body.trim());

    if let Some(design) = &result.design {
        println!("\n--- DESIGN.md ---\n");
        println!("{}", design.trim());
    }

    if !result.files.is_empty() {
        println!("\nKey files:");
        for f in &result.files {
            println!("  {}", f);
        }
    }

    Ok(())
}

/// Extract file paths from the ## Key Files section.
///
/// Looks for a fenced code block (```) under a `## Key Files` heading
/// and extracts non-empty lines, taking only the first whitespace-delimited
/// token from each line (to skip inline comments like "— description").
fn extract_key_files(body: &str) -> Vec<String> {
    let mut files = Vec::new();
    let mut in_key_files = false;
    let mut in_fence = false;

    for line in body.lines() {
        if line.starts_with("## Key Files") {
            in_key_files = true;
            continue;
        }
        if in_key_files && !in_fence && line.starts_with("## ") {
            // Hit next section
            break;
        }
        if in_key_files && line.trim_start().starts_with("```") {
            if in_fence {
                break; // Closing fence — done
            }
            in_fence = true;
            continue;
        }
        if in_key_files && in_fence {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                // Take first token (file path), skip trailing comments
                if let Some(path) = trimmed.split_whitespace().next() {
                    files.push(path.to_string());
                }
            }
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_key_files_basic() {
        let body = r#"
## Problem

Some problem description.

## Key Files

```
src/commands/spec/internal/queries.rs  — new show_spec_value()
src/mcp/server.rs                      — new spec.show tool handler
```

## Exit Criteria
"#;
        let files = extract_key_files(body);
        assert_eq!(
            files,
            vec!["src/commands/spec/internal/queries.rs", "src/mcp/server.rs",]
        );
    }

    #[test]
    fn test_extract_key_files_empty() {
        let body = "## Problem\n\nNo key files section here.\n";
        let files = extract_key_files(body);
        assert!(files.is_empty());
    }

    #[test]
    fn test_extract_key_files_no_fence() {
        let body = "## Key Files\n\nJust text, no code fence.\n\n## Exit Criteria\n";
        let files = extract_key_files(body);
        assert!(files.is_empty());
    }
}
