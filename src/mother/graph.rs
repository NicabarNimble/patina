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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "project" => Some(NodeType::Project),
            "reference" => Some(NodeType::Reference),
            _ => None,
        }
    }
}

/// Edge types - relationships between nodes
///
/// Note: These are hypothesized types from layer/surface/build/spec-mothership-graph.md.
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

    pub fn from_str(s: &str) -> Option<Self> {
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
    pub from_node: String,
    pub to_node: String,
    pub edge_type: EdgeType,
    pub weight: f32,
    pub evidence: Option<String>,
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
                node_type: NodeType::from_str(&row.get::<_, String>(1)?).unwrap_or(NodeType::Reference),
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
                    node_type: NodeType::from_str(&row.get::<_, String>(1)?)
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
            "SELECT from_node, to_node, edge_type, weight, evidence FROM edges WHERE from_node = ?1",
        )?;

        let edges = stmt
            .query_map(params![node], |row| {
                Ok(Edge {
                    from_node: row.get(0)?,
                    to_node: row.get(1)?,
                    edge_type: EdgeType::from_str(&row.get::<_, String>(2)?)
                        .unwrap_or(EdgeType::Uses),
                    weight: row.get(3)?,
                    evidence: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(edges)
    }

    /// Get all edges (for display)
    pub fn list_edges(&self) -> Result<Vec<Edge>> {
        let mut stmt = self.conn.prepare(
            "SELECT from_node, to_node, edge_type, weight, evidence FROM edges ORDER BY from_node, to_node",
        )?;

        let edges = stmt
            .query_map([], |row| {
                Ok(Edge {
                    from_node: row.get(0)?,
                    to_node: row.get(1)?,
                    edge_type: EdgeType::from_str(&row.get::<_, String>(2)?)
                        .unwrap_or(EdgeType::Uses),
                    weight: row.get(3)?,
                    evidence: row.get(4)?,
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
            .query_map(rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())), |row| {
                let domains_json: String = row.get(3)?;
                let domains: Vec<String> = serde_json::from_str(&domains_json).unwrap_or_default();

                Ok(Node {
                    id: row.get(0)?,
                    node_type: NodeType::from_str(&row.get::<_, String>(1)?)
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
        graph.add_node("patina", NodeType::Project, std::path::Path::new("/patina"), &[])?;
        graph.add_node("dojo", NodeType::Reference, std::path::Path::new("/dojo"), &[])?;

        // Add edge
        graph.add_edge("patina", "dojo", EdgeType::TestsWith, Some("benchmark subject"))?;

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
        graph.add_node("patina", NodeType::Project, std::path::Path::new("/patina"), &[])?;
        graph.add_node("dojo", NodeType::Reference, std::path::Path::new("/dojo"), &[])?;
        graph.add_node("SDL", NodeType::Reference, std::path::Path::new("/SDL"), &[])?;

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
}
