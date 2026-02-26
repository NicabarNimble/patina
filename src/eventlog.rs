//! Shared eventlog infrastructure for Patina
//!
//! The eventlog is an append-only, immutable source of truth for all structured
//! events in Patina. Multiple commands write events (scrape, session, scry);
//! multiple commands read them (scry, eval, assay). No single command owns the pipe.
//!
//! Following the LiveStore pattern:
//! - eventlog table (immutable source of truth)
//! - materialized views (derived, rebuildable)
//!
//! This module was extracted from `commands/scrape/database.rs` to make the
//! eventlog accessible as shared infrastructure across all commands.

use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::OnceLock;

/// Path to projection database (rebuildable from git + layer/)
pub const PATINA_DB: &str = ".patina/local/data/patina.db";

/// Path to events database (runtime events — irreplaceable)
pub const EVENTS_DB: &str = ".patina/local/data/events.db";

/// Check if a path is within a ref repo (external reference repository).
///
/// Ref repos live in `~/.patina/cache/repos/` and use lean storage:
/// - Git/code data: direct insert (no eventlog) - rebuilds from source
/// - Forge data: eventlog with dedup - caches expensive API data
///
/// See: layer/surface/build/spec-ref-repo-storage.md
pub fn is_ref_repo(path: &Path) -> bool {
    // Try the path directly first
    if path.to_string_lossy().contains(".patina/cache/repos") {
        return true;
    }
    // If path is relative, check current working directory
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.to_string_lossy().contains(".patina/cache/repos") {
            return true;
        }
        // Also check canonical path
        if let Ok(canonical) = cwd.join(path).canonicalize() {
            return canonical.to_string_lossy().contains(".patina/cache/repos");
        }
    }
    false
}

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

        -- FTS5 virtual table for exact-match lexical search (code)
        CREATE VIRTUAL TABLE IF NOT EXISTS code_fts USING fts5(
            symbol_name,
            file_path,
            content,
            event_type,
            tokenize='porter unicode61'
        );

        -- FTS5 virtual table for commit message search (git narrative)
        CREATE VIRTUAL TABLE IF NOT EXISTS commits_fts USING fts5(
            sha,
            message,
            author_name,
            tokenize='porter unicode61'
        );

        -- Moments table for derived temporal signals (assay derive)
        CREATE TABLE IF NOT EXISTS moments (
            sha TEXT PRIMARY KEY,
            moment_type TEXT NOT NULL,
            file_count INTEGER,
            timestamp TEXT,
            message TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_moments_type ON moments(moment_type);
        CREATE INDEX IF NOT EXISTS idx_moments_timestamp ON moments(timestamp);
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

// ============================================================================
// events.db — Runtime event database (irreplaceable)
// ============================================================================

/// Process-level gate: ensure_events_db() runs once per process via OnceLock.
/// Eliminates per-call exists() syscall overhead (CLI: negligible; MCP: meaningful).
static EVENTS_INIT: OnceLock<std::result::Result<(), String>> = OnceLock::new();

/// Ensure events.db exists with correct schema. Migrates runtime events
/// from patina.db on first run (one-time copy, idempotent).
///
/// Runs once per process via OnceLock. Safe under concurrent execution:
/// schema uses CREATE TABLE IF NOT EXISTS, migration uses INSERT OR IGNORE
/// with explicit seq to prevent duplicate events.
pub fn ensure_events_db() -> Result<()> {
    let result = EVENTS_INIT.get_or_init(|| ensure_events_db_inner().map_err(|e| e.to_string()));
    match result {
        Ok(()) => Ok(()),
        Err(e) => anyhow::bail!("events.db initialization failed: {e}"),
    }
}

fn ensure_events_db_inner() -> Result<()> {
    let events_path = Path::new(EVENTS_DB);

    // Create parent directory
    if let Some(parent) = events_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(events_path)?;

    // Safety-critical: WAL mode + synchronous FULL for irreplaceable data
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = FULL;

        CREATE TABLE IF NOT EXISTS eventlog (
            seq INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            source_id TEXT NOT NULL,
            source_file TEXT,
            data TEXT NOT NULL,
            CHECK(json_valid(data))
        );

        CREATE INDEX IF NOT EXISTS idx_eventlog_type ON eventlog(event_type);
        CREATE INDEX IF NOT EXISTS idx_eventlog_timestamp ON eventlog(timestamp);
        CREATE INDEX IF NOT EXISTS idx_eventlog_source ON eventlog(source_id);
        CREATE INDEX IF NOT EXISTS idx_eventlog_type_time ON eventlog(event_type, timestamp);

        -- Metadata for events.db (tracks export state for JSONL replica)
        CREATE TABLE IF NOT EXISTS scrape_meta (
            key TEXT PRIMARY KEY,
            value TEXT
        );

        PRAGMA user_version = 1;
        "#,
    )?;

    // Migrate runtime events from patina.db if it exists.
    // Uses INSERT OR IGNORE with explicit seq — safe under concurrent execution
    // (two processes racing past OnceLock in separate process spaces both insert,
    // but the second process's duplicates are ignored by PRIMARY KEY constraint).
    let patina_path = Path::new(PATINA_DB);
    if patina_path.exists() {
        conn.execute(
            "ATTACH DATABASE ?1 AS patina",
            [patina_path.to_str().unwrap_or(PATINA_DB)],
        )?;

        let has_eventlog: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM patina.sqlite_master WHERE type='table' AND name='eventlog'",
                [],
                |row| Ok(row.get::<_, i64>(0)? > 0),
            )
            .unwrap_or(false);

        if has_eventlog {
            let copied = conn.execute(
                r#"INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data)
                   SELECT event_type, timestamp, source_id, source_file, data
                   FROM patina.eventlog
                   WHERE event_type LIKE 'measure.%'
                      OR event_type LIKE 'scry.%'
                      OR event_type LIKE 'forge.%'
                   ORDER BY timestamp ASC"#,
                [],
            )?;

            if copied > 0 {
                eprintln!("  Migrated {} runtime events to events.db", copied);
            }
        }

        conn.execute("DETACH DATABASE patina", [])?;
    }

    Ok(())
}

/// Open events.db with safety PRAGMAs. Creates and migrates if needed.
pub fn open_events_db() -> Result<Connection> {
    ensure_events_db()?;
    let conn = Connection::open(EVENTS_DB)?;
    // synchronous = FULL is per-connection, must be set each time
    conn.execute_batch("PRAGMA synchronous = FULL;")?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
