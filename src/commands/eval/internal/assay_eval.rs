//! Independent assay eval — tests factual retrieval (FTS5) in isolation
//!
//! Calls assay_search() directly, measures P@5, P@10, MRR against expected
//! file paths. Designed for post-split architecture where assay handles facts
//! and scry handles meaning.

use anyhow::{Context, Result};

use crate::commands::assay::{assay_search, SearchOptions};

use super::helpers::{compute_metrics, print_metrics, print_per_query_detail, QueryCase};

/// Execute independent assay eval
pub fn execute() -> Result<()> {
    println!("📊 Assay Eval — Independent Factual Retrieval\n");
    println!("Testing FTS5 keyword search quality (assay only, no scry)...\n");

    let test_path = "resources/eval/assay-queries.json";
    let content = std::fs::read_to_string(test_path).context(format!("Cannot read {test_path}"))?;
    let cases: Vec<QueryCase> =
        serde_json::from_str(&content).context("Failed to parse assay-queries.json")?;

    let train_count = cases.iter().filter(|c| c.split == "train").count();
    let test_count = cases.iter().filter(|c| c.split == "test").count();
    println!(
        "Loaded {} queries ({} train, {} test)\n",
        cases.len(),
        train_count,
        test_count
    );

    // Query function: call assay_search, return source_ids
    let query_fn = |q: &str| -> Vec<String> {
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

    // Per-query detail
    print_per_query_detail(&cases, &query_fn);

    // Overall metrics
    let all_metrics = compute_metrics(&cases, &query_fn, "assay (all)");
    println!("\n━━━ Overall ━━━\n");
    print_metrics(&all_metrics);

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
        let train_m = compute_metrics(&train_cases, &query_fn, "assay (train)");
        let test_m = compute_metrics(&test_cases, &query_fn, "assay (test)");

        println!("\n━━━ Train vs Test ━━━\n");
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

        let delta_p10 = (test_m.p10 - train_m.p10) * 100.0;
        println!("\n  Train-test gap: {:+.1}pp P@10", delta_p10);
    }

    // Summary
    println!("\n━━━ Summary ━━━\n");
    println!("  Mean P@5:    {:.1}%", all_metrics.p5 * 100.0);
    println!("  Mean P@10:   {:.1}%", all_metrics.p10 * 100.0);
    println!("  MRR:         {:.3}", all_metrics.mrr);
    println!("  Hit rate:    {:.1}%", all_metrics.hit_rate * 100.0);

    Ok(())
}
