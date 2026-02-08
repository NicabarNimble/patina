//! Independent scry eval — tests semantic retrieval (vector search) in isolation
//!
//! Calls QueryEngine::query() directly, measures P@5, P@10, MRR against expected
//! beliefs and patterns. Includes scry-vs-assay comparison to prove semantic adds
//! value beyond keyword search (Phase 4 exit criterion: ≥5/20 scry-only hits).

use anyhow::{Context, Result};

use crate::commands::assay::{assay_search, SearchOptions};
use crate::retrieval::QueryEngine;

use super::helpers::{
    compute_metrics, extract_file_from_doc_id, normalize_path, print_metrics,
    print_per_query_detail, QueryCase,
};

/// Execute independent scry eval + scry-vs-assay comparison
pub fn execute() -> Result<()> {
    println!("📊 Scry Eval — Independent Semantic Retrieval\n");
    println!("Testing vector search quality (scry only, no FTS5)...\n");

    let test_path = "resources/eval/scry-queries.json";
    let content = std::fs::read_to_string(test_path).context(format!("Cannot read {test_path}"))?;
    let cases: Vec<QueryCase> =
        serde_json::from_str(&content).context("Failed to parse scry-queries.json")?;

    let train_count = cases.iter().filter(|c| c.split == "train").count();
    let test_count = cases.iter().filter(|c| c.split == "test").count();
    println!(
        "Loaded {} queries ({} train, {} test)\n",
        cases.len(),
        train_count,
        test_count
    );

    let engine = QueryEngine::new();

    // Query function for scry: return doc_ids from semantic search
    let scry_fn = |q: &str| -> Vec<String> {
        match engine.query(q, 10) {
            Ok(results) => results.into_iter().map(|r| r.doc_id).collect(),
            Err(_) => Vec::new(),
        }
    };

    // Per-query detail
    println!("━━━ Per-Query Detail (Scry) ━━━\n");
    print_per_query_detail(&cases, &scry_fn);

    // Overall scry metrics
    let scry_metrics = compute_metrics(&cases, &scry_fn, "scry (all)");
    println!("\n━━━ Scry Overall ━━━\n");
    print_metrics(&scry_metrics);

    // Train/test split
    let train_cases: Vec<QueryCase> = cases
        .iter()
        .filter(|c| c.split == "train")
        .map(|c| QueryCase {
            query: c.query.clone(),
            expected: c.expected.clone(),
            category: c.category.clone(),
            note: c.note.clone(),
            split: c.split.clone(),
        })
        .collect();
    let test_cases: Vec<QueryCase> = cases
        .iter()
        .filter(|c| c.split == "test")
        .map(|c| QueryCase {
            query: c.query.clone(),
            expected: c.expected.clone(),
            category: c.category.clone(),
            note: c.note.clone(),
            split: c.split.clone(),
        })
        .collect();

    if !train_cases.is_empty() && !test_cases.is_empty() {
        let train_m = compute_metrics(&train_cases, &scry_fn, "scry (train)");
        let test_m = compute_metrics(&test_cases, &scry_fn, "scry (test)");

        println!("\n━━━ Train vs Test (Scry) ━━━\n");
        println!(
            "{:<25} {:>6} {:>8} {:>8} {:>8}",
            "Split", "N", "P@5", "P@10", "MRR"
        );
        println!("{}", "─".repeat(58));
        for m in [&train_m, &test_m] {
            println!(
                "{:<25} {:>6} {:>7.1}% {:>7.1}% {:>8.3}",
                m.name,
                m.num_queries,
                m.p5 * 100.0,
                m.p10 * 100.0,
                m.mrr,
            );
        }
    }

    // ================================================================
    // Scry-vs-Assay comparison (Phase 4 exit criterion: ≥5/20)
    // ================================================================
    println!("\n━━━ Scry vs Assay Comparison ━━━\n");
    println!("Running same conceptual queries through both systems...\n");

    // Assay query function
    let assay_fn = |q: &str| -> Vec<String> {
        let options = SearchOptions {
            limit: 10,
            include_issues: false,
            repo: None,
        };
        match assay_search(q, &options) {
            Ok(results) => results.into_iter().map(|r| r.source_id).collect(),
            Err(_) => Vec::new(),
        }
    };

    let mut scry_only_hits = 0usize;
    let mut assay_only_hits = 0usize;
    let mut both_hit = 0usize;
    let mut both_miss = 0usize;

    println!(
        "{:<55} {:>10} {:>10}",
        "Query", "Scry", "Assay"
    );
    println!("{}", "─".repeat(77));

    for case in &cases {
        let expected: std::collections::HashSet<String> =
            case.expected.iter().map(|p| normalize_path(p)).collect();

        let scry_results = scry_fn(&case.query);
        let assay_results = assay_fn(&case.query);

        let scry_hit = scry_results
            .iter()
            .take(10)
            .any(|id| expected.contains(&extract_file_from_doc_id(id)));
        let assay_hit = assay_results
            .iter()
            .take(10)
            .any(|id| expected.contains(&extract_file_from_doc_id(id)));

        match (scry_hit, assay_hit) {
            (true, false) => scry_only_hits += 1,
            (false, true) => assay_only_hits += 1,
            (true, true) => both_hit += 1,
            (false, false) => both_miss += 1,
        }

        let scry_str = if scry_hit { "HIT" } else { "miss" };
        let assay_str = if assay_hit { "HIT" } else { "miss" };

        let display_q = if case.query.len() > 53 {
            format!("{}...", &case.query[..50])
        } else {
            case.query.clone()
        };
        println!("{:<55} {:>10} {:>10}", display_q, scry_str, assay_str);
    }

    let total = cases.len();
    println!("\n━━━ Comparison Summary ━━━\n");
    println!("  Scry HIT, Assay miss:  {} / {} queries", scry_only_hits, total);
    println!("  Both HIT:              {} / {} queries", both_hit, total);
    println!("  Assay HIT, Scry miss:  {} / {} queries", assay_only_hits, total);
    println!("  Both miss:             {} / {} queries", both_miss, total);

    // Phase 4 exit criterion
    let criterion_met = scry_only_hits >= 5;
    println!(
        "\n  Phase 4 criterion (scry finds ≥5/20 that assay misses): {} ({}/20)",
        if criterion_met { "PASS" } else { "FAIL" },
        scry_only_hits
    );

    // Summary
    println!("\n━━━ Summary ━━━\n");
    println!("  Scry Mean P@5:    {:.1}%", scry_metrics.p5 * 100.0);
    println!("  Scry Mean P@10:   {:.1}%", scry_metrics.p10 * 100.0);
    println!("  Scry MRR:         {:.3}", scry_metrics.mrr);
    println!(
        "  Scry-only value:  {} queries where semantic finds answers FTS5 misses",
        scry_only_hits
    );

    Ok(())
}
