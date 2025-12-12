//! Internal implementation for bench command
//!
//! Metrics: MRR, Recall@K, Latency p50/p95

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::retrieval::QueryEngine;

/// A single benchmark query with ground truth
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BenchQuery {
    /// Unique query identifier
    pub id: String,
    /// The query text
    pub query: String,
    /// Relevant document identifiers (substring matching)
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
struct QueryResult {
    query_id: String,
    latency: Duration,
    reciprocal_rank: f64,
    recall_at_5: f64,
    recall_at_10: f64,
    retrieved_docs: Vec<String>,
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

/// Check if retrieved content matches any relevant term (case-insensitive)
fn is_relevant(content: &str, relevant: &[String]) -> bool {
    let content_lower = content.to_lowercase();
    relevant
        .iter()
        .any(|r| content_lower.contains(&r.to_lowercase()))
}

/// Calculate reciprocal rank (1/rank of first relevant result, or 0)
fn reciprocal_rank(retrieved: &[String], relevant: &[String]) -> f64 {
    for (rank, doc_id) in retrieved.iter().enumerate() {
        if is_relevant(doc_id, relevant) {
            return 1.0 / (rank + 1) as f64;
        }
    }
    0.0
}

/// Calculate recall at K (fraction of relevant docs found in top K)
fn recall_at_k(retrieved: &[String], relevant: &[String], k: usize) -> f64 {
    if relevant.is_empty() {
        return 1.0; // No relevant docs = perfect recall (vacuous truth)
    }
    let top_k: Vec<_> = retrieved.iter().take(k).collect();
    let found = relevant
        .iter()
        .filter(|r| top_k.iter().any(|doc| is_relevant(doc, &[(*r).clone()])))
        .count();
    found as f64 / relevant.len() as f64
}

/// Calculate percentile from sorted latencies
fn percentile(sorted_latencies: &[Duration], p: f64) -> Duration {
    if sorted_latencies.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((sorted_latencies.len() as f64 - 1.0) * p / 100.0).round() as usize;
    sorted_latencies[idx.min(sorted_latencies.len() - 1)]
}

/// Run the benchmark and report results
pub fn run_benchmark(query_set: &QuerySet, limit: usize, json_output: bool) -> Result<()> {
    println!("🔬 Patina Retrieval Benchmark");
    println!(
        "   Query set: {} ({} queries)",
        query_set.name,
        query_set.queries.len()
    );
    println!("   Limit: {} results per query", limit);
    println!();

    // Initialize query engine
    let engine = QueryEngine::new();

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

        let rr = reciprocal_rank(&retrieved_docs, &bench_query.relevant);
        let r5 = recall_at_k(&retrieved_docs, &bench_query.relevant, 5);
        let r10 = recall_at_k(&retrieved_docs, &bench_query.relevant, 10);

        println!(
            "{:.0}ms (RR={:.2}, R@5={:.0}%, R@10={:.0}%)",
            latency.as_millis(),
            rr,
            r5 * 100.0,
            r10 * 100.0
        );

        results.push(QueryResult {
            query_id: bench_query.id.clone(),
            latency,
            reciprocal_rank: rr,
            recall_at_5: r5,
            recall_at_10: r10,
            retrieved_docs,
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
