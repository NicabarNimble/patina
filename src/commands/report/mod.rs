//! Report command - Generate comprehensive project state reports
//!
//! Uses patina's own tools (scry, assay) to generate reports.
//! Report quality = tool quality. If scry can't answer "main modules",
//! that's a bug in scry, not the report.
//!
//! Phase 1: Summary metrics + scry queries + RAG health

mod internal;

use anyhow::Result;

/// Options for report generation
#[derive(Debug, Clone, Default)]
pub struct ReportOptions {
    /// Output path (default: layer/surface/reports/state/YYYY-MM-DD-state.md)
    pub output: Option<String>,
    /// Query a specific registered repo
    pub repo: Option<String>,
    /// Output as JSON instead of markdown
    pub json: bool,
}

/// Execute report command
pub fn execute(options: ReportOptions) -> Result<()> {
    internal::generate_report(options)
}
