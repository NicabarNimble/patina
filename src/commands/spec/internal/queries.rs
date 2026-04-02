use anyhow::{Context, Result};
use regex::Regex;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

use patina::spec::{parse_spec_file, SpecFrontmatter, SpecStatus};

use super::archive::load_spec;
use super::db_path;

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

/// A spec ready to work on (status=ready/active, all blockers complete)
#[derive(Debug, Clone, Serialize)]
pub struct ReadySpec {
    pub id: String,
    pub status: SpecStatus,
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
    let status_map: HashMap<String, SpecStatus> = all_specs
        .iter()
        .filter_map(|s| s.status.map(|st| (s.id.clone(), st)))
        .collect();

    let mut specs: Vec<ReadySpec> = all_specs
        .iter()
        .filter(|s| matches!(s.status, Some(SpecStatus::Ready) | Some(SpecStatus::Active)))
        .filter(|s| {
            // All blockers must be terminal (or not found on disk = archived = done)
            s.blocked_by
                .iter()
                .all(|blocker_id| status_map.get(blocker_id).is_none_or(|st| st.is_terminal()))
        })
        .map(|s| ReadySpec {
            id: s.id.clone(),
            status: s.status.unwrap_or(SpecStatus::Draft),
            target: s.target.clone(),
            title: s.title.clone(),
        })
        .collect();

    specs.sort_by(|a, b| a.target.cmp(&b.target).then(a.id.cmp(&b.id)));

    Ok(specs)
}

/// A blocker preventing a spec from being ready
#[derive(Debug, Clone, Serialize)]
pub struct Blocker {
    pub id: String,
    pub status: SpecStatus,
}

/// A spec that is blocked by incomplete dependencies
#[derive(Debug, Clone, Serialize)]
pub struct BlockedSpec {
    pub id: String,
    pub status: SpecStatus,
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
    let status_map: HashMap<String, SpecStatus> = all_specs
        .iter()
        .filter_map(|s| s.status.map(|st| (s.id.clone(), st)))
        .collect();

    let mut specs: Vec<BlockedSpec> = Vec::new();

    for spec in &all_specs {
        let status: Option<SpecStatus> = spec.status;

        // Resolve blockers from frontmatter blocked_by
        let incomplete_blockers: Vec<Blocker> = spec
            .blocked_by
            .iter()
            .filter_map(|blocker_id| {
                let blocker_status = status_map
                    .get(blocker_id)
                    .copied()
                    // Not on disk = archived = treat as done
                    .unwrap_or(SpecStatus::Complete);
                if !blocker_status.is_terminal() {
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
                status: status.unwrap_or(SpecStatus::Blocked),
                target: spec.target.clone(),
                title: spec.title.clone(),
                blocked_by: incomplete_blockers,
            });
        } else if status == Some(SpecStatus::Blocked) {
            // status='blocked' but no incomplete blockers in frontmatter
            specs.push(BlockedSpec {
                id: spec.id.clone(),
                status: SpecStatus::Blocked,
                target: spec.target.clone(),
                title: spec.title.clone(),
                blocked_by: vec![],
            });
        }
    }

    specs.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(specs)
}

/// Spec info for list display
#[derive(Debug, Clone, Serialize)]
pub struct SpecInfo {
    pub id: String,
    pub status: Option<SpecStatus>,
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
    pub status: Option<SpecStatus>,
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
    let db_path = db_path()?;
    if db_path.exists() {
        if let Ok(conn) = Connection::open(&db_path) {
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
    if let Some(status_filter) = filters.status {
        specs.retain(|s| s.status == Some(status_filter));
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

/// Spec context — heading outlines + file paths for targeted reading
#[derive(Debug, Clone, Serialize)]
pub struct ShowResult {
    pub id: String,
    pub frontmatter: SpecFrontmatter,
    /// Heading outline of SPEC.md
    pub outline: Vec<String>,
    /// Heading outline of DESIGN.md (if it exists)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design_outline: Option<Vec<String>>,
    pub files: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub direct_code_targets: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resolved_decisions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub implementation_order: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub verification_points: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub open_questions: Vec<String>,
    /// File path to SPEC.md — use Read tool for specific sections
    pub path: String,
    /// File path to DESIGN.md (if it exists)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design_path: Option<String>,
}

/// Extract markdown headings (lines starting with #) from text.
/// Skips headings inside fenced code blocks (``` or ~~~).
fn extract_outline(text: &str) -> Vec<String> {
    let mut in_fence = false;
    let mut headings = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence && trimmed.starts_with('#') && trimmed.contains(' ') {
            headings.push(line.to_string());
        }
    }

    headings
}

pub(super) fn extract_section_items(text: &str, heading: &str) -> Vec<String> {
    let mut in_section = false;
    let mut items = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == heading {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with("## ") {
            break;
        }
        if in_section
            && (trimmed.starts_with("- ") || trimmed.starts_with(|c: char| c.is_ascii_digit()))
        {
            items.push(trimmed.to_string());
        }
    }

    items
}

fn extract_code_targets(text: &str) -> Vec<String> {
    let mut targets = extract_section_items(text, "## Direct Code Targets");
    if targets.is_empty() {
        targets = extract_key_files(text);
    }
    targets
}

/// Load spec context: frontmatter + heading outlines + key files.
///
/// Default mode returns outlines only — compact for MCP (~500 tokens vs 17k).
/// Full mode includes complete body and design text (for CLI display).
pub fn show_spec_value(id: &str) -> Result<ShowResult> {
    let loaded = load_spec(id)?;
    let spec_path = loaded.file_path.clone();

    // Check for DESIGN.md in the same directory as SPEC.md
    let design_path = Path::new(&loaded.file_path)
        .parent()
        .map(|dir| dir.join("DESIGN.md"))
        .filter(|p| p.exists());

    let design_text = design_path
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok());

    let outline = extract_outline(&loaded.body);
    let design_outline = design_text.as_ref().map(|d| extract_outline(d));
    let files = extract_key_files(&loaded.body);
    let direct_code_targets = design_text
        .as_ref()
        .map(|d| extract_code_targets(d))
        .unwrap_or_default();
    let resolved_decisions = extract_section_items(&loaded.body, "## Resolved Decisions");
    let implementation_order = extract_section_items(&loaded.body, "## Implementation Order");
    let verification_points = extract_section_items(&loaded.body, "## Verification");
    let open_questions = design_text
        .as_ref()
        .map(|d| extract_section_items(d, "## Open Questions"))
        .unwrap_or_default();

    Ok(ShowResult {
        id: loaded.frontmatter.id.clone(),
        frontmatter: loaded.frontmatter,
        outline,
        design_outline,
        files,
        direct_code_targets,
        resolved_decisions,
        implementation_order,
        verification_points,
        open_questions,
        path: spec_path,
        design_path: design_path.map(|p| p.to_string_lossy().to_string()),
    })
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
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                // Take first token (file path), skip trailing comments
                // Lines starting with # are comments inside the code fence
                if let Some(path) = trimmed.split_whitespace().next() {
                    files.push(path.to_string());
                }
            }
        }
    }

    files
}

/// A lifecycle event reconstructed from git tags
#[derive(Debug, Clone, Serialize)]
pub struct LifecycleEvent {
    pub date: String,
    pub state: String,
    pub tag: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_in_state: Option<u64>,
}

/// History of a spec's lifecycle from git tags
#[derive(Debug, Clone, Serialize)]
pub struct HistoryResult {
    pub spec_id: String,
    pub events: Vec<LifecycleEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_days: Option<u64>,
}

/// Parse a tag name suffix (after `spec/{id}`) into a state label.
fn tag_suffix_to_state(suffix: &str) -> &'static str {
    if suffix.is_empty() {
        "archived"
    } else if suffix == "-start" {
        "active"
    } else if suffix.starts_with("-paused-") {
        "paused"
    } else if suffix.starts_with("-resumed-") {
        "active"
    } else if suffix.starts_with("-blocked-") {
        "blocked"
    } else if suffix.ends_with("-complete") {
        "split"
    } else {
        "unknown"
    }
}

/// Load spec lifecycle history from git tags.
///
/// Queries all tags matching `spec/{id}*`, parses timestamps and event types,
/// and calculates time-in-state between consecutive events.
pub fn history_spec_value(id: &str) -> Result<HistoryResult> {
    use std::process::Command;

    let pattern = format!("refs/tags/spec/{}*", id);
    let output = Command::new("git")
        .args([
            "for-each-ref",
            "--sort=creatordate",
            "--format=%(refname:short)\t%(creatordate:iso-strict)\t%(subject)",
            &pattern,
        ])
        .output()
        .context("Failed to query git tags")?;

    if !output.status.success() {
        anyhow::bail!(
            "git for-each-ref failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let prefix = format!("spec/{}", id);

    let mut events: Vec<LifecycleEvent> = Vec::new();
    let mut dates: Vec<chrono::DateTime<chrono::FixedOffset>> = Vec::new();

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() < 3 {
            continue;
        }

        let tag = parts[0];
        let date_str = parts[1];
        let message = parts[2];

        // Extract suffix — tag must match our prefix exactly
        let suffix = if tag == prefix {
            ""
        } else if let Some(s) = tag.strip_prefix(&prefix) {
            // Must start with '-' to be a lifecycle event for this spec
            // (avoids spec/foo matching spec/foo-v2)
            if s.starts_with('-') {
                s
            } else {
                continue;
            }
        } else {
            continue;
        };

        let state = tag_suffix_to_state(suffix);
        let date = date_str.split('T').next().unwrap_or(date_str);
        let parsed_date = chrono::DateTime::parse_from_rfc3339(date_str).ok();

        events.push(LifecycleEvent {
            date: date.to_string(),
            state: state.to_string(),
            tag: tag.to_string(),
            message: message.to_string(),
            days_in_state: None,
        });
        dates.push(parsed_date.unwrap_or_else(|| {
            chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00+00:00").unwrap()
        }));
    }

    // Calculate time-in-state (days until next event)
    for i in 0..events.len().saturating_sub(1) {
        let duration = dates[i + 1].signed_duration_since(dates[i]);
        let days = duration.num_days().unsigned_abs();
        events[i].days_in_state = Some(days);
    }

    let total_days = if dates.len() >= 2 {
        let duration = dates.last().unwrap().signed_duration_since(dates[0]);
        Some(duration.num_days().unsigned_abs())
    } else {
        None
    };

    Ok(HistoryResult {
        spec_id: id.to_string(),
        events,
        total_days,
    })
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

    #[test]
    fn test_extract_section_items_bullets_and_numbers() {
        let text = "## Resolved Decisions\n\n- first\n- second\n\n## Next\n\n1. third\n";
        assert_eq!(
            extract_section_items(text, "## Resolved Decisions"),
            vec!["- first", "- second"]
        );
    }

    #[test]
    fn test_extract_section_items_handles_multi_digit_numbering() {
        let text = "## Implementation Order\n\n9. ninth\n10. tenth\n11. eleventh\n\n## Next\n";
        assert_eq!(
            extract_section_items(text, "## Implementation Order"),
            vec!["9. ninth", "10. tenth", "11. eleventh"]
        );
    }

    #[test]
    fn test_tag_suffix_to_state() {
        assert_eq!(tag_suffix_to_state(""), "archived");
        assert_eq!(tag_suffix_to_state("-start"), "active");
        assert_eq!(tag_suffix_to_state("-paused-1"), "paused");
        assert_eq!(tag_suffix_to_state("-paused-3"), "paused");
        assert_eq!(tag_suffix_to_state("-resumed-1"), "active");
        assert_eq!(tag_suffix_to_state("-blocked-1"), "blocked");
        assert_eq!(tag_suffix_to_state("-v1-complete"), "split");
        assert_eq!(tag_suffix_to_state("-something-else"), "unknown");
    }
}
