//! Shared measure vocabulary for all actors in the Patina workspace.
//!
//! Provides the canonical event envelope (`MeasureEvent`), verb vocabulary
//! (`VALID_VERBS`), and validation (`validate`). Each actor — Mother, children,
//! plugins — uses these types to emit telemetry to their own local storage.
//! Per [[telemetry-is-process-owned]].
//!
//! # Registered tools and modes
//!
//! | Tool     | Modes                                       | Owner          |
//! |----------|---------------------------------------------|----------------|
//! | scrape   | code, git, beliefs, layer, structure        | core           |
//! | oxidize  | build, train                                | core           |
//! | eval     | dimension, feedback, nl                     | core           |
//! | belief   | audit                                       | core           |
//! | session  | lifecycle                                   | core           |
//! | pipe     | http                                        | core + children|
//! | hook     | (hook name, e.g. post-commit)               | core           |
//! | bench    | (query set names)                           | core           |
//! | doctor   | (tbd)                                       | core           |
//! | lake     | ingest, cursor, error                       | children       |
//!
//! New tools require updating this vocabulary. Modes are tool-scoped and
//! documented per tool — add new modes to the table above.

/// Valid protocol verbs for measurement events.
///
/// These are the only verbs accepted by `validate()`. The vocabulary is
/// intentionally small and stable:
/// - `capture` — data acquisition (scrape, pipe/http, lake ingest)
/// - `index` — embedding and index construction (oxidize)
/// - `search` — query execution (eval, bench)
/// - `believe` — belief system operations (belief audit)
/// - `evolve` — schema or model evolution
pub const VALID_VERBS: &[&str] = &["capture", "index", "search", "believe", "evolve"];

/// Registered tools in the measure vocabulary.
///
/// New tools require updating this list. Children cannot define new tools
/// without updating the shared vocabulary. See module-level docs for the
/// full tool/mode/owner table.
pub const REGISTERED_TOOLS: &[&str] = &[
    "scrape", "oxidize", "eval", "belief", "session", "pipe", "hook", "bench", "doctor", "lake",
];

/// Canonical measure event envelope.
///
/// Every measure event — from core, plugins, or children — uses this shape.
/// Storage is local to each actor; the shared envelope enables cross-store
/// queries with consistent field names.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MeasureEvent {
    /// Schema version for forward compatibility. Current: 1.
    pub schema_version: u32,
    /// Protocol verb — must be in `VALID_VERBS`.
    pub verb: String,
    /// What measured — the subsystem that produced the data (e.g. scrape, pipe, lake).
    pub tool: String,
    /// Sub-category within the tool (e.g. code, http, ingest).
    pub mode: String,
    /// JSON object with numeric metric values.
    pub metrics: serde_json::Value,
    /// RFC 3339 timestamp.
    pub timestamp: String,
    /// Who emitted — "core", "plugin:\<name\>", or "child:\<name\>".
    pub source: String,
}

/// Validate a measure event.
///
/// Checks:
/// - verb is in `VALID_VERBS`
/// - tool is in `REGISTERED_TOOLS`
/// - mode is non-empty
/// - source is non-empty and has a recognized prefix
/// - metrics is a JSON object
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

    // Source must be "core", "plugin:<name>", or "child:<name>"
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(verb: &str, tool: &str, mode: &str, source: &str) -> MeasureEvent {
        MeasureEvent {
            schema_version: 1,
            verb: verb.to_string(),
            tool: tool.to_string(),
            mode: mode.to_string(),
            metrics: serde_json::json!({"count": 42}),
            timestamp: "2026-03-10T00:00:00Z".to_string(),
            source: source.to_string(),
        }
    }

    #[test]
    fn valid_verbs_accepted() {
        for verb in VALID_VERBS {
            let event = make_event(verb, "scrape", "code", "core");
            assert!(validate(&event).is_ok(), "verb '{}' should be valid", verb);
        }
    }

    #[test]
    fn invalid_verb_rejected() {
        let event = make_event("destroy", "scrape", "code", "core");
        let err = validate(&event).unwrap_err();
        assert!(err.contains("invalid verb"));
        assert!(err.contains("destroy"));
    }

    #[test]
    fn unregistered_tool_rejected() {
        let event = make_event("capture", "invented-tool", "unit", "core");
        let err = validate(&event).unwrap_err();
        assert!(err.contains("invalid tool"));
        assert!(err.contains("invented-tool"));
    }

    #[test]
    fn empty_tool_rejected() {
        let event = make_event("capture", "", "unit", "core");
        assert!(validate(&event).unwrap_err().contains("invalid tool"));
    }

    #[test]
    fn registered_tools_accepted() {
        for tool in REGISTERED_TOOLS {
            let event = make_event("capture", tool, "unit", "core");
            assert!(validate(&event).is_ok(), "tool '{}' should be valid", tool);
        }
    }

    #[test]
    fn empty_mode_rejected() {
        let event = make_event("capture", "scrape", "", "core");
        assert!(validate(&event)
            .unwrap_err()
            .contains("mode must not be empty"));
    }

    #[test]
    fn empty_source_rejected() {
        let event = make_event("capture", "scrape", "code", "");
        assert!(validate(&event)
            .unwrap_err()
            .contains("source must not be empty"));
    }

    #[test]
    fn source_core_accepted() {
        let event = make_event("capture", "scrape", "code", "core");
        assert!(validate(&event).is_ok());
    }

    #[test]
    fn source_plugin_accepted() {
        let event = make_event("capture", "scrape", "code", "plugin:github-connector");
        assert!(validate(&event).is_ok());
    }

    #[test]
    fn source_child_accepted() {
        let event = make_event("capture", "lake", "ingest", "child:ducklake");
        assert!(validate(&event).is_ok());
    }

    #[test]
    fn source_unknown_prefix_rejected() {
        let event = make_event("capture", "scrape", "code", "rogue:attacker");
        let err = validate(&event).unwrap_err();
        assert!(err.contains("invalid source"));
    }

    #[test]
    fn metrics_must_be_object() {
        let mut event = make_event("capture", "scrape", "code", "core");
        event.metrics = serde_json::json!([1, 2, 3]);
        assert!(validate(&event)
            .unwrap_err()
            .contains("metrics must be a JSON object"));
    }

    #[test]
    fn metrics_object_accepted() {
        let event = make_event("capture", "scrape", "code", "core");
        assert!(validate(&event).is_ok());
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let event = make_event("index", "oxidize", "build", "core");
        let json = serde_json::to_string(&event).unwrap();
        let back: MeasureEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.verb, "index");
        assert_eq!(back.tool, "oxidize");
        assert_eq!(back.source, "core");
        assert_eq!(back.schema_version, 1);
    }
}
