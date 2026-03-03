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

/// Populate FTS5 index from eventlog code events.
///
/// When `changed_files` is Some, only rebuilds FTS5 rows for those file paths
/// (incremental update). When None, does a full rebuild of all code.* entries.
/// See [[scrape-diff-driven]] EC6.
pub fn populate_fts5(conn: &Connection, changed_files: Option<&[String]>) -> Result<usize> {
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

    match changed_files {
        Some(files) if !files.is_empty() => {
            // Incremental: delete + re-insert only for changed files.
            // source_id in eventlog is "path::symbol", so match with prefix.
            let mut total = 0;
            for file in files {
                let prefix = format!("{}%", file);

                // Delete existing FTS entries for this file
                conn.execute(
                    "DELETE FROM code_fts WHERE file_path LIKE ?1 AND event_type LIKE 'code.%'",
                    [&prefix],
                )?;

                // Re-insert from eventlog for this file only
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
                      AND source_id LIKE ?1
                    GROUP BY source_id, event_type
                    "#,
                    [&prefix],
                )?;
                total += count;
            }
            Ok(total)
        }
        _ => {
            // Full rebuild (force, rebuild, or standalone scrape code)
            conn.execute("DELETE FROM code_fts WHERE event_type LIKE 'code.%'", [])?;

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
    }
}

/// Populate FTS5 index for commit messages (git narrative search).
///
/// When `full` is false (default), only inserts commits not already in the FTS5
/// index. Commits are immutable so incremental = append-only. When `full` is true,
/// does DELETE + full rebuild. See [[scrape-diff-driven]] EC6.
pub fn populate_commits_fts5(conn: &Connection, full: bool) -> Result<usize> {
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

    if full {
        // Full rebuild
        conn.execute("DELETE FROM commits_fts", [])?;

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
    } else {
        // Incremental: only insert commits not yet in FTS5 index
        let count = conn.execute(
            r#"
            INSERT INTO commits_fts (sha, message, author_name)
            SELECT c.sha, c.message, c.author_name
            FROM commits c
            LEFT JOIN commits_fts f ON c.sha = f.sha
            WHERE c.message IS NOT NULL
              AND f.sha IS NULL
            "#,
            [],
        )?;
        Ok(count)
    }
}

/// Populate FTS5 index for session events (keyword search over decisions, patterns, work, context)
pub fn populate_eventlog_fts5(conn: &Connection) -> Result<usize> {
    // Create FTS5 table if it doesn't exist (migration for existing databases)
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS eventlog_fts USING fts5(
            source_id UNINDEXED,
            event_type UNINDEXED,
            content,
            tokenize='porter unicode61'
        )",
        [],
    )?;

    // Clear existing FTS5 data
    conn.execute("DELETE FROM eventlog_fts", [])?;

    // Populate from session observation events in eventlog.
    // GROUP BY dedupes across multiple scrape runs (eventlog is append-only).
    // Same dedup strategy and filters as query_session_corpus() in oxidize.
    let count = conn.execute(
        r#"
        INSERT INTO eventlog_fts (source_id, event_type, content)
        SELECT source_id, event_type,
               json_extract(data, '$.content') as content
        FROM eventlog
        WHERE event_type IN ('session.decision', 'session.pattern',
                             'session.work', 'session.context')
          AND length(json_extract(data, '$.content')) > 50
        GROUP BY source_id, event_type, json_extract(data, '$.content')
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
