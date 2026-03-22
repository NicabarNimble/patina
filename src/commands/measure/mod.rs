//! Measurement consumer — project health from eventlog metrics
//!
//! Reads measurement data from two sources:
//! 1. `measure.*` events (from Phase 1-2 tools: eval, bench, scrape, oxidize, doctor)
//! 2. Existing typed events (belief.surface, session.ended) that carry measurement data
//!
//! One command, two views: user view (health language) and system view (raw metrics).

mod internal;

use anyhow::Result;

pub use internal::{mcp_measure, VerbMetrics};

/// Options for the measure command
#[derive(Debug, Clone, Default)]
pub struct MeasureOptions {
    /// Show raw metrics and history (maintainer view)
    pub system: bool,
    /// Output as machine-readable JSON
    pub json: bool,
    /// Drill-down into a specific verb with history
    pub verb: Option<String>,
    /// Show full health report with freshness, diagnostics, and health summary
    pub full: bool,
}

/// Execute the measure command
pub fn execute(options: MeasureOptions) -> Result<()> {
    if let Ok(client) = try_daemon_client() {
        if let Ok(result) = client.request(
            "measure",
            serde_json::json!({
                "system": options.system,
                "json": options.json,
                "verb": options.verb,
                "full": options.full,
            }),
        ) {
            if let Some(output) = result.get("output").and_then(|v| v.as_str()) {
                if !output.contains("not yet implemented") {
                    println!("{}", output);
                    return Ok(());
                }
            }
        }
    }

    internal::run(options)
}

fn try_daemon_client() -> Result<patina::mother::DaemonClient> {
    let client = patina::mother::DaemonClient::new();
    client.ensure_running()?;
    Ok(client)
}
