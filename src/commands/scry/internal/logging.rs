//! Query logging for feedback loop analysis
//!
//! Logs scry queries, usage, and feedback to the eventlog for later analysis.
//! Best-effort logging - failures are silently ignored to not disrupt scry.

use std::sync::Mutex;

use anyhow::Result;
use rusqlite::Connection;

use patina::eventlog;

use super::super::ScryResult;

/// Edge info for routing context logging
#[derive(Debug, Clone)]
pub struct EdgeInfo {
    pub id: i64,
    pub from_node: String,
    pub to_node: String,
    pub edge_type: String,
    pub weight: f32,
}

/// Routing context captured during graph-based routing
#[derive(Debug, Clone, Default)]
pub struct RoutingContext {
    /// Routing strategy used ("graph" or "all")
    pub strategy: String,
    /// Source project for the query
    pub source_project: String,
    /// Edges that contributed to routing decisions
    pub edges_used: Vec<EdgeInfo>,
    /// Repos that were actually searched
    pub repos_searched: Vec<String>,
    /// Total repos available
    pub repos_available: usize,
    /// Whether domain filtering was applied
    pub domain_filter_applied: bool,
}

/// Last query ID for use by scry open/copy/feedback commands
pub static LAST_QUERY_ID: Mutex<Option<String>> = Mutex::new(None);

/// Generate a query ID in the format: q_YYYYMMDD_HHMMSS_xxx
pub fn generate_query_id() -> String {
    let now = chrono::Utc::now();
    let random_suffix: String = (0..3)
        .map(|_| (b'a' + fastrand::u8(0..26)) as char)
        .collect();
    format!("q_{}_{}", now.format("%Y%m%d_%H%M%S"), random_suffix)
}

/// Get the active session ID from .patina/local/active-session.md
///
/// Returns None if no active session or file doesn't exist.
/// This is best-effort - we don't want to fail scry if session detection fails.
/// Supports both YAML frontmatter (new) and `**ID**: value` (legacy) formats.
pub fn get_active_session_id() -> Option<String> {
    let content = std::fs::read_to_string(".patina/local/active-session.md").ok()?;

    // Try YAML frontmatter first (new format: id: field between --- markers)
    // Only match top-level id: (no leading whitespace) to avoid nested fields
    if let Some(after_start) = content.strip_prefix("---") {
        if let Some(end) = after_start.find("---") {
            let frontmatter = &after_start[..end];
            for line in frontmatter.lines() {
                if let Some(value) = line.strip_prefix("id:") {
                    return Some(value.trim().to_string());
                }
            }
        }
    }

    // Fall back to legacy format
    for line in content.lines() {
        if line.starts_with("**ID**:") {
            return Some(line.replace("**ID**:", "").trim().to_string());
        }
    }
    None
}

/// Log a scry query to the eventlog for feedback loop analysis
///
/// Best-effort logging - failures are silently ignored to not disrupt scry.
/// Returns the query_id for reference by open/copy/feedback commands.
pub fn log_scry_query(query: &str, mode: &str, results: &[ScryResult]) -> Option<String> {
    let session_id = get_active_session_id()?;

    let query_id = generate_query_id();

    // Build results array for logging
    let results_json: Vec<serde_json::Value> = results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            serde_json::json!({
                "doc_id": r.source_id,
                "score": r.score,
                "rank": i + 1,
                "event_type": r.event_type
            })
        })
        .collect();

    let query_data = serde_json::json!({
        "query": query,
        "query_id": query_id,
        "mode": mode,
        "session_id": session_id,
        "results": results_json
    });

    // Best-effort insert into eventlog
    let insert_result = (|| -> Result<()> {
        let conn = Connection::open(eventlog::PATINA_DB)?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        eventlog::insert_event(
            &conn,
            "scry.query",
            &timestamp,
            &query_id, // Use query_id as source_id for lookup
            None,
            &query_data.to_string(),
        )?;
        Ok(())
    })();

    if insert_result.is_ok() {
        // Store as last query for open/copy/feedback without explicit query_id
        if let Ok(mut last) = LAST_QUERY_ID.lock() {
            *last = Some(query_id.clone());
        }
        Some(query_id)
    } else {
        None
    }
}

/// Result with source repo for routing-aware logging
#[derive(Debug, Clone)]
pub struct RoutedResult {
    pub source_repo: String,
    pub weight: f32,
    pub result: ScryResult,
}

/// Log a scry query with routing context for feedback loop analysis
///
/// Extended version of log_scry_query that includes graph routing metadata.
/// Returns the query_id for reference by open/copy/feedback commands.
pub fn log_scry_query_with_routing(
    query: &str,
    results: &[RoutedResult],
    routing: &RoutingContext,
) -> Option<String> {
    let session_id = get_active_session_id()?;

    let query_id = generate_query_id();

    // Build results array with source repo info
    let results_json: Vec<serde_json::Value> = results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            serde_json::json!({
                "doc_id": r.result.source_id,
                "score": r.result.score,
                "rank": i + 1,
                "event_type": r.result.event_type,
                "source_repo": r.source_repo,
                "weight": r.weight
            })
        })
        .collect();

    // Build edges array
    let edges_json: Vec<serde_json::Value> = routing
        .edges_used
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "from": e.from_node,
                "to": e.to_node,
                "type": e.edge_type,
                "weight": e.weight
            })
        })
        .collect();

    let query_data = serde_json::json!({
        "query": query,
        "query_id": query_id,
        "mode": "graph",
        "session_id": session_id,
        "routing": {
            "strategy": routing.strategy,
            "source_project": routing.source_project,
            "edges_used": edges_json,
            "repos_searched": routing.repos_searched,
            "repos_available": routing.repos_available,
            "domain_filter_applied": routing.domain_filter_applied
        },
        "results": results_json
    });

    // Best-effort insert into eventlog
    let insert_result = (|| -> Result<()> {
        let conn = Connection::open(eventlog::PATINA_DB)?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        eventlog::insert_event(
            &conn,
            "scry.query",
            &timestamp,
            &query_id,
            None,
            &query_data.to_string(),
        )?;
        Ok(())
    })();

    if insert_result.is_ok() {
        // Store as last query for open/copy/feedback without explicit query_id
        if let Ok(mut last) = LAST_QUERY_ID.lock() {
            *last = Some(query_id.clone());
        }
        Some(query_id)
    } else {
        None
    }
}

/// Log usage of a scry result (scry.use event)
///
/// Called by scry open, scry copy, and MCP callback.
/// Also marks edge_usage as useful for graph routing feedback loop (G2.5).
pub fn log_scry_use(query_id: &str, doc_id: &str, rank: usize) {
    let session_id = get_active_session_id();

    let use_data = serde_json::json!({
        "query_id": query_id,
        "result_used": doc_id,
        "rank": rank,
        "session_id": session_id
    });

    // Best-effort insert
    let _ = (|| -> Result<()> {
        let conn = Connection::open(eventlog::PATINA_DB)?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        eventlog::insert_event(
            &conn,
            "scry.use",
            &timestamp,
            query_id,
            None,
            &use_data.to_string(),
        )?;
        Ok(())
    })();

    // Mark edge_usage as useful for feedback loop (G2.5)
    // Best-effort - don't fail if this doesn't work
    let _ = mark_edge_usage_from_query(query_id, rank);
}

/// Mark edge_usage as useful based on query results
///
/// Looks up the query's results to find the source_repo for the used result,
/// then marks the corresponding edge_usage record as useful.
fn mark_edge_usage_from_query(query_id: &str, rank: usize) -> Result<()> {
    use patina::mother::Graph;

    // Look up the query from eventlog
    let conn = Connection::open(eventlog::PATINA_DB)?;
    let data: String = conn.query_row(
        "SELECT data FROM eventlog WHERE event_type = 'scry.query' AND source_id = ?",
        [query_id],
        |row| row.get(0),
    )?;

    let parsed: serde_json::Value = serde_json::from_str(&data)?;

    // Check if this was a graph-routed query (has routing context)
    if parsed.get("routing").is_none() {
        return Ok(()); // Not a graph-routed query, nothing to update
    }

    // Find the result at the given rank and get its source_repo
    let results = parsed["results"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No results in query"))?;

    let result = results
        .iter()
        .find(|r| r["rank"].as_u64() == Some(rank as u64))
        .ok_or_else(|| anyhow::anyhow!("Result not found at rank {}", rank))?;

    let source_repo = result["source_repo"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No source_repo in result"))?;

    // Mark edge_usage as useful
    let graph = Graph::open()?;
    graph.mark_usage_useful(query_id, source_repo)?;

    Ok(())
}

/// Log explicit feedback on a scry result (scry.feedback event)
pub fn log_scry_feedback(query_id: &str, signal: &str, comment: Option<&str>) {
    let session_id = get_active_session_id();

    let feedback_data = serde_json::json!({
        "query_id": query_id,
        "signal": signal,
        "comment": comment,
        "session_id": session_id
    });

    // Best-effort insert
    let _ = (|| -> Result<()> {
        let conn = Connection::open(eventlog::PATINA_DB)?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        eventlog::insert_event(
            &conn,
            "scry.feedback",
            &timestamp,
            query_id,
            None,
            &feedback_data.to_string(),
        )?;
        Ok(())
    })();
}

/// Get results from a previous query by query_id
pub fn get_query_results(query_id: &str) -> Result<Vec<(String, f32)>> {
    let conn = Connection::open(eventlog::PATINA_DB)?;

    let data: String = conn.query_row(
        "SELECT data FROM eventlog WHERE event_type = 'scry.query' AND source_id = ?",
        [query_id],
        |row| row.get(0),
    )?;

    let parsed: serde_json::Value = serde_json::from_str(&data)?;
    let results = parsed["results"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No results in query"))?;

    Ok(results
        .iter()
        .map(|r| {
            (
                r["doc_id"].as_str().unwrap_or("").to_string(),
                r["score"].as_f64().unwrap_or(0.0) as f32,
            )
        })
        .collect())
}
