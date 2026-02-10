//! Combined eval — tests the full retrieval pipeline (assay + scry)
//!
//! Runs both factual (assay) and conceptual (scry) query sets through their
//! respective systems, then shows how the two complement each other.
//! This is the product metric: "does the system surface the right context?"

use anyhow::{Context, Result};

use crate::commands::assay::{assay_search, SearchOptions};
use crate::retrieval::QueryEngine;

use super::helpers::{compute_metrics, print_metrics, QueryCase};

/// Execute combined eval
pub fn execute() -> Result<()> {
    println!("📊 Combined Eval — Full Retrieval Pipeline\n");
    println!("Testing assay (factual) and scry (semantic) together...\n");

    // Load both query sets
    let assay_content = std::fs::read_to_string("resources/eval/assay-queries.json")
        .context("Cannot read assay-queries.json")?;
    let assay_cases: Vec<QueryCase> =
        serde_json::from_str(&assay_content).context("Failed to parse assay-queries.json")?;

    let scry_content = std::fs::read_to_string("resources/eval/scry-queries.json")
        .context("Cannot read scry-queries.json")?;
    let scry_cases: Vec<QueryCase> =
        serde_json::from_str(&scry_content).context("Failed to parse scry-queries.json")?;

    let engine = QueryEngine::new();

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

    // Scry query function
    let scry_fn = |q: &str| -> Vec<String> {
        match engine.query(q, 10) {
            Ok(results) => results.into_iter().map(|r| r.doc_id).collect(),
            Err(_) => Vec::new(),
        }
    };

    // Combined: union of both systems' results (facts first, then meaning)
    let combined_fn = |q: &str| -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut combined = Vec::new();

        // Facts first (assay)
        let options = SearchOptions {
            limit: 10,
            include_issues: false,
            repo: None,
        };
        if let Ok(results) = assay_search(q, &options) {
            for r in results {
                if seen.insert(r.source_id.clone()) {
                    combined.push(r.source_id);
                }
            }
        }

        // Then meaning (scry)
        if let Ok(results) = engine.query(q, 10) {
            for r in results {
                if seen.insert(r.doc_id.clone()) {
                    combined.push(r.doc_id);
                }
            }
        }

        combined
    };

    // --- Factual queries: assay alone vs combined ---
    println!("━━━ Factual Queries ({} queries) ━━━\n", assay_cases.len());

    let assay_on_factual = compute_metrics(&assay_cases, &assay_fn, "assay-only");
    let combined_on_factual = compute_metrics(&assay_cases, &combined_fn, "combined");

    println!(
        "{:<25} {:>8} {:>8} {:>8} {:>10}",
        "Pipeline", "P@5", "P@10", "MRR", "Hit Rate"
    );
    println!("{}", "─".repeat(63));
    for m in [&assay_on_factual, &combined_on_factual] {
        println!(
            "{:<25} {:>7.1}% {:>7.1}% {:>8.3} {:>9.1}%",
            m.name,
            m.p5 * 100.0,
            m.p10 * 100.0,
            m.mrr,
            m.hit_rate * 100.0,
        );
    }

    let delta_factual = (combined_on_factual.p10 - assay_on_factual.p10) * 100.0;
    println!(
        "\n  Combined vs assay-only: {:+.1}pp P@10 on factual queries",
        delta_factual
    );

    // --- Conceptual queries: scry alone vs combined ---
    println!(
        "\n━━━ Conceptual Queries ({} queries) ━━━\n",
        scry_cases.len()
    );

    let scry_on_conceptual = compute_metrics(&scry_cases, &scry_fn, "scry-only");
    let combined_on_conceptual = compute_metrics(&scry_cases, &combined_fn, "combined");

    println!(
        "{:<25} {:>8} {:>8} {:>8} {:>10}",
        "Pipeline", "P@5", "P@10", "MRR", "Hit Rate"
    );
    println!("{}", "─".repeat(63));
    for m in [&scry_on_conceptual, &combined_on_conceptual] {
        println!(
            "{:<25} {:>7.1}% {:>7.1}% {:>8.3} {:>9.1}%",
            m.name,
            m.p5 * 100.0,
            m.p10 * 100.0,
            m.mrr,
            m.hit_rate * 100.0,
        );
    }

    let delta_conceptual = (combined_on_conceptual.p10 - scry_on_conceptual.p10) * 100.0;
    println!(
        "\n  Combined vs scry-only: {:+.1}pp P@10 on conceptual queries",
        delta_conceptual
    );

    // --- Cross-system: what does each system contribute? ---
    println!("\n━━━ Cross-System Contribution ━━━\n");

    // Assay on conceptual queries (should be weak — keyword mismatch)
    let assay_on_conceptual = compute_metrics(&scry_cases, &assay_fn, "assay on conceptual");
    // Scry on factual queries (may find some via embedded content)
    let scry_on_factual = compute_metrics(&assay_cases, &scry_fn, "scry on factual");

    println!(
        "{:<30} {:>8} {:>8} {:>8}",
        "Pipeline × Query Type", "P@5", "P@10", "MRR"
    );
    println!("{}", "─".repeat(58));
    println!(
        "{:<30} {:>7.1}% {:>7.1}% {:>8.3}",
        "assay on factual (expected)",
        assay_on_factual.p5 * 100.0,
        assay_on_factual.p10 * 100.0,
        assay_on_factual.mrr,
    );
    println!(
        "{:<30} {:>7.1}% {:>7.1}% {:>8.3}",
        "scry on conceptual (expected)",
        scry_on_conceptual.p5 * 100.0,
        scry_on_conceptual.p10 * 100.0,
        scry_on_conceptual.mrr,
    );
    println!(
        "{:<30} {:>7.1}% {:>7.1}% {:>8.3}",
        "assay on conceptual (cross)",
        assay_on_conceptual.p5 * 100.0,
        assay_on_conceptual.p10 * 100.0,
        assay_on_conceptual.mrr,
    );
    println!(
        "{:<30} {:>7.1}% {:>7.1}% {:>8.3}",
        "scry on factual (cross)",
        scry_on_factual.p5 * 100.0,
        scry_on_factual.p10 * 100.0,
        scry_on_factual.mrr,
    );

    // --- Overall summary ---
    let all_cases: Vec<QueryCase> = assay_cases.into_iter().chain(scry_cases).collect();
    let combined_all = compute_metrics(&all_cases, &combined_fn, "combined (all)");

    println!("\n━━━ Summary ━━━\n");
    print_metrics(&combined_all);

    Ok(())
}
