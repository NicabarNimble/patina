//! Measurement consumer — project health from eventlog metrics
//!
//! Reads measurement data from two sources:
//! 1. `measure.*` events (from Phase 1-2 tools: eval, bench, scrape, oxidize, doctor)
//! 2. Existing typed events (belief.surface, session.ended) that carry measurement data
//!
//! One command, two views: user view (health language) and system view (raw metrics).

mod internal;

use anyhow::Result;

pub use internal::mcp_measure;

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
    internal::run(options)
}
