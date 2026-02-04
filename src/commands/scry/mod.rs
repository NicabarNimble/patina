//! Scry command - Query knowledge using vector search
//!
//! Unified query interface for searching project knowledge.
//! Phase 2.5b: MVP implementation for validating retrieval quality.
//!
//! # Remote Execution
//! If `PATINA_MOTHER` is set, queries are routed to a remote daemon.
//! This enables containers to query the Mac mother.

pub mod internal;

use anyhow::Result;
use patina::mother;

use crate::commands::persona;

use internal::enrichment::{find_belief_impact, truncate_content};
use internal::hybrid::execute_hybrid;
use internal::logging::log_scry_query;
use internal::routing::{execute_all_repos, execute_graph_routing, execute_via_mother};
use internal::search::{is_lexical_query, scry_belief, scry_file};

// Re-export routing strategy for CLI
pub use internal::routing::RoutingStrategy;

// Re-export subcommands for CLI
pub use internal::subcommands::{
    execute_copy, execute_feedback, execute_open, execute_orient, execute_recent, execute_why,
};

// Re-export search functions for external use
pub use internal::search::scry_belief as scry_belief_fn;
pub use internal::search::{scry, scry_lexical, scry_text};

/// Result from a scry query
#[derive(Debug, Clone)]
pub struct ScryResult {
    pub id: i64,
    pub content: String,
    pub score: f32,
    pub event_type: String,
    pub source_id: String,
    pub timestamp: String,
}

/// Options for scry query
#[derive(Debug, Clone)]
pub struct ScryOptions {
    pub limit: usize,
    pub min_score: f32,
    pub dimension: Option<String>,
    pub file: Option<String>,
    pub repo: Option<String>,
    pub all_repos: bool,
    pub include_issues: bool,
    pub include_persona: bool,
    pub hybrid: bool,
    pub explain: bool,
    /// Force lexical (FTS5) search mode, bypassing auto-detection heuristics
    pub lexical: bool,
    /// Routing strategy for cross-project queries (default: All)
    pub routing: RoutingStrategy,
    /// Belief ID for belief-grounding queries (E4.6a)
    pub belief: Option<String>,
    /// Content type filter for belief queries: code, commits, sessions, patterns, beliefs
    pub content_type: Option<String>,
    /// Show belief impact for code results — which beliefs are semantically close (E4.6a)
    pub impact: bool,
}

impl Default for ScryOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            min_score: 0.0,
            dimension: None, // Use semantic by default
            file: None,
            repo: None,
            all_repos: false,
            include_issues: false,
            include_persona: true, // Include persona by default
            hybrid: false,
            explain: false,
            lexical: false,
            routing: RoutingStrategy::default(),
            belief: None,
            content_type: None,
            impact: false,
        }
    }
}

/// Execute scry command
pub fn execute(query: Option<&str>, options: ScryOptions) -> Result<()> {
    // Check if we should route to mother
    if mother::is_configured() {
        return execute_via_mother(query, &options);
    }

    println!("🔮 Scry - Searching knowledge base\n");

    // Handle cross-project routing modes
    if options.all_repos {
        // Check routing strategy
        match options.routing {
            RoutingStrategy::Graph => {
                return execute_graph_routing(query, &options);
            }
            RoutingStrategy::All => {
                return execute_all_repos(query, &options);
            }
        }
    }

    // Handle --hybrid mode (uses QueryEngine with RRF fusion)
    if options.hybrid {
        return execute_hybrid(query, &options);
    }

    // Show repo context if specified
    if let Some(ref repo) = options.repo {
        println!("Repo: {}", repo);
    }

    // Show if including issues
    if options.include_issues {
        println!("Including: GitHub issues");
    }
    println!();

    // Determine query mode
    let mut results = match (&options.belief, &options.file, query) {
        (Some(belief_id), _, _) => {
            println!("Belief: {}", belief_id);
            if let Some(ref ct) = options.content_type {
                println!("Filter: {} only", ct);
            }
            println!();
            scry_belief(belief_id, &options)?
        }
        (None, Some(file), _) => {
            println!("File: {}\n", file);
            scry_file(file, &options)?
        }
        (None, None, Some(q)) => {
            // Escape hatches: --lexical and --dimension force single-oracle paths
            if options.lexical {
                println!("Query: \"{}\"\n", q);
                println!("Mode: Lexical (FTS5) [forced]\n");
                internal::search::scry_lexical(q, &options)?
            } else if options.dimension.is_some() {
                println!("Query: \"{}\"\n", q);
                println!(
                    "Mode: Vector ({} dimension)\n",
                    options.dimension.as_deref().unwrap()
                );
                scry_text(q, &options)?
            } else {
                // Default: QueryEngine with all oracles + RRF fusion
                return execute_hybrid(query, &options);
            }
        }
        (None, None, None) => {
            anyhow::bail!("Either a query text, --file, or --belief must be provided");
        }
    };

    // Query persona if enabled and we have a text query
    if options.include_persona {
        if let Some(q) = query {
            if let Ok(persona_results) = persona::query(q, options.limit, options.min_score, None) {
                for p in persona_results {
                    results.push(ScryResult {
                        id: 0,
                        content: p.content,
                        score: p.score,
                        event_type: "[PERSONA]".to_string(),
                        source_id: format!("{} ({})", p.source, p.domains.join(", ")),
                        timestamp: p.timestamp,
                    });
                }
            }
        }
    }

    // Sort combined results by score
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(options.limit);

    // Log query for feedback loop (Phase 3)
    let query_id = if let Some(q) = query {
        let mode = if options.lexical {
            "lexical"
        } else if options.dimension.is_some() {
            options.dimension.as_deref().unwrap_or("semantic")
        } else if is_lexical_query(q) {
            "lexical"
        } else {
            "semantic"
        };
        log_scry_query(q, mode, &results)
    } else {
        None
    };

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    println!("Found {} results:\n", results.len());
    println!("{}", "─".repeat(60));

    // E4.6a step 4: Compute belief impact for code results
    let impact_map = if options.impact {
        find_belief_impact(&results).unwrap_or_default()
    } else {
        Default::default()
    };

    for (i, result) in results.iter().enumerate() {
        let timestamp_display = if result.timestamp.is_empty() {
            String::new()
        } else {
            format!(" | {}", result.timestamp)
        };
        println!(
            "\n[{}] Score: {:.3} | {} | {}{}",
            i + 1,
            result.score,
            result.event_type,
            result.source_id,
            timestamp_display
        );
        println!("    {}", truncate_content(&result.content, 200));

        // Show belief impact for code results
        if let Some(beliefs) = impact_map.get(&result.source_id) {
            let belief_strs: Vec<String> = beliefs
                .iter()
                .map(|(id, score)| format!("{} ({:.2})", id, score))
                .collect();
            println!("    beliefs: {}", belief_strs.join(", "));
        }
    }

    println!("\n{}", "─".repeat(60));

    // Show query_id for feedback commands
    if let Some(ref qid) = query_id {
        println!("\nQuery ID: {} (use with 'scry open/copy/feedback')", qid);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let opts = ScryOptions::default();
        assert_eq!(opts.limit, 10);
        assert_eq!(opts.min_score, 0.0);
        assert!(opts.dimension.is_none());
        assert!(opts.include_persona); // Persona enabled by default
    }
}
