//! Internal implementation for bench command
//!
//! Metrics: MRR, Recall@K, Latency p50/p95

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::retrieval::{QueryEngine, RetrievalConfig};
use patina::project;

/// A single benchmark query with ground truth
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BenchQuery {
    /// Unique query identifier
    pub id: String,
    /// The query text
    pub query: String,
    /// Relevant documents by path/ID (preferred - explicit document matching)
    #[serde(default)]
    pub relevant_docs: Vec<String>,
    /// Legacy: keywords for substring matching (deprecated, use relevant_docs)
    #[serde(default)]
    pub relevant: Vec<String>,
}

/// Query set for benchmarking
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuerySet {
    /// Name of this query set
    pub name: String,
    /// Optional description
    #[serde(default)]
    pub description: String,
    /// Benchmark queries with ground truth
    pub queries: Vec<BenchQuery>,
}

impl QuerySet {
    /// Load query set from JSON file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read query set: {}", path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse query set: {}", path.display()))
    }
}

/// Results from a single query
/// Note: query_id and retrieved_docs removed (unused). Add back for --verbose mode.
struct QueryResult {
    latency: Duration,
    reciprocal_rank: f64,
    recall_at_5: f64,
    recall_at_10: f64,
}

/// Aggregate benchmark results
#[derive(Debug, Serialize)]
struct BenchmarkResults {
    query_set: String,
    num_queries: usize,
    mrr: f64,
    recall_at_5: f64,
    recall_at_10: f64,
    latency_p50_ms: f64,
    latency_p95_ms: f64,
    latency_mean_ms: f64,
}

/// Ground truth for a benchmark query
/// Supports both document ID matching (preferred) and keyword matching (legacy)
struct GroundTruth<'a> {
    /// Explicit document paths/IDs (strong ground truth)
    docs: &'a [String],
    /// Keywords for substring matching (weak ground truth, legacy)
    keywords: &'a [String],
}

impl<'a> GroundTruth<'a> {
    fn from_query(query: &'a BenchQuery) -> Self {
        Self {
            docs: &query.relevant_docs,
            keywords: &query.relevant,
        }
    }

    /// Check if a retrieved doc_id matches ground truth
    /// Prefers doc matching; falls back to keyword matching if no docs specified
    fn matches(&self, doc_id: &str) -> bool {
        if !self.docs.is_empty() {
            // Strong matching: doc_id must contain one of the relevant doc paths
            // e.g., doc_id="src/retrieval/fusion.rs:42" matches "src/retrieval/fusion.rs"
            self.docs
                .iter()
                .any(|d| doc_id.contains(d) || d.contains(doc_id))
        } else {
            // Legacy keyword matching (weak)
            let doc_lower = doc_id.to_lowercase();
            self.keywords
                .iter()
                .any(|k| doc_lower.contains(&k.to_lowercase()))
        }
    }

    /// Count of expected relevant documents
    fn expected_count(&self) -> usize {
        if !self.docs.is_empty() {
            self.docs.len()
        } else {
            self.keywords.len()
        }
    }

    /// Check if using strong (doc-based) or weak (keyword-based) ground truth
    fn is_strong(&self) -> bool {
        !self.docs.is_empty()
    }
}

/// Calculate reciprocal rank (1/rank of first relevant result, or 0)
fn reciprocal_rank(retrieved: &[String], ground_truth: &GroundTruth) -> f64 {
    for (rank, doc_id) in retrieved.iter().enumerate() {
        if ground_truth.matches(doc_id) {
            return 1.0 / (rank + 1) as f64;
        }
    }
    0.0
}

/// Calculate recall at K (fraction of relevant docs found in top K)
fn recall_at_k(retrieved: &[String], ground_truth: &GroundTruth, k: usize) -> f64 {
    let expected = ground_truth.expected_count();
    if expected == 0 {
        return 1.0; // No relevant docs = perfect recall (vacuous truth)
    }

    let top_k: Vec<_> = retrieved.iter().take(k).collect();

    if ground_truth.is_strong() {
        // Count how many expected docs were found
        let found = ground_truth
            .docs
            .iter()
            .filter(|d| {
                top_k
                    .iter()
                    .any(|doc| doc.contains(*d) || d.contains(doc.as_str()))
            })
            .count();
        found as f64 / expected as f64
    } else {
        // Legacy: count keywords matched (weaker signal)
        let found = ground_truth
            .keywords
            .iter()
            .filter(|k| {
                let k_lower = k.to_lowercase();
                top_k
                    .iter()
                    .any(|doc| doc.to_lowercase().contains(&k_lower))
            })
            .count();
        found as f64 / expected as f64
    }
}

/// Calculate percentile from sorted latencies
fn percentile(sorted_latencies: &[Duration], p: f64) -> Duration {
    if sorted_latencies.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((sorted_latencies.len() as f64 - 1.0) * p / 100.0).round() as usize;
    sorted_latencies[idx.min(sorted_latencies.len() - 1)]
}

/// Build retrieval config from project config with optional CLI overrides
pub fn build_retrieval_config(
    rrf_k_override: Option<usize>,
    fetch_multiplier_override: Option<usize>,
    oracle_filter: Option<Vec<String>>,
) -> RetrievalConfig {
    // Try to load from project config, fall back to defaults
    let project_config = project::load(Path::new(".")).ok();

    let base_rrf_k = project_config
        .as_ref()
        .map(|c| c.retrieval.rrf_k)
        .unwrap_or(60);

    let base_fetch_multiplier = project_config
        .as_ref()
        .map(|c| c.retrieval.fetch_multiplier)
        .unwrap_or(2);

    RetrievalConfig {
        rrf_k: rrf_k_override.unwrap_or(base_rrf_k),
        fetch_multiplier: fetch_multiplier_override.unwrap_or(base_fetch_multiplier),
        oracle_filter,
    }
}

/// Run the benchmark and report results
pub fn run_benchmark(
    query_set: &QuerySet,
    limit: usize,
    json_output: bool,
    config: RetrievalConfig,
) -> Result<()> {
    println!("🔬 Patina Retrieval Benchmark");
    println!(
        "   Query set: {} ({} queries)",
        query_set.name,
        query_set.queries.len()
    );
    println!("   Limit: {} results per query", limit);
    println!(
        "   Config: rrf_k={}, fetch_multiplier={}",
        config.rrf_k, config.fetch_multiplier
    );

    // Show oracle filter for ablation clarity
    let oracle_desc = match &config.oracle_filter {
        Some(oracles) => oracles.join(", "),
        None => "all".to_string(),
    };
    println!("   Oracles: {}", oracle_desc);
    println!();

    // Initialize query engine with config
    let engine = QueryEngine::with_config(config);

    // Run each query
    let mut results: Vec<QueryResult> = Vec::new();

    for (i, bench_query) in query_set.queries.iter().enumerate() {
        print!(
            "   [{}/{}] {} ... ",
            i + 1,
            query_set.queries.len(),
            bench_query.id
        );

        let start = Instant::now();
        let fused_results = engine.query(&bench_query.query, limit)?;
        let latency = start.elapsed();

        let retrieved_docs: Vec<String> = fused_results.iter().map(|r| r.doc_id.clone()).collect();

        let ground_truth = GroundTruth::from_query(bench_query);
        let rr = reciprocal_rank(&retrieved_docs, &ground_truth);
        let r5 = recall_at_k(&retrieved_docs, &ground_truth, 5);
        let r10 = recall_at_k(&retrieved_docs, &ground_truth, 10);

        println!(
            "{:.0}ms (RR={:.2}, R@5={:.0}%, R@10={:.0}%)",
            latency.as_millis(),
            rr,
            r5 * 100.0,
            r10 * 100.0
        );

        results.push(QueryResult {
            latency,
            reciprocal_rank: rr,
            recall_at_5: r5,
            recall_at_10: r10,
        });
    }

    // Calculate aggregate metrics
    let num_queries = results.len();
    let mrr = results.iter().map(|r| r.reciprocal_rank).sum::<f64>() / num_queries as f64;
    let recall_5 = results.iter().map(|r| r.recall_at_5).sum::<f64>() / num_queries as f64;
    let recall_10 = results.iter().map(|r| r.recall_at_10).sum::<f64>() / num_queries as f64;

    let mut latencies: Vec<Duration> = results.iter().map(|r| r.latency).collect();
    latencies.sort();

    let latency_mean = latencies.iter().sum::<Duration>() / num_queries as u32;
    let latency_p50 = percentile(&latencies, 50.0);
    let latency_p95 = percentile(&latencies, 95.0);

    let benchmark_results = BenchmarkResults {
        query_set: query_set.name.clone(),
        num_queries,
        mrr,
        recall_at_5: recall_5,
        recall_at_10: recall_10,
        latency_p50_ms: latency_p50.as_secs_f64() * 1000.0,
        latency_p95_ms: latency_p95.as_secs_f64() * 1000.0,
        latency_mean_ms: latency_mean.as_secs_f64() * 1000.0,
    };

    if json_output {
        println!("{}", serde_json::to_string_pretty(&benchmark_results)?);
    } else {
        println!();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📊 Results: {}", query_set.name);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        println!("   Relevance Metrics:");
        println!("   ├─ MRR:        {:.3}", mrr);
        println!("   ├─ Recall@5:   {:.1}%", recall_5 * 100.0);
        println!("   └─ Recall@10:  {:.1}%", recall_10 * 100.0);
        println!();
        println!("   Latency:");
        println!("   ├─ p50:  {:.0}ms", latency_p50.as_millis());
        println!("   ├─ p95:  {:.0}ms", latency_p95.as_millis());
        println!("   └─ mean: {:.0}ms", latency_mean.as_millis());
        println!();

        // Quality assessment
        let quality = if mrr >= 0.5 && recall_10 >= 0.7 {
            "✅ Good"
        } else if mrr >= 0.3 && recall_10 >= 0.5 {
            "⚠️  Acceptable"
        } else {
            "❌ Needs improvement"
        };
        println!("   Quality: {}", quality);
    }

    Ok(())
}
