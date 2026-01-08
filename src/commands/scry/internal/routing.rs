//! Remote and multi-repo routing for scry
//!
//! Handles routing queries to mothership daemon and cross-repo searches.
//!
//! Routing strategies:
//! - **all**: Dumb routing - search ALL repos (current behavior)
//! - **graph**: Smart routing - use mother graph to filter relevant repos

use std::path::Path;

use anyhow::Result;

use patina::mother::{self, EdgeType, Graph};

use crate::commands::persona;

use super::super::{ScryOptions, ScryResult};
use super::enrichment::truncate_content;
use super::logging::{log_scry_query_with_routing, EdgeInfo, RoutedResult, RoutingContext};
use super::search::scry_text;

/// Routing strategy for cross-project queries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoutingStrategy {
    /// Search all repos (dumb routing)
    #[default]
    All,
    /// Use mother graph for smart routing
    Graph,
}

impl RoutingStrategy {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "all" => Some(RoutingStrategy::All),
            "graph" => Some(RoutingStrategy::Graph),
            _ => None,
        }
    }
}

/// Execute scry via mothership daemon
pub fn execute_via_mothership(query: Option<&str>, options: &ScryOptions) -> Result<()> {
    let address = mother::get_address().unwrap_or_else(|| "unknown".to_string());
    println!("🔮 Scry - Querying mothership at {}\n", address);

    // File-based queries not supported via mothership yet
    if options.file.is_some() {
        anyhow::bail!("File-based queries (--file) not supported via mothership. Run locally.");
    }

    let query = query.ok_or_else(|| anyhow::anyhow!("Query text required"))?;
    println!("Query: \"{}\"\n", query);

    // Build request
    let request = mother::ScryRequest {
        query: query.to_string(),
        dimension: options.dimension.clone(),
        repo: options.repo.clone(),
        all_repos: options.all_repos,
        include_issues: options.include_issues,
        include_persona: options.include_persona,
        limit: options.limit,
        min_score: options.min_score,
    };

    // Execute query
    let response = mother::scry(request)?;

    if response.results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    println!("Found {} results:\n", response.count);
    println!("{}", "─".repeat(60));

    for (i, result) in response.results.iter().enumerate() {
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
    }

    println!("\n{}", "─".repeat(60));

    Ok(())
}

/// Execute query across all repos (current project + all reference repos)
pub fn execute_all_repos(query: Option<&str>, options: &ScryOptions) -> Result<()> {
    let query = query.ok_or_else(|| anyhow::anyhow!("Query required for --all-repos"))?;

    println!("Mode: All Repos (cross-project search)\n");
    println!("Query: \"{}\"\n", query);

    let mut all_results: Vec<(String, ScryResult)> = Vec::new();

    // 1. Query current project if we're in one
    let in_project = Path::new(".patina/data/patina.db").exists();
    if in_project {
        println!("📂 Searching current project...");
        let project_options = ScryOptions {
            repo: None,
            all_repos: false,
            ..options.clone()
        };
        match scry_text(query, &project_options) {
            Ok(results) => {
                println!("   Found {} results", results.len());
                for r in results {
                    all_results.push(("[PROJECT]".to_string(), r));
                }
            }
            Err(e) => {
                eprintln!("   ⚠️  Project search failed: {}", e);
            }
        }
    }

    // 2. Query all registered reference repos
    let repos = crate::commands::repo::list()?;
    for repo in repos {
        println!("📚 Searching {}...", repo.name);
        let repo_options = ScryOptions {
            repo: Some(repo.name.clone()),
            all_repos: false,
            ..options.clone()
        };
        match scry_text(query, &repo_options) {
            Ok(results) => {
                println!("   Found {} results", results.len());
                for r in results {
                    all_results.push((format!("[{}]", repo.name.to_uppercase()), r));
                }
            }
            Err(e) => {
                eprintln!("   ⚠️  {} search failed: {}", repo.name, e);
            }
        }
    }

    // 3. Query persona if enabled
    if options.include_persona {
        println!("🧠 Searching persona...");
        if let Ok(persona_results) = persona::query(query, options.limit, options.min_score, None) {
            println!("   Found {} results", persona_results.len());
            for p in persona_results {
                all_results.push((
                    "[PERSONA]".to_string(),
                    ScryResult {
                        id: 0,
                        content: p.content,
                        score: p.score,
                        event_type: p.source.clone(),
                        source_id: p.domains.join(", "),
                        timestamp: p.timestamp,
                    },
                ));
            }
        }
    }

    // 4. Sort by score and take top limit
    all_results.sort_by(|a, b| {
        b.1.score
            .partial_cmp(&a.1.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all_results.truncate(options.limit);

    println!();

    if all_results.is_empty() {
        println!("No results found across any repos.");
        return Ok(());
    }

    println!("Found {} results (combined):\n", all_results.len());
    println!("{}", "─".repeat(60));

    for (i, (source, result)) in all_results.iter().enumerate() {
        let timestamp_display = if result.timestamp.is_empty() {
            String::new()
        } else {
            format!(" | {}", result.timestamp)
        };
        println!(
            "\n[{}] {} Score: {:.3} | {} | {}{}",
            i + 1,
            source,
            result.score,
            result.event_type,
            result.source_id,
            timestamp_display
        );
        println!("    {}", truncate_content(&result.content, 200));
    }

    println!("\n{}", "─".repeat(60));

    Ok(())
}

/// Execute query using graph-based routing
///
/// Smart routing flow:
/// 1. Detect current project from graph
/// 2. Query graph for related nodes (USES, TESTS_WITH, LEARNS_FROM)
/// 3. Filter by domain match if query contains domain terms
/// 4. Execute federated search on project + related repos
/// 5. Weight results by relationship strength
pub fn execute_graph_routing(query: Option<&str>, options: &ScryOptions) -> Result<()> {
    let query = query.ok_or_else(|| anyhow::anyhow!("Query required for graph routing"))?;

    println!("Mode: Graph Routing (smart cross-project search)\n");
    println!("Query: \"{}\"\n", query);

    // 1. Open graph and detect current project
    let graph = Graph::open()?;
    let current_project = detect_current_project(&graph)?;

    println!("📍 Current project: {}", current_project);

    // 2. Get related nodes from graph
    let edge_types = [EdgeType::Uses, EdgeType::TestsWith, EdgeType::LearnsFrom];
    let related_nodes = graph.get_related(&current_project, &edge_types)?;

    if related_nodes.is_empty() {
        println!("⚠️  No related repos in graph. Falling back to current project only.");
        println!("   Tip: Use 'patina mother link' to add relationships.\n");
    } else {
        println!(
            "🔗 Related repos: {}",
            related_nodes
                .iter()
                .map(|n| n.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // 3. Filter by domain match (optional - check if query terms match node domains)
    let query_lower = query.to_lowercase();
    let filtered_nodes: Vec<_> = if should_filter_by_domain(&query_lower) {
        let filtered: Vec<_> = related_nodes
            .iter()
            .filter(|n| node_matches_query_domain(n, &query_lower))
            .collect();

        if !filtered.is_empty() && filtered.len() < related_nodes.len() {
            println!(
                "🎯 Domain filter: {} (matched {} of {} related)",
                filtered
                    .iter()
                    .map(|n| n.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                filtered.len(),
                related_nodes.len()
            );
            filtered.into_iter().cloned().collect()
        } else {
            related_nodes.clone()
        }
    } else {
        related_nodes.clone()
    };

    // Build list of repos to search (for routing context)
    let repos_to_search: Vec<String> = filtered_nodes.iter().map(|n| n.id.clone()).collect();

    // Get edges for weighting
    let edges = graph.get_edges_from(&current_project)?;

    // Build EdgeInfo for routing context (G2.5)
    let edges_used: Vec<EdgeInfo> = edges
        .iter()
        .filter(|e| repos_to_search.contains(&e.to_node))
        .map(|e| EdgeInfo {
            id: e.id,
            from_node: e.from_node.clone(),
            to_node: e.to_node.clone(),
            edge_type: e.edge_type.as_str().to_string(),
            weight: e.weight,
        })
        .collect();

    // Track whether domain filtering was applied
    let domain_filter_applied = filtered_nodes.len() < related_nodes.len();

    println!();

    // 4. Execute federated search
    // Tuple: (source_label, repo_id, weight, result)
    let mut all_results: Vec<(String, String, f32, ScryResult)> = Vec::new();

    // Search current project
    let in_project = Path::new(".patina/data/patina.db").exists();
    if in_project {
        println!("📂 Searching current project...");
        let project_options = ScryOptions {
            repo: None,
            all_repos: false,
            ..options.clone()
        };
        match scry_text(query, &project_options) {
            Ok(results) => {
                println!("   Found {} results", results.len());
                for r in results {
                    // Current project gets weight 1.0 (baseline)
                    all_results.push(("[PROJECT]".to_string(), current_project.clone(), 1.0, r));
                }
            }
            Err(e) => {
                eprintln!("   ⚠️  Project search failed: {}", e);
            }
        }
    }

    // Search related repos
    let repos_searched = repos_to_search.len();

    for repo_id in &repos_to_search {
        println!("📚 Searching {}...", repo_id);
        let repo_options = ScryOptions {
            repo: Some(repo_id.clone()),
            all_repos: false,
            ..options.clone()
        };
        match scry_text(query, &repo_options) {
            Ok(results) => {
                println!("   Found {} results", results.len());

                // 5. Apply relationship weighting
                let weight = get_relationship_weight(&edges, repo_id);

                for r in results {
                    all_results.push((
                        format!("[{}]", repo_id.to_uppercase()),
                        repo_id.clone(),
                        weight,
                        r,
                    ));
                }
            }
            Err(e) => {
                eprintln!("   ⚠️  {} search failed: {}", repo_id, e);
            }
        }
    }

    // Query persona if enabled
    if options.include_persona {
        println!("🧠 Searching persona...");
        if let Ok(persona_results) = persona::query(query, options.limit, options.min_score, None) {
            println!("   Found {} results", persona_results.len());
            for p in persona_results {
                all_results.push((
                    "[PERSONA]".to_string(),
                    "persona".to_string(), // Persona is a special source
                    1.0,                   // Persona gets baseline weight
                    ScryResult {
                        id: 0,
                        content: p.content,
                        score: p.score,
                        event_type: p.source.clone(),
                        source_id: p.domains.join(", "),
                        timestamp: p.timestamp,
                    },
                ));
            }
        }
    }

    // Sort by weighted score and take top limit
    // Tuple: (source_label, repo_id, weight, result)
    all_results.sort_by(|a, b| {
        let weighted_a = a.3.score * a.2;
        let weighted_b = b.3.score * b.2;
        weighted_b
            .partial_cmp(&weighted_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all_results.truncate(options.limit);

    // Build routing context for logging (G2.5)
    let total_repos = crate::commands::repo::list().map(|r| r.len()).unwrap_or(0);
    let routing_context = RoutingContext {
        strategy: "graph".to_string(),
        source_project: current_project.clone(),
        edges_used,
        repos_searched: repos_to_search.clone(),
        repos_available: total_repos + 1, // +1 for current project
        domain_filter_applied,
    };

    // Convert to RoutedResult for logging
    let routed_results: Vec<RoutedResult> = all_results
        .iter()
        .map(|(_, repo_id, weight, result)| RoutedResult {
            source_repo: repo_id.clone(),
            weight: *weight,
            result: result.clone(),
        })
        .collect();

    // Log query with routing context (G2.5)
    let query_id = log_scry_query_with_routing(query, &routed_results, &routing_context);

    // Record edge usage for each edge that contributed (G2.5)
    if let Some(ref qid) = query_id {
        for edge in &routing_context.edges_used {
            // Find best rank for this edge's target repo in results
            let best_rank = routed_results
                .iter()
                .enumerate()
                .find(|(_, r)| r.source_repo == edge.to_node)
                .map(|(i, _)| i + 1);

            // Record usage (best-effort, don't fail on error)
            let _ = graph.record_edge_usage(edge.id, qid, &edge.to_node, best_rank);
        }
    }

    println!();

    if all_results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    // Report routing efficiency
    println!(
        "Found {} results (searched {} of {} repos):\n",
        all_results.len(),
        repos_searched + 1, // +1 for current project
        total_repos + 1
    );
    println!("{}", "─".repeat(60));

    for (i, (source, _repo_id, weight, result)) in all_results.iter().enumerate() {
        let timestamp_display = if result.timestamp.is_empty() {
            String::new()
        } else {
            format!(" | {}", result.timestamp)
        };
        let weight_display = if (*weight - 1.0).abs() > 0.01 {
            format!(" (w={:.2})", weight)
        } else {
            String::new()
        };
        println!(
            "\n[{}] {} Score: {:.3}{} | {} | {}{}",
            i + 1,
            source,
            result.score,
            weight_display,
            result.event_type,
            result.source_id,
            timestamp_display
        );
        println!("    {}", truncate_content(&result.content, 200));
    }

    println!("\n{}", "─".repeat(60));

    // Show query_id for feedback commands
    if let Some(ref qid) = query_id {
        println!("\nQuery ID: {} (use with 'scry open/copy/feedback')", qid);
    }

    Ok(())
}

/// Detect current project from graph
///
/// Looks up the current working directory in the graph nodes.
/// Falls back to directory name if not found.
fn detect_current_project(graph: &Graph) -> Result<String> {
    let cwd = std::env::current_dir()?;

    // Try to find a node matching the current directory
    let nodes = graph.list_nodes()?;
    for node in &nodes {
        if node.path == cwd {
            return Ok(node.id.clone());
        }
    }

    // Fallback: use directory name
    let dir_name = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Check if this name exists in graph
    if graph.get_node(dir_name)?.is_some() {
        return Ok(dir_name.to_string());
    }

    // Return directory name even if not in graph (will have no edges)
    Ok(dir_name.to_string())
}

/// Check if query should trigger domain filtering
///
/// Returns true if query contains domain-specific terms that could narrow results.
fn should_filter_by_domain(query: &str) -> bool {
    // Domain keywords that suggest filtering would help
    let domain_hints = [
        "cairo",
        "rust",
        "typescript",
        "javascript",
        "python",
        "go",
        "java",
        "c++",
        "cpp",
        "solidity",
        "prolog",
        "dojo",
        "starknet",
        "mcp",
        "ecs",
        "vector",
        "embedding",
    ];

    domain_hints.iter().any(|hint| query.contains(hint))
}

/// Check if a node's domains match query terms
fn node_matches_query_domain(node: &mother::Node, query: &str) -> bool {
    // Check if any domain matches query
    for domain in &node.domains {
        if query.contains(&domain.to_lowercase()) {
            return true;
        }
    }

    // Check if node ID matches query (e.g., "dojo" in "how does dojo handle...")
    if query.contains(&node.id.to_lowercase()) {
        return true;
    }

    false
}

/// Get relationship weight for a repo based on edge type
///
/// Weighting rationale (from G0 error analysis):
/// - TESTS_WITH: High relevance for benchmark/testing queries
/// - LEARNS_FROM: Medium relevance for pattern/implementation queries
/// - USES: Medium relevance for dependency queries
/// - Default: Baseline weight
fn get_relationship_weight(edges: &[mother::Edge], repo_id: &str) -> f32 {
    for edge in edges {
        if edge.to_node == repo_id {
            return match edge.edge_type {
                EdgeType::TestsWith => 1.2,  // Boost test subjects
                EdgeType::LearnsFrom => 1.1, // Slight boost for learning sources
                EdgeType::Uses => 1.1,       // Slight boost for dependencies
                EdgeType::Sibling => 1.0,    // Baseline for siblings
                EdgeType::Domain => 1.0,     // Baseline for domain connections
            };
        }
    }
    1.0 // Default weight
}
