use anyhow::Result;
use regex::Regex;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

use patina::spec::{parse_spec_file, SpecFrontmatter};

use super::archive::load_spec;
use super::queue::{load_dep_counts, spec_age_days_from_list};
use super::DB_PATH;

/// An unchecked exit criterion (for check results)
#[derive(Debug, Clone, Serialize)]
pub struct UncheckedCriterion {
    pub id: String,
    pub text: String,
}

/// Result of checking a spec's exit criteria
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub spec_id: String,
    pub total: usize,
    pub checked: usize,
    pub unchecked: Vec<UncheckedCriterion>,
    pub passed: bool,
}

/// Check exit criteria status for a spec.
///
/// Returns pass/fail with details. Specs without exit criteria pass by default
/// (backward compatible).
pub fn check_spec_value(id: &str) -> Result<CheckResult> {
    let loaded = load_spec(id)?;
    let criteria = &loaded.frontmatter.exit_criteria;

    if criteria.is_empty() {
        return Ok(CheckResult {
            spec_id: id.to_string(),
            total: 0,
            checked: 0,
            unchecked: Vec::new(),
            passed: true,
        });
    }

    let checked_count = criteria.iter().filter(|c| c.checked).count();
    let unchecked: Vec<UncheckedCriterion> = criteria
        .iter()
        .filter(|c| !c.checked)
        .map(|c| UncheckedCriterion {
            id: c.id.clone(),
            text: c.text.clone(),
        })
        .collect();

    Ok(CheckResult {
        spec_id: id.to_string(),
        total: criteria.len(),
        checked: checked_count,
        unchecked,
        passed: checked_count == criteria.len(),
    })
}

/// Display check results (human-readable or JSON)
pub fn check_spec(id: &str, json: bool) -> Result<()> {
    let result = check_spec_value(id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    if result.total == 0 {
        println!("Exit criteria: none defined (pass by default)");
        return Ok(());
    }

    println!(
        "Exit criteria: {}/{} complete",
        result.checked, result.total
    );

    // Reload to get full criteria list for display
    let loaded = load_spec(id)?;
    for c in &loaded.frontmatter.exit_criteria {
        if c.checked {
            println!("  \u{2713} {}", c.id);
        } else {
            println!("  \u{2717} {} \u{2014} {}", c.id, c.text);
        }
    }

    if !result.passed {
        let noun = if result.unchecked.len() == 1 {
            "criterion"
        } else {
            "criteria"
        };
        println!(
            "\nCannot complete: {} unchecked {}",
            result.unchecked.len(),
            noun
        );
    }

    Ok(())
}

/// A spec ready to work on (status=ready/active, all blockers complete)
#[derive(Debug, Clone, Serialize)]
pub struct ReadySpec {
    pub id: String,
    pub status: String,
    pub target: Option<String>,
    pub title: String,
}

/// Query specs ready to work on (filesystem truth).
///
/// Returns specs where:
/// - Exists on disk in layer/surface/build/ (filesystem = truth for existence)
/// - status IN ('ready', 'active')
/// - All blocked_by specs (from frontmatter) have status 'complete' or 'done'
pub fn get_ready_specs() -> Result<Vec<ReadySpec>> {
    let all_specs = get_all_specs(&ListFilters::default())?;

    // Build status lookup for blocker resolution
    let status_map: HashMap<String, String> = all_specs
        .iter()
        .filter_map(|s| s.status.as_ref().map(|st| (s.id.clone(), st.clone())))
        .collect();

    let mut specs: Vec<ReadySpec> = all_specs
        .iter()
        .filter(|s| matches!(s.status.as_deref(), Some("ready") | Some("active")))
        .filter(|s| {
            // All blockers must be complete/done (or not found on disk = archived = done)
            s.blocked_by.iter().all(|blocker_id| {
                match status_map.get(blocker_id).map(|s| s.as_str()) {
                    Some("complete") | Some("done") | None => true,
                    _ => false,
                }
            })
        })
        .map(|s| ReadySpec {
            id: s.id.clone(),
            status: s.status.clone().unwrap_or_default(),
            target: s.target.clone(),
            title: s.title.clone(),
        })
        .collect();

    specs.sort_by(|a, b| a.target.cmp(&b.target).then(a.id.cmp(&b.id)));

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

    // Get all spec groups from a single scan
    let all_specs = get_all_specs(&ListFilters::default()).unwrap_or_default();
    let drafts: Vec<_> = all_specs
        .iter()
        .filter(|s| s.status.as_deref() == Some("draft"))
        .collect();
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

/// Query specs that are blocked by incomplete dependencies or have status='blocked'
/// (filesystem truth).
///
/// Returns specs where:
/// - Exists on disk in layer/surface/build/ (filesystem = truth for existence)
/// - Has at least one blocker (from frontmatter) with status not in ('complete', 'done')
///   OR has status = 'blocked' (set by `spec block` command)
pub fn get_blocked_specs() -> Result<Vec<BlockedSpec>> {
    let all_specs = get_all_specs(&ListFilters::default())?;

    // Build status lookup for blocker resolution
    let status_map: HashMap<String, String> = all_specs
        .iter()
        .filter_map(|s| s.status.as_ref().map(|st| (s.id.clone(), st.clone())))
        .collect();

    let mut specs: Vec<BlockedSpec> = Vec::new();

    for spec in &all_specs {
        let status = spec.status.as_deref().unwrap_or("");

        // Resolve blockers from frontmatter blocked_by
        let incomplete_blockers: Vec<Blocker> = spec
            .blocked_by
            .iter()
            .filter_map(|blocker_id| {
                let blocker_status = status_map
                    .get(blocker_id)
                    .cloned()
                    // Not on disk = archived = treat as done
                    .unwrap_or_else(|| "complete".to_string());
                if blocker_status != "complete" && blocker_status != "done" {
                    Some(Blocker {
                        id: blocker_id.clone(),
                        status: blocker_status,
                    })
                } else {
                    None
                }
            })
            .collect();

        if !incomplete_blockers.is_empty() {
            specs.push(BlockedSpec {
                id: spec.id.clone(),
                status: status.to_string(),
                target: spec.target.clone(),
                title: spec.title.clone(),
                blocked_by: incomplete_blockers,
            });
        } else if status == "blocked" {
            // status='blocked' but no incomplete blockers in frontmatter
            specs.push(BlockedSpec {
                id: spec.id.clone(),
                status: status.to_string(),
                target: spec.target.clone(),
                title: spec.title.clone(),
                blocked_by: vec![],
            });
        }
    }

    specs.sort_by(|a, b| a.id.cmp(&b.id));

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
    /// Parsed during scan — avoids re-reading files for age display
    #[serde(skip)]
    pub paused_date: Option<String>,
    #[serde(skip)]
    pub blocked_date: Option<String>,
    /// File path on disk — avoids double-walk in find_spec fallback
    #[serde(skip)]
    pub file_path: Option<String>,
    /// Frontmatter blocked_by — filesystem truth for dependency resolution
    #[serde(skip)]
    pub blocked_by: Vec<String>,
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

                let file_path = path.to_string_lossy().to_string();
                specs.push(SpecInfo {
                    id: frontmatter.id,
                    status: frontmatter.status,
                    target: frontmatter.target,
                    title,
                    unscraped: true, // disk-only until merged with DB
                    paused_date: frontmatter.paused_date,
                    blocked_date: frontmatter.blocked_date,
                    file_path: Some(file_path),
                    blocked_by: frontmatter.blocked_by,
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
