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
    /// Dimension override for eval/ablation testing (no CLI flag — oracles auto-detect)
    pub dimension: Option<String>,
    pub file: Option<String>,
    pub repo: Option<String>,
    pub all_repos: bool,
    pub include_issues: bool,
    pub include_persona: bool,
    pub explain: bool,
    /// Routing strategy for cross-project queries (default: All)
    pub routing: RoutingStrategy,
    /// Belief ID for belief-grounding queries (E4.6a)
    pub belief: Option<String>,
    /// Content type filter for belief queries: code, commits, sessions, patterns, beliefs
    pub content_type: Option<String>,
    /// Show belief impact for code results — which beliefs are semantically close (E4.6a)
    pub impact: bool,
    /// Use legacy single-oracle search (deprecated, removed in v0.12.0)
    pub legacy: bool,
}

impl Default for ScryOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            min_score: 0.0,
            dimension: None,
            file: None,
            repo: None,
            all_repos: false,
            include_issues: false,
            include_persona: true, // Include persona by default
            explain: false,
            routing: RoutingStrategy::default(),
            belief: None,
            content_type: None,
            impact: false,
            legacy: false,
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

    // Handle special modes that bypass QueryEngine
    match (&options.belief, &options.file) {
        (Some(belief_id), _) => {
            println!("Belief: {}", belief_id);
            if let Some(ref ct) = options.content_type {
                println!("Filter: {} only", ct);
            }
            println!();
            return execute_legacy_belief(belief_id, &options);
        }
        (_, Some(file)) => {
            println!("File: {}\n", file);
            return execute_legacy_file(file, &options);
        }
        _ => {}
    }

    // Require query text for default search
    if query.is_none() {
        anyhow::bail!("Either a query text, --file, or --belief must be provided");
    }

    // --legacy: deprecated single-oracle path (removed in v0.12.0)
    if options.legacy {
        eprintln!("⚠️  --legacy is deprecated and will be removed in v0.12.0");
        return execute_legacy_search(query, &options);
    }

    // Default: QueryEngine with all oracles + RRF fusion
    execute_hybrid(query, &options)
}

/// Legacy belief grounding query (specialized, not changing in D0)
fn execute_legacy_belief(belief_id: &str, options: &ScryOptions) -> Result<()> {
    let results = scry_belief(belief_id, options)?;
    display_legacy_results(None, &results, options)
}

/// Legacy file co-change query (specialized, not changing in D0)
fn execute_legacy_file(file: &str, options: &ScryOptions) -> Result<()> {
    let results = scry_file(file, options)?;
    display_legacy_results(None, &results, options)
}

/// Legacy single-oracle search (deprecated, behind --legacy flag)
fn execute_legacy_search(query: Option<&str>, options: &ScryOptions) -> Result<()> {
    let q = query.ok_or_else(|| anyhow::anyhow!("Query required"))?;
    println!("Query: \"{}\"\n", q);

    let mut results = if is_lexical_query(q) {
        println!("Mode: Lexical (FTS5)\n");
        internal::search::scry_lexical(q, options)?
    } else {
        println!("Mode: Semantic (vector)\n");
        scry_text(q, options)?
    };

    // Bolt on persona results
    if options.include_persona {
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

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(options.limit);

    display_legacy_results(query, &results, options)
}

/// Display results in legacy ScryResult format
fn display_legacy_results(
    query: Option<&str>,
    results: &[ScryResult],
    options: &ScryOptions,
) -> Result<()> {
    let query_id = if let Some(q) = query {
        log_scry_query(q, "legacy", results)
    } else {
        None
    };

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    println!("Found {} results:\n", results.len());
    println!("{}", "─".repeat(60));

    let impact_map = if options.impact {
        find_belief_impact(results).unwrap_or_default()
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

        if let Some(beliefs) = impact_map.get(&result.source_id) {
            let belief_strs: Vec<String> = beliefs
                .iter()
                .map(|(id, score)| format!("{} ({:.2})", id, score))
                .collect();
            println!("    beliefs: {}", belief_strs.join(", "));
        }
    }

    println!("\n{}", "─".repeat(60));

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
        assert!(opts.include_persona); // Persona enabled by default
        assert!(!opts.legacy); // Legacy off by default
    }
}
