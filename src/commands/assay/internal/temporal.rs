//! Temporal co-change analysis
//!
//! Finds files that frequently change together based on git history.
//! Given a query, finds files matching the query pattern and returns
//! their co-change neighbors ranked by frequency.
//!
//! Moved from scry during semantic-structural split (Phase 1).
//! Co-change relationships are factual data — they belong in assay.

use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;

use super::search::SearchResult;

/// Query co-change neighbors for files matching the query pattern
#[allow(dead_code)] // Used in Phase 1 completion when temporal oracle removed from scry
pub fn query_co_changes(conn: &Connection, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
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
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|t| format!("%{}%", t.to_lowercase()))
        .collect();

    if terms.is_empty() {
        return Ok(Vec::new());
    }

    // Find files matching any query term, then get their co-change neighbors
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

    // Remove files that match the query themselves
    let query_lower = query.to_lowercase();
    neighbor_counts.retain(|file, _| !file.to_lowercase().contains(&query_lower));

    // Convert to results and sort by aggregated count
    let mut results: Vec<(String, i32, Vec<String>)> = neighbor_counts
        .into_iter()
        .map(|(file, count)| {
            let sources = neighbor_sources.remove(&file).unwrap_or_default();
            (file, count, sources)
        })
        .collect();

    results.sort_by(|a, b| b.1.cmp(&a.1));
    results.truncate(limit);

    Ok(results
        .into_iter()
        .map(|(file_path, count, related_to)| {
            let related_files: Vec<&str> = related_to.iter().take(3).map(|s| s.as_str()).collect();
            let content = if related_files.is_empty() {
                format!("{} (co-changes: {})", file_path, count)
            } else {
                format!(
                    "{} — co-changes {} times with: {}",
                    file_path,
                    count,
                    related_files.join(", ")
                )
            };

            SearchResult {
                content,
                score: count as f32,
                event_type: "co-change".to_string(),
                source_id: file_path,
                timestamp: String::new(),
            }
        })
        .collect())
}

/// Check if co_changes data is available
#[allow(dead_code)] // Used in Phase 1 completion when temporal oracle removed from scry
pub fn is_available(db_path: &str) -> bool {
    if !std::path::Path::new(db_path).exists() {
        return false;
    }

    Connection::open(db_path)
        .and_then(|conn| {
            conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM co_changes LIMIT 1)",
                [],
                |row| row.get::<_, bool>(0),
            )
        })
        .unwrap_or(false)
}

/// Execute co-change analysis for a specific file (CLI output)
pub fn execute_cochange(file: &str, limit: usize, db_path: &str) -> Result<()> {
    let conn = Connection::open(db_path)?;

    let mut stmt = conn.prepare(
        "SELECT file_b, count FROM co_changes
         WHERE file_a = ?1
         UNION ALL
         SELECT file_a, count FROM co_changes
         WHERE file_b = ?1
         ORDER BY count DESC
         LIMIT ?2",
    )?;

    let rows: Vec<(String, i32)> = stmt
        .query_map(rusqlite::params![file, limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if rows.is_empty() {
        println!("No co-change data found for: {}", file);
        println!("Run 'patina scrape' to index git history.");
        return Ok(());
    }

    println!("Co-change analysis for: {}\n", file);
    println!("{}", "─".repeat(60));

    for (i, (neighbor, count)) in rows.iter().enumerate() {
        println!("[{}] {} (co-changes: {})", i + 1, neighbor, count);
    }

    println!("{}", "─".repeat(60));
    Ok(())
}

/// Execute co-change analysis and return JSON
pub fn execute_cochange_json(file: &str, limit: usize, db_path: &str) -> Result<String> {
    let conn = Connection::open(db_path)?;

    let mut stmt = conn.prepare(
        "SELECT file_b, count FROM co_changes
         WHERE file_a = ?1
         UNION ALL
         SELECT file_a, count FROM co_changes
         WHERE file_b = ?1
         ORDER BY count DESC
         LIMIT ?2",
    )?;

    let rows: Vec<serde_json::Value> = stmt
        .query_map(rusqlite::params![file, limit], |row| {
            let neighbor: String = row.get(0)?;
            let count: i32 = row.get(1)?;
            Ok(serde_json::json!({
                "file": neighbor,
                "co_change_count": count,
            }))
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(serde_json::to_string_pretty(&rows)?)
}
