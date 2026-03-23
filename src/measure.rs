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

/// Registered tools in the measure vocabulary.
pub const REGISTERED_TOOLS: &[&str] = &[
    "scrape", "oxidize", "eval", "belief", "session", "pipe", "hook", "bench", "doctor", "lake",
];

/// Canonical measure event envelope.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MeasureEvent {
    pub schema_version: u32,
    pub verb: String,
    pub tool: String,
    pub mode: String,
    pub metrics: serde_json::Value,
    pub timestamp: String,
    pub source: String,
}

/// Validate a measure event.
pub fn validate(event: &MeasureEvent) -> Result<(), String> {
    if !VALID_VERBS.contains(&event.verb.as_str()) {
        return Err(format!(
            "invalid verb '{}': must be one of {:?}",
            event.verb, VALID_VERBS
        ));
    }

    if !REGISTERED_TOOLS.contains(&event.tool.as_str()) {
        return Err(format!(
            "invalid tool '{}': must be one of {:?}",
            event.tool, REGISTERED_TOOLS
        ));
    }

    if event.mode.is_empty() {
        return Err("mode must not be empty".to_string());
    }

    if event.source.is_empty() {
        return Err("source must not be empty".to_string());
    }

    if event.source != "core"
        && !event.source.starts_with("plugin:")
        && !event.source.starts_with("child:")
    {
        return Err(format!(
            "invalid source '{}': must be 'core', 'plugin:<name>', or 'child:<name>'",
            event.source
        ));
    }

    if !event.metrics.is_object() {
        return Err("metrics must be a JSON object".to_string());
    }

    Ok(())
}

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
pub fn emit(verb: &str, tool: &str, mode: &str, metrics: &serde_json::Value) -> Result<()> {
    let timestamp = chrono::Utc::now().to_rfc3339();

    // Build and validate using the canonical envelope
    let event = MeasureEvent {
        schema_version: 1,
        verb: verb.to_string(),
        tool: tool.to_string(),
        mode: mode.to_string(),
        metrics: metrics.clone(),
        timestamp: timestamp.clone(),
        source: "core".to_string(),
    };
    validate(&event).map_err(|e| anyhow::anyhow!("measure validation: {}", e))?;

    let event_type = format!("measure.{}", verb);
    let source_id = format!("{}:{}", tool, mode);
    let data = serde_json::to_value(&event)?;

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

/// Emit a measurement event, warning on failure instead of silently dropping.
///
/// Use this at call sites where the operation should succeed but isn't worth
/// crashing for. The warning makes degradation visible.
pub fn emit_or_warn(verb: &str, tool: &str, mode: &str, metrics: &serde_json::Value) {
    if let Err(e) = emit(verb, tool, mode, metrics) {
        eprintln!("patina: warning: failed to record event: {e}");
    }
}
