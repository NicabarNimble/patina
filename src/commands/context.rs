//! Context command — project patterns, beliefs, and conventions
//!
//! Shared between MCP (`context` tool) and CLI (`patina context`).
//! Returns core patterns (eternal principles), surface patterns (active architecture),
//! and epistemic beliefs.

use anyhow::Result;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::commands::assay::{assay_search, search_beliefs, SearchOptions};
use crate::retrieval::QueryEngine;
use rusqlite::Connection;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ContextOptions {
    pub topic: Option<String>,
    pub limits: ContextLimits,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextLimits {
    pub search_results: usize,
    pub belief_results: usize,
    pub top_beliefs: usize,
}

impl Default for ContextLimits {
    fn default() -> Self {
        Self {
            search_results: 5,
            belief_results: 5,
            top_beliefs: 10,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ContextReport {
    pub topic: Option<String>,
    pub project_health: Option<ContextHealth>,
    pub core_patterns: Vec<ContextPattern>,
    pub surface_patterns: Vec<ContextPattern>,
    pub factual_matches: Vec<ContextMatch>,
    pub semantic_matches: Vec<ContextMatch>,
    pub beliefs: ContextBeliefs,
    pub recall: RecallDirective,
    pub diagnostics: Vec<String>,
}

impl ContextReport {
    fn empty(topic: Option<String>) -> Self {
        Self {
            topic,
            recall: RecallDirective::default(),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextHealth {
    pub status: String,
    pub summary: String,
    pub structure: Option<ContextStructure>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextStructure {
    pub module_count: i64,
    pub pub_interface_count: i64,
    pub dependency_count: i64,
    pub coupling_avg: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextPattern {
    pub name: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextMatch {
    pub source_id: String,
    pub event_type: String,
    pub score: Option<f32>,
    pub content: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ContextBeliefs {
    pub topic: Option<String>,
    pub metrics: Option<ContextBeliefMetrics>,
    pub matches: Vec<ContextMatch>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextBeliefMetrics {
    pub total: i64,
    pub grounded: i64,
    pub reach_files: i64,
    pub precision: i64,
    pub verification_total: i64,
    pub verification_passed: i64,
    pub verification_failed: i64,
    pub top_beliefs: Vec<ContextBeliefUse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextBeliefUse {
    pub id: String,
    pub use_count: i64,
    pub entrenchment: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecallDirective {
    pub intro: String,
    pub meaning: String,
    pub facts: String,
    pub beliefs: String,
}

impl Default for RecallDirective {
    fn default() -> Self {
        Self {
            intro:
                "Project knowledge accumulates in beliefs — check them before assuming defaults."
                    .to_string(),
            meaning: "scry(query=\"your question\") — semantic/conceptual search".to_string(),
            facts: "assay(query_type=\"search\", query=\"your question\") — keyword/factual search"
                .to_string(),
            beliefs: "scry(content_type=\"beliefs\", query=\"your question\") — belief grounding"
                .to_string(),
        }
    }
}

/// Get project context from the knowledge layer
///
/// Reads patterns from layer/core/ (eternal principles) and layer/surface/ (active patterns)
/// Optionally filters by topic if provided
pub fn get_project_context(topic: Option<&str>) -> Result<String> {
    let report = get_project_context_report(topic)?;
    Ok(render_context_markdown(&report))
}

pub fn get_project_context_json(topic: Option<&str>) -> Result<String> {
    let report = get_project_context_report(topic)?;
    Ok(serde_json::to_string(&report)?)
}

pub fn get_project_context_report(topic: Option<&str>) -> Result<ContextReport> {
    collect_project_context(ContextOptions {
        topic: topic.map(str::to_string),
        ..ContextOptions::default()
    })
}

pub fn collect_project_context(options: ContextOptions) -> Result<ContextReport> {
    let mut report = ContextReport::empty(options.topic.clone());

    // Check if we're in a patina project
    let layer_path = Path::new("layer");
    if !layer_path.exists() {
        report.diagnostics.push(
            "No knowledge layer found. Run 'patina init' to initialize a project.".to_string(),
        );
        return Ok(report);
    }

    // Ambient health — one line so every LLM interaction carries project health
    if let Ok(measure_report) = crate::commands::measure::mcp_measure() {
        let mut structure = None;
        if let Some(capture) = measure_report.verbs.get("capture") {
            for src in &capture.sources {
                if let crate::commands::measure::VerbMetrics::CaptureStructure(m) =
                    &src.latest_metrics
                {
                    structure = Some(ContextStructure {
                        module_count: m.module_count,
                        pub_interface_count: m.pub_interface_count,
                        dependency_count: m.dependency_count,
                        coupling_avg: m.coupling_avg,
                    });
                    break;
                }
            }
        }

        report.project_health = Some(ContextHealth {
            status: measure_report.health.status.to_string(),
            summary: measure_report.health.summary,
            structure,
        });
    }

    // Read core patterns (eternal principles)
    let core_path = layer_path.join("core");
    report.core_patterns = read_patterns(&core_path, options.topic.as_deref())?;

    // Read surface patterns (active architecture)
    let surface_path = layer_path.join("surface");
    report.surface_patterns = read_patterns(&surface_path, options.topic.as_deref())?;

    // Topic-specific search: factual (assay) + semantic (scry) fusion
    // Two signals, simple merge: facts first, meaning for gaps
    if let Some(t) = options.topic.as_deref() {
        let (factual_matches, semantic_matches) =
            collect_topic_search_results(t, options.limits.search_results);
        report.factual_matches = factual_matches;
        report.semantic_matches = semantic_matches;
    }

    // Beliefs are always eligible — topic changes the query, not whether beliefs exist
    if let Some(t) = options.topic.as_deref() {
        // Topic provided: FTS5 ranking via assay belief search
        report.beliefs = collect_topic_beliefs(t, options.limits.belief_results);
    } else {
        // No topic: aggregate stats + top beliefs by use count
        report.beliefs.metrics = collect_belief_metrics(options.limits.top_beliefs)?;
    }

    Ok(report)
}

pub fn render_context_markdown(report: &ContextReport) -> String {
    if let Some(first) = report.diagnostics.first() {
        return first.clone();
    }

    let mut output = String::new();

    if let Some(health) = &report.project_health {
        output.push_str(&format!(
            "# Project Health\n\n{} — {}\n",
            health.status, health.summary
        ));
        if let Some(structure) = &health.structure {
            output.push_str(&format!(
                "Structure: {} modules, {} pub interfaces, {} deps, avg fan-out {:.1}\n",
                structure.module_count,
                structure.pub_interface_count,
                structure.dependency_count,
                structure.coupling_avg
            ));
        }
        output.push('\n');
    }

    render_patterns(
        &mut output,
        "# Core Patterns (Eternal Principles)",
        &report.core_patterns,
    );
    render_patterns(
        &mut output,
        "# Surface Patterns (Active Architecture)",
        &report.surface_patterns,
    );

    render_matches(
        &mut output,
        "# Factual Matches (keyword search)",
        &report.factual_matches,
        false,
    );
    render_matches(
        &mut output,
        "# Semantic Matches (conceptually related)",
        &report.semantic_matches,
        true,
    );

    if !report.beliefs.matches.is_empty() {
        let topic = report.beliefs.topic.as_deref().unwrap_or("");
        output.push_str(&format!(
            "# Active Beliefs (ranked by relevance to \"{}\")\n\n",
            topic
        ));
        for r in &report.beliefs.matches {
            output.push_str(&format!(
                "- **{}** (score: {:.2}): {}\n",
                r.source_id,
                r.score.unwrap_or_default(),
                r.content
            ));
        }
        output.push('\n');
    } else if let Some(metrics) = &report.beliefs.metrics {
        output.push_str(&render_belief_metrics(metrics));
    }

    if output.is_empty() {
        if let Some(t) = report.topic.as_deref() {
            output = format!("No patterns found matching topic: '{}'", t);
        } else {
            output = "No patterns found in the knowledge layer.".to_string();
        }
    }

    output.push_str("## Recall Directive\n\n");
    output.push_str(&report.recall.intro);
    output.push('\n');
    output.push_str(&format!("  Meaning:  {}\n", report.recall.meaning));
    output.push_str(&format!("  Facts:    {}\n", report.recall.facts));
    output.push_str(&format!("  Beliefs:  {}\n", report.recall.beliefs));

    output
}

fn render_patterns(output: &mut String, heading: &str, patterns: &[ContextPattern]) {
    if patterns.is_empty() {
        return;
    }

    output.push_str(heading);
    output.push_str("\n\n");
    for pattern in patterns {
        output.push_str(&format!("## {}\n\n{}\n\n", pattern.name, pattern.summary));
    }
}

fn render_matches(
    output: &mut String,
    heading: &str,
    matches: &[ContextMatch],
    include_score: bool,
) {
    if matches.is_empty() {
        return;
    }

    output.push_str(heading);
    output.push_str("\n\n");
    for r in matches {
        let truncated = truncate_match_content(&r.content);
        let ellipsis = if r.content.trim().chars().count() > 150 {
            "..."
        } else {
            ""
        };
        if include_score {
            output.push_str(&format!(
                "- **{}** ({}, {:.3}): {}{}\n",
                r.source_id,
                r.event_type,
                r.score.unwrap_or_default(),
                truncated,
                ellipsis
            ));
        } else {
            output.push_str(&format!(
                "- **{}** ({}): {}{}\n",
                r.source_id, r.event_type, truncated, ellipsis
            ));
        }
    }
    output.push('\n');
}

fn truncate_match_content(content: &str) -> String {
    content.trim().chars().take(150).collect()
}

fn render_belief_metrics(metrics: &ContextBeliefMetrics) -> String {
    let mut output = String::from("# Epistemic Beliefs\n\n");
    output.push_str(&format!(
        "**Total:** {} beliefs | **Grounded:** {}/{} ({:.0}%) | **Reach files:** {} ({}% precision)\n",
        metrics.total,
        metrics.grounded,
        metrics.total,
        if metrics.total > 0 {
            metrics.grounded as f64 / metrics.total as f64 * 100.0
        } else {
            0.0
        },
        metrics.reach_files,
        metrics.precision,
    ));
    output.push_str(&format!(
        "**Verification:** {}/{} passed ({} failed)\n\n",
        metrics.verification_passed, metrics.verification_total, metrics.verification_failed,
    ));

    if !metrics.top_beliefs.is_empty() {
        output.push_str("**Top beliefs by use:**\n");
        for belief in &metrics.top_beliefs {
            output.push_str(&format!(
                "- {} (use: {}, entrenchment: {}, status: {})\n",
                belief.id, belief.use_count, belief.entrenchment, belief.status,
            ));
        }
        output.push('\n');
    }

    output
}

fn collect_belief_metrics(limit: usize) -> Result<Option<ContextBeliefMetrics>> {
    let db_path = patina::eventlog::patina_db_path()?;
    let conn = Connection::open(db_path)?;

    // Check if beliefs table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='beliefs'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !table_exists {
        return Ok(None);
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
        return Ok(None);
    }

    let precision = if reach_files > 0 { 100 } else { 0 }; // All reach files are source code (filtered at hop)

    // Top beliefs by use count
    let mut stmt = conn.prepare(
        "SELECT id, cited_by_beliefs + cited_by_sessions + applied_in as use_count,
                entrenchment, status
         FROM beliefs
         ORDER BY use_count DESC
         LIMIT ?1",
    )?;

    let top_beliefs: Vec<ContextBeliefUse> = stmt
        .query_map([limit as i64], |row| {
            Ok(ContextBeliefUse {
                id: row.get::<_, String>(0)?,
                use_count: row.get::<_, i64>(1)?,
                entrenchment: row.get::<_, String>(2)?,
                status: row.get::<_, String>(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(Some(ContextBeliefMetrics {
        total,
        grounded,
        reach_files,
        precision,
        verification_total: verif_total,
        verification_passed: verif_pass,
        verification_failed: verif_fail,
        top_beliefs,
    }))
}

/// Two-signal fusion: assay (factual/keyword) + scry (semantic) search results
///
/// Called when a topic is provided. Returns factual matches first (what files,
/// commits, patterns match by keyword), then semantic matches for gaps (what's
/// conceptually related but not keyword-matched). Simple merge — no tuning.
fn collect_topic_search_results(
    topic: &str,
    limit: usize,
) -> (Vec<ContextMatch>, Vec<ContextMatch>) {
    let mut factual_matches = Vec::new();
    let mut semantic_matches = Vec::new();
    let mut seen_ids = HashSet::new();

    // 1. Factual: assay keyword search (FTS5 across code, commits, patterns)
    let search_opts = SearchOptions {
        limit,
        include_issues: false,
        repo: None,
    };

    if let Ok(assay_results) = assay_search(topic, &search_opts) {
        for r in &assay_results {
            seen_ids.insert(r.source_id.clone());
            factual_matches.push(ContextMatch {
                source_id: r.source_id.clone(),
                event_type: r.event_type.clone(),
                score: Some(r.score),
                content: r.content.replace('\n', " "),
            });
        }
    }

    // 2. Semantic: scry vector search (conceptually related items)
    let engine = QueryEngine::new();
    if let Ok(scry_results) = engine.query(topic, limit) {
        let novel: Vec<_> = scry_results
            .iter()
            .filter(|r| !seen_ids.contains(&r.doc_id))
            .take(limit)
            .collect();

        for r in &novel {
            semantic_matches.push(ContextMatch {
                source_id: r.doc_id.clone(),
                event_type: r
                    .metadata
                    .event_type
                    .as_deref()
                    .unwrap_or("unknown")
                    .to_string(),
                score: Some(r.fused_score),
                content: r.content.replace('\n', " "),
            });
        }
    }

    (factual_matches, semantic_matches)
}

/// Query beliefs ranked by relevance to a topic
///
/// Uses FTS5 keyword search via assay's belief module to find beliefs
/// relevant to the given topic. Falls back to aggregate metrics if
/// the database is unavailable.
fn collect_topic_beliefs(topic: &str, limit: usize) -> ContextBeliefs {
    let mut beliefs = ContextBeliefs {
        topic: Some(topic.to_string()),
        ..ContextBeliefs::default()
    };

    match search_beliefs(topic, limit) {
        Ok(results) if !results.is_empty() => {
            beliefs.matches = results
                .into_iter()
                .map(|r| ContextMatch {
                    source_id: r.source_id,
                    event_type: r.event_type,
                    score: Some(r.score),
                    content: r.content,
                })
                .collect();
        }
        _ => {
            beliefs.metrics = collect_belief_metrics(10).unwrap_or_default();
        }
    }

    beliefs
}

/// Read markdown patterns from a directory
fn read_patterns(dir: &Path, topic: Option<&str>) -> Result<Vec<ContextPattern>> {
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
                patterns.push(ContextPattern { name, summary });
            } else {
                let content = fs::read_to_string(&path)?;
                let summary = extract_summary(&content);
                patterns.push(ContextPattern { name, summary });
            }
        }
    }

    // Sort by name for consistent output
    patterns.sort_by(|a, b| a.name.cmp(&b.name));
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
    let start = std::time::Instant::now();
    let output = get_project_context(topic)?;
    println!("{}", output);

    // Emit usage event to events.db (best-effort)
    let duration_ms = start.elapsed().as_millis() as u64;
    if let Err(e) = (|| -> Result<()> {
        let conn = patina::eventlog::open_events_db()?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        patina::eventlog::insert_event(
            &conn,
            "context.query",
            &timestamp,
            topic.unwrap_or("(none)"),
            None,
            &serde_json::json!({
                "topic": topic,
                "duration_ms": duration_ms,
            })
            .to_string(),
        )?;
        Ok(())
    })() {
        eprintln!("patina: warning: failed to record context.query event: {e}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_report() -> ContextReport {
        ContextReport {
            topic: Some("architecture".to_string()),
            project_health: Some(ContextHealth {
                status: "good".to_string(),
                summary: "1/5 verbs healthy".to_string(),
                structure: Some(ContextStructure {
                    module_count: 10,
                    pub_interface_count: 3,
                    dependency_count: 4,
                    coupling_avg: 1.5,
                }),
            }),
            core_patterns: vec![ContextPattern {
                name: "dependable-rust".to_string(),
                summary: "Keep public interfaces small.".to_string(),
            }],
            surface_patterns: Vec::new(),
            factual_matches: vec![ContextMatch {
                source_id: "src/foo.rs".to_string(),
                event_type: "code.function".to_string(),
                score: Some(1.0),
                content: "fn foo()".to_string(),
            }],
            semantic_matches: Vec::new(),
            beliefs: ContextBeliefs {
                topic: Some("architecture".to_string()),
                metrics: None,
                matches: Vec::new(),
            },
            recall: RecallDirective::default(),
            diagnostics: Vec::new(),
        }
    }

    #[test]
    fn render_context_report_keeps_markdown_sections() {
        let markdown = render_context_markdown(&sample_report());

        assert!(markdown.contains("# Project Health"));
        assert!(markdown.contains("good — 1/5 verbs healthy"));
        assert!(markdown.contains("# Core Patterns (Eternal Principles)"));
        assert!(markdown.contains("## dependable-rust"));
        assert!(markdown.contains("# Factual Matches (keyword search)"));
        assert!(markdown.contains("## Recall Directive"));
    }

    #[test]
    fn context_report_json_has_structured_sections() {
        let value = serde_json::to_value(sample_report()).expect("serialize report");

        assert_eq!(value["topic"], "architecture");
        assert_eq!(value["project_health"]["status"], "good");
        assert_eq!(value["core_patterns"][0]["name"], "dependable-rust");
        assert_eq!(value["factual_matches"][0]["source_id"], "src/foo.rs");
        assert!(value["recall"]["meaning"]
            .as_str()
            .unwrap()
            .contains("scry"));
    }

    #[test]
    fn extract_summary_skips_frontmatter_and_title() {
        let content = r#"---
id: demo
---
# Demo Pattern

First paragraph.

Second paragraph.
"#;

        assert_eq!(
            extract_summary(content),
            "First paragraph. \nSecond paragraph."
        );
    }
}
