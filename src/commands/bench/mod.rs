//! Benchmark command - measure retrieval quality
//!
//! Public interface:
//! - `execute()` - run retrieval benchmarks
//! - `generate()` - generate querysets from git commits
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
    /// Query a specific registered repo instead of current project
    pub repo: Option<String>,
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
        options.repo,
    )
}

/// Options for generating querysets from git commits
pub struct GenerateOptions {
    /// Repository name (None = current project)
    pub repo: Option<String>,
    /// Maximum number of queries to generate
    pub limit: usize,
    /// Output file path (None = stdout)
    pub output: Option<String>,
}

/// Generate a queryset from git commits
pub fn generate(options: GenerateOptions) -> Result<()> {
    let config = internal::GenerateConfig {
        repo: options.repo,
        limit: options.limit,
        ..Default::default()
    };

    let query_set = internal::generate_from_commits(config)?;

    let json = serde_json::to_string_pretty(&query_set)?;

    if let Some(output_path) = options.output {
        std::fs::write(&output_path, &json)?;
        println!(
            "Generated {} queries → {}",
            query_set.queries.len(),
            output_path
        );
    } else {
        println!("{}", json);
    }

    Ok(())
}
