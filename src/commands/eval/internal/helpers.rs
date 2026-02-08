//! Shared helpers for eval modules

use std::collections::HashSet;

/// Normalize path by stripping "./" prefix
pub fn normalize_path(path: &str) -> String {
    path.strip_prefix("./").unwrap_or(path).to_string()
}

/// Extract file path from a doc_id (strip "::suffix" for code facts)
pub fn extract_file_from_doc_id(doc_id: &str) -> String {
    let path = if let Some(idx) = doc_id.find("::") {
        &doc_id[..idx]
    } else {
        doc_id
    };
    normalize_path(path)
}

/// NL query test case loaded from JSON (shared format for scry and assay queries)
#[derive(serde::Deserialize, Debug)]
pub struct QueryCase {
    pub query: String,
    pub expected: Vec<String>,
    pub category: String,
    #[allow(dead_code)]
    pub note: Option<String>,
    pub split: String,
}

/// Aggregated eval metrics for one engine
#[derive(Debug, Clone)]
pub struct EvalMetrics {
    pub name: String,
    pub num_queries: usize,
    pub p5: f32,
    pub p10: f32,
    pub mrr: f32,
    pub hit_rate: f32,
}

/// Compute metrics for a set of queries given result doc_ids
///
/// For each query: check which expected items appear in results.
/// P@K = |expected ∩ top-K| / min(|expected|, K)
/// MRR = 1/rank of first hit
/// Hit rate = fraction of queries with at least one hit
pub fn compute_metrics(
    queries: &[QueryCase],
    results_fn: &dyn Fn(&str) -> Vec<String>,
    name: &str,
) -> EvalMetrics {
    let mut total_p5 = 0.0f32;
    let mut total_p10 = 0.0f32;
    let mut total_rr = 0.0f32;
    let mut hits = 0usize;
    let n = queries.len();

    for case in queries {
        let result_ids = results_fn(&case.query);
        let expected: HashSet<String> = case.expected.iter().map(|p| normalize_path(p)).collect();

        // Deduplicate results by normalized ID (file-level for code results)
        let unique_5: HashSet<String> = result_ids
            .iter()
            .take(5)
            .map(|id| extract_file_from_doc_id(id))
            .filter(|id| expected.contains(id))
            .collect();
        let unique_10: HashSet<String> = result_ids
            .iter()
            .take(10)
            .map(|id| extract_file_from_doc_id(id))
            .filter(|id| expected.contains(id))
            .collect();

        let denom_5 = expected.len().clamp(1, 5) as f32;
        let denom_10 = expected.len().clamp(1, 10) as f32;
        total_p5 += unique_5.len() as f32 / denom_5;
        total_p10 += unique_10.len() as f32 / denom_10;

        // MRR: rank of first hit
        let rr = result_ids
            .iter()
            .enumerate()
            .find(|(_, id)| expected.contains(&extract_file_from_doc_id(id)))
            .map(|(i, _)| 1.0 / (i as f32 + 1.0))
            .unwrap_or(0.0);
        total_rr += rr;

        if rr > 0.0 {
            hits += 1;
        }
    }

    EvalMetrics {
        name: name.to_string(),
        num_queries: n,
        p5: if n > 0 { total_p5 / n as f32 } else { 0.0 },
        p10: if n > 0 { total_p10 / n as f32 } else { 0.0 },
        mrr: if n > 0 { total_rr / n as f32 } else { 0.0 },
        hit_rate: if n > 0 {
            hits as f32 / n as f32
        } else {
            0.0
        },
    }
}

/// Print a metrics summary table
pub fn print_metrics(metrics: &EvalMetrics) {
    println!("  Queries:    {}", metrics.num_queries);
    println!("  P@5:        {:.1}%", metrics.p5 * 100.0);
    println!("  P@10:       {:.1}%", metrics.p10 * 100.0);
    println!("  MRR:        {:.3}", metrics.mrr);
    println!("  Hit rate:   {:.1}%", metrics.hit_rate * 100.0);
}

/// Print a per-query detail table
pub fn print_per_query_detail(
    cases: &[QueryCase],
    results_fn: &dyn Fn(&str) -> Vec<String>,
) {
    println!(
        "{:<55} {:>6} {:>6} {:>6}",
        "Query", "P@5", "P@10", "RR"
    );
    println!("{}", "─".repeat(77));

    for case in cases {
        let result_ids = results_fn(&case.query);
        let expected: HashSet<String> = case.expected.iter().map(|p| normalize_path(p)).collect();

        let unique_5: HashSet<String> = result_ids
            .iter()
            .take(5)
            .map(|id| extract_file_from_doc_id(id))
            .filter(|id| expected.contains(id))
            .collect();
        let unique_10: HashSet<String> = result_ids
            .iter()
            .take(10)
            .map(|id| extract_file_from_doc_id(id))
            .filter(|id| expected.contains(id))
            .collect();

        let denom_5 = expected.len().clamp(1, 5) as f32;
        let denom_10 = expected.len().clamp(1, 10) as f32;
        let p5 = unique_5.len() as f32 / denom_5;
        let p10 = unique_10.len() as f32 / denom_10;

        let rr = result_ids
            .iter()
            .enumerate()
            .find(|(_, id)| expected.contains(&extract_file_from_doc_id(id)))
            .map(|(i, _)| 1.0 / (i as f32 + 1.0))
            .unwrap_or(0.0);

        let display_q = if case.query.len() > 53 {
            format!("{}...", &case.query[..50])
        } else {
            case.query.clone()
        };
        println!(
            "{:<55} {:>5.0}% {:>5.0}% {:>.3}",
            display_q,
            p5 * 100.0,
            p10 * 100.0,
            rr
        );
    }
}
