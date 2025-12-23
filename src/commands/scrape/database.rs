//! Unified database schema for all scraped events
//!
//! Following the LiveStore pattern, we maintain:
//! - eventlog table (immutable source of truth)
//! - materialized views (derived, rebuildable)

use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

/// Path to unified database
pub const PATINA_DB: &str = ".patina/data/patina.db";

/// Initialize the unified patina.db with eventlog table and indexes
pub fn initialize(db_path: &Path) -> Result<Connection> {
    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(db_path)?;

    // Create eventlog table (LiveStore pattern - immutable source of truth)
    conn.execute_batch(
        r#"
        -- Eventlog: Unified source of truth for ALL events
        CREATE TABLE IF NOT EXISTS eventlog (
            seq INTEGER PRIMARY KEY AUTOINCREMENT,  -- Global ordering
            event_type TEXT NOT NULL,                -- e.g. 'git.commit', 'session.decision'
            timestamp TEXT NOT NULL,                 -- ISO8601 when event occurred
            source_id TEXT NOT NULL,                 -- sha, session_id, function_name, etc
            source_file TEXT,                        -- Original file path
            data TEXT NOT NULL,                      -- Event-specific JSON payload
            CHECK(json_valid(data))
        );

        -- Indexes for common queries
        CREATE INDEX IF NOT EXISTS idx_eventlog_type ON eventlog(event_type);
        CREATE INDEX IF NOT EXISTS idx_eventlog_timestamp ON eventlog(timestamp);
        CREATE INDEX IF NOT EXISTS idx_eventlog_source ON eventlog(source_id);
        CREATE INDEX IF NOT EXISTS idx_eventlog_type_time ON eventlog(event_type, timestamp);

        -- Scrape metadata (track last processed for incremental updates)
        CREATE TABLE IF NOT EXISTS scrape_meta (
            key TEXT PRIMARY KEY,
            value TEXT
        );

        -- FTS5 virtual table for exact-match lexical search
        CREATE VIRTUAL TABLE IF NOT EXISTS code_fts USING fts5(
            symbol_name,
            file_path,
            content,
            event_type,
            tokenize='porter unicode61'
        );
        "#,
    )?;

    Ok(conn)
}

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

/// Insert an event into the unified eventlog
pub fn insert_event(
    conn: &Connection,
    event_type: &str,
    timestamp: &str,
    source_id: &str,
    source_file: Option<&str>,
    data: &str,
) -> Result<i64> {
    let seq = conn.execute(
        "INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![event_type, timestamp, source_id, source_file, data],
    )?;
    Ok(seq as i64)
}

/// Get the last processed value for a scraper (for incremental updates)
pub fn get_last_processed(conn: &Connection, scraper: &str) -> Result<Option<String>> {
    let key = format!("last_processed_{}", scraper);
    let result: Result<String, _> = conn.query_row(
        "SELECT value FROM scrape_meta WHERE key = ?1",
        [&key],
        |row| row.get(0),
    );

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Update the last processed value for a scraper
pub fn set_last_processed(conn: &Connection, scraper: &str, value: &str) -> Result<()> {
    let key = format!("last_processed_{}", scraper);
    conn.execute(
        "INSERT OR REPLACE INTO scrape_meta (key, value) VALUES (?1, ?2)",
        rusqlite::params![&key, value],
    )?;
    Ok(())
}

// ============================================================================
// Feedback Loop Views (Phase 3)
// ============================================================================

/// Create SQL views for feedback loop analysis
///
/// These views correlate scry queries with subsequent commits to measure
/// retrieval precision in real-world usage.
pub fn create_feedback_views(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- View: Queries made during each session
        CREATE VIEW IF NOT EXISTS feedback_session_queries AS
        SELECT
            json_extract(data, '$.session_id') as session_id,
            json_extract(data, '$.query_id') as query_id,
            json_extract(data, '$.query') as query,
            json_extract(data, '$.mode') as mode,
            json_extract(data, '$.results') as results,
            timestamp
        FROM eventlog
        WHERE event_type = 'scry.query'
          AND json_extract(data, '$.session_id') IS NOT NULL;

        -- View: Files committed during each session (from latest scrape only)
        -- Uses window function to get only the most recent event per commit SHA
        CREATE VIEW IF NOT EXISTS feedback_commit_files AS
        SELECT
            session_id,
            sha,
            file_path,
            change_type,
            timestamp
        FROM (
            SELECT
                json_extract(data, '$.session_id') as session_id,
                json_extract(data, '$.sha') as sha,
                json_extract(f.value, '$.path') as file_path,
                json_extract(f.value, '$.change_type') as change_type,
                timestamp,
                ROW_NUMBER() OVER (PARTITION BY json_extract(data, '$.sha') ORDER BY seq DESC) as rn
            FROM eventlog, json_each(json_extract(data, '$.files')) as f
            WHERE event_type = 'git.commit'
              AND json_extract(data, '$.session_id') IS NOT NULL
        )
        WHERE rn = 1;

        -- View: Query results matched to committed files
        -- A "hit" is when a retrieved doc_id matches a file that was committed
        CREATE VIEW IF NOT EXISTS feedback_query_hits AS
        SELECT
            q.session_id,
            q.query,
            q.mode,
            q.timestamp as query_time,
            json_extract(r.value, '$.doc_id') as retrieved_doc_id,
            json_extract(r.value, '$.rank') as rank,
            json_extract(r.value, '$.score') as score,
            CASE
                WHEN EXISTS (
                    SELECT 1 FROM feedback_commit_files cf
                    WHERE cf.session_id = q.session_id
                      AND cf.file_path LIKE '%' || json_extract(r.value, '$.doc_id') || '%'
                ) THEN 1
                ELSE 0
            END as is_hit
        FROM feedback_session_queries q,
             json_each(q.results) as r;

        -- View: scry.use events (Phase 3) - explicit result usage from agents
        CREATE VIEW IF NOT EXISTS feedback_usage AS
        SELECT
            json_extract(data, '$.query_id') as query_id,
            json_extract(data, '$.result_used') as doc_id,
            json_extract(data, '$.rank') as rank,
            json_extract(data, '$.session_id') as session_id,
            timestamp
        FROM eventlog
        WHERE event_type = 'scry.use';

        -- View: scry.feedback events (Phase 3) - explicit good/bad ratings
        CREATE VIEW IF NOT EXISTS feedback_ratings AS
        SELECT
            json_extract(data, '$.query_id') as query_id,
            json_extract(data, '$.signal') as signal,
            json_extract(data, '$.comment') as comment,
            json_extract(data, '$.session_id') as session_id,
            timestamp
        FROM eventlog
        WHERE event_type = 'scry.feedback';

        -- View: Combined query analysis with usage and feedback
        CREATE VIEW IF NOT EXISTS feedback_query_analysis AS
        SELECT
            q.session_id,
            json_extract(q.data, '$.query_id') as query_id,
            json_extract(q.data, '$.query') as query,
            json_extract(q.data, '$.mode') as mode,
            q.timestamp as query_time,
            (SELECT COUNT(*) FROM eventlog u
             WHERE u.event_type = 'scry.use'
               AND json_extract(u.data, '$.query_id') = json_extract(q.data, '$.query_id')
            ) as use_count,
            (SELECT json_group_array(json_extract(u.data, '$.rank'))
             FROM eventlog u
             WHERE u.event_type = 'scry.use'
               AND json_extract(u.data, '$.query_id') = json_extract(q.data, '$.query_id')
            ) as used_ranks,
            (SELECT json_extract(f.data, '$.signal')
             FROM eventlog f
             WHERE f.event_type = 'scry.feedback'
               AND json_extract(f.data, '$.query_id') = json_extract(q.data, '$.query_id')
             LIMIT 1
            ) as feedback_signal
        FROM eventlog q
        WHERE q.event_type = 'scry.query';
        "#,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Count events by type (test helper)
    fn count_events_by_type(conn: &Connection, event_type: &str) -> Result<i64> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM eventlog WHERE event_type = ?1",
            [event_type],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get total event count (test helper)
    fn count_total_events(conn: &Connection) -> Result<i64> {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM eventlog", [], |row| row.get(0))?;
        Ok(count)
    }
    use tempfile::tempdir;

    #[test]
    fn test_initialize_creates_tables() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.db");
        let conn = initialize(&db_path)?;

        // Check eventlog table exists
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;

        assert!(tables.contains(&"eventlog".to_string()));
        assert!(tables.contains(&"scrape_meta".to_string()));

        Ok(())
    }

    #[test]
    fn test_insert_and_count_events() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.db");
        let conn = initialize(&db_path)?;

        // Insert a test event
        let data = r#"{"message": "test commit", "author": "test"}"#;
        insert_event(
            &conn,
            "git.commit",
            "2025-11-21T12:00:00Z",
            "abc123",
            Some("test.rs"),
            data,
        )?;

        // Count events
        assert_eq!(count_total_events(&conn)?, 1);
        assert_eq!(count_events_by_type(&conn, "git.commit")?, 1);
        assert_eq!(count_events_by_type(&conn, "session.decision")?, 0);

        Ok(())
    }

    #[test]
    fn test_last_processed_tracking() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.db");
        let conn = initialize(&db_path)?;

        // Initially no value
        assert_eq!(get_last_processed(&conn, "git")?, None);

        // Set value
        set_last_processed(&conn, "git", "abc123")?;
        assert_eq!(
            get_last_processed(&conn, "git")?,
            Some("abc123".to_string())
        );

        // Update value
        set_last_processed(&conn, "git", "def456")?;
        assert_eq!(
            get_last_processed(&conn, "git")?,
            Some("def456".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_json_validation() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test.db");
        let conn = initialize(&db_path)?;

        // Valid JSON should work
        let valid_json = r#"{"key": "value"}"#;
        assert!(insert_event(
            &conn,
            "test.event",
            "2025-11-21T12:00:00Z",
            "test1",
            None,
            valid_json
        )
        .is_ok());

        // Invalid JSON should fail
        let invalid_json = r#"{not valid json"#;
        assert!(insert_event(
            &conn,
            "test.event",
            "2025-11-21T12:00:00Z",
            "test2",
            None,
            invalid_json
        )
        .is_err());

        Ok(())
    }
}
