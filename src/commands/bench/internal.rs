//! Internal implementation for bench command
//!
//! Metrics: MRR, Recall@K, Latency p50/p95

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::retrieval::{QueryEngine, QueryOptions, RetrievalConfig};
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
    /// Relevant commits (for commit-derived querysets)
    #[serde(default)]
    pub relevant_commits: Vec<String>,
    /// Source commit SHA (for commit-derived querysets)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_commit: Option<String>,
    /// Legacy: keywords for substring matching (deprecated, use relevant_docs)
    #[serde(default)]
    pub relevant: Vec<String>,
    /// Expected repos for cross-project queries (G2.5)
    #[serde(default)]
    pub expected_repos: Vec<String>,
}

/// Query set for benchmarking
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuerySet {
    /// Name of this query set
    pub name: String,
    /// Optional description
    #[serde(default)]
    pub description: String,
    /// Source of the queryset (e.g., "git commits", "sessions", "manual")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Repository name (for ref repo querysets)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    /// When this queryset was generated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated: Option<String>,
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
    /// Repo recall for cross-project queries (None if no expected_repos)
    repo_recall: Option<f64>,
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
    /// Repo recall for cross-project queries (None if not applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    repo_recall: Option<f64>,
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

/// Calculate repo recall for cross-project queries (G2.5)
///
/// Measures: did we find results from the expected repos?
/// Returns None if no expected_repos specified.
fn repo_recall(retrieved: &[String], expected_repos: &[String]) -> Option<f64> {
    if expected_repos.is_empty() {
        return None;
    }

    // Extract repos from doc_ids (format: "repo:path" or just "path")
    // Local docs (no colon) are skipped - they don't contribute to cross-project recall
    let found_repos: std::collections::HashSet<String> = retrieved
        .iter()
        .filter_map(|doc_id| doc_id.find(':').map(|idx| doc_id[..idx].to_string()))
        .collect();

    // Count how many expected repos had results
    let matched = expected_repos
        .iter()
        .filter(|exp| {
            found_repos
                .iter()
                .any(|found| found.eq_ignore_ascii_case(exp) || found.contains(exp.as_str()))
        })
        .count();

    Some(matched as f64 / expected_repos.len() as f64)
}

/// Print detailed analysis for a query (verbose mode)
fn print_verbose_analysis(
    query: &BenchQuery,
    retrieved: &[String],
    ground_truth: &GroundTruth,
    rr: f64,
) {
    println!("      ┌─ Query: \"{}\"", truncate(&query.query, 60));

    // Show expected documents
    if !ground_truth.docs.is_empty() {
        println!("      │  Expected: {:?}", ground_truth.docs);
    } else if !ground_truth.keywords.is_empty() {
        println!("      │  Keywords: {:?}", ground_truth.keywords);
    }

    // Show top 5 retrieved
    println!("      │  Retrieved (top 5):");
    for (i, doc) in retrieved.iter().take(5).enumerate() {
        let matches = ground_truth.matches(doc);
        let marker = if matches { "✓" } else { " " };
        println!("      │    {}. {} {}", i + 1, marker, truncate(doc, 50));
    }

    // Analysis for failures
    if rr == 0.0 {
        println!("      │  ");
        println!("      │  ⚠ FAILURE ANALYSIS:");

        // Check if expected docs exist in retrieved at all
        let mut found_any = false;
        for expected in ground_truth.docs.iter() {
            for (rank, doc) in retrieved.iter().enumerate() {
                if doc.contains(expected) || expected.contains(doc) {
                    println!(
                        "      │    Found '{}' at rank {} (not in top 10)",
                        expected,
                        rank + 1
                    );
                    found_any = true;
                    break;
                }
            }
        }

        if !found_any && !ground_truth.docs.is_empty() {
            println!("      │    Expected docs NOT in retrieved results at all");
            println!("      │    Possible causes:");
            println!("      │      - Document not indexed (run: patina scrape && patina oxidize)");
            println!("      │      - Query doesn't match document content semantically");
            println!("      │      - Lexical terms don't appear in doc symbols");
        }
    }

    println!("      └─");
}

/// Truncate string for display
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
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
    verbose: bool,
    config: RetrievalConfig,
    repo: Option<String>,
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

    // Show repo if specified
    if let Some(ref repo_name) = repo {
        println!("   Repo: {}", repo_name);
    }
    println!();

    // Initialize query engine with config
    let engine = QueryEngine::with_config(config);

    // Build query options for repo-specific queries
    let query_options = QueryOptions {
        repo: repo.clone(),
        ..Default::default()
    };

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
        let fused_results = if repo.is_some() {
            engine.query_with_options(&bench_query.query, limit, &query_options)?
        } else {
            engine.query(&bench_query.query, limit)?
        };
        let latency = start.elapsed();

        let retrieved_docs: Vec<String> = fused_results.iter().map(|r| r.doc_id.clone()).collect();

        let ground_truth = GroundTruth::from_query(bench_query);
        let rr = reciprocal_rank(&retrieved_docs, &ground_truth);
        let r5 = recall_at_k(&retrieved_docs, &ground_truth, 5);
        let r10 = recall_at_k(&retrieved_docs, &ground_truth, 10);
        let rrepo = repo_recall(&retrieved_docs, &bench_query.expected_repos);

        // Show repo recall if this is a cross-project query
        if let Some(repo_r) = rrepo {
            println!(
                "{:.0}ms (RR={:.2}, R@5={:.0}%, R@10={:.0}%, Repo={:.0}%)",
                latency.as_millis(),
                rr,
                r5 * 100.0,
                r10 * 100.0,
                repo_r * 100.0
            );
        } else {
            println!(
                "{:.0}ms (RR={:.2}, R@5={:.0}%, R@10={:.0}%)",
                latency.as_millis(),
                rr,
                r5 * 100.0,
                r10 * 100.0
            );
        }

        // Verbose: show detailed analysis for failures or all queries
        if verbose {
            print_verbose_analysis(bench_query, &retrieved_docs, &ground_truth, rr);
        }

        results.push(QueryResult {
            latency,
            reciprocal_rank: rr,
            recall_at_5: r5,
            recall_at_10: r10,
            repo_recall: rrepo,
        });
    }

    // Calculate aggregate metrics
    let num_queries = results.len();
    let mrr = results.iter().map(|r| r.reciprocal_rank).sum::<f64>() / num_queries as f64;
    let recall_5 = results.iter().map(|r| r.recall_at_5).sum::<f64>() / num_queries as f64;
    let recall_10 = results.iter().map(|r| r.recall_at_10).sum::<f64>() / num_queries as f64;

    // Calculate repo recall if any queries had expected_repos
    let repo_recalls: Vec<f64> = results.iter().filter_map(|r| r.repo_recall).collect();
    let avg_repo_recall = if !repo_recalls.is_empty() {
        Some(repo_recalls.iter().sum::<f64>() / repo_recalls.len() as f64)
    } else {
        None
    };

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
        repo_recall: avg_repo_recall,
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

        // Show routing metrics if this is a cross-project queryset
        if let Some(repo_r) = avg_repo_recall {
            println!();
            println!("   Routing Metrics (cross-project):");
            println!(
                "   └─ Repo Recall: {:.1}% ({}/{} queries with expected_repos)",
                repo_r * 100.0,
                repo_recalls.len(),
                num_queries
            );
        }

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

/// Internal configuration for queryset generation
/// (Public interface is in mod.rs::GenerateOptions)
pub struct GenerateConfig {
    /// Repository name (None = current project)
    pub repo: Option<String>,
    /// Maximum number of queries to generate
    pub limit: usize,
    /// Minimum commit message length (implementation detail)
    pub min_message_len: usize,
    /// Maximum commit message length (implementation detail)
    pub max_message_len: usize,
    /// Minimum files changed per commit (implementation detail)
    pub min_files: usize,
    /// Maximum files changed per commit (implementation detail)
    pub max_files: usize,
}

impl Default for GenerateConfig {
    fn default() -> Self {
        Self {
            repo: None,
            limit: 100,
            min_message_len: 20,
            max_message_len: 200,
            min_files: 2,
            max_files: 15,
        }
    }
}

/// Generate a queryset from git commits
pub fn generate_from_commits(config: GenerateConfig) -> Result<QuerySet> {
    use chrono::Utc;
    use rusqlite::Connection;

    // Determine database path
    let db_path = if let Some(ref repo_name) = config.repo {
        crate::commands::repo::get_db_path(repo_name)?
    } else {
        ".patina/local/data/patina.db".to_string()
    };

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Query for good commits
    let sql = r#"
        SELECT
            c.sha,
            c.message,
            GROUP_CONCAT(cf.file_path, '|') as files
        FROM commits c
        JOIN commit_files cf ON c.sha = cf.sha
        WHERE length(c.message) > ?
          AND length(c.message) < ?
          AND c.message NOT LIKE 'Merge%'
          AND c.message NOT LIKE 'WIP%'
          AND c.message NOT LIKE 'wip%'
          AND c.message NOT LIKE 'fixup%'
          AND c.message NOT LIKE 'squash%'
        GROUP BY c.sha
        HAVING COUNT(cf.file_path) >= ? AND COUNT(cf.file_path) <= ?
        ORDER BY c.timestamp DESC
        LIMIT ?
    "#;

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(
        rusqlite::params![
            config.min_message_len,
            config.max_message_len,
            config.min_files,
            config.max_files,
            config.limit
        ],
        |row| {
            Ok((
                row.get::<_, String>(0)?, // sha
                row.get::<_, String>(1)?, // message
                row.get::<_, String>(2)?, // files (pipe-separated)
            ))
        },
    )?;

    let mut queries = Vec::new();
    for row in rows {
        let (sha, message, files_str) = row?;

        // Clean the commit message (take first line, remove conventional commit prefix)
        let query = clean_commit_message(&message);
        if query.is_empty() {
            continue;
        }

        // Parse files
        let files: Vec<String> = files_str.split('|').map(|s| s.to_string()).collect();

        // Create short SHA for ID
        let short_sha = &sha[..8.min(sha.len())];

        queries.push(BenchQuery {
            id: format!("q_{}", short_sha),
            query,
            relevant_docs: files,
            relevant_commits: vec![sha.clone()],
            source_commit: Some(sha),
            relevant: vec![],       // No legacy keywords
            expected_repos: vec![], // No cross-project expectations for commit-derived queries
        });
    }

    let repo_name = config.repo.clone().unwrap_or_else(|| "local".to_string());
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    Ok(QuerySet {
        name: format!("{}-commits-v1", repo_name),
        description: format!(
            "Auto-generated from {} git commits ({} queries)",
            repo_name,
            queries.len()
        ),
        source: Some("git commits (auto-generated)".to_string()),
        repo: config.repo,
        generated: Some(timestamp),
        queries,
    })
}

/// Clean a commit message for use as a query
fn clean_commit_message(message: &str) -> String {
    // Take first line only
    let first_line = message.lines().next().unwrap_or(message);

    // Remove conventional commit prefix (feat:, fix:, docs:, etc.)
    let cleaned = if let Some(idx) = first_line.find(':') {
        let prefix = &first_line[..idx];
        // Check if it looks like a conventional commit prefix
        if prefix.len() < 20
            && prefix
                .chars()
                .all(|c| c.is_alphanumeric() || c == '(' || c == ')')
        {
            first_line[idx + 1..].trim()
        } else {
            first_line
        }
    } else {
        first_line
    };

    // Remove PR references like (#123)
    let cleaned = cleaned
        .split(" (#")
        .next()
        .unwrap_or(cleaned)
        .trim()
        .to_string();

    cleaned
}
