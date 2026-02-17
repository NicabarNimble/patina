//! Internal implementation for mother command
//!
//! Syncs graph from registry, manages edges.

use anyhow::{bail, Result};
use std::path::Path;

use patina::mother::{BeliefEntry, EdgeType, Graph, NodeType, MIN_SAMPLES};
use patina::paths;

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

    // =========================================================================
    // Knowledge sync: collect beliefs from projects + persona values
    // =========================================================================

    println!();
    println!("📚 Syncing knowledge...\n");

    let mut knowledge: Vec<BeliefEntry> = Vec::new();
    let mut synced_sources: Vec<String> = Vec::new();
    let mut beliefs_synced = 0;
    let mut values_synced = 0;

    // Collect beliefs from current project (auto-detected, may not be in registry)
    if let Ok(project_root) = patina::session::SessionManager::find_project_root() {
        let project_name = project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let db_path = project_root.join(".patina/local/data/patina.db");
        match collect_project_beliefs(project_name, &db_path) {
            Ok(entries) => {
                let count = entries.len();
                beliefs_synced += count;
                synced_sources.push(project_name.to_string());
                if count > 0 {
                    println!("  + {} beliefs from {} (current)", count, project_name);
                }
                knowledge.extend(entries);
            }
            Err(e) => {
                // Failed sources are NOT added to synced_sources,
                // so their previously indexed data is preserved.
                eprintln!("  ⚠ {} (current): {}", project_name, e);
            }
        }
    }

    // For each registered project, try to open patina.db and read beliefs
    for (name, entry) in &registry.projects {
        let db_path = Path::new(&entry.path).join(".patina/local/data/patina.db");
        match collect_project_beliefs(name, &db_path) {
            Ok(entries) => {
                let count = entries.len();
                beliefs_synced += count;
                synced_sources.push(name.clone());
                if count > 0 {
                    println!("  + {} beliefs from {}", count, name);
                }
                knowledge.extend(entries);
            }
            Err(e) => {
                eprintln!("  ⚠ {}: {}", name, e);
            }
        }
    }

    // Ref repos: skip knowledge sync entirely (per SPEC)

    // Read persona values from ~/.patina/layer/surface/beliefs/
    match collect_persona_values() {
        Ok(entries) => {
            values_synced = entries.len();
            synced_sources.push("persona".to_string());
            if values_synced > 0 {
                println!("  + {} values from persona", values_synced);
            }
            knowledge.extend(entries);
        }
        Err(e) => {
            eprintln!("  ⚠ persona: {}", e);
        }
    }

    // Sync knowledge — only rebuilds entries for successfully collected sources.
    // Failed sources retain their previously indexed data.
    graph.sync_beliefs(&knowledge, &synced_sources)?;

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
    println!(
        "   Knowledge: {} beliefs + {} values = {} total",
        beliefs_synced,
        values_synced,
        graph.belief_count()?
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

/// Collect beliefs from a project's patina.db
///
/// Opens the project's patina.db and reads the beliefs table (12 columns).
/// Returns empty vec with warning on missing db or missing table.
fn collect_project_beliefs(project_name: &str, db_path: &Path) -> Result<Vec<BeliefEntry>> {
    use rusqlite::Connection;

    if !db_path.exists() {
        anyhow::bail!("no patina.db (not yet scraped)");
    }

    let conn = Connection::open(db_path)?;

    // Check if beliefs table exists (might be legacy schema)
    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='beliefs'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !table_exists {
        anyhow::bail!("no beliefs table — run `patina scrape --rebuild`");
    }

    let mut stmt = conn.prepare(
        "SELECT id, statement, entrenchment, status, facets,
                cited_by_beliefs, cited_by_sessions, applied_in,
                evidence_count, evidence_verified, health_score, contested_by
         FROM beliefs WHERE status != 'archived'",
    )?;

    let entries: Vec<BeliefEntry> = stmt
        .query_map([], |row| {
            Ok(BeliefEntry {
                id: row.get(0)?,
                source: project_name.to_string(),
                kind: "belief".to_string(),
                statement: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                entrenchment: row
                    .get::<_, Option<String>>(2)?
                    .unwrap_or_else(|| "medium".to_string()),
                status: row
                    .get::<_, Option<String>>(3)?
                    .unwrap_or_else(|| "active".to_string()),
                facets: row
                    .get::<_, Option<String>>(4)?
                    .unwrap_or_else(|| "[]".to_string()),
                cited_by_beliefs: row.get::<_, Option<i32>>(5)?.unwrap_or(0),
                cited_by_sessions: row.get::<_, Option<i32>>(6)?.unwrap_or(0),
                applied_in: row.get::<_, Option<i32>>(7)?.unwrap_or(0),
                evidence_count: row.get::<_, Option<i32>>(8)?.unwrap_or(0),
                evidence_verified: row.get::<_, Option<i32>>(9)?.unwrap_or(0),
                health_score: row.get::<_, Option<f64>>(10)?.unwrap_or(0.0),
                contested_by: row
                    .get::<_, Option<String>>(11)?
                    .unwrap_or_default(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

/// Collect persona values from ~/.patina/layer/surface/beliefs/*.md
///
/// Parses YAML frontmatter for id, statement, entrenchment, status, facets.
/// Required: id (from filename if missing) + statement (first non-empty line after heading).
/// Malformed files: warn to stderr, skip, continue.
fn collect_persona_values() -> Result<Vec<BeliefEntry>> {
    let beliefs_dir = paths::user_layer::beliefs_dir();

    if !beliefs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();

    let dir_entries: Vec<_> = std::fs::read_dir(&beliefs_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .collect();

    for dir_entry in dir_entries {
        let path = dir_entry.path();
        match parse_persona_value(&path) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                eprintln!(
                    "  ⚠ persona file {}: {}",
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    e
                );
            }
        }
    }

    Ok(entries)
}

/// Parse a single persona value markdown file
fn parse_persona_value(path: &Path) -> Result<BeliefEntry> {
    let content = std::fs::read_to_string(path)?;

    // Split frontmatter from body
    let (frontmatter, body) = if let Some(stripped) = content.strip_prefix("---") {
        if let Some(end) = stripped.find("---") {
            let fm = &stripped[..end];
            let body = &stripped[end + 3..];
            (fm.trim(), body)
        } else {
            anyhow::bail!("unclosed frontmatter");
        }
    } else {
        anyhow::bail!("no frontmatter");
    };

    // Parse frontmatter as YAML
    let yaml: serde_yaml::Value = serde_yaml::from_str(frontmatter)?;

    // Extract id (required — fall back to filename stem)
    let id = yaml["id"].as_str().map(String::from).unwrap_or_else(|| {
        path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    // Extract statement: first non-empty line after # heading in body
    let statement = extract_statement(body).unwrap_or_else(|| id.clone());

    // Defaults per SPEC: entrenchment=medium, status=active, facets=[]
    let entrenchment = yaml["entrenchment"]
        .as_str()
        .unwrap_or("medium")
        .to_string();
    let status = yaml["status"].as_str().unwrap_or("active").to_string();
    let facets = if let Some(seq) = yaml["facets"].as_sequence() {
        let tags: Vec<String> = seq
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string())
    } else {
        "[]".to_string()
    };

    Ok(BeliefEntry {
        id,
        source: "persona".to_string(),
        kind: "value".to_string(),
        statement,
        entrenchment,
        status,
        facets,
        cited_by_beliefs: 0,
        cited_by_sessions: 0,
        applied_in: 0,
        evidence_count: 0,
        evidence_verified: 0,
        health_score: 0.0,
        contested_by: String::new(),
    })
}

/// Extract statement from markdown body: first non-empty, non-heading line
fn extract_statement(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        return Some(trimmed.to_string());
    }
    None
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
        println!(
            "   Need {} more uses per edge to enable learning.",
            MIN_SAMPLES
        );
        println!("   Use 'patina scry --routing graph' and act on results.");
    }

    Ok(())
}

/// Search cross-project knowledge via CLI
///
/// FTS5 search across all synced knowledge in graph.db.
/// Per SPEC: statement truncated to 200 chars, one entry per 2 lines.
pub fn search_beliefs_cli(query: &str, limit: usize) -> Result<()> {
    let graph = Graph::open()?;
    let results = graph.search_beliefs(query, limit)?;

    if results.is_empty() {
        println!("No results.");
        return Ok(());
    }

    // Count unique sources for summary
    let mut source_set = std::collections::HashSet::new();
    let mut has_persona = false;

    for entry in &results {
        source_set.insert(entry.source.clone());
        if entry.source == "persona" {
            has_persona = true;
        }
    }

    println!();
    for entry in &results {
        // Line 1: [source] id kind entrenchment
        let source_display = if entry.source == "persona" {
            "[persona]".to_string()
        } else {
            format!("[{}]", entry.source)
        };

        println!(
            "{:<20} {:<30} {:<8} {}",
            source_display,
            truncate(&entry.id, 30),
            entry.kind,
            entry.entrenchment
        );

        // Line 2: statement (truncated to 200 chars)
        let stmt_display = if entry.statement.len() > 200 {
            format!("{}...", &entry.statement[..197])
        } else {
            entry.statement.clone()
        };
        println!("{:20} \"{}\"", "", stmt_display);
        println!();
    }

    // Unique project count (excluding persona)
    let unique_projects = source_set.len() - if has_persona { 1 } else { 0 };
    let persona_suffix = if has_persona { " + persona" } else { "" };
    println!(
        "{} results from {} project{}{}",
        results.len(),
        unique_projects,
        if unique_projects != 1 { "s" } else { "" },
        persona_suffix
    );

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
    println!(
        "   Edges with {} or more uses are 'ready' for weight learning.",
        MIN_SAMPLES
    );
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
