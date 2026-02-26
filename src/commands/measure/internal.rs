//! Internal implementation for the measure command
//!
//! Queries the eventlog for measurement data from two sources:
//! - measure.* events (new, from Phase 1-2 tools)
//! - Existing typed events that carry measurement data

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;
use std::path::Path;

use super::MeasureOptions;
use patina::eventlog;

// ============================================================================
// Data Structures
// ============================================================================

/// A single verb's measurement summary
#[derive(Debug, Serialize)]
pub struct VerbSummary {
    pub verb: String,
    pub status: VerbStatus,
    pub latest_timestamp: Option<String>,
    pub sources: Vec<SourceSummary>,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq)]
pub enum VerbStatus {
    Good,
    NeedsAttention,
    NoData,
}

impl std::fmt::Display for VerbStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerbStatus::Good => write!(f, "good"),
            VerbStatus::NeedsAttention => write!(f, "needs attention"),
            VerbStatus::NoData => write!(f, "no data"),
        }
    }
}

/// Metrics from a single source (tool+mode or existing event type)
#[derive(Debug, Serialize)]
pub struct SourceSummary {
    pub source_type: String, // "measure.*" or "belief.surface" etc.
    pub tool: String,
    pub mode: String,
    pub latest_metrics: serde_json::Value,
    pub timestamp: String,
    pub event_count: i64,
}

/// Full measurement report
#[derive(Debug, Serialize)]
pub struct MeasureReport {
    pub generated: String,
    pub verbs: Vec<VerbSummary>,
    pub total_measurement_events: i64,
    pub actions: Vec<String>,
}

/// History entry for verb drill-down
#[derive(Debug, Serialize)]
struct HistoryEntry {
    timestamp: String,
    tool: String,
    mode: String,
    metrics: serde_json::Value,
}

// ============================================================================
// Main Entry Point
// ============================================================================

pub fn run(options: MeasureOptions) -> Result<()> {
    let db_path = Path::new(eventlog::PATINA_DB);
    if !db_path.exists() {
        print_empty_state();
        return Ok(());
    }

    let conn = Connection::open(db_path).context("Failed to open patina.db")?;

    // Check if ANY measurement data exists
    let total_events = count_measurement_events(&conn)?;
    let has_existing = has_existing_measurement_events(&conn)?;

    if total_events == 0 && !has_existing {
        print_empty_state();
        return Ok(());
    }

    if let Some(ref verb) = options.verb {
        return run_verb_drilldown(&conn, verb, &options);
    }

    let report = build_report(&conn)?;

    if options.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if options.system {
        render_system_view(&conn, &report)?;
    } else {
        render_user_view(&report);
    }

    Ok(())
}

// ============================================================================
// Data Collection
// ============================================================================

fn count_measurement_events(conn: &Connection) -> Result<i64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM eventlog WHERE event_type LIKE 'measure.%'",
        [],
        |row| row.get(0),
    )?;
    Ok(count)
}

fn has_existing_measurement_events(conn: &Connection) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM eventlog WHERE event_type IN ('belief.surface', 'session.ended')",
        [],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn build_report(conn: &Connection) -> Result<MeasureReport> {
    let mut verbs = Vec::new();
    let mut actions = Vec::new();

    // Capture verb
    let capture = build_capture_summary(conn)?;
    if capture.status == VerbStatus::NoData {
        actions.push("Run `patina scrape` to populate capture metrics".to_string());
    }
    verbs.push(capture);

    // Index verb
    let index = build_index_summary(conn)?;
    if index.status == VerbStatus::NoData {
        actions.push("Run `patina oxidize` to populate index metrics".to_string());
    }
    verbs.push(index);

    // Search verb
    let search = build_search_summary(conn)?;
    if search.status == VerbStatus::NoData {
        actions.push("Run `patina eval` to populate search metrics".to_string());
    }
    verbs.push(search);

    // Believe verb
    let believe = build_believe_summary(conn)?;
    if believe.status == VerbStatus::NoData {
        actions.push("Run `patina scrape` to populate belief metrics".to_string());
    }
    verbs.push(believe);

    // Evolve verb
    let evolve = build_evolve_summary(conn)?;
    if evolve.status == VerbStatus::NoData {
        actions
            .push("Run a session (`patina session start`) to populate evolve metrics".to_string());
    }
    verbs.push(evolve);

    let total_measurement_events = count_measurement_events(conn)?;

    Ok(MeasureReport {
        generated: chrono::Utc::now().to_rfc3339(),
        verbs,
        total_measurement_events,
        actions,
    })
}

/// Build capture verb summary from measure.capture + git.commit
fn build_capture_summary(conn: &Connection) -> Result<VerbSummary> {
    let mut sources = Vec::new();

    // measure.capture events
    collect_measure_sources(conn, "capture", &mut sources)?;

    // git.commit as coverage proxy: count distinct files in recent commits
    let git_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM eventlog WHERE event_type = 'git.commit'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if git_count > 0 {
        let latest_ts: String = conn
            .query_row(
                "SELECT MAX(timestamp) FROM eventlog WHERE event_type = 'git.commit'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_default();

        // Count distinct files touched in recent commits (by timestamp, not insertion order)
        let file_count: i64 = conn
            .query_row(
                r#"SELECT COUNT(DISTINCT json_each.value)
                   FROM eventlog, json_each(json_extract(data, '$.files'))
                   WHERE event_type = 'git.commit'
                     AND timestamp >= (
                       SELECT timestamp FROM eventlog
                       WHERE event_type = 'git.commit'
                       ORDER BY timestamp DESC
                       LIMIT 1 OFFSET 99
                     )"#,
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        sources.push(SourceSummary {
            source_type: "git.commit".to_string(),
            tool: "scrape".to_string(),
            mode: "git".to_string(),
            latest_metrics: serde_json::json!({ "files_tracked": file_count, "total_commits": git_count }),
            timestamp: latest_ts,
            event_count: git_count,
        });
    }

    let status = determine_status(&sources);
    let latest_timestamp = sources
        .iter()
        .map(|s| s.timestamp.as_str())
        .max()
        .map(String::from);

    Ok(VerbSummary {
        verb: "capture".to_string(),
        status,
        latest_timestamp,
        sources,
    })
}

/// Build index verb summary from measure.index
fn build_index_summary(conn: &Connection) -> Result<VerbSummary> {
    let mut sources = Vec::new();
    collect_measure_sources(conn, "index", &mut sources)?;

    let status = determine_status(&sources);
    let latest_timestamp = sources
        .iter()
        .map(|s| s.timestamp.as_str())
        .max()
        .map(String::from);

    Ok(VerbSummary {
        verb: "index".to_string(),
        status,
        latest_timestamp,
        sources,
    })
}

/// Build search verb summary from measure.search
fn build_search_summary(conn: &Connection) -> Result<VerbSummary> {
    let mut sources = Vec::new();
    collect_measure_sources(conn, "search", &mut sources)?;

    let status = if sources.is_empty() {
        VerbStatus::NoData
    } else {
        // Check P@5 threshold if available
        let mut has_low_precision = false;
        for src in &sources {
            if let Some(p5) = src.latest_metrics.get("p_at_5").and_then(|v| v.as_f64()) {
                if p5 < 0.4 {
                    has_low_precision = true;
                }
            }
        }
        if has_low_precision {
            VerbStatus::NeedsAttention
        } else {
            VerbStatus::Good
        }
    };

    let latest_timestamp = sources
        .iter()
        .map(|s| s.timestamp.as_str())
        .max()
        .map(String::from);

    Ok(VerbSummary {
        verb: "search".to_string(),
        status,
        latest_timestamp,
        sources,
    })
}

/// Build believe verb summary from beliefs table + belief.surface events
fn build_believe_summary(conn: &Connection) -> Result<VerbSummary> {
    let mut sources = Vec::new();

    // Read from beliefs table (has current grounding scores, updated after oxidize)
    // Falls back to belief.surface events if beliefs table doesn't exist
    let has_beliefs_table = conn.prepare("SELECT COUNT(*) FROM beliefs LIMIT 1").is_ok();

    if has_beliefs_table {
        let result = conn.query_row(
            r#"SELECT
                COUNT(*) as total_beliefs,
                COALESCE(SUM(CASE WHEN grounding_score = 0 THEN 1 ELSE 0 END), 0) as floating,
                COALESCE(AVG(evidence_count), 0) as avg_evidence,
                COALESCE(AVG(health_score), 0) as avg_health
            FROM beliefs"#,
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                ))
            },
        );

        // Get latest timestamp from belief.surface events
        let latest_ts: String = conn
            .query_row(
                "SELECT MAX(timestamp) FROM eventlog WHERE event_type = 'belief.surface'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let event_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM eventlog WHERE event_type = 'belief.surface'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if let Ok((total, floating, avg_evidence, avg_health)) = result {
            if total > 0 {
                sources.push(SourceSummary {
                    source_type: "beliefs".to_string(),
                    tool: "scrape".to_string(),
                    mode: "beliefs".to_string(),
                    latest_metrics: serde_json::json!({
                        "total_beliefs": total,
                        "floating_count": floating,
                        "grounded_count": total - floating,
                        "avg_evidence": (avg_evidence * 100.0).round() / 100.0,
                        "avg_health": (avg_health * 100.0).round() / 100.0,
                    }),
                    timestamp: latest_ts,
                    event_count,
                });
            }
        }
    }

    // Also check measure.believe if any exist
    collect_measure_sources(conn, "believe", &mut sources)?;

    let status = if sources.is_empty() {
        VerbStatus::NoData
    } else {
        // Check floating threshold
        let floating_pct = sources
            .iter()
            .filter(|s| s.source_type == "beliefs")
            .find_map(|s| {
                let total = s.latest_metrics.get("total_beliefs")?.as_f64()?;
                let floating = s.latest_metrics.get("floating_count")?.as_f64()?;
                if total > 0.0 {
                    Some(floating / total)
                } else {
                    None
                }
            })
            .unwrap_or(0.0);

        if floating_pct > 0.1 {
            VerbStatus::NeedsAttention
        } else {
            VerbStatus::Good
        }
    };

    let latest_timestamp = sources
        .iter()
        .map(|s| s.timestamp.as_str())
        .max()
        .map(String::from);

    Ok(VerbSummary {
        verb: "believe".to_string(),
        status,
        latest_timestamp,
        sources,
    })
}

/// Build evolve verb summary from session.ended (existing events)
fn build_evolve_summary(conn: &Connection) -> Result<VerbSummary> {
    let mut sources = Vec::new();

    let session_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM eventlog WHERE event_type = 'session.ended'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if session_count > 0 {
        // Aggregate recent session metrics
        let result = conn.query_row(
            r#"SELECT
                COUNT(*) as total_sessions,
                COALESCE(SUM(json_extract(data, '$.commits_made')), 0) as total_commits,
                COALESCE(SUM(json_extract(data, '$.files_changed')), 0) as total_files,
                COALESCE(SUM(json_extract(data, '$.beliefs_captured')), 0) as total_beliefs,
                COALESCE(SUM(json_extract(data, '$.patterns_modified')), 0) as total_patterns,
                MAX(timestamp) as latest_ts
            FROM eventlog
            WHERE event_type = 'session.ended'"#,
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        );

        if let Ok((sessions, commits, files, beliefs, patterns, latest_ts)) = result {
            sources.push(SourceSummary {
                source_type: "session.ended".to_string(),
                tool: "session".to_string(),
                mode: "lifecycle".to_string(),
                latest_metrics: serde_json::json!({
                    "total_sessions": sessions,
                    "total_commits": commits,
                    "total_files_changed": files,
                    "total_beliefs_captured": beliefs,
                    "total_patterns_modified": patterns,
                }),
                timestamp: latest_ts,
                event_count: session_count,
            });
        }
    }

    // Also check measure.evolve if any
    collect_measure_sources(conn, "evolve", &mut sources)?;

    let status = determine_status(&sources);
    let latest_timestamp = sources
        .iter()
        .map(|s| s.timestamp.as_str())
        .max()
        .map(String::from);

    Ok(VerbSummary {
        verb: "evolve".to_string(),
        status,
        latest_timestamp,
        sources,
    })
}

/// Collect measure.* sources for a given verb, grouped by tool:mode
fn collect_measure_sources(
    conn: &Connection,
    verb: &str,
    sources: &mut Vec<SourceSummary>,
) -> Result<()> {
    let event_type = format!("measure.{}", verb);

    // Get distinct tool:mode combinations with latest metrics
    let mut stmt = conn.prepare(
        r#"SELECT
            json_extract(data, '$.tool') as tool,
            json_extract(data, '$.mode') as mode,
            json_extract(data, '$.metrics') as metrics,
            timestamp,
            (SELECT COUNT(*) FROM eventlog e2
             WHERE e2.event_type = ?1
               AND json_extract(e2.data, '$.tool') = json_extract(eventlog.data, '$.tool')
               AND json_extract(e2.data, '$.mode') = json_extract(eventlog.data, '$.mode')
            ) as event_count
        FROM eventlog
        WHERE event_type = ?1
          AND seq IN (
            SELECT MAX(seq) FROM eventlog
            WHERE event_type = ?1
            GROUP BY json_extract(data, '$.tool'), json_extract(data, '$.mode')
          )
        ORDER BY timestamp DESC"#,
    )?;

    let rows = stmt.query_map([&event_type], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;

    for row in rows {
        let (tool, mode, metrics_str, timestamp, count) = row?;
        let metrics: serde_json::Value =
            serde_json::from_str(&metrics_str).unwrap_or(serde_json::Value::Null);

        sources.push(SourceSummary {
            source_type: event_type.clone(),
            tool,
            mode,
            latest_metrics: metrics,
            timestamp,
            event_count: count,
        });
    }

    Ok(())
}

fn determine_status(sources: &[SourceSummary]) -> VerbStatus {
    if sources.is_empty() {
        VerbStatus::NoData
    } else {
        VerbStatus::Good
    }
}

// ============================================================================
// User View (default)
// ============================================================================

fn render_user_view(report: &MeasureReport) {
    println!("\n  Project Health\n");

    let good_count = report
        .verbs
        .iter()
        .filter(|v| v.status == VerbStatus::Good)
        .count();
    let attention_count = report
        .verbs
        .iter()
        .filter(|v| v.status == VerbStatus::NeedsAttention)
        .count();
    let no_data_count = report
        .verbs
        .iter()
        .filter(|v| v.status == VerbStatus::NoData)
        .count();

    // Overall status line
    if no_data_count == 5 {
        println!("  No measurements yet. Run some tools to get started.\n");
        return;
    }

    println!(
        "  {}/{} verbs reporting{}{}",
        good_count + attention_count,
        5,
        if attention_count > 0 {
            format!(", {} needs attention", attention_count)
        } else {
            String::new()
        },
        if no_data_count > 0 {
            format!(", {} missing", no_data_count)
        } else {
            String::new()
        }
    );
    println!();

    // Per-verb summary
    for verb_summary in &report.verbs {
        let icon = match verb_summary.status {
            VerbStatus::Good => "+",
            VerbStatus::NeedsAttention => "!",
            VerbStatus::NoData => "-",
        };

        let age = verb_summary
            .latest_timestamp
            .as_ref()
            .map(|ts| format_age(ts))
            .unwrap_or_else(|| "never".to_string());

        println!(
            "  [{}] {:<10} {:<18} {}",
            icon,
            verb_summary.verb,
            verb_summary.status,
            if verb_summary.status != VerbStatus::NoData {
                format!("({})", age)
            } else {
                String::new()
            }
        );

        // Show key metrics in user-friendly language
        for src in &verb_summary.sources {
            let summary = user_friendly_metrics(&verb_summary.verb, src);
            if !summary.is_empty() {
                println!("        {}", summary);
            }
        }
    }

    // Action items
    if !report.actions.is_empty() {
        println!("\n  Actions:");
        for action in &report.actions {
            println!("    - {}", action);
        }
    }

    println!();
}

fn user_friendly_metrics(verb: &str, src: &SourceSummary) -> String {
    let m = &src.latest_metrics;
    match (verb, src.source_type.as_str()) {
        ("capture", "measure.capture") if src.mode == "code" => {
            let files = m.get("files_parsed").and_then(|v| v.as_i64()).unwrap_or(0);
            let funcs = m
                .get("functions_found")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            format!("{}: {} files, {} functions parsed", src.mode, files, funcs)
        }
        ("capture", "measure.capture") => {
            // Doctor or other capture sources — show tool:mode and key metrics
            let parts: Vec<String> = m
                .as_object()
                .map(|obj| {
                    obj.iter()
                        .filter(|(_, v)| v.as_i64().map(|n| n > 0).unwrap_or(false))
                        .map(|(k, v)| format!("{} {}", v, k))
                        .collect()
                })
                .unwrap_or_default();
            if parts.is_empty() {
                format!("{}: checked", src.mode)
            } else {
                format!("{}: {}", src.mode, parts.join(", "))
            }
        }
        ("capture", "git.commit") => {
            let files = m.get("files_tracked").and_then(|v| v.as_i64()).unwrap_or(0);
            format!("{} files tracked in git", files)
        }
        ("index", "measure.index") => {
            let docs = m
                .get("documents_embedded")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            format!("{} documents embedded", docs)
        }
        ("search", "measure.search") => {
            if let Some(p5) = m.get("p_at_5").and_then(|v| v.as_f64()) {
                let mrr = m.get("mrr").and_then(|v| v.as_f64());
                match mrr {
                    Some(mrr_val) => {
                        format!("{}: P@5={:.0}%, MRR={:.2}", src.mode, p5 * 100.0, mrr_val)
                    }
                    None => format!("{}: P@5={:.0}%", src.mode, p5 * 100.0),
                }
            } else if let Some(recall) = m.get("recall_at_5").and_then(|v| v.as_f64()) {
                format!("{}: Recall@5={:.0}%", src.mode, recall * 100.0)
            } else {
                format!("{}: metrics recorded", src.mode)
            }
        }
        ("believe", "beliefs") => {
            let total = m.get("total_beliefs").and_then(|v| v.as_i64()).unwrap_or(0);
            let grounded = m
                .get("grounded_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let floating = m
                .get("floating_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let avg_health = m.get("avg_health").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if floating > 0 {
                format!(
                    "{} beliefs, {} grounded, {} floating, avg health {:.2}",
                    total, grounded, floating, avg_health
                )
            } else {
                format!(
                    "{} beliefs, all grounded, avg health {:.2}",
                    total, avg_health
                )
            }
        }
        ("evolve", "session.ended") => {
            let sessions = m
                .get("total_sessions")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let commits = m.get("total_commits").and_then(|v| v.as_i64()).unwrap_or(0);
            let beliefs = m
                .get("total_beliefs_captured")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            format!(
                "{} {}, {} {}, {} {} captured",
                sessions,
                if sessions == 1 { "session" } else { "sessions" },
                commits,
                if commits == 1 { "commit" } else { "commits" },
                beliefs,
                if beliefs == 1 { "belief" } else { "beliefs" },
            )
        }
        _ => String::new(),
    }
}

// ============================================================================
// System View (--system)
// ============================================================================

fn render_system_view(conn: &Connection, report: &MeasureReport) -> Result<()> {
    println!("\n  Measurement System — Raw Metrics\n");
    println!(
        "  Total measure.* events: {}",
        report.total_measurement_events
    );
    println!();

    for verb_summary in &report.verbs {
        println!(
            "  === {} ({}) ===",
            verb_summary.verb.to_uppercase(),
            verb_summary.status
        );

        if verb_summary.sources.is_empty() {
            println!("    No data\n");
            continue;
        }

        for src in &verb_summary.sources {
            println!(
                "    Source: {} ({}:{}) — {} events",
                src.source_type, src.tool, src.mode, src.event_count
            );
            println!("    Latest: {}", src.timestamp);

            // Print all metrics
            if let Some(obj) = src.latest_metrics.as_object() {
                for (key, val) in obj {
                    println!("      {}: {}", key, val);
                }
            }
            println!();
        }

        // Show recent history for measure.* sources
        let event_type = format!("measure.{}", verb_summary.verb);
        let history = get_recent_history(conn, &event_type, 5)?;
        if !history.is_empty() {
            println!("    Recent history:");
            for entry in &history {
                println!(
                    "      {} {}:{} {}",
                    &entry.timestamp[..19],
                    entry.tool,
                    entry.mode,
                    format_metrics_inline(&entry.metrics)
                );
            }
            println!();
        }
    }

    Ok(())
}

// ============================================================================
// Verb Drill-Down (--verb <name>)
// ============================================================================

fn run_verb_drilldown(conn: &Connection, verb: &str, options: &MeasureOptions) -> Result<()> {
    // Validate verb
    if !patina::measure::VALID_VERBS.contains(&verb) {
        anyhow::bail!(
            "Unknown verb '{}'. Valid verbs: {}",
            verb,
            patina::measure::VALID_VERBS.join(", ")
        );
    }

    let event_type = format!("measure.{}", verb);
    let history = get_recent_history(conn, &event_type, 20)?;

    // Also get existing event data for this verb
    let existing = match verb {
        "believe" => get_believe_history(conn, 10)?,
        "evolve" => get_evolve_history(conn, 10)?,
        _ => Vec::new(),
    };

    if options.json {
        let output = serde_json::json!({
            "verb": verb,
            "measure_events": history,
            "existing_events": existing,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("\n  {} — Drill-Down\n", verb.to_uppercase());

    // Show current state from source tables (not events) for dual-source verbs
    if verb == "believe" {
        render_believe_current_state(conn);
    }

    if history.is_empty() && existing.is_empty() {
        if verb != "believe" {
            println!("  No measurement data for verb '{}'.\n", verb);
        }
        return Ok(());
    }

    if !history.is_empty() {
        println!("  measure.{} events ({} total):\n", verb, history.len());
        println!(
            "    {:<22} {:<12} {:<12} METRICS",
            "TIMESTAMP", "TOOL", "MODE"
        );
        println!(
            "    {:<22} {:<12} {:<12} ───────",
            "─────────", "────", "────"
        );
        for entry in &history {
            println!(
                "    {:<22} {:<12} {:<12} {}",
                &entry.timestamp[..19.min(entry.timestamp.len())],
                entry.tool,
                entry.mode,
                format_metrics_inline(&entry.metrics)
            );
        }
        println!();
    }

    if !existing.is_empty() {
        let source_name = match verb {
            "believe" => "belief creation by date",
            "evolve" => "session.ended",
            _ => "existing",
        };
        println!("  {} ({} shown):\n", source_name, existing.len());
        println!(
            "    {:<22} {:<12} {:<12} METRICS",
            "TIMESTAMP", "TOOL", "MODE"
        );
        println!(
            "    {:<22} {:<12} {:<12} ───────",
            "─────────", "────", "────"
        );
        for entry in &existing {
            println!(
                "    {:<22} {:<12} {:<12} {}",
                &entry.timestamp[..19.min(entry.timestamp.len())],
                entry.tool,
                entry.mode,
                format_metrics_inline(&entry.metrics)
            );
        }
        println!();
    }

    Ok(())
}

// ============================================================================
// History Queries
// ============================================================================

fn get_recent_history(
    conn: &Connection,
    event_type: &str,
    limit: usize,
) -> Result<Vec<HistoryEntry>> {
    let mut stmt = conn.prepare(
        r#"SELECT
            timestamp,
            json_extract(data, '$.tool') as tool,
            json_extract(data, '$.mode') as mode,
            json_extract(data, '$.metrics') as metrics
        FROM eventlog
        WHERE event_type = ?1
        ORDER BY seq DESC
        LIMIT ?2"#,
    )?;

    let entries: Vec<HistoryEntry> = stmt
        .query_map(rusqlite::params![event_type, limit as i64], |row| {
            let metrics_str: String = row.get(3)?;
            let metrics: serde_json::Value =
                serde_json::from_str(&metrics_str).unwrap_or(serde_json::Value::Null);
            Ok(HistoryEntry {
                timestamp: row.get(0)?,
                tool: row.get(1)?,
                mode: row.get(2)?,
                metrics,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

fn get_believe_history(conn: &Connection, limit: usize) -> Result<Vec<HistoryEntry>> {
    // Get distinct scrape runs for belief.surface (grouped by timestamp prefix)
    let mut stmt = conn.prepare(
        r#"SELECT
            MAX(timestamp) as latest_ts,
            COUNT(*) as belief_count,
            COALESCE(SUM(CASE WHEN json_extract(data, '$.metrics.grounding.score') = 0 THEN 1 ELSE 0 END), 0) as floating,
            COALESCE(AVG(json_extract(data, '$.metrics.truth.evidence_count')), 0) as avg_evidence
        FROM eventlog
        WHERE event_type = 'belief.surface'
        GROUP BY SUBSTR(timestamp, 1, 16)
        ORDER BY latest_ts DESC
        LIMIT ?1"#,
    )?;

    let entries: Vec<HistoryEntry> = stmt
        .query_map([limit as i64], |row| {
            Ok(HistoryEntry {
                timestamp: row.get(0)?,
                tool: "scrape".to_string(),
                mode: "beliefs".to_string(),
                metrics: serde_json::json!({
                    "beliefs": row.get::<_, i64>(1)?,
                    "floating": row.get::<_, i64>(2)?,
                    "avg_evidence": (row.get::<_, f64>(3)? * 100.0).round() / 100.0,
                }),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

fn get_evolve_history(conn: &Connection, limit: usize) -> Result<Vec<HistoryEntry>> {
    let mut stmt = conn.prepare(
        r#"SELECT
            timestamp,
            COALESCE(json_extract(data, '$.commits_made'), 0) as commits,
            COALESCE(json_extract(data, '$.files_changed'), 0) as files,
            COALESCE(json_extract(data, '$.beliefs_captured'), 0) as beliefs,
            COALESCE(json_extract(data, '$.patterns_modified'), 0) as patterns
        FROM eventlog
        WHERE event_type = 'session.ended'
        ORDER BY seq DESC
        LIMIT ?1"#,
    )?;

    let entries: Vec<HistoryEntry> = stmt
        .query_map([limit as i64], |row| {
            Ok(HistoryEntry {
                timestamp: row.get(0)?,
                tool: "session".to_string(),
                mode: "lifecycle".to_string(),
                metrics: serde_json::json!({
                    "commits": row.get::<_, i64>(1)?,
                    "files": row.get::<_, i64>(2)?,
                    "beliefs": row.get::<_, i64>(3)?,
                    "patterns": row.get::<_, i64>(4)?,
                }),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

// ============================================================================
// Rendering Helpers
// ============================================================================

fn render_believe_current_state(conn: &Connection) {
    let result = conn.query_row(
        r#"SELECT
            COUNT(*) as total,
            COALESCE(SUM(CASE WHEN grounding_score = 0 THEN 1 ELSE 0 END), 0) as floating,
            COALESCE(SUM(CASE WHEN grounding_score > 0 THEN 1 ELSE 0 END), 0) as grounded,
            COALESCE(AVG(health_score), 0) as avg_health
        FROM beliefs"#,
        [],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, f64>(3)?,
            ))
        },
    );

    if let Ok((total, floating, grounded, avg_health)) = result {
        if total > 0 {
            println!("  Current state (beliefs table):");
            println!(
                "    {} beliefs, {} grounded, {} floating, avg health {:.2}\n",
                total, grounded, floating, avg_health
            );
        }
    }
}

fn print_empty_state() {
    println!("\n  No measurements recorded yet.\n");
    println!("  Getting started:");
    println!("    1. patina scrape    — capture code and belief metrics");
    println!("    2. patina oxidize   — build embeddings (index metrics)");
    println!("    3. patina eval      — evaluate search quality");
    println!("    4. patina measure   — see project health here\n");
}

fn format_age(timestamp: &str) -> String {
    // Try RFC3339 first (full datetime with timezone)
    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(ts);
        return if duration.num_days() > 0 {
            format!("{}d ago", duration.num_days())
        } else if duration.num_hours() > 0 {
            format!("{}h ago", duration.num_hours())
        } else {
            format!("{}m ago", duration.num_minutes())
        };
    }

    // Fall back to date-only (YYYY-MM-DD) — used by belief.surface
    if let Ok(date) = chrono::NaiveDate::parse_from_str(timestamp, "%Y-%m-%d") {
        let today = chrono::Utc::now().date_naive();
        let days = (today - date).num_days();
        return if days == 0 {
            "today".to_string()
        } else {
            format!("{}d ago", days)
        };
    }

    "unknown".to_string()
}

fn format_metrics_inline(metrics: &serde_json::Value) -> String {
    let Some(obj) = metrics.as_object() else {
        return metrics.to_string();
    };

    obj.iter()
        .map(|(k, v)| {
            if let Some(f) = v.as_f64() {
                if f == f.floor() && f.abs() < 1_000_000.0 {
                    format!("{}={}", k, f as i64)
                } else {
                    format!("{}={:.3}", k, f)
                }
            } else {
                format!("{}={}", k, v)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ============================================================================
// MCP Support
// ============================================================================

/// Generate JSON health summary for MCP tool
pub fn mcp_measure() -> Result<serde_json::Value> {
    let db_path = Path::new(eventlog::PATINA_DB);
    if !db_path.exists() {
        return Ok(serde_json::json!({
            "status": "no_data",
            "message": "No measurements recorded yet. Run patina scrape, oxidize, and eval first."
        }));
    }

    let conn = Connection::open(db_path).context("Failed to open patina.db")?;

    let total_events = count_measurement_events(&conn)?;
    let has_existing = has_existing_measurement_events(&conn)?;

    if total_events == 0 && !has_existing {
        return Ok(serde_json::json!({
            "status": "no_data",
            "message": "No measurements recorded yet. Run patina scrape, oxidize, and eval first."
        }));
    }

    let report = build_report(&conn)?;
    Ok(serde_json::to_value(&report)?)
}
