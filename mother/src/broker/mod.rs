//! Broker — data types and source configuration for Mother's routing engine.
//!
//! The broker's orchestration logic (run_source, route_lake_via_knowledge_child)
//! remains in the CLI binary until its dependencies (connect, child engine,
//! eventlog) are also consolidated into the mother crate.

pub mod cursor;
pub mod sources;

/// Result of writing a batch of facts.
#[derive(Debug, Clone)]
pub struct WriteResult {
    pub inserted: u64,
    pub dedup_skipped: u64,
    pub cursor: Option<String>,
}

/// Source status information for display.
#[derive(Debug)]
pub struct SourceStatus {
    pub name: String,
    pub last_run: Option<String>,
    pub fact_count: i64,
    pub status: String,
}
