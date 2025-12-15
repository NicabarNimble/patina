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
    /// Show detailed per-query analysis
    pub verbose: bool,
    /// Override RRF k value (default: from config or 60)
    pub rrf_k: Option<usize>,
    /// Override fetch multiplier (default: from config or 2)
    pub fetch_multiplier: Option<usize>,
    /// Filter to specific oracle(s) for ablation testing
    pub oracle: Option<Vec<String>>,
}

/// Execute retrieval benchmark
pub fn execute(options: BenchOptions) -> Result<()> {
    let query_set = QuerySet::load(Path::new(&options.query_set))?;

    // Build retrieval config from project config with CLI overrides
    let config =
        internal::build_retrieval_config(options.rrf_k, options.fetch_multiplier, options.oracle);

    internal::run_benchmark(
        &query_set,
        options.limit,
        options.json,
        options.verbose,
        config,
    )
}
