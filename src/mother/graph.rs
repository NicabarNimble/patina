//! Graph storage and traversal
//!
//! SQLite-backed relationship graph for cross-project awareness.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::paths;

/// Sanitize user input for FTS5 MATCH queries.
///
/// FTS5 interprets bare tokens as column names if they match (e.g. `llm` matches
/// the column, causing "no such column" errors). Hyphens are token separators,
/// and words like AND/OR/NOT are operators.
///
/// Fix: split on whitespace and hyphens, quote each non-empty token, join with spaces.
/// `"secrets-llm-boundary"` → `"secrets" "llm" "boundary"` (implicit AND).
fn sanitize_fts5_query(query: &str) -> String {
    query
        .split(|c: char| c.is_whitespace() || c == '-')
        .filter(|token| !token.is_empty())
        .map(|token| {
            // Strip any existing quotes, then wrap in double quotes
            let clean = token.replace('"', "");
            format!("\"{}\"", clean)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

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

/// Typed belief status — the belief lifecycle state machine.
///
/// Follows [[enum-not-string-for-finite-states]]: 4 variants, no stringly-typed
/// comparisons in control flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeliefStatus {
    Active,
    Scoped,
    Defeated,
    Archived,
}

impl BeliefStatus {
    /// Parse from string with safe default to Active for unknown values.
    ///
    /// Belief data is less critical to control flow than spec status,
    /// so unknown values default rather than error (ADR-4 in DESIGN.md).
    pub fn from_str_or_default(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "scoped" => Self::Scoped,
            "defeated" => Self::Defeated,
            "archived" => Self::Archived,
            _ => Self::Active,
        }
    }

    /// Canonical string form.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Scoped => "scoped",
            Self::Defeated => "defeated",
            Self::Archived => "archived",
        }
    }
}

impl std::fmt::Display for BeliefStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A belief entry for cross-project search (beliefs + persona values)
#[derive(Debug, Clone, Serialize)]
pub struct BeliefEntry {
    pub id: String,
    pub source: String,
    pub kind: String,
    pub statement: String,
    pub entrenchment: String,
    pub status: BeliefStatus,
    pub facets: String,
    // Metrics (synced from patina.db)
    pub cited_by_beliefs: i32,
    pub cited_by_sessions: i32,
    pub applied_in: i32,
    pub evidence_count: i32,
    pub evidence_verified: i32,
    pub health_score: f64,
    pub contested_by: String,
    pub imported: bool,
    // E4.6a Grounding (Area 3: data-mother-schema)
    pub grounding_score: f64,
    pub grounding_code_count: i32,
    pub grounding_commit_count: i32,
    pub grounding_session_count: i32,
    pub grounding_forge_count: i32,
    // Verification (Area 3: data-mother-schema)
    pub verification_total: i32,
    pub verification_passed: i32,
    pub verification_failed: i32,
    pub verification_errored: i32,
    // Temporal (Area 3: data-mother-schema)
    pub last_activity: Option<String>,
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

            -- Migration: drop legacy knowledge tables (rebuildable cache)
            DROP TABLE IF EXISTS knowledge;
            DROP TABLE IF EXISTS knowledge_search;

            -- Cross-project belief index (beliefs + persona values)
            CREATE TABLE IF NOT EXISTS beliefs (
                id TEXT NOT NULL,
                source TEXT NOT NULL,
                kind TEXT NOT NULL,
                statement TEXT NOT NULL,
                entrenchment TEXT DEFAULT 'medium',
                status TEXT DEFAULT 'active',
                facets TEXT,
                cited_by_beliefs INTEGER DEFAULT 0,
                cited_by_sessions INTEGER DEFAULT 0,
                applied_in INTEGER DEFAULT 0,
                evidence_count INTEGER DEFAULT 0,
                evidence_verified INTEGER DEFAULT 0,
                health_score REAL DEFAULT 0.0,
                contested_by TEXT DEFAULT '',
                imported INTEGER DEFAULT 0,
                -- E4.6a Grounding (Area 3: data-mother-schema)
                grounding_score REAL DEFAULT 0.0,
                grounding_code_count INTEGER DEFAULT 0,
                grounding_commit_count INTEGER DEFAULT 0,
                grounding_session_count INTEGER DEFAULT 0,
                grounding_forge_count INTEGER DEFAULT 0,
                -- Verification (Area 3: data-mother-schema)
                verification_total INTEGER DEFAULT 0,
                verification_passed INTEGER DEFAULT 0,
                verification_failed INTEGER DEFAULT 0,
                verification_errored INTEGER DEFAULT 0,
                -- Temporal (Area 3: data-mother-schema)
                last_activity TEXT,
                last_indexed TEXT NOT NULL,
                PRIMARY KEY (id, source)
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS belief_search USING fts5(
                id, source, kind, statement, facets,
                tokenize='porter unicode61'
            );

            -- Belief-to-belief relationship edges
            CREATE TABLE IF NOT EXISTS belief_supports (
                from_belief TEXT NOT NULL,
                to_belief TEXT NOT NULL,
                source_project TEXT NOT NULL,
                last_indexed TEXT NOT NULL,
                PRIMARY KEY (from_belief, to_belief, source_project)
            );

            CREATE TABLE IF NOT EXISTS belief_attacks (
                from_belief TEXT NOT NULL,
                to_belief TEXT NOT NULL,
                source_project TEXT NOT NULL,
                defeated INTEGER DEFAULT 0,
                last_indexed TEXT NOT NULL,
                PRIMARY KEY (from_belief, to_belief, source_project)
            );

            -- Belief provenance (Phase E populates this)
            CREATE TABLE IF NOT EXISTS belief_applied_in (
                belief_id TEXT NOT NULL,
                project TEXT NOT NULL,
                originated INTEGER DEFAULT 0,
                last_indexed TEXT NOT NULL,
                PRIMARY KEY (belief_id, project)
            );
            "#,
        )?;

        // Migration: add Area 3 columns to existing graph.db beliefs tables
        let columns_to_add = [
            ("grounding_score", "REAL DEFAULT 0.0"),
            ("grounding_code_count", "INTEGER DEFAULT 0"),
            ("grounding_commit_count", "INTEGER DEFAULT 0"),
            ("grounding_session_count", "INTEGER DEFAULT 0"),
            ("grounding_forge_count", "INTEGER DEFAULT 0"),
            ("verification_total", "INTEGER DEFAULT 0"),
            ("verification_passed", "INTEGER DEFAULT 0"),
            ("verification_failed", "INTEGER DEFAULT 0"),
            ("verification_errored", "INTEGER DEFAULT 0"),
            ("last_activity", "TEXT"),
        ];

        for (col_name, col_type) in &columns_to_add {
            let sql = format!("ALTER TABLE beliefs ADD COLUMN {} {}", col_name, col_type);
            // Ignore error if column already exists
            let _ = self.conn.execute(&sql, []);
        }

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

    // =========================================================================
    // Belief Operations (Cross-Project Beliefs)
    // =========================================================================

    /// Sync belief entries into graph.db (per-source rebuild)
    ///
    /// Only deletes entries for sources that were successfully collected.
    /// Sources that failed collection retain their previously indexed data.
    /// This prevents running sync from project B from wiping project A's beliefs.
    ///
    /// `synced_sources` is the set of source names that were successfully collected
    /// (even if they returned 0 entries — 0 means "delete old, insert nothing").
    pub fn sync_beliefs(&self, entries: &[BeliefEntry], synced_sources: &[String]) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute("BEGIN", [])?;

        // Delete only entries from successfully collected sources
        for source in synced_sources {
            if let Err(e) = self
                .conn
                .execute("DELETE FROM beliefs WHERE source = ?1", params![source])
            {
                let _ = self.conn.execute("ROLLBACK", []);
                return Err(e.into());
            }
            if let Err(e) = self.conn.execute(
                "DELETE FROM belief_search WHERE source = ?1",
                params![source],
            ) {
                let _ = self.conn.execute("ROLLBACK", []);
                return Err(e.into());
            }
        }

        // Repopulate
        for entry in entries {
            if let Err(e) = self.conn.execute(
                r#"
                INSERT INTO beliefs (id, source, kind, statement, entrenchment, status, facets,
                    cited_by_beliefs, cited_by_sessions, applied_in,
                    evidence_count, evidence_verified, health_score, contested_by, imported,
                    grounding_score, grounding_code_count, grounding_commit_count,
                    grounding_session_count, grounding_forge_count,
                    verification_total, verification_passed, verification_failed,
                    verification_errored, last_activity,
                    last_indexed)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                        ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)
                "#,
                params![
                    entry.id,
                    entry.source,
                    entry.kind,
                    entry.statement,
                    entry.entrenchment,
                    entry.status.as_str(),
                    entry.facets,
                    entry.cited_by_beliefs,
                    entry.cited_by_sessions,
                    entry.applied_in,
                    entry.evidence_count,
                    entry.evidence_verified,
                    entry.health_score,
                    entry.contested_by,
                    entry.imported as i32,
                    entry.grounding_score,
                    entry.grounding_code_count,
                    entry.grounding_commit_count,
                    entry.grounding_session_count,
                    entry.grounding_forge_count,
                    entry.verification_total,
                    entry.verification_passed,
                    entry.verification_failed,
                    entry.verification_errored,
                    entry.last_activity,
                    now
                ],
            ) {
                let _ = self.conn.execute("ROLLBACK", []);
                return Err(e.into());
            }

            if let Err(e) = self.conn.execute(
                r#"
                INSERT INTO belief_search (id, source, kind, statement, facets)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![
                    entry.id,
                    entry.source,
                    entry.kind,
                    entry.statement,
                    entry.facets,
                ],
            ) {
                let _ = self.conn.execute("ROLLBACK", []);
                return Err(e.into());
            }
        }

        self.conn.execute("COMMIT", [])?;
        Ok(())
    }

    /// Sync belief relationship edges into graph.db (per-source rebuild)
    ///
    /// Deletes edges for successfully synced sources, then repopulates.
    pub fn sync_belief_edges(
        &self,
        supports: &[(String, String, String)], // (from, to, source_project)
        attacks: &[(String, String, String, bool)], // (from, to, source_project, defeated)
        synced_sources: &[String],
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute("BEGIN", [])?;

        // Delete edges for successfully synced sources
        for source in synced_sources {
            if let Err(e) = self.conn.execute(
                "DELETE FROM belief_supports WHERE source_project = ?1",
                params![source],
            ) {
                let _ = self.conn.execute("ROLLBACK", []);
                return Err(e.into());
            }
            if let Err(e) = self.conn.execute(
                "DELETE FROM belief_attacks WHERE source_project = ?1",
                params![source],
            ) {
                let _ = self.conn.execute("ROLLBACK", []);
                return Err(e.into());
            }
        }

        // Insert supports edges
        for (from, to, source) in supports {
            if let Err(e) = self.conn.execute(
                r#"
                INSERT OR IGNORE INTO belief_supports (from_belief, to_belief, source_project, last_indexed)
                VALUES (?1, ?2, ?3, ?4)
                "#,
                params![from, to, source, now],
            ) {
                let _ = self.conn.execute("ROLLBACK", []);
                return Err(e.into());
            }
        }

        // Insert attacks edges with defeated merge rule: defeated=1 wins
        for (from, to, source, defeated) in attacks {
            if let Err(e) = self.conn.execute(
                r#"
                INSERT OR IGNORE INTO belief_attacks (from_belief, to_belief, source_project, defeated, last_indexed)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![from, to, source, *defeated as i32, now],
            ) {
                let _ = self.conn.execute("ROLLBACK", []);
                return Err(e.into());
            }
            // Defeated=1 wins: upgrade non-defeated to defeated if needed
            if *defeated {
                if let Err(e) = self.conn.execute(
                    "UPDATE belief_attacks SET defeated = 1 WHERE from_belief = ?1 AND to_belief = ?2 AND source_project = ?3 AND defeated = 0",
                    params![from, to, source],
                ) {
                    let _ = self.conn.execute("ROLLBACK", []);
                    return Err(e.into());
                }
            }
        }

        self.conn.execute("COMMIT", [])?;
        Ok(())
    }

    /// Search belief entries via FTS5
    ///
    /// Returns entries ranked by FTS5 relevance, limited to `limit` results.
    /// User input is sanitized for FTS5: each token is double-quoted to prevent
    /// column name collisions (e.g. "llm" matching the column) and operator
    /// misinterpretation (hyphens, AND/OR/NOT).
    pub fn search_beliefs(&self, query: &str, limit: usize) -> Result<Vec<BeliefEntry>> {
        let sanitized = sanitize_fts5_query(query);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT b.id, b.source, b.kind, b.statement, b.entrenchment, b.status, b.facets,
                   b.cited_by_beliefs, b.cited_by_sessions, b.applied_in,
                   b.evidence_count, b.evidence_verified, b.health_score, b.contested_by,
                   b.imported,
                   b.grounding_score, b.grounding_code_count, b.grounding_commit_count,
                   b.grounding_session_count, b.grounding_forge_count,
                   b.verification_total, b.verification_passed, b.verification_failed,
                   b.verification_errored,
                   b.last_activity
            FROM belief_search bs
            JOIN beliefs b ON bs.id = b.id AND bs.source = b.source
            WHERE belief_search MATCH ?1
            ORDER BY rank
            LIMIT ?2
            "#,
        )?;

        let entries = stmt
            .query_map(params![sanitized, limit as i64], |row| {
                Ok(BeliefEntry {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    kind: row.get(2)?,
                    statement: row.get(3)?,
                    entrenchment: row.get(4)?,
                    status: BeliefStatus::from_str_or_default(&row.get::<_, String>(5)?),
                    facets: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                    cited_by_beliefs: row.get(7)?,
                    cited_by_sessions: row.get(8)?,
                    applied_in: row.get(9)?,
                    evidence_count: row.get(10)?,
                    evidence_verified: row.get(11)?,
                    health_score: row.get(12)?,
                    contested_by: row.get::<_, Option<String>>(13)?.unwrap_or_default(),
                    imported: row.get::<_, i32>(14).unwrap_or(0) != 0,
                    grounding_score: row.get::<_, Option<f64>>(15)?.unwrap_or(0.0),
                    grounding_code_count: row.get::<_, Option<i32>>(16)?.unwrap_or(0),
                    grounding_commit_count: row.get::<_, Option<i32>>(17)?.unwrap_or(0),
                    grounding_session_count: row.get::<_, Option<i32>>(18)?.unwrap_or(0),
                    grounding_forge_count: row.get::<_, Option<i32>>(19)?.unwrap_or(0),
                    verification_total: row.get::<_, Option<i32>>(20)?.unwrap_or(0),
                    verification_passed: row.get::<_, Option<i32>>(21)?.unwrap_or(0),
                    verification_failed: row.get::<_, Option<i32>>(22)?.unwrap_or(0),
                    verification_errored: row.get::<_, Option<i32>>(23)?.unwrap_or(0),
                    last_activity: row.get::<_, Option<String>>(24)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Find dangling edges: edges referencing belief IDs not in the beliefs table
    pub fn find_dangling_edges(&self) -> Result<Vec<(String, String, String, String)>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 'supports' AS type, s.from_belief, s.to_belief, s.source_project
            FROM belief_supports s
            WHERE s.from_belief NOT IN (SELECT id FROM beliefs)
               OR s.to_belief NOT IN (SELECT id FROM beliefs)
            UNION ALL
            SELECT 'attacks', a.from_belief, a.to_belief, a.source_project
            FROM belief_attacks a
            WHERE a.from_belief NOT IN (SELECT id FROM beliefs)
               OR a.to_belief NOT IN (SELECT id FROM beliefs)
            "#,
        )?;

        let dangling = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(dangling)
    }

    /// Delete dangling edges: edges referencing belief IDs not in the beliefs table
    ///
    /// Returns the number of edges deleted.
    pub fn delete_dangling_edges(&self) -> Result<usize> {
        let deleted_supports = self.conn.execute(
            r#"
            DELETE FROM belief_supports
            WHERE from_belief NOT IN (SELECT id FROM beliefs)
               OR to_belief NOT IN (SELECT id FROM beliefs)
            "#,
            [],
        )?;

        let deleted_attacks = self.conn.execute(
            r#"
            DELETE FROM belief_attacks
            WHERE from_belief NOT IN (SELECT id FROM beliefs)
               OR to_belief NOT IN (SELECT id FROM beliefs)
            "#,
            [],
        )?;

        Ok(deleted_supports + deleted_attacks)
    }

    /// Query beliefs that support a given belief (across all projects)
    pub fn query_supports(&self, belief_id: &str) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT from_belief, source_project FROM belief_supports WHERE to_belief = ?1 ORDER BY source_project, from_belief",
        )?;

        let results = stmt
            .query_map(params![belief_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Query beliefs that attack a given belief (across all projects)
    pub fn query_attacks(&self, belief_id: &str) -> Result<Vec<(String, String, bool)>> {
        let mut stmt = self.conn.prepare(
            "SELECT from_belief, source_project, defeated FROM belief_attacks WHERE to_belief = ?1 ORDER BY source_project, from_belief",
        )?;

        let results = stmt
            .query_map(params![belief_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i32>(2)? != 0,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Query which projects have a given belief
    pub fn query_projects(&self, belief_id: &str) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT source, entrenchment FROM beliefs WHERE id = ?1 ORDER BY source",
        )?;

        let results = stmt
            .query_map(params![belief_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Get a single belief by ID (with source project path from nodes table)
    ///
    /// Returns the belief entry and the source project's filesystem path.
    /// Used by `belief import` to locate the source file.
    pub fn get_belief(
        &self,
        belief_id: &str,
        source: &str,
    ) -> Result<Option<(BeliefEntry, PathBuf)>> {
        // Get belief
        let entry = self.conn.query_row(
            r#"
            SELECT b.id, b.source, b.kind, b.statement, b.entrenchment, b.status, b.facets,
                   b.cited_by_beliefs, b.cited_by_sessions, b.applied_in,
                   b.evidence_count, b.evidence_verified, b.health_score, b.contested_by,
                   b.imported,
                   b.grounding_score, b.grounding_code_count, b.grounding_commit_count,
                   b.grounding_session_count, b.grounding_forge_count,
                   b.verification_total, b.verification_passed, b.verification_failed,
                   b.verification_errored,
                   b.last_activity
            FROM beliefs b
            WHERE b.id = ?1 AND b.source = ?2
            "#,
            params![belief_id, source],
            |row| {
                Ok(BeliefEntry {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    kind: row.get(2)?,
                    statement: row.get(3)?,
                    entrenchment: row.get(4)?,
                    status: BeliefStatus::from_str_or_default(&row.get::<_, String>(5)?),
                    facets: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                    cited_by_beliefs: row.get(7)?,
                    cited_by_sessions: row.get(8)?,
                    applied_in: row.get(9)?,
                    evidence_count: row.get(10)?,
                    evidence_verified: row.get(11)?,
                    health_score: row.get(12)?,
                    contested_by: row.get::<_, Option<String>>(13)?.unwrap_or_default(),
                    imported: row.get::<_, i32>(14).unwrap_or(0) != 0,
                    grounding_score: row.get::<_, Option<f64>>(15)?.unwrap_or(0.0),
                    grounding_code_count: row.get::<_, Option<i32>>(16)?.unwrap_or(0),
                    grounding_commit_count: row.get::<_, Option<i32>>(17)?.unwrap_or(0),
                    grounding_session_count: row.get::<_, Option<i32>>(18)?.unwrap_or(0),
                    grounding_forge_count: row.get::<_, Option<i32>>(19)?.unwrap_or(0),
                    verification_total: row.get::<_, Option<i32>>(20)?.unwrap_or(0),
                    verification_passed: row.get::<_, Option<i32>>(21)?.unwrap_or(0),
                    verification_failed: row.get::<_, Option<i32>>(22)?.unwrap_or(0),
                    verification_errored: row.get::<_, Option<i32>>(23)?.unwrap_or(0),
                    last_activity: row.get::<_, Option<String>>(24)?,
                })
            },
        );

        match entry {
            Ok(e) => {
                // Get project path from nodes table
                let path: String = self.conn.query_row(
                    "SELECT path FROM nodes WHERE id = ?1",
                    params![source],
                    |row| row.get(0),
                )?;
                Ok(Some((e, PathBuf::from(path))))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Query which projects a belief appears in (from belief_applied_in table)
    ///
    /// Returns project names for the given belief ID.
    pub fn query_belief_applied_in(&self, belief_id: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT project FROM belief_applied_in WHERE belief_id = ?1 ORDER BY project",
        )?;

        let projects = stmt
            .query_map(params![belief_id], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(projects)
    }

    /// Sync belief_applied_in table (Phase E provenance)
    ///
    /// Per-source rebuild: deletes rows for synced sources, then repopulates.
    pub fn sync_belief_applied_in(
        &self,
        entries: &[BeliefEntry],
        synced_sources: &[String],
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute("BEGIN", [])?;

        // Delete rows for synced sources (per-source rebuild)
        for source in synced_sources {
            if let Err(e) = self.conn.execute(
                "DELETE FROM belief_applied_in WHERE project = ?1",
                params![source],
            ) {
                let _ = self.conn.execute("ROLLBACK", []);
                return Err(e.into());
            }
        }

        // Re-insert from synced entries
        for entry in entries {
            // originated = 1 when imported = 0 (native), 0 when imported = 1
            let originated = if entry.imported { 0 } else { 1 };
            if let Err(e) = self.conn.execute(
                r#"
                INSERT OR IGNORE INTO belief_applied_in (belief_id, project, originated, last_indexed)
                VALUES (?1, ?2, ?3, ?4)
                "#,
                params![entry.id, entry.source, originated, now],
            ) {
                let _ = self.conn.execute("ROLLBACK", []);
                return Err(e.into());
            }
        }

        self.conn.execute("COMMIT", [])?;
        Ok(())
    }

    /// Count belief entries
    pub fn belief_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM beliefs", [], |row| row.get(0))?;
        Ok(count as usize)
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

    fn make_belief_entry(
        id: &str,
        source: &str,
        kind: &str,
        statement: &str,
        entrenchment: &str,
        status: &str,
        facets: &str,
    ) -> BeliefEntry {
        BeliefEntry {
            id: id.to_string(),
            source: source.to_string(),
            kind: kind.to_string(),
            statement: statement.to_string(),
            entrenchment: entrenchment.to_string(),
            status: BeliefStatus::from_str_or_default(status),
            facets: facets.to_string(),
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
        }
    }

    #[test]
    fn test_sync_and_search_beliefs() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("graph.db");
        let conn = Connection::open(&db_path)?;
        let graph = Graph { conn };
        graph.init_schema()?;

        let entries = vec![
            make_belief_entry(
                "explicit-error-types",
                "patina",
                "belief",
                "Prefer explicit error types over string errors",
                "high",
                "active",
                "[\"rust\", \"error-handling\"]",
            ),
            make_belief_entry(
                "prefer-result-over-panics",
                "persona",
                "value",
                "I prefer Result<T,E> over panics for error handling",
                "medium",
                "active",
                "[\"rust\"]",
            ),
        ];

        let sources = vec!["patina".to_string(), "persona".to_string()];
        graph.sync_beliefs(&entries, &sources)?;
        assert_eq!(graph.belief_count()?, 2);

        // FTS5 search
        let results = graph.search_beliefs("error handling", 10)?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].source, "patina");
        assert_eq!(results[0].kind, "belief");

        // Search for something specific
        let results = graph.search_beliefs("panics", 10)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "prefer-result-over-panics");
        assert_eq!(results[0].source, "persona");
        assert_eq!(results[0].kind, "value");

        // Idempotent: sync again with same data → same count
        graph.sync_beliefs(&entries, &sources)?;
        assert_eq!(graph.belief_count()?, 2);

        // Empty results
        let results = graph.search_beliefs("nonexistent topic xyz", 10)?;
        assert!(results.is_empty());

        Ok(())
    }

    #[test]
    fn test_per_source_rebuild_preserves_other_sources() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("graph.db");
        let conn = Connection::open(&db_path)?;
        let graph = Graph { conn };
        graph.init_schema()?;

        // Sync project-a beliefs
        let entries_a = vec![make_belief_entry(
            "test-belief",
            "project-a",
            "belief",
            "Test belief from project A",
            "medium",
            "active",
            "[]",
        )];

        graph.sync_beliefs(&entries_a, &["project-a".to_string()])?;
        assert_eq!(graph.belief_count()?, 1);

        // Sync project-b beliefs — project-a should be preserved
        let entries_b = vec![
            make_belief_entry(
                "belief-one",
                "project-b",
                "belief",
                "First belief",
                "high",
                "active",
                "[]",
            ),
            make_belief_entry(
                "belief-two",
                "project-b",
                "belief",
                "Second belief",
                "low",
                "active",
                "[]",
            ),
        ];

        graph.sync_beliefs(&entries_b, &["project-b".to_string()])?;
        // project-a (1) + project-b (2) = 3 total
        assert_eq!(graph.belief_count()?, 3);

        // project-a belief still searchable
        let results = graph.search_beliefs("project A", 10)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, "project-a");

        // Re-sync project-a with 0 entries → old project-a entries deleted
        graph.sync_beliefs(&[], &["project-a".to_string()])?;
        assert_eq!(graph.belief_count()?, 2); // only project-b remains

        let results = graph.search_beliefs("project A", 10)?;
        assert!(results.is_empty());

        Ok(())
    }

    #[test]
    fn test_sanitize_fts5_query() {
        // Basic words → quoted
        assert_eq!(sanitize_fts5_query("secrets"), r#""secrets""#);
        assert_eq!(
            sanitize_fts5_query("error handling"),
            r#""error" "handling""#
        );

        // Hyphens split into separate quoted tokens
        assert_eq!(
            sanitize_fts5_query("secrets-llm-boundary"),
            r#""secrets" "llm" "boundary""#
        );

        // Mixed hyphens and spaces
        assert_eq!(
            sanitize_fts5_query("use-whats-in the tree"),
            r#""use" "whats" "in" "the" "tree""#
        );

        // Existing quotes stripped and re-wrapped
        assert_eq!(
            sanitize_fts5_query(r#""already quoted""#),
            r#""already" "quoted""#
        );

        // Empty / whitespace-only
        assert_eq!(sanitize_fts5_query(""), "");
        assert_eq!(sanitize_fts5_query("  "), "");
    }

    #[test]
    fn test_search_beliefs_with_column_name_token() -> Result<()> {
        // Regression: "secrets-llm-boundary" caused "no such column: llm"
        // because FTS5 interpreted bare `llm` as a column reference.
        let dir = tempdir()?;
        let db_path = dir.path().join("graph.db");
        let conn = Connection::open(&db_path)?;
        let graph = Graph { conn };
        graph.init_schema()?;

        let entries = vec![make_belief_entry(
            "secrets-llm-boundary",
            "patina",
            "belief",
            "Never expose secrets to LLM context windows",
            "high",
            "active",
            "[\"security\", \"llm\"]",
        )];

        graph.sync_beliefs(&entries, &["patina".to_string()])?;

        // These queries previously crashed with "no such column: llm"
        let results = graph.search_beliefs("secrets-llm-boundary", 10)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "secrets-llm-boundary");

        let results = graph.search_beliefs("llm", 10)?;
        assert_eq!(results.len(), 1);

        let results = graph.search_beliefs("secrets llm boundary", 10)?;
        assert_eq!(results.len(), 1);

        Ok(())
    }
}
