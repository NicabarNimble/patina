//! Measurement emission for compiled-in core tools.
//!
//! This module is the core-side equivalent of the WIT `patina:host/measure`
//! interface. Core tools call `emit()` to write measurement events to the
//! eventlog. WASM plugins use the WIT interface instead.
//!
//! Both paths produce identical event schemas in the eventlog.

use anyhow::Result;

/// Valid protocol verbs for measurement events.
pub const VALID_VERBS: &[&str] = &["capture", "index", "search", "believe", "evolve"];

/// Emit a measurement event to events.db.
///
/// Core tools call this after computing metrics. The event lands in
/// events.db with `event_type = "measure.<verb>"` and `source = "core"`.
///
/// # Arguments
/// - `verb` — protocol verb: capture, index, search, believe, evolve
/// - `tool` — tool name: eval, bench, scrape, oxidize, etc.
/// - `mode` — tool-specific sub-mode: nl, feedback, ablation, etc.
/// - `metrics` — JSON object with numeric values
///
/// # Errors
/// Returns error if verb is invalid or eventlog write fails.
pub fn emit(
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

    let conn = crate::eventlog::open_events_db()?;
    crate::eventlog::insert_event(
        &conn,
        &event_type,
        &timestamp,
        &source_id,
        None,
        &data.to_string(),
    )?;

    Ok(())
}
