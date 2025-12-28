//! Query logging for feedback loop analysis
//!
//! Logs scry queries, usage, and feedback to the eventlog for later analysis.
//! Best-effort logging - failures are silently ignored to not disrupt scry.

use std::sync::Mutex;

use anyhow::Result;
use rusqlite::Connection;

use crate::commands::scrape::database;

use super::super::ScryResult;

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

/// Get the active session ID from .claude/context/active-session.md
///
/// Returns None if no active session or file doesn't exist.
/// This is best-effort - we don't want to fail scry if session detection fails.
pub fn get_active_session_id() -> Option<String> {
    let content = std::fs::read_to_string(".claude/context/active-session.md").ok()?;
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
        let conn = Connection::open(database::PATINA_DB)?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        database::insert_event(
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

/// Log usage of a scry result (scry.use event)
///
/// Called by scry open, scry copy, and MCP callback.
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
        let conn = Connection::open(database::PATINA_DB)?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        database::insert_event(
            &conn,
            "scry.use",
            &timestamp,
            query_id,
            None,
            &use_data.to_string(),
        )?;
        Ok(())
    })();
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
        let conn = Connection::open(database::PATINA_DB)?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        database::insert_event(
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
    let conn = Connection::open(database::PATINA_DB)?;

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
