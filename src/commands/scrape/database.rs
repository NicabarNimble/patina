//! Scrape database module — re-exports shared eventlog infrastructure
//!
//! The core eventlog (schema, insert_event, initialize) lives in `patina::eventlog`.
//! This module re-exports those symbols for backward compatibility with scrape
//! submodules that `use super::database`, and adds scrape-specific FTS population.

use anyhow::Result;
use rusqlite::Connection;

// Re-export shared eventlog infrastructure
// All scrape submodules use `database::insert_event`, `database::PATINA_DB`, etc.
pub use patina::eventlog::get_last_processed;
pub use patina::eventlog::initialize;
pub use patina::eventlog::insert_event;
pub use patina::eventlog::is_ref_repo;
pub use patina::eventlog::set_last_processed;
pub use patina::eventlog::PATINA_DB;

// ============================================================================
// Scrape-specific FTS population (not shared infrastructure)
// ============================================================================

/// Populate FTS5 index from eventlog code events
pub fn populate_fts5(conn: &Connection) -> Result<usize> {
    // Create FTS5 table if it doesn't exist (migration for existing databases)
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS code_fts USING fts5(
            symbol_name,
            file_path,
            content,
            event_type,
            tokenize='porter unicode61'
        )",
        [],
    )?;

    // Clear existing FTS5 data
    conn.execute("DELETE FROM code_fts", [])?;

    // Populate from code events in eventlog
    // Note: Exclude 'code.symbol' to avoid duplication - functions/types already
    // have richer fact types (code.function, code.struct, etc.) that are indexed.
    // GROUP BY dedupes across multiple scrape runs (eventlog is append-only).
    // See: spec-fts-deduplication.md for full context on this fix.
    let count = conn.execute(
        r#"
        INSERT INTO code_fts (symbol_name, file_path, content, event_type)
        SELECT
            json_extract(data, '$.name') as symbol_name,
            source_id as file_path,
            COALESCE(json_extract(data, '$.content'), json_extract(data, '$.signature'), '') as content,
            event_type
        FROM eventlog
        WHERE event_type LIKE 'code.%'
          AND event_type != 'code.symbol'
          AND json_extract(data, '$.name') IS NOT NULL
        GROUP BY source_id, event_type
        "#,
        [],
    )?;

    Ok(count)
}

/// Populate FTS5 index for commit messages (git narrative search)
pub fn populate_commits_fts5(conn: &Connection) -> Result<usize> {
    // Create FTS5 table if it doesn't exist (migration for existing databases)
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS commits_fts USING fts5(
            sha,
            message,
            author_name,
            tokenize='porter unicode61'
        )",
        [],
    )?;

    // Clear existing FTS5 data
    conn.execute("DELETE FROM commits_fts", [])?;

    // Populate from commits table (materialized view)
    let count = conn.execute(
        r#"
        INSERT INTO commits_fts (sha, message, author_name)
        SELECT sha, message, author_name
        FROM commits
        WHERE message IS NOT NULL
        "#,
        [],
    )?;

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_reexports_work() -> Result<()> {
        // Verify re-exported functions are accessible through this module
        let dir = tempdir()?;
        let db_path = dir.path().join("test.db");
        let conn = initialize(&db_path)?;

        let data = r#"{"test": true}"#;
        insert_event(
            &conn,
            "test.event",
            "2026-01-30T00:00:00Z",
            "test1",
            None,
            data,
        )?;

        assert_eq!(get_last_processed(&conn, "test")?, None);
        set_last_processed(&conn, "test", "val")?;
        assert_eq!(get_last_processed(&conn, "test")?, Some("val".to_string()));

        Ok(())
    }
}
