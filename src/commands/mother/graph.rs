//! Internal implementation for mother command
//!
//! Syncs graph from registry, manages edges.

use anyhow::{bail, Context, Result};
use rusqlite::OptionalExtension;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use patina::mother::{BeliefEntry, BeliefStatus, EdgeType, Graph, NodeType, MIN_SAMPLES};
use patina::paths;

use super::adapters;

/// Sync graph nodes from registry
///
/// Creates nodes for all projects and repos in ~/.patina/registry.yaml.
/// Also adds the current project if we're in a patina project directory.
pub fn sync_from_registry() -> Result<()> {
    let registry_backend = adapters::RepoRegistryBackend;
    let root_provider = adapters::SessionProjectRootProvider;
    sync_from_registry_with(&registry_backend, &root_provider)
}

fn sync_from_registry_with(
    registry_backend: &dyn adapters::RegistryBackend,
    root_provider: &dyn adapters::ProjectRootProvider,
) -> Result<()> {
    println!("🔄 Syncing graph from registry...\n");

    let mut registry = registry_backend.load_snapshot()?;

    // Include Mother checked-in projects even if they are not in registry.yaml.
    // Project check-in is the canonical node-local discovery mechanism.
    let store = patina::mother::MotherRuntimeStore::default();
    if let Ok(checked_in_projects) = store.list_registered_projects() {
        let mut known_paths: HashSet<String> = registry
            .projects
            .iter()
            .map(|entry| {
                std::fs::canonicalize(&entry.path)
                    .unwrap_or_else(|_| Path::new(&entry.path).to_path_buf())
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        for project in checked_in_projects {
            let canonical = std::fs::canonicalize(&project.project_path)
                .unwrap_or_else(|_| Path::new(&project.project_path).to_path_buf())
                .to_string_lossy()
                .to_string();
            if !known_paths.insert(canonical.clone()) {
                continue;
            }
            let canonical_path = Path::new(&canonical).to_path_buf();
            if !canonical_path.exists() {
                continue;
            }
            let name = canonical_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(project.project_uid.as_str())
                .to_string();
            let domains = detect_project_domains(&canonical_path);
            registry.projects.push(adapters::RegistryProject {
                name,
                path: canonical,
                domains,
            });
        }
    }

    let graph = Graph::open()?;

    let mut projects_added = 0;
    let mut repos_added = 0;

    // Add current project if we're in one
    if let Some(project_root) = root_provider.find_current_project_root() {
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
    for entry in &registry.projects {
        let path = Path::new(&entry.path);
        graph.add_node(&entry.name, NodeType::Project, path, &entry.domains)?;
        projects_added += 1;
        println!("  + {} (project)", entry.name);
    }

    // Add repos
    for entry in &registry.repos {
        let path = Path::new(&entry.path);
        graph.add_node(&entry.name, NodeType::Reference, path, &entry.domains)?;
        repos_added += 1;
        println!("  + {} (reference)", entry.name);
    }

    // =========================================================================
    // Belief sync: collect beliefs from projects + persona values
    // =========================================================================

    println!();
    println!("📚 Syncing beliefs...\n");

    let mut knowledge: Vec<BeliefEntry> = Vec::new();
    let mut synced_sources: Vec<String> = Vec::new();
    let mut beliefs_synced = 0;
    let mut values_synced = 0;

    // Detect current project root for dedup guard
    let current_project_root = root_provider.find_current_project_root();

    // Collect beliefs from current project (auto-detected, may not be in registry)
    if let Some(ref project_root) = current_project_root {
        let project_name = project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let db_path = patina::eventlog::resolve_patina_db_path(project_root);
        match collect_project_beliefs(project_name, project_root, &db_path) {
            Ok(entries) => {
                let b = entries.iter().filter(|e| e.kind != "value").count();
                let v = entries.iter().filter(|e| e.kind == "value").count();
                beliefs_synced += b;
                values_synced += v;
                synced_sources.push(project_name.to_string());
                if b + v > 0 {
                    if v > 0 {
                        println!(
                            "  + {} beliefs + {} values from {} (current)",
                            b, v, project_name
                        );
                    } else {
                        println!("  + {} beliefs from {} (current)", b, project_name);
                    }
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
    // Dedup guard: skip registry entry if its path matches current project root
    for entry in &registry.projects {
        let registry_path = Path::new(&entry.path);
        if let Some(ref project_root) = current_project_root {
            if registry_path == project_root.as_path() {
                continue; // Already collected as current project
            }
        }
        let db_path = patina::eventlog::resolve_patina_db_path(registry_path);
        match collect_project_beliefs(&entry.name, registry_path, &db_path) {
            Ok(entries) => {
                let b = entries.iter().filter(|e| e.kind != "value").count();
                let v = entries.iter().filter(|e| e.kind == "value").count();
                beliefs_synced += b;
                values_synced += v;
                synced_sources.push(entry.name.clone());
                if b + v > 0 {
                    if v > 0 {
                        println!("  + {} beliefs + {} values from {}", b, v, entry.name);
                    } else {
                        println!("  + {} beliefs from {}", b, entry.name);
                    }
                }
                knowledge.extend(entries);
            }
            Err(e) => {
                eprintln!("  ⚠ {}: {}", entry.name, e);
            }
        }
    }

    // Ref repos: skip knowledge sync entirely (per SPEC)

    // Read persona values from ~/.patina/layer/surface/beliefs/
    match collect_persona_values() {
        Ok(entries) => {
            let persona_count = entries.len();
            values_synced += persona_count;
            synced_sources.push("persona".to_string());
            if persona_count > 0 {
                println!("  + {} values from persona", persona_count);
            }
            knowledge.extend(entries);
        }
        Err(e) => {
            eprintln!("  ⚠ persona: {}", e);
        }
    }

    // Sync beliefs — only rebuilds entries for successfully collected sources.
    // Failed sources retain their previously indexed data.
    graph.sync_beliefs(&knowledge, &synced_sources)?;

    // Populate belief_applied_in from synced beliefs (Phase E)
    // Per-source rebuild: delete then re-insert for each synced source
    sync_belief_applied_in(&graph, &knowledge, &synced_sources)?;

    // =========================================================================
    // Edge sync: collect belief relationship edges from projects
    // =========================================================================

    let mut supports_edges: Vec<(String, String, String)> = Vec::new();
    let mut attacks_edges: Vec<(String, String, String, bool)> = Vec::new();
    let mut edge_synced_sources: Vec<String> = Vec::new();

    // Collect edges from current project
    if let Some(ref project_root) = current_project_root {
        let project_name = project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let db_path = patina::eventlog::resolve_patina_db_path(project_root);
        match collect_belief_edges(project_name, &db_path) {
            Ok((s, a)) => {
                let count = s.len() + a.len();
                if count > 0 {
                    println!("  + {} edges from {} (current)", count, project_name);
                }
                edge_synced_sources.push(project_name.to_string());
                supports_edges.extend(s);
                attacks_edges.extend(a);
            }
            Err(_) => {
                // Edge tables may not exist yet — not an error
            }
        }
    }

    // Collect edges from registered projects (dedup guard: skip current project)
    for entry in &registry.projects {
        let registry_path = Path::new(&entry.path);
        if let Some(ref project_root) = current_project_root {
            if registry_path == project_root.as_path() {
                continue;
            }
        }
        let db_path = patina::eventlog::resolve_patina_db_path(registry_path);
        match collect_belief_edges(&entry.name, &db_path) {
            Ok((s, a)) => {
                let count = s.len() + a.len();
                if count > 0 {
                    println!("  + {} edges from {}", count, entry.name);
                }
                edge_synced_sources.push(entry.name.clone());
                supports_edges.extend(s);
                attacks_edges.extend(a);
            }
            Err(_) => {
                // Edge tables may not exist yet — not an error
            }
        }
    }

    // Sync edges into graph.db
    graph.sync_belief_edges(&supports_edges, &attacks_edges, &edge_synced_sources)?;

    // Auto-clean dangling edges: delete edges referencing non-existent beliefs
    clean_dangling_edges(&graph)?;

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
        "   Beliefs: {} beliefs + {} values = {} total",
        beliefs_synced,
        values_synced,
        graph.belief_count()?
    );

    Ok(())
}

/// Sync belief_applied_in table from synced beliefs (Phase E)
fn sync_belief_applied_in(
    graph: &patina::mother::Graph,
    entries: &[BeliefEntry],
    synced_sources: &[String],
) -> Result<()> {
    graph.sync_belief_applied_in(entries, synced_sources)?;
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

#[derive(Debug, Clone, Default)]
struct SourceProvenance {
    project_uid: Option<String>,
    project_id: Option<String>,
    commit_sha: Option<String>,
    revision: Option<String>,
}

fn resolve_source_provenance(project_root: &Path) -> SourceProvenance {
    let project_uid = patina::project::get_uid(project_root);

    let project_id = project_uid.as_ref().and_then(|uid| {
        let store = patina::mother::MotherRuntimeStore::default();
        let conn = rusqlite::Connection::open(store.path()).ok()?;
        conn.query_row(
            "SELECT project_id FROM mother_project_identities WHERE project_uid = ?1",
            rusqlite::params![uid],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()?
    });

    let commit_sha = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
            } else {
                None
            }
        });

    SourceProvenance {
        revision: commit_sha.clone(),
        project_uid,
        project_id,
        commit_sha,
    }
}

/// Collect beliefs from a project's patina.db
///
/// Opens the project's patina.db and reads the beliefs table (23 columns).
/// Returns empty vec with warning on missing db or missing table.
fn collect_project_beliefs(
    project_name: &str,
    project_root: &Path,
    db_path: &Path,
) -> Result<Vec<BeliefEntry>> {
    use rusqlite::Connection;

    if !db_path.exists() {
        anyhow::bail!("no patina.db (not yet scraped)");
    }

    let conn = Connection::open(db_path)
        .with_context(|| format!("opening project belief db {}", db_path.display()))?;
    let provenance = resolve_source_provenance(project_root);

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

    // Query all columns. Handle missing columns gracefully for older project DBs
    // by checking which columns exist before building the SELECT.
    let has_kind = conn.prepare("SELECT kind FROM beliefs LIMIT 0").is_ok();
    let has_grounding = conn
        .prepare("SELECT grounding_score FROM beliefs LIMIT 0")
        .is_ok();
    let has_verification = conn
        .prepare("SELECT verification_total FROM beliefs LIMIT 0")
        .is_ok();
    let has_last_activity = conn
        .prepare("SELECT last_activity FROM beliefs LIMIT 0")
        .is_ok();

    let kind_col = if has_kind { ", kind" } else { ", 'belief'" };
    let grounding_cols = if has_grounding {
        ", grounding_score, grounding_code_count, grounding_commit_count, grounding_session_count, grounding_forge_count"
    } else {
        ", 0.0, 0, 0, 0, 0"
    };
    let verification_cols = if has_verification {
        ", verification_total, verification_passed, verification_failed, verification_errored"
    } else {
        ", 0, 0, 0, 0"
    };
    let last_activity_col = if has_last_activity {
        ", last_activity"
    } else {
        ", NULL"
    };

    let sql = format!(
        "SELECT id, statement, entrenchment, status, facets,
                cited_by_beliefs, cited_by_sessions, applied_in,
                evidence_count, evidence_verified, health_score, contested_by, imported
                {}{}{}{}
         FROM beliefs WHERE status != 'archived'",
        kind_col, grounding_cols, verification_cols, last_activity_col
    );

    let mut stmt = conn.prepare(&sql)?;

    let entries: Vec<BeliefEntry> = stmt
        .query_map([], |row| {
            Ok(BeliefEntry {
                id: row.get(0)?,
                source: project_name.to_string(),
                source_project_uid: provenance.project_uid.clone(),
                source_project_id: provenance.project_id.clone(),
                source_commit_sha: provenance.commit_sha.clone(),
                source_revision: provenance.revision.clone(),
                kind: row
                    .get::<_, Option<String>>(13)?
                    .unwrap_or_else(|| "belief".to_string()),
                statement: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                entrenchment: row
                    .get::<_, Option<String>>(2)?
                    .unwrap_or_else(|| "medium".to_string()),
                status: BeliefStatus::from_str_or_default(
                    &row.get::<_, Option<String>>(3)?
                        .unwrap_or_else(|| "active".to_string()),
                ),
                facets: row
                    .get::<_, Option<String>>(4)?
                    .unwrap_or_else(|| "[]".to_string()),
                cited_by_beliefs: row.get::<_, Option<i32>>(5)?.unwrap_or(0),
                cited_by_sessions: row.get::<_, Option<i32>>(6)?.unwrap_or(0),
                applied_in: row.get::<_, Option<i32>>(7)?.unwrap_or(0),
                evidence_count: row.get::<_, Option<i32>>(8)?.unwrap_or(0),
                evidence_verified: row.get::<_, Option<i32>>(9)?.unwrap_or(0),
                health_score: row.get::<_, Option<f64>>(10)?.unwrap_or(0.0),
                contested_by: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
                imported: row.get::<_, Option<i32>>(12)?.unwrap_or(0) != 0,
                grounding_score: row.get::<_, Option<f64>>(14)?.unwrap_or(0.0),
                grounding_code_count: row.get::<_, Option<i32>>(15)?.unwrap_or(0),
                grounding_commit_count: row.get::<_, Option<i32>>(16)?.unwrap_or(0),
                grounding_session_count: row.get::<_, Option<i32>>(17)?.unwrap_or(0),
                grounding_forge_count: row.get::<_, Option<i32>>(18)?.unwrap_or(0),
                verification_total: row.get::<_, Option<i32>>(19)?.unwrap_or(0),
                verification_passed: row.get::<_, Option<i32>>(20)?.unwrap_or(0),
                verification_failed: row.get::<_, Option<i32>>(21)?.unwrap_or(0),
                verification_errored: row.get::<_, Option<i32>>(22)?.unwrap_or(0),
                last_activity: row.get::<_, Option<String>>(23)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

/// Collect belief edges from a project's patina.db
///
/// Returns (supports, attacks) tuples with source_project name attached.
/// Schema version guard: if tables don't exist, returns empty.
#[allow(clippy::type_complexity)]
fn collect_belief_edges(
    project_name: &str,
    db_path: &Path,
) -> Result<(
    Vec<(String, String, String)>, // supports: (from, to, source_project)
    Vec<(String, String, String, bool)>, // attacks: (from, to, source_project, defeated)
)> {
    use rusqlite::Connection;

    if !db_path.exists() {
        return Ok((Vec::new(), Vec::new()));
    }

    let conn = Connection::open(db_path)?;

    // Schema version guard: check if edge tables exist
    let supports_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='belief_supports'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !supports_exists {
        return Ok((Vec::new(), Vec::new()));
    }

    let attacks_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='belief_attacks'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    // Collect supports
    let mut supports = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT from_belief, to_belief FROM belief_supports")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows.filter_map(|r| r.ok()) {
            supports.push((row.0, row.1, project_name.to_string()));
        }
    }

    // Collect attacks (if table exists)
    let mut attacks = Vec::new();
    if attacks_exists {
        let mut stmt =
            conn.prepare("SELECT from_belief, to_belief, defeated FROM belief_attacks")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)? != 0,
            ))
        })?;
        for row in rows.filter_map(|r| r.ok()) {
            attacks.push((row.0, row.1, project_name.to_string(), row.2));
        }
    }

    Ok((supports, attacks))
}

/// Clean dangling edges: edges referencing belief IDs not in the beliefs table.
/// Deletes them and logs the count.
fn clean_dangling_edges(graph: &Graph) -> Result<()> {
    let deleted = graph.delete_dangling_edges()?;

    if deleted > 0 {
        println!(
            "  🧹 Cleaned {} dangling edge{}",
            deleted,
            if deleted == 1 { "" } else { "s" }
        );
    }

    Ok(())
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
    let status = BeliefStatus::from_str_or_default(yaml["status"].as_str().unwrap_or("active"));
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
        source_project_uid: None,
        source_project_id: None,
        source_commit_sha: None,
        source_revision: None,
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
        imported: false,
        grounding_score: 0.0,
        grounding_code_count: 0,
        grounding_commit_count: 0,
        grounding_session_count: 0,
        grounding_forge_count: 0,
        verification_total: 0,
        verification_passed: 0,
        verification_failed: 0,
        verification_errored: 0,
        last_activity: None,
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
        // Line 1: [source] id kind entrenchment health=X.XX
        let source_display = if entry.source == "persona" {
            "[persona]".to_string()
        } else {
            format!("[{}]", entry.source)
        };

        println!(
            "{:<20} {:<30} {:<8} {} health={:.2}",
            source_display,
            truncate(&entry.id, 30),
            entry.kind,
            entry.entrenchment,
            entry.health_score
        );

        // Line 2: statement (truncated to 200 chars)
        let stmt_display = if entry.statement.len() > 200 {
            format!("{}...", &entry.statement[..197])
        } else {
            entry.statement.clone()
        };
        println!("{:20} \"{}\"", "", stmt_display);

        // Line 3: applied_in projects (if any)
        let projects = graph.query_belief_applied_in(&entry.id).unwrap_or_default();
        if !projects.is_empty() {
            println!("{:20} projects: {}", "", projects.join(", "));
        }

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

/// Query the belief graph — CLI entry point for `mother graph query` subcommands
pub fn query_beliefs_cli(command: super::QueryCommands) -> Result<()> {
    use patina::mother::Graph;

    let graph = Graph::open()?;

    match command {
        super::QueryCommands::Belief { query, limit } => {
            let results = graph.search_beliefs(&query, limit)?;

            if results.is_empty() {
                println!("No beliefs found for \"{}\".", query);
                return Ok(());
            }

            println!(
                "\n  Beliefs matching \"{}\" ({} results)\n",
                query,
                results.len()
            );

            for entry in &results {
                let source_display = if entry.source == "persona" {
                    "[persona]".to_string()
                } else {
                    format!("[{}]", entry.source)
                };

                println!(
                    "  {:<18} {:<35} {} health={:.2} evid={} cited={}",
                    source_display,
                    truncate(&entry.id, 35),
                    entry.entrenchment,
                    entry.health_score,
                    entry.evidence_count,
                    entry.cited_by_beliefs + entry.cited_by_sessions,
                );

                let stmt_display = if entry.statement.len() > 100 {
                    format!("{}...", &entry.statement[..97])
                } else {
                    entry.statement.clone()
                };
                println!("  {:18} \"{}\"\n", "", stmt_display);
            }

            Ok(())
        }
        super::QueryCommands::Supports { belief_id } => {
            let supports = graph.query_supports(&belief_id)?;

            if supports.is_empty() {
                println!("No beliefs support \"{}\".", belief_id);
                return Ok(());
            }

            println!(
                "\n  Beliefs supporting \"{}\" ({} edges)\n",
                belief_id,
                supports.len()
            );

            for (from_belief, source_project) in &supports {
                println!(
                    "  {} ← {} (from {})",
                    belief_id, from_belief, source_project
                );
            }
            println!();

            Ok(())
        }
        super::QueryCommands::Attacks { belief_id } => {
            let attacks = graph.query_attacks(&belief_id)?;

            if attacks.is_empty() {
                println!("No beliefs attack \"{}\".", belief_id);
                return Ok(());
            }

            println!(
                "\n  Beliefs attacking \"{}\" ({} edges)\n",
                belief_id,
                attacks.len()
            );

            for (from_belief, source_project, defeated) in &attacks {
                let status = if *defeated { "defeated" } else { "active" };
                println!(
                    "  {} ← {} ({}, from {})",
                    belief_id, from_belief, status, source_project
                );
            }
            println!();

            Ok(())
        }
        super::QueryCommands::Projects { belief_id } => {
            let projects = graph.query_projects(&belief_id)?;

            if projects.is_empty() {
                println!("Belief \"{}\" not found in any project.", belief_id);
                return Ok(());
            }

            println!(
                "\n  Projects with \"{}\" ({} found)\n",
                belief_id,
                projects.len()
            );

            for (source, entrenchment) in &projects {
                println!("  {} (entrenchment: {})", source, entrenchment);
            }
            println!();

            Ok(())
        }
    }
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
