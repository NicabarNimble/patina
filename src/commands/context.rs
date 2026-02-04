//! Context command — project patterns, beliefs, and conventions
//!
//! Shared between MCP (`context` tool) and CLI (`patina context`).
//! Returns core patterns (eternal principles), surface patterns (active architecture),
//! and epistemic beliefs.

use anyhow::Result;
use std::fs;
use std::path::Path;

/// Get project context from the knowledge layer
///
/// Reads patterns from layer/core/ (eternal principles) and layer/surface/ (active patterns)
/// Optionally filters by topic if provided
pub fn get_project_context(topic: Option<&str>) -> Result<String> {
    let mut output = String::new();

    // Check if we're in a patina project
    let layer_path = Path::new("layer");
    if !layer_path.exists() {
        return Ok(
            "No knowledge layer found. Run 'patina init' to initialize a project.".to_string(),
        );
    }

    // Read core patterns (eternal principles)
    let core_path = layer_path.join("core");
    let core_patterns = read_patterns(&core_path, topic)?;

    // Read surface patterns (active architecture)
    let surface_path = layer_path.join("surface");
    let surface_patterns = read_patterns(&surface_path, topic)?;

    // Format output
    if !core_patterns.is_empty() {
        output.push_str("# Core Patterns (Eternal Principles)\n\n");
        for (name, content) in &core_patterns {
            output.push_str(&format!("## {}\n\n{}\n\n", name, content));
        }
    }

    if !surface_patterns.is_empty() {
        output.push_str("# Surface Patterns (Active Architecture)\n\n");
        for (name, content) in &surface_patterns {
            output.push_str(&format!("## {}\n\n{}\n\n", name, content));
        }
    }

    // Beliefs are always eligible — topic changes the query, not whether beliefs exist
    if let Ok(belief_section) = get_belief_metrics() {
        output.push_str(&belief_section);
    }

    if output.is_empty() {
        if let Some(t) = topic {
            output = format!("No patterns found matching topic: '{}'", t);
        } else {
            output = "No patterns found in the knowledge layer.".to_string();
        }
    }

    Ok(output)
}

/// Query belief metrics from the database for the context tool
pub fn get_belief_metrics() -> Result<String> {
    use rusqlite::Connection;

    const DB_PATH: &str = ".patina/local/data/patina.db";
    let conn = Connection::open(DB_PATH)?;

    // Check if beliefs table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='beliefs'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !table_exists {
        return Ok(String::new());
    }

    // Aggregate stats
    let (total, grounded, reach_files, verif_total, verif_pass, verif_fail): (
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
    ) = conn.query_row(
        "SELECT
            COUNT(*),
            SUM(CASE WHEN grounding_code_count > 0 THEN 1 ELSE 0 END),
            SUM(grounding_code_count),
            SUM(verification_total),
            SUM(verification_passed),
            SUM(verification_failed)
         FROM beliefs",
        [],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        },
    )?;

    if total == 0 {
        return Ok(String::new());
    }

    let precision = if reach_files > 0 { 100 } else { 0 }; // All reach files are source code (filtered at hop)

    let mut output = String::from("# Epistemic Beliefs\n\n");
    output.push_str(&format!(
        "**Total:** {} beliefs | **Grounded:** {}/{} ({:.0}%) | **Reach files:** {} ({}% precision)\n",
        total,
        grounded,
        total,
        if total > 0 { grounded as f64 / total as f64 * 100.0 } else { 0.0 },
        reach_files,
        precision,
    ));
    output.push_str(&format!(
        "**Verification:** {}/{} passed ({} failed)\n\n",
        verif_pass, verif_total, verif_fail,
    ));

    // Top beliefs by use count
    let mut stmt = conn.prepare(
        "SELECT id, cited_by_beliefs + cited_by_sessions + applied_in as use_count,
                entrenchment, status
         FROM beliefs
         ORDER BY use_count DESC
         LIMIT 10",
    )?;

    let top_beliefs: Vec<(String, i64, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if !top_beliefs.is_empty() {
        output.push_str("**Top beliefs by use:**\n");
        for (id, use_count, entrenchment, status) in &top_beliefs {
            output.push_str(&format!(
                "- {} (use: {}, entrenchment: {}, status: {})\n",
                id, use_count, entrenchment, status,
            ));
        }
        output.push('\n');
    }

    Ok(output)
}

/// Read markdown patterns from a directory
fn read_patterns(dir: &Path, topic: Option<&str>) -> Result<Vec<(String, String)>> {
    let mut patterns = Vec::new();

    if !dir.exists() {
        return Ok(patterns);
    }

    // Read .md files in the directory
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only process markdown files
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Skip certain files
            if name == "README" || name.starts_with('.') {
                continue;
            }

            // If topic filter provided, match against filename and title only
            // (not full body — substring on markdown bodies returns false positives)
            if let Some(t) = topic {
                let topic_lower = t.to_lowercase();
                let name_lower = name.to_lowercase();

                // Extract title from first # line without reading full content
                let content = fs::read_to_string(&path)?;
                let title = extract_title(&content);
                let title_lower = title.to_lowercase();

                if !name_lower.contains(&topic_lower) && !title_lower.contains(&topic_lower) {
                    continue;
                }

                let summary = extract_summary(&content);
                patterns.push((name, summary));
            } else {
                let content = fs::read_to_string(&path)?;
                let summary = extract_summary(&content);
                patterns.push((name, summary));
            }
        }
    }

    // Sort by name for consistent output
    patterns.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(patterns)
}

/// Extract the title from markdown content (first # line after frontmatter)
fn extract_title(content: &str) -> String {
    let mut in_frontmatter = false;
    for line in content.lines() {
        if line == "---" {
            in_frontmatter = !in_frontmatter;
            continue;
        }
        if in_frontmatter {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            return trimmed.trim_start_matches('#').trim().to_string();
        }
        // Stop after first non-empty, non-frontmatter line that isn't a title
        if !trimmed.is_empty() {
            break;
        }
    }
    String::new()
}

/// Extract a summary from markdown content (skip frontmatter, get first paragraphs)
pub fn extract_summary(content: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();

    // Skip YAML frontmatter if present
    if lines.first().map(|l| *l == "---").unwrap_or(false) {
        if let Some(end) = lines.iter().skip(1).position(|l| *l == "---") {
            lines = lines[end + 2..].to_vec();
        }
    }

    // Skip title line (# ...)
    if lines.first().map(|l| l.starts_with('#')).unwrap_or(false) {
        lines = lines[1..].to_vec();
    }

    // Get first ~500 chars of meaningful content
    let mut summary = String::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !summary.is_empty() {
                summary.push('\n');
            }
            continue;
        }
        summary.push_str(trimmed);
        summary.push(' ');

        if summary.len() > 500 {
            // Truncate at char boundary
            let truncated: String = summary.chars().take(500).collect();
            summary = truncated;
            summary.push_str("...");
            break;
        }
    }

    summary.trim().to_string()
}

/// Execute CLI context command
pub fn execute(topic: Option<&str>) -> Result<()> {
    let output = get_project_context(topic)?;
    println!("{}", output);
    Ok(())
}
