//! Benchmark command - measure retrieval quality
//!
//! Public interface:
//! - `execute()` - run retrieval benchmarks
//! - `QuerySet` - benchmark query set format
//!
//! Follows dependable-rust: metrics calculation is internal

mod internal;

use anyhow::Result;
use std::path::Path;

pub use internal::QuerySet;

/// Options for benchmark execution
pub struct BenchOptions {
    /// Path to query set JSON file
    pub query_set: String,
    /// Number of results to retrieve per query (default: 10)
    pub limit: usize,
    /// Output as JSON
    pub json: bool,
}

/// Execute retrieval benchmark
pub fn execute(options: BenchOptions) -> Result<()> {
    let query_set = QuerySet::load(Path::new(&options.query_set))?;
    internal::run_benchmark(&query_set, options.limit, options.json)
}
