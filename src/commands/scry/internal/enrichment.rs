//! Result enrichment wrappers for scry command.

use std::collections::HashMap;

use anyhow::Result;
use rusqlite::Connection;

use super::super::ScryResult;

pub use crate::retrieval::enrichment::{truncate_content, SearchResults};

pub fn enrich_results(
    conn: &Connection,
    results: &SearchResults,
    dimension: &str,
    min_score: f32,
) -> Result<Vec<ScryResult>> {
    let enriched =
        crate::retrieval::enrichment::enrich_results(conn, results, dimension, min_score)?;
    Ok(enriched
        .into_iter()
        .map(|result| ScryResult {
            id: result.id,
            content: result.content,
            score: result.score,
            event_type: result.event_type,
            source_id: result.source_id,
            timestamp: result.timestamp,
        })
        .collect())
}

/// Find beliefs related to code results via multi-hop grounding (E4.6a-fix)
///
/// Uses belief_code_reach table (belief → commit → file) instead of broken
/// direct cosine similarity. Extracts file_path from code result source_id
/// and looks up which beliefs reach that file through commit neighbors.
///
/// Returns a map from source_id (e.g., "src/foo.rs::bar") to matching beliefs.
pub fn find_belief_impact(results: &[ScryResult]) -> Result<HashMap<String, Vec<(String, f32)>>> {
    let db_path = patina::eventlog::patina_db_path()?;
    let conn = Connection::open(db_path)?;

    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='belief_code_reach'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !table_exists {
        return Ok(HashMap::new());
    }

    let mut impact_map: HashMap<String, Vec<(String, f32)>> = HashMap::new();

    let mut reach_stmt = conn.prepare(
        "SELECT belief_id, reach_score FROM belief_code_reach
         WHERE file_path = ?1
         ORDER BY reach_score DESC
         LIMIT 3",
    )?;

    for result in results.iter().filter(|r| r.event_type.starts_with("code.")) {
        let raw_path = result
            .source_id
            .split("::")
            .next()
            .unwrap_or(&result.source_id);
        let file_path = raw_path.strip_prefix("./").unwrap_or(raw_path);

        let beliefs: Vec<(String, f32)> = reach_stmt
            .query_map([file_path], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f32>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        if !beliefs.is_empty() {
            impact_map.insert(result.source_id.clone(), beliefs);
        }
    }

    Ok(impact_map)
}
