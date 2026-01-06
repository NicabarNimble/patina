//! Internal implementation for mother command
//!
//! Syncs graph from registry, manages edges.

use anyhow::{bail, Result};
use std::path::Path;

use patina::mother::{EdgeType, Graph, NodeType, MIN_SAMPLES};

use crate::commands::repo::internal::Registry;

/// Sync graph nodes from registry
///
/// Creates nodes for all projects and repos in ~/.patina/registry.yaml.
/// Also adds the current project if we're in a patina project directory.
pub fn sync_from_registry() -> Result<()> {
    println!("🔄 Syncing graph from registry...\n");

    let registry = Registry::load()?;
    let graph = Graph::open()?;

    let mut projects_added = 0;
    let mut repos_added = 0;

    // Add current project if we're in one
    if let Ok(project_root) = patina::session::SessionManager::find_project_root() {
        let project_name = project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Detect domains from project (simple heuristic)
        let domains = detect_project_domains(&project_root);

        graph.add_node(project_name, NodeType::Project, &project_root, &domains)?;
        projects_added += 1;
        println!("  + {} (current project)", project_name);
    }

    // Add registered projects
    for (name, entry) in &registry.projects {
        let path = Path::new(&entry.path);
        graph.add_node(name, NodeType::Project, path, &entry.domains)?;
        projects_added += 1;
        println!("  + {} (project)", name);
    }

    // Add repos
    for (name, entry) in &registry.repos {
        let path = Path::new(&entry.path);
        graph.add_node(name, NodeType::Reference, path, &entry.domains)?;
        repos_added += 1;
        println!("  + {} (reference)", name);
    }

    println!();
    println!(
        "✅ Synced {} projects, {} repos",
        projects_added, repos_added
    );
    println!(
        "   Graph: {} nodes, {} edges",
        graph.node_count()?,
        graph.edge_count()?
    );

    Ok(())
}

/// Detect project domains from file extensions
fn detect_project_domains(project_root: &Path) -> Vec<String> {
    let mut domains = Vec::new();

    // Check for Cargo.toml → rust
    if project_root.join("Cargo.toml").exists() {
        domains.push("rust".to_string());
    }
    // Check for package.json → javascript/typescript
    if project_root.join("package.json").exists() {
        domains.push("javascript".to_string());
    }
    // Check for Scarb.toml → cairo
    if project_root.join("Scarb.toml").exists() {
        domains.push("cairo".to_string());
    }

    domains
}

/// Show graph state
pub fn show_graph(nodes_only: bool, edges_only: bool) -> Result<()> {
    let graph = Graph::open()?;

    let node_count = graph.node_count()?;
    let edge_count = graph.edge_count()?;

    // Check if empty
    if node_count == 0 {
        println!("📊 Graph is empty.\n");
        println!("Run 'patina mother sync' to populate from registry.");
        return Ok(());
    }

    println!("📊 Mother Graph\n");
    println!("   Nodes: {}  Edges: {}\n", node_count, edge_count);

    // Show nodes
    if !edges_only {
        let nodes = graph.list_nodes()?;

        println!("┌─ Nodes ────────────────────────────────────────────────────┐");
        println!("│ {:<20} {:<12} {:<30} │", "ID", "TYPE", "DOMAINS");
        println!("├────────────────────────────────────────────────────────────┤");

        for node in &nodes {
            let type_str = match node.node_type {
                NodeType::Project => "project",
                NodeType::Reference => "reference",
            };
            let domains = if node.domains.is_empty() {
                "-".to_string()
            } else {
                node.domains.join(", ")
            };
            // Truncate domains if too long
            let domains_display = if domains.len() > 28 {
                format!("{}...", &domains[..25])
            } else {
                domains
            };
            println!(
                "│ {:<20} {:<12} {:<30} │",
                truncate(&node.id, 20),
                type_str,
                domains_display
            );
        }
        println!("└────────────────────────────────────────────────────────────┘");
    }

    // Show edges
    if !nodes_only {
        let edges = graph.list_edges()?;

        if edges.is_empty() {
            if !edges_only {
                println!();
            }
            println!("No edges defined yet.");
            println!("\nAdd relationships with:");
            println!("  patina mother link <from> <to> <TYPE>");
            println!("\nEdge types: USES, LEARNS_FROM, TESTS_WITH, SIBLING, DOMAIN");
        } else {
            println!();
            println!("┌─ Edges ────────────────────────────────────────────────────┐");
            println!(
                "│ {:<15} {:<15} {:<15} {:<15} │",
                "FROM", "TO", "TYPE", "EVIDENCE"
            );
            println!("├────────────────────────────────────────────────────────────┤");

            for edge in &edges {
                let evidence = edge.evidence.as_deref().unwrap_or("-");
                println!(
                    "│ {:<15} {:<15} {:<15} {:<15} │",
                    truncate(&edge.from_node, 15),
                    truncate(&edge.to_node, 15),
                    edge.edge_type.as_str(),
                    truncate(evidence, 15)
                );
            }
            println!("└────────────────────────────────────────────────────────────┘");
        }
    }

    Ok(())
}

/// Add a relationship between nodes
pub fn add_link(from: &str, to: &str, edge_type_str: &str, evidence: Option<&str>) -> Result<()> {
    let graph = Graph::open()?;

    // Parse edge type
    let edge_type = EdgeType::parse(edge_type_str).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown edge type: '{}'. Valid types: USES, LEARNS_FROM, TESTS_WITH, SIBLING, DOMAIN",
            edge_type_str
        )
    })?;

    // Check nodes exist
    if graph.get_node(from)?.is_none() {
        bail!("Node '{}' not found. Run 'patina mother sync' first.", from);
    }
    if graph.get_node(to)?.is_none() {
        bail!("Node '{}' not found. Run 'patina mother sync' first.", to);
    }

    // Add edge
    graph.add_edge(from, to, edge_type, evidence)?;

    println!("✅ Added: {} {} {}", from, edge_type.as_str(), to);
    if let Some(ev) = evidence {
        println!("   Evidence: {}", ev);
    }

    Ok(())
}

/// Remove a relationship
pub fn remove_link(from: &str, to: &str, edge_type_str: &str) -> Result<()> {
    let graph = Graph::open()?;

    // Parse edge type
    let edge_type = EdgeType::parse(edge_type_str).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown edge type: '{}'. Valid types: USES, LEARNS_FROM, TESTS_WITH, SIBLING, DOMAIN",
            edge_type_str
        )
    })?;

    // Remove edge
    let removed = graph.remove_edge(from, to, edge_type)?;

    if removed {
        println!("✅ Removed: {} {} {}", from, edge_type.as_str(), to);
    } else {
        println!("⚠️  Edge not found: {} {} {}", from, edge_type.as_str(), to);
    }

    Ok(())
}

/// Truncate string with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        format!("{}...", &s[..max_len - 3])
    } else {
        s[..max_len].to_string()
    }
}

/// Learn edge weights from usage data
pub fn learn_weights(alpha: f32) -> Result<()> {
    let graph = Graph::open()?;

    println!(
        "📈 Learning edge weights (α={:.2}, min_samples={})\n",
        alpha, MIN_SAMPLES
    );

    let report = graph.learn_weights(alpha)?;

    if report.edges_updated == 0 && report.edges_skipped_insufficient == 0 {
        println!("   No edges in graph. Run 'patina mother sync' first.");
        return Ok(());
    }

    println!(
        "   Updated: {} edge{}",
        report.edges_updated,
        if report.edges_updated == 1 { "" } else { "s" }
    );
    println!(
        "   Skipped: {} edge{} (insufficient data)",
        report.edges_skipped_insufficient,
        if report.edges_skipped_insufficient == 1 {
            ""
        } else {
            "s"
        }
    );

    if !report.changes.is_empty() {
        println!("\n   Changes:");
        for change in &report.changes {
            let pct_change = if change.old_weight != 0.0 {
                ((change.new_weight - change.old_weight) / change.old_weight) * 100.0
            } else {
                0.0
            };

            let sign = if pct_change >= 0.0 { "+" } else { "" };

            println!(
                "     {} → {} ({}): {:.2} → {:.2} ({}{:.1}%, precision={:.0}%)",
                change.from_node,
                change.to_node,
                change.edge_type.as_str(),
                change.old_weight,
                change.new_weight,
                sign,
                pct_change,
                change.precision * 100.0
            );
        }
    }

    println!();
    if report.edges_skipped_insufficient > 0 {
        println!("   Need {} more uses per edge to enable learning.", MIN_SAMPLES);
        println!("   Use 'patina scry --routing graph' and act on results.");
    }

    Ok(())
}

/// Show edge usage statistics
pub fn show_stats() -> Result<()> {
    let graph = Graph::open()?;
    let stats = graph.get_all_usage_stats()?;

    if stats.is_empty() {
        println!("📊 Edge Usage Statistics\n");
        println!("   No usage data yet.\n");
        println!("   Usage is recorded when:");
        println!("   1. scry queries use --routing graph");
        println!("   2. Users act on results (scry use <query_id> <rank>)");
        return Ok(());
    }

    println!("📊 Edge Usage Statistics\n");
    println!("┌────────────────────────────────────────────────────────────────────────────┐");
    println!(
        "│ {:<30} {:>8} {:>8} {:>8} {:>10} {:>8} │",
        "EDGE", "USES", "USEFUL", "PREC%", "WEIGHT", "STATUS"
    );
    println!("├────────────────────────────────────────────────────────────────────────────┤");

    let mut total_uses = 0;
    let mut total_useful = 0;

    for stat in &stats {
        total_uses += stat.total_uses;
        total_useful += stat.useful_uses;

        let edge_label = format!(
            "{} → {} ({})",
            stat.from_node,
            stat.to_node,
            stat.edge_type.as_str()
        );

        let precision = if stat.total_uses > 0 {
            (stat.useful_uses as f32 / stat.total_uses as f32) * 100.0
        } else {
            0.0
        };

        let status = if stat.total_uses >= MIN_SAMPLES {
            "ready"
        } else {
            "needs data"
        };

        println!(
            "│ {:<30} {:>8} {:>8} {:>7.1}% {:>10.2} {:>8} │",
            truncate(&edge_label, 30),
            stat.total_uses,
            stat.useful_uses,
            precision,
            stat.current_weight,
            status
        );
    }

    println!("├────────────────────────────────────────────────────────────────────────────┤");

    let overall_precision = if total_uses > 0 {
        (total_useful as f32 / total_uses as f32) * 100.0
    } else {
        0.0
    };

    println!(
        "│ {:<30} {:>8} {:>8} {:>7.1}%                    │",
        "TOTAL", total_uses, total_useful, overall_precision
    );
    println!("└────────────────────────────────────────────────────────────────────────────┘");

    println!();
    println!("   Edges with {} or more uses are 'ready' for weight learning.", MIN_SAMPLES);
    println!("   Run 'patina mother learn' to update weights from usage data.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("hi", 2), "hi");
    }
}
