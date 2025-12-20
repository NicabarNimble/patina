//! Structural oracle - module signals from assay derive

use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;

use crate::retrieval::oracle::{Oracle, OracleMetadata, OracleResult};

pub struct StructuralOracle {
    db_path: PathBuf,
}

impl StructuralOracle {
    pub fn new() -> Self {
        Self {
            db_path: PathBuf::from(".patina/data/patina.db"),
        }
    }
}

impl Oracle for StructuralOracle {
    fn name(&self) -> &'static str {
        "structural"
    }

    fn query(&self, query: &str, limit: usize) -> Result<Vec<OracleResult>> {
        let conn = Connection::open(&self.db_path)?;

        // Check if module_signals table exists
        let table_exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='module_signals'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !table_exists {
            return Ok(vec![]);
        }

        // Search for modules matching the query
        // Match against path and prioritize by importer_count (usage) and centrality
        let sql = r#"
            SELECT
                path,
                is_used,
                importer_count,
                activity_level,
                centrality_score
            FROM module_signals
            WHERE path LIKE ?
            ORDER BY
                importer_count DESC,
                centrality_score DESC
            LIMIT ?
        "#;

        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(sql)?;

        let results: Vec<OracleResult> = stmt
            .query_map([&pattern, &limit.to_string()], |row| {
                let path: String = row.get(0)?;
                let is_used: bool = row.get::<_, i32>(1)? != 0;
                let importer_count: i64 = row.get(2)?;
                let activity_level: String = row.get(3)?;
                let centrality_score: f64 = row.get(4)?;

                // Build content summary
                let content = format!(
                    "{}: {} importers, activity={}, centrality={:.2}{}",
                    path,
                    importer_count,
                    activity_level,
                    centrality_score,
                    if is_used { " [used]" } else { " [unused]" }
                );

                Ok(OracleResult {
                    doc_id: path.clone(),
                    content,
                    source: "structural",
                    metadata: OracleMetadata {
                        file_path: Some(path),
                        timestamp: None,
                        event_type: Some("module.signal".to_string()),
                    },
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    fn is_available(&self) -> bool {
        if !self.db_path.exists() {
            return false;
        }

        // Check if module_signals table exists and has data
        if let Ok(conn) = Connection::open(&self.db_path) {
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM module_signals", [], |row| row.get(0))
                .unwrap_or(0);
            count > 0
        } else {
            false
        }
    }
}
