//! Temporal oracle - Git co-change relationships
//!
//! Finds files that frequently change together based on git history.
//! Given a query, finds files matching the query pattern and returns
//! their co-change neighbors ranked by frequency.

use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::retrieval::oracle::{Oracle, OracleMetadata, OracleResult};

pub struct TemporalOracle {
    db_path: PathBuf,
}

impl TemporalOracle {
    pub fn new() -> Self {
        Self {
            db_path: PathBuf::from(".patina/local/data/patina.db"),
        }
    }

    /// Query co-change neighbors for files matching the query pattern
    fn query_co_changes(&self, query: &str, limit: usize) -> Result<Vec<CoChangeResult>> {
        let conn = Connection::open(&self.db_path)?;

        // Check if co_changes table exists and has data
        let has_data: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM co_changes LIMIT 1)",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_data {
            return Ok(Vec::new());
        }

        // Build search pattern from query
        // Split on whitespace and create LIKE patterns for each term
        let terms: Vec<String> = query
            .split_whitespace()
            .map(|t| format!("%{}%", t.to_lowercase()))
            .collect();

        if terms.is_empty() {
            return Ok(Vec::new());
        }

        // Find files matching any query term, then get their co-change neighbors
        // Aggregate counts for files that appear as neighbors to multiple matches
        let mut neighbor_counts: HashMap<String, i32> = HashMap::new();
        let mut neighbor_sources: HashMap<String, Vec<String>> = HashMap::new();

        for term in &terms {
            // Query for neighbors where file_a matches the pattern
            let mut stmt = conn.prepare(
                "SELECT file_b, count, file_a FROM co_changes
                 WHERE LOWER(file_a) LIKE ?1
                 ORDER BY count DESC
                 LIMIT ?2",
            )?;

            let rows = stmt.query_map([term, &(limit * 3).to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;

            for row in rows {
                let (neighbor, count, source) = row?;
                *neighbor_counts.entry(neighbor.clone()).or_insert(0) += count;
                neighbor_sources.entry(neighbor).or_default().push(source);
            }

            // Query for neighbors where file_b matches the pattern
            let mut stmt = conn.prepare(
                "SELECT file_a, count, file_b FROM co_changes
                 WHERE LOWER(file_b) LIKE ?1
                 ORDER BY count DESC
                 LIMIT ?2",
            )?;

            let rows = stmt.query_map([term, &(limit * 3).to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;

            for row in rows {
                let (neighbor, count, source) = row?;
                *neighbor_counts.entry(neighbor.clone()).or_insert(0) += count;
                neighbor_sources.entry(neighbor).or_default().push(source);
            }
        }

        // Remove files that match the query themselves (we want neighbors, not matches)
        let query_lower = query.to_lowercase();
        neighbor_counts.retain(|file, _| !file.to_lowercase().contains(&query_lower));

        // Convert to results and sort by aggregated count
        let mut results: Vec<CoChangeResult> = neighbor_counts
            .into_iter()
            .map(|(file, count)| {
                let sources = neighbor_sources.remove(&file).unwrap_or_default();
                CoChangeResult {
                    file_path: file,
                    co_change_count: count,
                    related_to: sources,
                }
            })
            .collect();

        results.sort_by(|a, b| b.co_change_count.cmp(&a.co_change_count));
        results.truncate(limit);

        Ok(results)
    }
}

/// Internal result from co-change query
struct CoChangeResult {
    file_path: String,
    co_change_count: i32,
    related_to: Vec<String>,
}

impl Oracle for TemporalOracle {
    fn name(&self) -> &'static str {
        "temporal"
    }

    fn query(&self, query: &str, limit: usize) -> Result<Vec<OracleResult>> {
        let results = self.query_co_changes(query, limit)?;
        let source = self.name();

        Ok(results
            .into_iter()
            .map(|r| {
                // Generate content describing the co-change relationship
                let related_files: Vec<&str> =
                    r.related_to.iter().take(3).map(|s| s.as_str()).collect();
                let content = if related_files.is_empty() {
                    format!("{} (co-changes: {})", r.file_path, r.co_change_count)
                } else {
                    format!(
                        "{} — co-changes {} times with: {}",
                        r.file_path,
                        r.co_change_count,
                        related_files.join(", ")
                    )
                };

                OracleResult {
                    doc_id: r.file_path.clone(),
                    content,
                    source,
                    score: r.co_change_count as f32,
                    score_type: "co_change_count",
                    metadata: OracleMetadata {
                        file_path: Some(r.file_path),
                        timestamp: None,
                        event_type: Some("co-change".to_string()),
                        matches: None,
                    },
                }
            })
            .collect())
    }

    fn is_available(&self) -> bool {
        if !self.db_path.exists() {
            return false;
        }

        // Check if co_changes table has data
        Connection::open(&self.db_path)
            .and_then(|conn| {
                conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM co_changes LIMIT 1)",
                    [],
                    |row| row.get::<_, bool>(0),
                )
            })
            .unwrap_or(false)
    }
}
