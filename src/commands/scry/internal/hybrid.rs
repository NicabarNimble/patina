//! Hybrid search using RRF fusion
//!
//! Combines multiple search oracles (semantic, lexical, temporal, etc.)
//! using Reciprocal Rank Fusion (RRF) for better results.

use anyhow::Result;

use crate::retrieval::{QueryEngine, QueryOptions};

use super::super::{ScryOptions, ScryResult};
use super::enrichment::truncate_content;
use super::logging::log_scry_query;

/// Execute hybrid search using QueryEngine with RRF fusion
pub fn execute_hybrid(query: Option<&str>, options: &ScryOptions) -> Result<()> {
    let query = query.ok_or_else(|| anyhow::anyhow!("Query text required"))?;

    println!("Mode: Hybrid (RRF fusion of all oracles)\n");
    println!("Query: \"{}\"\n", query);

    let engine = QueryEngine::new();

    // Show available oracles
    let available = engine.available_oracles();
    println!("Oracles: {}\n", available.join(", "));

    // Build query options
    let query_opts = QueryOptions {
        repo: options.repo.clone(),
        all_repos: options.all_repos,
        include_issues: options.include_issues,
    };

    let results = engine.query_with_options(query, options.limit, &query_opts)?;

    // Log query for feedback loop (Phase 3) - convert at boundary
    let log_results: Vec<ScryResult> = results
        .iter()
        .map(|r| ScryResult {
            id: 0,
            source_id: r.doc_id.clone(),
            score: r.fused_score,
            event_type: r.metadata.event_type.clone().unwrap_or_default(),
            content: r.content.clone(),
            timestamp: String::new(),
        })
        .collect();
    let query_id = log_scry_query(query, "hybrid", &log_results);

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    println!("Found {} results:\n", results.len());
    println!("{}", "─".repeat(60));

    for (i, result) in results.iter().enumerate() {
        let event_type = result.metadata.event_type.as_deref().unwrap_or("unknown");
        let source_tag = if result.sources.contains(&"persona") {
            "[PERSONA] "
        } else {
            ""
        };

        if options.explain {
            // Detailed output with per-oracle contributions
            println!(
                "\n{}. {}{} ({})",
                i + 1,
                source_tag,
                result.doc_id,
                event_type
            );

            // Show each oracle's contribution
            for (oracle_name, contrib) in &result.contributions {
                let score_display = match contrib.score_type {
                    "co_change_count" => format!("co-changes: {}", contrib.raw_score as i32),
                    "bm25" => format!("{:.1} BM25", contrib.raw_score),
                    _ => format!("{:.2} {}", contrib.raw_score, contrib.score_type),
                };

                let matches_display = if let Some(ref matches) = contrib.matches {
                    if !matches.is_empty() {
                        format!(" matched: {}", matches.join(", "))
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                println!(
                    "   {:>8}: #{} ({}){}",
                    oracle_name, contrib.rank, score_display, matches_display
                );
            }

            // Show structural annotations if available
            let ann = &result.annotations;
            if ann.importer_count.is_some() || ann.activity_level.is_some() {
                let mut parts = Vec::new();
                if let Some(count) = ann.importer_count {
                    parts.push(format!("{} importers", count));
                }
                if let Some(ref level) = ann.activity_level {
                    parts.push(format!("{} activity", level));
                }
                if let Some(true) = ann.is_entry_point {
                    parts.push("entry_point".to_string());
                }
                if let Some(true) = ann.is_test_file {
                    parts.push("test".to_string());
                }
                if !parts.is_empty() {
                    println!("   Structural: {}", parts.join(", "));
                }
            }

            println!("   Content: {}", truncate_content(&result.content, 150));
        } else {
            // Default concise output with ranks
            let mut contributions_str: String = result
                .contributions
                .iter()
                .map(|(name, c)| format!("{} #{}", &name[..3.min(name.len())], c.rank))
                .collect::<Vec<_>>()
                .join(" | ");

            // Add importer count if available
            if let Some(count) = result.annotations.importer_count {
                if count > 0 {
                    contributions_str.push_str(&format!(" | imp {}", count));
                }
            }

            println!(
                "\n[{}] {}{} (score: {:.3}) ({})",
                i + 1,
                source_tag,
                result.doc_id,
                result.fused_score,
                contributions_str
            );
            println!("    {}", truncate_content(&result.content, 200));
        }
    }

    println!("\n{}", "─".repeat(60));

    // Show query_id for feedback commands
    if let Some(ref qid) = query_id {
        println!("\nQuery ID: {} (use with 'scry open/copy/feedback')", qid);
    }

    Ok(())
}
