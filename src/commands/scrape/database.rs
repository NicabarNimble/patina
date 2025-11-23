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
        "#,
    )?;

    Ok(conn)
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
