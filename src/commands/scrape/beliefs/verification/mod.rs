//! Belief verification — parse, validate, execute, and store verification queries
//!
//! Verification queries live in belief markdown files as `## Verification` sections
//! with fenced code blocks using the `verify` info-string. Results are stored in
//! the `belief_verifications` table and aggregated on the `beliefs` table.
//!
//! Design: queries are source data (authored intent), results are derived data (DB only).
//! See: layer/surface/build/feat/belief-verification/SPEC.md

mod internal;

use anyhow::Result;
use rusqlite::Connection;

/// A parsed verification query from a belief's `## Verification` section
#[derive(Debug, Clone)]
pub struct VerificationQuery {
    pub query_type: String, // "sql", "assay", "temporal"
    pub label: String,
    pub expect: String,     // "= 0", "> 5", ">= 1", "< 10"
    pub query_text: String, // SQL or assay command
}

/// Result of executing a single verification query
#[derive(Debug)]
pub struct VerificationResult {
    pub label: String,
    pub query_type: String,
    pub query_text: String,
    pub expectation: String,
    pub status: VerificationStatus,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum VerificationStatus {
    Pass,
    Contested,
    Error,
}

impl VerificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            VerificationStatus::Pass => "pass",
            VerificationStatus::Contested => "contested",
            VerificationStatus::Error => "error",
        }
    }
}

/// Aggregate verification counts for a belief
#[derive(Debug, Default)]
pub struct VerificationAggregates {
    pub total: i32,
    pub passed: i32,
    pub failed: i32,
    pub errored: i32,
}

// Public interface — delegates to internal modules.
// beliefs/mod.rs uses only these 3 functions + the types above.

/// Parse `## Verification` section from belief markdown content.
pub fn parse_verification_blocks(content: &str) -> Vec<VerificationQuery> {
    internal::parse::parse_verification_blocks(content)
}

/// Run all verification queries for a belief.
pub fn run_verification_queries(
    conn: &Connection,
    belief_id: &str,
    queries: &[VerificationQuery],
    data_freshness: &str,
) -> (Vec<VerificationResult>, VerificationAggregates) {
    let mut results = Vec::new();
    let mut aggregates = VerificationAggregates::default();

    for query in queries {
        let result = internal::exec::execute_verification_query(conn, query);

        aggregates.total += 1;
        match result.status {
            VerificationStatus::Pass => aggregates.passed += 1,
            VerificationStatus::Contested => aggregates.failed += 1,
            VerificationStatus::Error => aggregates.errored += 1,
        }

        results.push(result);
    }

    // Store results in belief_verifications table
    if let Err(e) =
        internal::exec::store_verification_results(conn, belief_id, &results, data_freshness)
    {
        eprintln!(
            "  Warning: failed to store verification results for {}: {}",
            belief_id, e
        );
    }

    (results, aggregates)
}

/// Create the belief_verifications table and add aggregate columns to beliefs.
pub fn create_tables(conn: &Connection) -> Result<()> {
    internal::exec::create_tables(conn)
}
