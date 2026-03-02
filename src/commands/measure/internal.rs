//! Internal implementation for the measure command
//!
//! Queries the eventlog for measurement data from two sources:
//! - measure.* events (new, from Phase 1-2 tools)
//! - Existing typed events that carry measurement data

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
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
    pub latest_metrics: VerbMetrics,
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
    metrics: VerbMetrics,
}

// ============================================================================
// Typed Metric Variants
// ============================================================================

/// Typed metrics for each verb — replaces serde_json::Value as domain state.
///
/// Serializes flat (untagged) so JSON output shape is preserved.
/// Does NOT derive Deserialize — use `from_db()` for manual dispatch (ADR-1).
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum VerbMetrics {
    CaptureCode(CaptureCodeMetrics),
    CaptureGit(CaptureGitMetrics),
    CaptureBeliefs(CaptureBeliefsMetrics),
    CaptureLayer(CaptureLayerMetrics),
    CaptureGitScrape(CaptureGitScrapeMetrics),
    CaptureHealthCheck(CaptureHealthCheckMetrics),
    Index(IndexMetrics),
    Search(SearchMetrics),
    Believe(BelieveMetrics),
    Evolve(EvolveMetrics),
    BelieveHistory(BelieveHistoryMetrics),
    EvolveHistory(EvolveHistoryMetrics),
    /// Fallback for unrecognized metric shapes — preserves data, logs warning.
    Raw(serde_json::Value),
}

impl VerbMetrics {
    /// Parse metrics JSON at the DB boundary, dispatching to the correct typed
    /// struct based on verb and mode context from the same DB row (ADR-1).
    pub fn from_db(verb: &str, mode: &str, json_str: &str) -> Self {
        let result = match (verb, mode) {
            ("capture", "code") => {
                serde_json::from_str::<CaptureCodeMetrics>(json_str).map(VerbMetrics::CaptureCode)
            }
            ("capture", "beliefs") => {
                serde_json::from_str::<CaptureBeliefsMetrics>(json_str)
                    .map(VerbMetrics::CaptureBeliefs)
            }
            ("capture", "layer") => {
                serde_json::from_str::<CaptureLayerMetrics>(json_str)
                    .map(VerbMetrics::CaptureLayer)
            }
            ("capture", "git") => {
                serde_json::from_str::<CaptureGitScrapeMetrics>(json_str)
                    .map(VerbMetrics::CaptureGitScrape)
            }
            ("capture", "health-check") => {
                serde_json::from_str::<CaptureHealthCheckMetrics>(json_str)
                    .map(VerbMetrics::CaptureHealthCheck)
            }
            ("capture", _) => {
                tracing::warn!(mode, "Unknown capture mode — falling back to raw metrics");
                let value =
                    serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null);
                return VerbMetrics::Raw(value);
            }
            ("index", _) => serde_json::from_str::<IndexMetrics>(json_str).map(VerbMetrics::Index),
            ("search", _) => {
                serde_json::from_str::<SearchMetrics>(json_str).map(VerbMetrics::Search)
            }
            ("believe", _) => {
                serde_json::from_str::<BelieveMetrics>(json_str).map(VerbMetrics::Believe)
            }
            ("evolve", _) => {
                serde_json::from_str::<EvolveMetrics>(json_str).map(VerbMetrics::Evolve)
            }
            _ => {
                tracing::warn!(verb, mode, "Unknown verb — falling back to raw metrics");
                let value = serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null);
                return VerbMetrics::Raw(value);
            }
        };

        result.unwrap_or_else(|e| {
            tracing::warn!(verb, mode, error = %e, "Falling back to raw metrics");
            let value = serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null);
            VerbMetrics::Raw(value)
        })
    }

    /// Return key-value pairs for generic display. Units standardized:
    /// `ms` suffix for durations, `%` for rates, plain integers for counts.
    pub fn format_kv(&self) -> Vec<(String, String)> {
        match self {
            VerbMetrics::CaptureCode(m) => vec![
                ("files_parsed".into(), m.files_parsed.to_string()),
                ("functions_found".into(), m.functions_found.to_string()),
                ("types_found".into(), m.types_found.to_string()),
                ("fts_symbols".into(), m.fts_symbols.to_string()),
            ],
            VerbMetrics::CaptureGit(m) => vec![
                ("files_tracked".into(), m.files_tracked.to_string()),
                ("total_commits".into(), m.total_commits.to_string()),
            ],
            VerbMetrics::CaptureBeliefs(m) => vec![
                ("beliefs_processed".into(), m.beliefs_processed.to_string()),
                ("beliefs_verified".into(), m.beliefs_verified.to_string()),
                ("beliefs_skipped".into(), m.beliefs_skipped.to_string()),
                ("supports_edges".into(), m.supports_edges.to_string()),
                ("attacks_edges".into(), m.attacks_edges.to_string()),
                ("values_processed".into(), m.values_processed.to_string()),
                ("duration_ms".into(), format!("{}ms", m.duration_ms)),
            ],
            VerbMetrics::CaptureLayer(m) => vec![
                ("patterns_processed".into(), m.patterns_processed.to_string()),
                ("sessions_processed".into(), m.sessions_processed.to_string()),
                ("duration_ms".into(), format!("{}ms", m.duration_ms)),
            ],
            VerbMetrics::CaptureGitScrape(m) => vec![
                ("commits_processed".into(), m.commits_processed.to_string()),
                ("tracked_files".into(), m.tracked_files.to_string()),
                ("tags_indexed".into(), m.tags_indexed.to_string()),
                ("co_change_pairs".into(), m.co_change_pairs.to_string()),
                ("duration_ms".into(), format!("{}ms", m.duration_ms)),
            ],
            VerbMetrics::CaptureHealthCheck(m) => vec![
                ("beliefs".into(), m.beliefs.to_string()),
                ("sessions".into(), m.sessions.to_string()),
                ("layer_patterns".into(), m.layer_patterns.to_string()),
                ("missing_tools".into(), m.missing_tools.to_string()),
                ("new_tools".into(), m.new_tools.to_string()),
            ],
            VerbMetrics::Index(m) => vec![
                ("documents_embedded".into(), m.documents_embedded.to_string()),
            ],
            VerbMetrics::Search(m) => {
                let mut pairs = Vec::new();
                if let Some(p5) = m.p_at_5 {
                    pairs.push(("p_at_5".into(), format!("{:.1}%", p5 * 100.0)));
                }
                if let Some(mrr) = m.mrr {
                    pairs.push(("mrr".into(), format!("{:.3}", mrr)));
                }
                if let Some(r5) = m.recall_at_5 {
                    pairs.push(("recall_at_5".into(), format!("{:.1}%", r5 * 100.0)));
                }
                pairs
            }
            VerbMetrics::Believe(m) => vec![
                ("total_beliefs".into(), m.total_beliefs.to_string()),
                ("floating_count".into(), m.floating_count.to_string()),
                ("grounded_count".into(), m.grounded_count.to_string()),
                ("avg_evidence".into(), format!("{:.2}", m.avg_evidence)),
                ("avg_health".into(), format!("{:.2}", m.avg_health)),
            ],
            VerbMetrics::Evolve(m) => vec![
                ("total_sessions".into(), m.total_sessions.to_string()),
                ("total_commits".into(), m.total_commits.to_string()),
                ("total_files_changed".into(), m.total_files_changed.to_string()),
                ("total_beliefs_captured".into(), m.total_beliefs_captured.to_string()),
                ("total_patterns_modified".into(), m.total_patterns_modified.to_string()),
            ],
            VerbMetrics::BelieveHistory(m) => vec![
                ("beliefs".into(), m.beliefs.to_string()),
                ("floating".into(), m.floating.to_string()),
                ("avg_evidence".into(), format!("{:.2}", m.avg_evidence)),
            ],
            VerbMetrics::EvolveHistory(m) => vec![
                ("commits".into(), m.commits.to_string()),
                ("files".into(), m.files.to_string()),
                ("beliefs".into(), m.beliefs.to_string()),
                ("patterns".into(), m.patterns.to_string()),
            ],
            VerbMetrics::Raw(v) => {
                if let Some(obj) = v.as_object() {
                    obj.iter()
                        .map(|(k, v)| {
                            let formatted = if let Some(f) = v.as_f64() {
                                if f == f.floor() && f.abs() < 1_000_000.0 {
                                    (f as i64).to_string()
                                } else {
                                    format!("{:.3}", f)
                                }
                            } else {
                                v.to_string()
                            };
                            (k.clone(), formatted)
                        })
                        .collect()
                } else {
                    vec![("value".into(), v.to_string())]
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureCodeMetrics {
    pub files_parsed: i64,
    pub functions_found: i64,
    pub types_found: i64,
    pub fts_symbols: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureGitMetrics {
    pub files_tracked: i64,
    pub total_commits: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureBeliefsMetrics {
    pub beliefs_processed: i64,
    pub beliefs_verified: i64,
    pub beliefs_skipped: i64,
    pub supports_edges: i64,
    pub attacks_edges: i64,
    pub values_processed: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureLayerMetrics {
    pub patterns_processed: i64,
    pub sessions_processed: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureGitScrapeMetrics {
    pub commits_processed: i64,
    pub tracked_files: i64,
    pub tags_indexed: i64,
    pub co_change_pairs: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureHealthCheckMetrics {
    pub beliefs: i64,
    pub sessions: i64,
    pub layer_patterns: i64,
    pub missing_tools: i64,
    pub new_tools: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexMetrics {
    pub documents_embedded: i64,
}

/// Search quality metrics. Option<f64> per ADR-2: missing != zero.
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchMetrics {
    pub p_at_5: Option<f64>,
    pub mrr: Option<f64>,
    pub recall_at_5: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BelieveMetrics {
    pub total_beliefs: i64,
    pub floating_count: i64,
    pub grounded_count: i64,
    pub avg_evidence: f64,
    pub avg_health: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvolveMetrics {
    pub total_sessions: i64,
    pub total_commits: i64,
    pub total_files_changed: i64,
    pub total_beliefs_captured: i64,
    pub total_patterns_modified: i64,
}

/// History-only metrics for believe drill-down. Field names are disjoint
/// from BelieveMetrics (beliefs vs total_beliefs) for unambiguous untagged serde.
#[derive(Debug, Serialize, Deserialize)]
pub struct BelieveHistoryMetrics {
    pub beliefs: i64,
    pub floating: i64,
    pub avg_evidence: f64,
}

/// History-only metrics for evolve drill-down. Field names are disjoint
/// from EvolveMetrics (commits vs total_commits) for unambiguous untagged serde.
#[derive(Debug, Serialize, Deserialize)]
pub struct EvolveHistoryMetrics {
    pub commits: i64,
    pub files: i64,
    pub beliefs: i64,
    pub patterns: i64,
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

    // Ensure events.db exists (migrates runtime events on first run)
    eventlog::ensure_events_db()?;

    // ATTACH events.db for cross-system queries (measure.* events live there)
    let events_path = Path::new(eventlog::EVENTS_DB);
    if events_path.exists() {
        conn.execute(
            "ATTACH DATABASE ?1 AS events",
            [events_path.to_str().unwrap_or(eventlog::EVENTS_DB)],
        )?;
    }

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
    // measure.* events are in events.db (attached as 'events')
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events.eventlog WHERE event_type LIKE 'measure.%'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
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
            latest_metrics: VerbMetrics::CaptureGit(CaptureGitMetrics {
                files_tracked: file_count,
                total_commits: git_count,
            }),
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
        // Check P@5 threshold if available (Option: None means not measured)
        let has_low_precision = sources.iter().any(|src| {
            if let VerbMetrics::Search(m) = &src.latest_metrics {
                matches!(m.p_at_5, Some(p5) if p5 < 0.4)
            } else {
                false
            }
        });
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

        // Get latest timestamp from belief.surface events, falling back to beliefs table
        let latest_ts: String = conn
            .query_row(
                "SELECT MAX(timestamp) FROM eventlog WHERE event_type = 'belief.surface'",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .unwrap_or(None)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                // Fallback: use beliefs table's last_activity or revised date
                conn.query_row(
                    "SELECT COALESCE(MAX(last_activity), MAX(revised), '') FROM beliefs",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap_or_default()
            });

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
                    latest_metrics: VerbMetrics::Believe(BelieveMetrics {
                        total_beliefs: total,
                        floating_count: floating,
                        grounded_count: total - floating,
                        avg_evidence: (avg_evidence * 100.0).round() / 100.0,
                        avg_health: (avg_health * 100.0).round() / 100.0,
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
            .find_map(|s| {
                if let VerbMetrics::Believe(m) = &s.latest_metrics {
                    if m.total_beliefs > 0 {
                        Some(m.floating_count as f64 / m.total_beliefs as f64)
                    } else {
                        None
                    }
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
                latest_metrics: VerbMetrics::Evolve(EvolveMetrics {
                    total_sessions: sessions,
                    total_commits: commits,
                    total_files_changed: files,
                    total_beliefs_captured: beliefs,
                    total_patterns_modified: patterns,
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

    // Get distinct tool:mode combinations with latest metrics (from events.db)
    let stmt_result = conn.prepare(
        r#"SELECT
            json_extract(data, '$.tool') as tool,
            json_extract(data, '$.mode') as mode,
            json_extract(data, '$.metrics') as metrics,
            timestamp,
            (SELECT COUNT(*) FROM events.eventlog e2
             WHERE e2.event_type = ?1
               AND json_extract(e2.data, '$.tool') = json_extract(events.eventlog.data, '$.tool')
               AND json_extract(e2.data, '$.mode') = json_extract(events.eventlog.data, '$.mode')
            ) as event_count
        FROM events.eventlog
        WHERE event_type = ?1
          AND seq IN (
            SELECT MAX(seq) FROM events.eventlog
            WHERE event_type = ?1
            GROUP BY json_extract(data, '$.tool'), json_extract(data, '$.mode')
          )
        ORDER BY timestamp DESC"#,
    );

    // If events.db isn't attached or has no data, return empty
    let mut stmt = match stmt_result {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

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
        let metrics = VerbMetrics::from_db(verb, &mode, &metrics_str);

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
            let summary = user_friendly_metrics(src);
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

fn user_friendly_metrics(src: &SourceSummary) -> String {
    match &src.latest_metrics {
        VerbMetrics::CaptureCode(m) => {
            format!(
                "{}: {} files, {} functions parsed",
                src.mode, m.files_parsed, m.functions_found
            )
        }
        VerbMetrics::CaptureGit(m) => {
            format!("{} files tracked in git", m.files_tracked)
        }
        VerbMetrics::CaptureBeliefs(m) => {
            format!(
                "{}: {} beliefs processed, {} verified, {}ms",
                src.mode, m.beliefs_processed, m.beliefs_verified, m.duration_ms
            )
        }
        VerbMetrics::CaptureLayer(m) => {
            format!(
                "{}: {} patterns, {} sessions, {}ms",
                src.mode, m.patterns_processed, m.sessions_processed, m.duration_ms
            )
        }
        VerbMetrics::CaptureGitScrape(m) => {
            format!(
                "{}: {} commits, {} files, {}ms",
                src.mode, m.commits_processed, m.tracked_files, m.duration_ms
            )
        }
        VerbMetrics::CaptureHealthCheck(m) => {
            format!(
                "{}: {} beliefs, {} sessions, {} patterns",
                src.mode, m.beliefs, m.sessions, m.layer_patterns
            )
        }
        VerbMetrics::Index(m) => {
            format!("{} documents embedded", m.documents_embedded)
        }
        VerbMetrics::Search(m) => match (m.p_at_5, m.mrr, m.recall_at_5) {
            (Some(p5), Some(mrr_val), _) => {
                format!("{}: P@5={:.0}%, MRR={:.2}", src.mode, p5 * 100.0, mrr_val)
            }
            (Some(p5), None, _) => {
                format!("{}: P@5={:.0}%", src.mode, p5 * 100.0)
            }
            (None, _, Some(recall)) => {
                format!("{}: Recall@5={:.0}%", src.mode, recall * 100.0)
            }
            _ => format!("{}: n/a", src.mode),
        },
        VerbMetrics::Believe(m) => {
            if m.floating_count > 0 {
                format!(
                    "{} beliefs, {} grounded, {} floating, avg health {:.2}",
                    m.total_beliefs, m.grounded_count, m.floating_count, m.avg_health
                )
            } else {
                format!(
                    "{} beliefs, all grounded, avg health {:.2}",
                    m.total_beliefs, m.avg_health
                )
            }
        }
        VerbMetrics::Evolve(m) => {
            format!(
                "{} {}, {} {}, {} {} captured",
                m.total_sessions,
                if m.total_sessions == 1 {
                    "session"
                } else {
                    "sessions"
                },
                m.total_commits,
                if m.total_commits == 1 {
                    "commit"
                } else {
                    "commits"
                },
                m.total_beliefs_captured,
                if m.total_beliefs_captured == 1 {
                    "belief"
                } else {
                    "beliefs"
                },
            )
        }
        VerbMetrics::BelieveHistory(_) | VerbMetrics::EvolveHistory(_) => String::new(),
        VerbMetrics::Raw(_) => String::new(),
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

            // Print all metrics via typed format_kv
            for (key, val) in src.latest_metrics.format_kv() {
                println!("      {}: {}", key, val);
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
    // measure.* events are in events.db
    let stmt_result = conn.prepare(
        r#"SELECT
            timestamp,
            json_extract(data, '$.tool') as tool,
            json_extract(data, '$.mode') as mode,
            json_extract(data, '$.metrics') as metrics
        FROM events.eventlog
        WHERE event_type = ?1
        ORDER BY seq DESC
        LIMIT ?2"#,
    );

    let mut stmt = match stmt_result {
        Ok(s) => s,
        Err(_) => return Ok(Vec::new()),
    };

    // Extract verb from event_type (e.g., "measure.capture" → "capture")
    let verb = event_type
        .strip_prefix("measure.")
        .unwrap_or(event_type);

    let entries: Vec<HistoryEntry> = stmt
        .query_map(rusqlite::params![event_type, limit as i64], |row| {
            let mode: String = row.get(2)?;
            let metrics_str: String = row.get(3)?;
            Ok(HistoryEntry {
                timestamp: row.get(0)?,
                tool: row.get(1)?,
                mode: mode.clone(),
                metrics: VerbMetrics::from_db(verb, &mode, &metrics_str),
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
                metrics: VerbMetrics::BelieveHistory(BelieveHistoryMetrics {
                    beliefs: row.get(1)?,
                    floating: row.get(2)?,
                    avg_evidence: (row.get::<_, f64>(3)? * 100.0).round() / 100.0,
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
                metrics: VerbMetrics::EvolveHistory(EvolveHistoryMetrics {
                    commits: row.get(1)?,
                    files: row.get(2)?,
                    beliefs: row.get(3)?,
                    patterns: row.get(4)?,
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

fn format_metrics_inline(metrics: &VerbMetrics) -> String {
    metrics
        .format_kv()
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
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

    // ATTACH events.db for cross-system queries
    let events_path = Path::new(eventlog::EVENTS_DB);
    if events_path.exists() {
        let _ = conn.execute(
            "ATTACH DATABASE ?1 AS events",
            [events_path.to_str().unwrap_or(eventlog::EVENTS_DB)],
        );
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_db_unknown_payload_returns_raw() {
        let unknown = r#"{"unknown_key": 42, "another": "value"}"#;

        // Verbs with required struct fields fall to Raw when payload doesn't match
        for (verb, mode) in &[
            ("capture", "code"),     // CaptureCodeMetrics has required i64 fields
            ("index", "default"),    // IndexMetrics has required documents_embedded
            ("believe", "audit"),    // BelieveMetrics has required i64/f64 fields
            ("evolve", "lifecycle"), // EvolveMetrics has required i64 fields
        ] {
            let result = VerbMetrics::from_db(verb, mode, unknown);
            assert!(
                matches!(result, VerbMetrics::Raw(_)),
                "Expected Raw for {}:{}, got {:?}",
                verb,
                mode,
                result
            );
        }

        // Typed capture modes with unknown payload also fall to Raw
        for (verb, mode) in &[
            ("capture", "beliefs"),
            ("capture", "layer"),
            ("capture", "git"),
            ("capture", "health-check"),
        ] {
            let result = VerbMetrics::from_db(verb, mode, unknown);
            assert!(
                matches!(result, VerbMetrics::Raw(_)),
                "Expected Raw for {}:{}, got {:?}",
                verb,
                mode,
                result
            );
        }

        // Unknown capture mode falls to Raw
        let result = VerbMetrics::from_db("capture", "unknown-mode", unknown);
        assert!(matches!(result, VerbMetrics::Raw(_)));

        // SearchMetrics accepts unknown JSON (all fields are Option, unknowns ignored)
        let result = VerbMetrics::from_db("search", "eval", unknown);
        assert!(matches!(result, VerbMetrics::Search(_)));

        // Unknown verb falls to Raw
        let result = VerbMetrics::from_db("unknown", "mode", unknown);
        assert!(matches!(result, VerbMetrics::Raw(_)));
    }

    #[test]
    fn from_db_raw_fallback_serializes_without_panic() {
        let unknown = r#"{"unexpected_field": 99}"#;
        // Use a verb with required fields to trigger Raw fallback
        let raw = VerbMetrics::from_db("index", "default", unknown);
        assert!(matches!(raw, VerbMetrics::Raw(_)));

        // Serialization must not panic and preserves all fields
        let json = serde_json::to_string(&raw).expect("Raw should serialize");
        assert!(json.contains("unexpected_field"));
        assert!(json.contains("99"));
    }

    #[test]
    fn from_db_valid_capture_code() {
        let json = r#"{"files_parsed": 248, "functions_found": 2427, "types_found": 989, "fts_symbols": 6164}"#;
        let result = VerbMetrics::from_db("capture", "code", json);
        assert!(matches!(result, VerbMetrics::CaptureCode(_)));

        if let VerbMetrics::CaptureCode(m) = result {
            assert_eq!(m.files_parsed, 248);
            assert_eq!(m.functions_found, 2427);
        }
    }

    #[test]
    fn from_db_valid_believe() {
        let json = r#"{"total_beliefs": 178, "floating_count": 135, "grounded_count": 43, "avg_evidence": 1.72, "avg_health": 0.88}"#;
        let result = VerbMetrics::from_db("believe", "beliefs", json);
        assert!(matches!(result, VerbMetrics::Believe(_)));
    }

    #[test]
    fn from_db_valid_evolve() {
        let json = r#"{"total_sessions": 675, "total_commits": 2543, "total_files_changed": 50221, "total_beliefs_captured": 247, "total_patterns_modified": 4222}"#;
        let result = VerbMetrics::from_db("evolve", "lifecycle", json);
        assert!(matches!(result, VerbMetrics::Evolve(_)));
    }

    #[test]
    fn from_db_capture_beliefs_typed() {
        let json = r#"{"beliefs_processed": 178, "beliefs_verified": 43, "beliefs_skipped": 0, "supports_edges": 96, "attacks_edges": 82, "values_processed": 10, "duration_ms": 1092}"#;
        let result = VerbMetrics::from_db("capture", "beliefs", json);
        assert!(matches!(result, VerbMetrics::CaptureBeliefs(_)));

        if let VerbMetrics::CaptureBeliefs(m) = result {
            assert_eq!(m.beliefs_processed, 178);
            assert_eq!(m.attacks_edges, 82);
            assert_eq!(m.duration_ms, 1092);
        }
    }

    #[test]
    fn from_db_capture_layer_typed() {
        let json = r#"{"patterns_processed": 12, "sessions_processed": 5, "duration_ms": 250}"#;
        let result = VerbMetrics::from_db("capture", "layer", json);
        assert!(matches!(result, VerbMetrics::CaptureLayer(_)));
    }

    #[test]
    fn from_db_capture_git_scrape_typed() {
        let json = r#"{"commits_processed": 100, "tracked_files": 248, "tags_indexed": 50, "co_change_pairs": 1200, "duration_ms": 3000}"#;
        let result = VerbMetrics::from_db("capture", "git", json);
        assert!(matches!(result, VerbMetrics::CaptureGitScrape(_)));
    }

    #[test]
    fn from_db_capture_health_check_typed() {
        let json = r#"{"beliefs": 178, "sessions": 42, "layer_patterns": 12, "missing_tools": 0, "new_tools": 1}"#;
        let result = VerbMetrics::from_db("capture", "health-check", json);
        assert!(matches!(result, VerbMetrics::CaptureHealthCheck(_)));
    }

    #[test]
    fn from_db_search_option_fields() {
        // All fields present
        let json = r#"{"p_at_5": 0.8, "mrr": 0.75, "recall_at_5": 0.9}"#;
        let result = VerbMetrics::from_db("search", "eval", json);
        if let VerbMetrics::Search(m) = result {
            assert_eq!(m.p_at_5, Some(0.8));
            assert_eq!(m.mrr, Some(0.75));
            assert_eq!(m.recall_at_5, Some(0.9));
        } else {
            panic!("Expected Search variant");
        }

        // Missing fields → None (not 0.0)
        let json = r#"{"p_at_5": 0.6}"#;
        let result = VerbMetrics::from_db("search", "eval", json);
        if let VerbMetrics::Search(m) = result {
            assert_eq!(m.p_at_5, Some(0.6));
            assert_eq!(m.mrr, None);
            assert_eq!(m.recall_at_5, None);
        } else {
            panic!("Expected Search variant");
        }
    }

    #[test]
    fn verb_metrics_untagged_serialization() {
        // Typed variant serializes flat (no wrapper)
        let m = VerbMetrics::CaptureGit(CaptureGitMetrics {
            files_tracked: 200,
            total_commits: 2892,
        });
        let json = serde_json::to_value(&m).unwrap();
        assert_eq!(json["files_tracked"], 200);
        assert_eq!(json["total_commits"], 2892);
        // No "CaptureGit" wrapper key
        assert!(json.get("CaptureGit").is_none());
    }
}
