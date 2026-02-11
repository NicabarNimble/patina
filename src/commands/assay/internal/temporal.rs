//! Temporal co-change analysis
//!
//! Finds files that frequently change together based on git history.
//! Given a file, returns its co-change neighbors ranked by frequency.
//!
//! Moved from scry during semantic-structural split (Phase 1).
//! Co-change relationships are factual data — they belong in assay.

use anyhow::Result;
use rusqlite::Connection;

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
