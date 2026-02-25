//! Measurement emission for compiled-in core tools.
//!
//! This module is the core-side equivalent of the WIT `patina:host/measure`
//! interface. Core tools call `emit()` to write measurement events to the
//! eventlog. WASM plugins use the WIT interface instead.
//!
//! Both paths produce identical event schemas in the eventlog.

use anyhow::Result;
use rusqlite::Connection;

/// Valid protocol verbs for measurement events.
pub const VALID_VERBS: &[&str] = &["capture", "index", "search", "believe", "evolve"];

/// Emit a measurement event to the eventlog.
///
/// Core tools call this after computing metrics. The event lands in the
/// eventlog with `event_type = "measure.<verb>"` and `source = "core"`.
///
/// # Arguments
/// - `conn` — open connection to patina.db (caller manages lifecycle)
/// - `verb` — protocol verb: capture, index, search, believe, evolve
/// - `tool` — tool name: eval, bench, scrape, oxidize, etc.
/// - `mode` — tool-specific sub-mode: nl, feedback, ablation, etc.
/// - `metrics` — JSON object with numeric values
///
/// # Errors
/// Returns error if verb is invalid or eventlog write fails.
pub fn emit(
    conn: &Connection,
    verb: &str,
    tool: &str,
    mode: &str,
    metrics: &serde_json::Value,
) -> Result<()> {
    anyhow::ensure!(
        VALID_VERBS.contains(&verb),
        "invalid verb '{}': must be one of {:?}",
        verb,
        VALID_VERBS
    );

    let event_type = format!("measure.{}", verb);
    let source_id = format!("{}:{}", tool, mode);
    let timestamp = chrono::Utc::now().to_rfc3339();

    let data = serde_json::json!({
        "verb": verb,
        "tool": tool,
        "mode": mode,
        "metrics": metrics,
        "source": "core",
    });

    crate::eventlog::insert_event(
        conn,
        &event_type,
        &timestamp,
        &source_id,
        None,
        &data.to_string(),
    )?;

    Ok(())
}
