//! Graph storage and traversal
//!
//! SQLite-backed relationship graph for cross-project awareness.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::PathBuf;

use crate::paths;

/// Node types in the graph
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    /// Full patina project (local)
    Project,
    /// Reference repository (cloned)
    Reference,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeType::Project => "project",
            NodeType::Reference => "reference",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "project" => Some(NodeType::Project),
            "reference" => Some(NodeType::Reference),
            _ => None,
        }
    }
}

/// Edge types - relationships between nodes
///
/// Note: These are hypothesized types from the archived spec (git tag: spec/mothership-graph).
/// Will be validated against real usage in Phase G2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    /// Project depends on/uses reference
    Uses,
    /// Project learns patterns from reference
    LearnsFrom,
    /// Project uses reference as test subject
    TestsWith,
    /// Projects share domains
    Sibling,
    /// Node belongs to domain (domain as pseudo-node)
    Domain,
}

impl EdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeType::Uses => "USES",
            EdgeType::LearnsFrom => "LEARNS_FROM",
            EdgeType::TestsWith => "TESTS_WITH",
            EdgeType::Sibling => "SIBLING",
            EdgeType::Domain => "DOMAIN",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "USES" => Some(EdgeType::Uses),
            "LEARNS_FROM" => Some(EdgeType::LearnsFrom),
            "TESTS_WITH" => Some(EdgeType::TestsWith),
            "SIBLING" => Some(EdgeType::Sibling),
            "DOMAIN" => Some(EdgeType::Domain),
            _ => None,
        }
    }

    /// All edge types for iteration
    pub fn all() -> &'static [EdgeType] {
        &[
            EdgeType::Uses,
            EdgeType::LearnsFrom,
            EdgeType::TestsWith,
            EdgeType::Sibling,
            EdgeType::Domain,
        ]
    }
}

/// A node in the graph (project or reference repo)
#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub node_type: NodeType,
    pub path: PathBuf,
    pub domains: Vec<String>,
    pub summary: Option<String>,
    pub importance: f32,
}

/// An edge between nodes
#[derive(Debug, Clone)]
pub struct Edge {
    pub id: i64,
    pub from_node: String,
    pub to_node: String,
    pub edge_type: EdgeType,
    pub weight: f32,
    pub evidence: Option<String>,
}

/// Usage statistics for an edge
#[derive(Debug, Clone)]
pub struct EdgeUsageStats {
    pub edge_id: i64,
    pub from_node: String,
    pub to_node: String,
    pub edge_type: EdgeType,
    pub total_uses: usize,
    pub useful_uses: usize,
    pub current_weight: f32,
}

// =========================================================================
// Weight Learning Constants (G2.5)
// =========================================================================

/// Minimum samples before updating weight (prevents noisy updates)
pub const MIN_SAMPLES: usize = 5;

/// Default learning rate (conservative to prevent oscillation)
pub const DEFAULT_ALPHA: f32 = 0.1;

/// Minimum weight bound (never completely ignore an edge)
pub const WEIGHT_MIN: f32 = 0.5;

/// Maximum weight bound (never over-amplify)
pub const WEIGHT_MAX: f32 = 2.0;

/// Report from weight learning run
#[derive(Debug, Clone)]
pub struct WeightLearningReport {
    pub edges_updated: usize,
    pub edges_skipped_insufficient: usize,
    pub changes: Vec<WeightChange>,
}

/// A single weight change
#[derive(Debug, Clone)]
pub struct WeightChange {
    pub edge_id: i64,
    pub from_node: String,
    pub to_node: String,
    pub edge_type: EdgeType,
    pub old_weight: f32,
    pub new_weight: f32,
    pub precision: f32,
    pub sample_count: usize,
}

/// The relationship graph
pub struct Graph {
    conn: Connection,
}

impl Graph {
    /// Open the graph database, creating it if necessary
    pub fn open() -> Result<Self> {
        let db_path = paths::mother::graph_db();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create mother directory: {:?}", parent))?;
        }

        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open graph database: {:?}", db_path))?;

        let graph = Self { conn };
        graph.init_schema()?;

        Ok(graph)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            -- Nodes: Projects and reference repos
            CREATE TABLE IF NOT EXISTS nodes (
                id TEXT PRIMARY KEY,
                node_type TEXT NOT NULL,
                path TEXT NOT NULL,
                domains TEXT,
                summary TEXT,
                last_indexed TEXT,
                importance REAL DEFAULT 1.0
            );

            -- Edges: Relationships between nodes
            CREATE TABLE IF NOT EXISTS edges (
                id INTEGER PRIMARY KEY,
                from_node TEXT NOT NULL,
                to_node TEXT NOT NULL,
                edge_type TEXT NOT NULL,
                weight REAL DEFAULT 1.0,
                created TEXT NOT NULL,
                evidence TEXT,
                FOREIGN KEY (from_node) REFERENCES nodes(id),
                FOREIGN KEY (to_node) REFERENCES nodes(id),
                UNIQUE(from_node, to_node, edge_type)
            );

            -- Indexes for traversal
            CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_node);
            CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_node);
            CREATE INDEX IF NOT EXISTS idx_edges_type ON edges(edge_type);

            -- Edge usage tracking for weight learning (G2.5)
            -- Records which edges contributed to queries and whether results were useful
            CREATE TABLE IF NOT EXISTS edge_usage (
                id INTEGER PRIMARY KEY,
                edge_id INTEGER NOT NULL,
                query_id TEXT NOT NULL,
                result_repo TEXT NOT NULL,
                result_rank INTEGER,
                was_useful INTEGER DEFAULT 0,
                created TEXT NOT NULL,
                FOREIGN KEY (edge_id) REFERENCES edges(id)
            );

            CREATE INDEX IF NOT EXISTS idx_edge_usage_edge ON edge_usage(edge_id);
            CREATE INDEX IF NOT EXISTS idx_edge_usage_query ON edge_usage(query_id);
            "#,
        )?;

        Ok(())
    }

    // =========================================================================
    // Node Operations
    // =========================================================================

    /// Add or update a node
    pub fn add_node(
        &self,
        id: &str,
        node_type: NodeType,
        path: &std::path::Path,
        domains: &[String],
    ) -> Result<()> {
        let domains_json = serde_json::to_string(domains)?;
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            r#"
            INSERT INTO nodes (id, node_type, path, domains, last_indexed)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET
                node_type = excluded.node_type,
                path = excluded.path,
                domains = excluded.domains,
                last_indexed = excluded.last_indexed
            "#,
            params![
                id,
                node_type.as_str(),
                path.to_string_lossy(),
                domains_json,
                now
            ],
        )?;

        Ok(())
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Result<Option<Node>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, node_type, path, domains, summary, importance FROM nodes WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| {
            let domains_json: String = row.get(3)?;
            let domains: Vec<String> = serde_json::from_str(&domains_json).unwrap_or_default();

            Ok(Node {
                id: row.get(0)?,
                node_type: NodeType::parse(&row.get::<_, String>(1)?)
                    .unwrap_or(NodeType::Reference),
                path: PathBuf::from(row.get::<_, String>(2)?),
                domains,
                summary: row.get(4)?,
                importance: row.get(5)?,
            })
        });

        match result {
            Ok(node) => Ok(Some(node)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all nodes
    pub fn list_nodes(&self) -> Result<Vec<Node>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, node_type, path, domains, summary, importance FROM nodes ORDER BY id",
        )?;

        let nodes = stmt
            .query_map([], |row| {
                let domains_json: String = row.get(3)?;
                let domains: Vec<String> = serde_json::from_str(&domains_json).unwrap_or_default();

                Ok(Node {
                    id: row.get(0)?,
                    node_type: NodeType::parse(&row.get::<_, String>(1)?)
                        .unwrap_or(NodeType::Reference),
                    path: PathBuf::from(row.get::<_, String>(2)?),
                    domains,
                    summary: row.get(4)?,
                    importance: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(nodes)
    }

    /// Count nodes
    pub fn node_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    // =========================================================================
    // Edge Operations
    // =========================================================================

    /// Add an edge between nodes
    pub fn add_edge(
        &self,
        from: &str,
        to: &str,
        edge_type: EdgeType,
        evidence: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            r#"
            INSERT INTO edges (from_node, to_node, edge_type, created, evidence)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(from_node, to_node, edge_type) DO UPDATE SET
                evidence = COALESCE(excluded.evidence, edges.evidence)
            "#,
            params![from, to, edge_type.as_str(), now, evidence],
        )?;

        Ok(())
    }

    /// Remove an edge
    pub fn remove_edge(&self, from: &str, to: &str, edge_type: EdgeType) -> Result<bool> {
        let deleted = self.conn.execute(
            "DELETE FROM edges WHERE from_node = ?1 AND to_node = ?2 AND edge_type = ?3",
            params![from, to, edge_type.as_str()],
        )?;

        Ok(deleted > 0)
    }

    /// Get all edges from a node
    pub fn get_edges_from(&self, node: &str) -> Result<Vec<Edge>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, from_node, to_node, edge_type, weight, evidence FROM edges WHERE from_node = ?1",
        )?;

        let edges = stmt
            .query_map(params![node], |row| {
                Ok(Edge {
                    id: row.get(0)?,
                    from_node: row.get(1)?,
                    to_node: row.get(2)?,
                    edge_type: EdgeType::parse(&row.get::<_, String>(3)?).unwrap_or(EdgeType::Uses),
                    weight: row.get(4)?,
                    evidence: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(edges)
    }

    /// Get all edges (for display)
    pub fn list_edges(&self) -> Result<Vec<Edge>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, from_node, to_node, edge_type, weight, evidence FROM edges ORDER BY from_node, to_node",
        )?;

        let edges = stmt
            .query_map([], |row| {
                Ok(Edge {
                    id: row.get(0)?,
                    from_node: row.get(1)?,
                    to_node: row.get(2)?,
                    edge_type: EdgeType::parse(&row.get::<_, String>(3)?).unwrap_or(EdgeType::Uses),
                    weight: row.get(4)?,
                    evidence: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(edges)
    }

    /// Count edges
    pub fn edge_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    // =========================================================================
    // Traversal
    // =========================================================================

    /// Get related nodes by edge types
    pub fn get_related(&self, node: &str, edge_types: &[EdgeType]) -> Result<Vec<Node>> {
        if edge_types.is_empty() {
            return Ok(vec![]);
        }

        let placeholders: Vec<_> = edge_types.iter().map(|_| "?").collect();
        let sql = format!(
            r#"
            SELECT DISTINCT n.id, n.node_type, n.path, n.domains, n.summary, n.importance
            FROM nodes n
            JOIN edges e ON n.id = e.to_node
            WHERE e.from_node = ?1 AND e.edge_type IN ({})
            ORDER BY e.weight DESC, n.id
            "#,
            placeholders.join(", ")
        );

        let mut stmt = self.conn.prepare(&sql)?;

        // Build params: node id + edge types
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(node.to_string())];
        for et in edge_types {
            params_vec.push(Box::new(et.as_str().to_string()));
        }

        let nodes = stmt
            .query_map(
                rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())),
                |row| {
                    let domains_json: String = row.get(3)?;
                    let domains: Vec<String> =
                        serde_json::from_str(&domains_json).unwrap_or_default();

                    Ok(Node {
                        id: row.get(0)?,
                        node_type: NodeType::parse(&row.get::<_, String>(1)?)
                            .unwrap_or(NodeType::Reference),
                        path: PathBuf::from(row.get::<_, String>(2)?),
                        domains,
                        summary: row.get(4)?,
                        importance: row.get(5)?,
                    })
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(nodes)
    }

    // =========================================================================
    // Edge Usage (G2.5 - Feedback Loop)
    // =========================================================================

    /// Record that an edge contributed to a query's routing
    ///
    /// Called when graph routing uses an edge to include a repo in the search.
    /// The result_rank is the best rank achieved by any result from that repo.
    pub fn record_edge_usage(
        &self,
        edge_id: i64,
        query_id: &str,
        result_repo: &str,
        result_rank: Option<usize>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            r#"
            INSERT INTO edge_usage (edge_id, query_id, result_repo, result_rank, created)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                edge_id,
                query_id,
                result_repo,
                result_rank.map(|r| r as i64),
                now
            ],
        )?;

        Ok(())
    }

    /// Mark edge usage as useful (called when scry.use event occurs)
    ///
    /// Finds edge_usage records for the query that led to the used repo
    /// and marks them as useful.
    pub fn mark_usage_useful(&self, query_id: &str, result_repo: &str) -> Result<usize> {
        let updated = self.conn.execute(
            "UPDATE edge_usage SET was_useful = 1 WHERE query_id = ?1 AND result_repo = ?2",
            params![query_id, result_repo],
        )?;

        Ok(updated)
    }

    /// Get usage statistics for an edge
    ///
    /// Returns (useful_count, total_count) for the edge.
    pub fn get_edge_usage_stats(&self, edge_id: i64) -> Result<(usize, usize)> {
        let result: (i64, i64) = self.conn.query_row(
            r#"
            SELECT
                COALESCE(SUM(was_useful), 0) as useful,
                COUNT(*) as total
            FROM edge_usage
            WHERE edge_id = ?1
            "#,
            params![edge_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        Ok((result.0 as usize, result.1 as usize))
    }

    /// Get usage statistics for all edges
    ///
    /// Returns stats for edges that have at least one usage record.
    pub fn get_all_usage_stats(&self) -> Result<Vec<EdgeUsageStats>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                e.id,
                e.from_node,
                e.to_node,
                e.edge_type,
                e.weight,
                COALESCE(SUM(eu.was_useful), 0) as useful,
                COUNT(eu.id) as total
            FROM edges e
            LEFT JOIN edge_usage eu ON e.id = eu.edge_id
            GROUP BY e.id
            ORDER BY total DESC, e.from_node, e.to_node
            "#,
        )?;

        let stats = stmt
            .query_map([], |row| {
                Ok(EdgeUsageStats {
                    edge_id: row.get(0)?,
                    from_node: row.get(1)?,
                    to_node: row.get(2)?,
                    edge_type: EdgeType::parse(&row.get::<_, String>(3)?).unwrap_or(EdgeType::Uses),
                    current_weight: row.get(4)?,
                    useful_uses: row.get::<_, i64>(5)? as usize,
                    total_uses: row.get::<_, i64>(6)? as usize,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(stats)
    }

    /// Get edge ID by from/to/type
    ///
    /// Used by routing to look up edge IDs when recording usage.
    pub fn get_edge_id(&self, from: &str, to: &str, edge_type: EdgeType) -> Result<Option<i64>> {
        let result = self.conn.query_row(
            "SELECT id FROM edges WHERE from_node = ?1 AND to_node = ?2 AND edge_type = ?3",
            params![from, to, edge_type.as_str()],
            |row| row.get(0),
        );

        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    // =========================================================================
    // Weight Learning (G2.5)
    // =========================================================================

    /// Get current weight for an edge
    pub fn get_edge_weight(&self, edge_id: i64) -> Result<f32> {
        let weight: f32 = self.conn.query_row(
            "SELECT weight FROM edges WHERE id = ?1",
            params![edge_id],
            |row| row.get(0),
        )?;
        Ok(weight)
    }

    /// Set weight for an edge (clamped to bounds)
    pub fn set_edge_weight(&self, edge_id: i64, weight: f32) -> Result<()> {
        let clamped = weight.clamp(WEIGHT_MIN, WEIGHT_MAX);
        self.conn.execute(
            "UPDATE edges SET weight = ?1 WHERE id = ?2",
            params![clamped, edge_id],
        )?;
        Ok(())
    }

    /// Update edge weight based on usage precision
    ///
    /// Algorithm:
    ///   precision = useful_uses / total_uses
    ///   weight_new = (1 - α) × weight_old + α × (1.0 + precision)
    ///
    /// Result: edges that lead to useful results get higher weight over time.
    /// Returns None if insufficient samples.
    pub fn update_edge_weight(&self, edge_id: i64, alpha: f32) -> Result<Option<WeightChange>> {
        // Get usage stats
        let (useful, total) = self.get_edge_usage_stats(edge_id)?;

        // Require minimum samples
        if total < MIN_SAMPLES {
            return Ok(None);
        }

        // Calculate precision
        let precision = useful as f32 / total as f32;

        // Get current weight
        let old_weight = self.get_edge_weight(edge_id)?;

        // Exponential moving average update
        // Base of 1.0 means: precision=0 → weight→1.0, precision=1 → weight→2.0
        let new_weight = (1.0 - alpha) * old_weight + alpha * (1.0 + precision);
        let clamped_weight = new_weight.clamp(WEIGHT_MIN, WEIGHT_MAX);

        // Update in database
        self.set_edge_weight(edge_id, clamped_weight)?;

        // Get edge info for report
        let edge = self.get_edge_by_id(edge_id)?;

        Ok(Some(WeightChange {
            edge_id,
            from_node: edge.from_node,
            to_node: edge.to_node,
            edge_type: edge.edge_type,
            old_weight,
            new_weight: clamped_weight,
            precision,
            sample_count: total,
        }))
    }

    /// Learn weights for all edges with sufficient data
    ///
    /// Iterates all edges, updates weights for those with >= MIN_SAMPLES usage.
    pub fn learn_weights(&self, alpha: f32) -> Result<WeightLearningReport> {
        let edges = self.list_edges()?;
        let mut updated = 0;
        let mut skipped = 0;
        let mut changes = Vec::new();

        for edge in edges {
            match self.update_edge_weight(edge.id, alpha)? {
                Some(change) => {
                    updated += 1;
                    changes.push(change);
                }
                None => {
                    skipped += 1;
                }
            }
        }

        Ok(WeightLearningReport {
            edges_updated: updated,
            edges_skipped_insufficient: skipped,
            changes,
        })
    }

    /// Get edge by ID (for reporting)
    fn get_edge_by_id(&self, edge_id: i64) -> Result<Edge> {
        let edge = self.conn.query_row(
            "SELECT id, from_node, to_node, edge_type, weight, evidence FROM edges WHERE id = ?1",
            params![edge_id],
            |row| {
                Ok(Edge {
                    id: row.get(0)?,
                    from_node: row.get(1)?,
                    to_node: row.get(2)?,
                    edge_type: EdgeType::parse(&row.get::<_, String>(3)?).unwrap_or(EdgeType::Uses),
                    weight: row.get(4)?,
                    evidence: row.get(5)?,
                })
            },
        )?;
        Ok(edge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // Note: tempdir must be held in test scope to prevent cleanup before test completes
    // Pattern from src/commands/scrape/database.rs

    #[test]
    fn test_add_and_get_node() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("graph.db");
        let conn = Connection::open(&db_path)?;
        let graph = Graph { conn };
        graph.init_schema()?;

        graph.add_node(
            "patina",
            NodeType::Project,
            std::path::Path::new("/home/user/patina"),
            &["rust".to_string()],
        )?;

        let node = graph.get_node("patina")?.expect("node should exist");
        assert_eq!(node.id, "patina");
        assert_eq!(node.node_type, NodeType::Project);
        assert_eq!(node.domains, vec!["rust"]);

        Ok(())
    }

    #[test]
    fn test_add_and_get_edge() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("graph.db");
        let conn = Connection::open(&db_path)?;
        let graph = Graph { conn };
        graph.init_schema()?;

        // Add nodes first
        graph.add_node(
            "patina",
            NodeType::Project,
            std::path::Path::new("/patina"),
            &[],
        )?;
        graph.add_node(
            "dojo",
            NodeType::Reference,
            std::path::Path::new("/dojo"),
            &[],
        )?;

        // Add edge
        graph.add_edge(
            "patina",
            "dojo",
            EdgeType::TestsWith,
            Some("benchmark subject"),
        )?;

        let edges = graph.get_edges_from("patina")?;
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].to_node, "dojo");
        assert_eq!(edges[0].edge_type, EdgeType::TestsWith);
        assert_eq!(edges[0].evidence, Some("benchmark subject".to_string()));

        Ok(())
    }

    #[test]
    fn test_get_related() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("graph.db");
        let conn = Connection::open(&db_path)?;
        let graph = Graph { conn };
        graph.init_schema()?;

        // Setup: patina -> dojo (TESTS_WITH), patina -> SDL (LEARNS_FROM)
        graph.add_node(
            "patina",
            NodeType::Project,
            std::path::Path::new("/patina"),
            &[],
        )?;
        graph.add_node(
            "dojo",
            NodeType::Reference,
            std::path::Path::new("/dojo"),
            &[],
        )?;
        graph.add_node(
            "SDL",
            NodeType::Reference,
            std::path::Path::new("/SDL"),
            &[],
        )?;

        graph.add_edge("patina", "dojo", EdgeType::TestsWith, None)?;
        graph.add_edge("patina", "SDL", EdgeType::LearnsFrom, None)?;

        // Query: get TESTS_WITH only
        let related = graph.get_related("patina", &[EdgeType::TestsWith])?;
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].id, "dojo");

        // Query: get both
        let related = graph.get_related("patina", &[EdgeType::TestsWith, EdgeType::LearnsFrom])?;
        assert_eq!(related.len(), 2);

        Ok(())
    }

    #[test]
    fn test_edge_usage() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("graph.db");
        let conn = Connection::open(&db_path)?;
        let graph = Graph { conn };
        graph.init_schema()?;

        // Setup: patina -> dojo edge
        graph.add_node(
            "patina",
            NodeType::Project,
            std::path::Path::new("/patina"),
            &[],
        )?;
        graph.add_node(
            "dojo",
            NodeType::Reference,
            std::path::Path::new("/dojo"),
            &[],
        )?;
        graph.add_edge("patina", "dojo", EdgeType::TestsWith, None)?;

        // Get edge ID
        let edge_id = graph
            .get_edge_id("patina", "dojo", EdgeType::TestsWith)?
            .expect("edge should exist");

        // Record usage
        graph.record_edge_usage(edge_id, "q_001", "dojo", Some(1))?;
        graph.record_edge_usage(edge_id, "q_002", "dojo", Some(3))?;
        graph.record_edge_usage(edge_id, "q_003", "dojo", Some(5))?;

        // Check stats before marking useful
        let (useful, total) = graph.get_edge_usage_stats(edge_id)?;
        assert_eq!(total, 3);
        assert_eq!(useful, 0);

        // Mark some as useful
        graph.mark_usage_useful("q_001", "dojo")?;
        graph.mark_usage_useful("q_002", "dojo")?;

        // Check stats after marking useful
        let (useful, total) = graph.get_edge_usage_stats(edge_id)?;
        assert_eq!(total, 3);
        assert_eq!(useful, 2);

        // Check get_all_usage_stats
        let all_stats = graph.get_all_usage_stats()?;
        assert_eq!(all_stats.len(), 1);
        assert_eq!(all_stats[0].edge_id, edge_id);
        assert_eq!(all_stats[0].total_uses, 3);
        assert_eq!(all_stats[0].useful_uses, 2);

        Ok(())
    }

    #[test]
    fn test_weight_learning() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("graph.db");
        let conn = Connection::open(&db_path)?;
        let graph = Graph { conn };
        graph.init_schema()?;

        // Setup: patina -> dojo edge
        graph.add_node(
            "patina",
            NodeType::Project,
            std::path::Path::new("/patina"),
            &[],
        )?;
        graph.add_node(
            "dojo",
            NodeType::Reference,
            std::path::Path::new("/dojo"),
            &[],
        )?;
        graph.add_edge("patina", "dojo", EdgeType::TestsWith, None)?;

        let edge_id = graph
            .get_edge_id("patina", "dojo", EdgeType::TestsWith)?
            .expect("edge should exist");

        // Initial weight should be 1.0
        let initial = graph.get_edge_weight(edge_id)?;
        assert!((initial - 1.0).abs() < 0.001);

        // With only 3 samples, should skip (MIN_SAMPLES = 5)
        graph.record_edge_usage(edge_id, "q_001", "dojo", Some(1))?;
        graph.record_edge_usage(edge_id, "q_002", "dojo", Some(2))?;
        graph.record_edge_usage(edge_id, "q_003", "dojo", Some(3))?;

        let result = graph.update_edge_weight(edge_id, 0.1)?;
        assert!(result.is_none(), "Should skip with < MIN_SAMPLES");

        // Add 2 more samples to reach MIN_SAMPLES
        graph.record_edge_usage(edge_id, "q_004", "dojo", Some(4))?;
        graph.record_edge_usage(edge_id, "q_005", "dojo", Some(5))?;

        // Mark 4 of 5 as useful (80% precision)
        graph.mark_usage_useful("q_001", "dojo")?;
        graph.mark_usage_useful("q_002", "dojo")?;
        graph.mark_usage_useful("q_003", "dojo")?;
        graph.mark_usage_useful("q_004", "dojo")?;

        // Now update should work
        let result = graph.update_edge_weight(edge_id, 0.1)?;
        assert!(result.is_some(), "Should update with >= MIN_SAMPLES");

        let change = result.unwrap();
        assert_eq!(change.sample_count, 5);
        assert!((change.precision - 0.8).abs() < 0.001); // 4/5 = 0.8

        // New weight: (1 - 0.1) * 1.0 + 0.1 * (1.0 + 0.8) = 0.9 + 0.18 = 1.08
        assert!((change.new_weight - 1.08).abs() < 0.001);

        // Weight in DB should match
        let db_weight = graph.get_edge_weight(edge_id)?;
        assert!((db_weight - 1.08).abs() < 0.001);

        Ok(())
    }

    #[test]
    fn test_learn_weights_batch() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("graph.db");
        let conn = Connection::open(&db_path)?;
        let graph = Graph { conn };
        graph.init_schema()?;

        // Setup: two edges
        graph.add_node(
            "patina",
            NodeType::Project,
            std::path::Path::new("/patina"),
            &[],
        )?;
        graph.add_node(
            "dojo",
            NodeType::Reference,
            std::path::Path::new("/dojo"),
            &[],
        )?;
        graph.add_node(
            "SDL",
            NodeType::Reference,
            std::path::Path::new("/SDL"),
            &[],
        )?;

        graph.add_edge("patina", "dojo", EdgeType::TestsWith, None)?;
        graph.add_edge("patina", "SDL", EdgeType::LearnsFrom, None)?;

        let edge1 = graph
            .get_edge_id("patina", "dojo", EdgeType::TestsWith)?
            .unwrap();
        let edge2 = graph
            .get_edge_id("patina", "SDL", EdgeType::LearnsFrom)?
            .unwrap();

        // Edge1: 5 uses, 5 useful (100% precision)
        for i in 0..5 {
            graph.record_edge_usage(edge1, &format!("q1_{}", i), "dojo", Some(i))?;
            graph.mark_usage_useful(&format!("q1_{}", i), "dojo")?;
        }

        // Edge2: only 2 uses (insufficient)
        graph.record_edge_usage(edge2, "q2_0", "SDL", Some(1))?;
        graph.record_edge_usage(edge2, "q2_1", "SDL", Some(2))?;

        // Learn all weights
        let report = graph.learn_weights(0.1)?;

        assert_eq!(report.edges_updated, 1);
        assert_eq!(report.edges_skipped_insufficient, 1);
        assert_eq!(report.changes.len(), 1);

        // Edge1 should have been updated: 100% precision -> weight increases
        let change = &report.changes[0];
        assert_eq!(change.edge_id, edge1);
        assert!((change.precision - 1.0).abs() < 0.001);
        // (1 - 0.1) * 1.0 + 0.1 * 2.0 = 0.9 + 0.2 = 1.1
        assert!((change.new_weight - 1.1).abs() < 0.001);

        Ok(())
    }
}
