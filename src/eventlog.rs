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
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Legacy project-local projection database path (migration source).
pub const PATINA_DB: &str = concat!(".patina", "/local/data/patina.db");

/// Legacy project-local events database path (migration source).
pub const EVENTS_DB: &str = concat!(".patina", "/local/data/events.db");

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

    // Allow concurrent access from overlapping scrapes (e.g., rapid post-commit hooks).
    // patina.db does NOT use WAL (rebuildable projections, not safety-critical data).
    // busy_timeout lets a second writer wait instead of failing with SQLITE_BUSY.
    conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

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
            provenance TEXT NOT NULL DEFAULT 'local', -- local | external | derived
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
static EVENTS_INIT: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();

pub fn resolve_events_db_path(project_root: &Path) -> PathBuf {
    if let Some(uid) = resolve_project_uid(project_root) {
        if let Ok(path) = crate::paths::mother::projects::events_db(&uid) {
            return path;
        }
    }
    project_root.join(EVENTS_DB)
}

pub fn events_db_path() -> Result<PathBuf> {
    let project_root = std::env::current_dir()?;
    Ok(resolve_events_db_path(&project_root))
}

pub fn resolve_patina_db_path(project_root: &Path) -> PathBuf {
    if let Some(uid) = resolve_project_uid(project_root) {
        if let Ok(path) = crate::paths::mother::projects::patina_db(&uid) {
            return path;
        }
    }
    project_root.join(PATINA_DB)
}

fn resolve_project_uid(project_root: &Path) -> Option<String> {
    if let Some(uid) = crate::project::get_uid(project_root) {
        return Some(uid);
    }
    if crate::project::is_patina_project(project_root)
        && project_root.join(".patina/config.toml").exists()
    {
        if let Ok(uid) = crate::project::register_with_mother(project_root) {
            return Some(uid);
        }
    }
    None
}

pub fn patina_db_path() -> Result<PathBuf> {
    let project_root = std::env::current_dir()?;
    Ok(resolve_patina_db_path(&project_root))
}

fn ensure_events_db_path(events_path: &Path, patina_path: &Path) -> Result<()> {
    if let Some(parent) = events_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(events_path)?;
    mother_crate::eventlog_schema::prepare_events_db(&conn)?;

    // Migrate runtime events from patina.db if it exists.
    // Uses INSERT OR IGNORE with explicit seq — safe under concurrent execution.
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
                r#"INSERT OR IGNORE INTO eventlog (seq, event_type, timestamp, source_id, source_file, data)
                   SELECT seq, event_type, timestamp, source_id, source_file, data
                   FROM patina.eventlog
                   WHERE event_type LIKE 'measure.%'
                      OR event_type LIKE 'scry.%'
                      OR event_type LIKE 'forge.%'
                      OR event_type LIKE 'github.%'
                      OR event_type LIKE 'context.%'
                      OR event_type LIKE 'assay.%'
                   ORDER BY seq ASC"#,
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

/// Ensure events.db exists with correct schema. Migrates runtime events
/// from patina.db on first run (one-time copy, idempotent).
///
/// Runs once per process via OnceLock. Safe under concurrent execution:
/// schema uses CREATE TABLE IF NOT EXISTS, migration uses INSERT OR IGNORE
/// with explicit seq to prevent duplicate events.
pub fn ensure_events_db() -> Result<()> {
    let project_root = std::env::current_dir()?;
    ensure_events_db_for_root(&project_root)
}

fn ensure_events_db_for_root(project_root: &Path) -> Result<()> {
    let events_path = resolve_events_db_path(project_root);
    let init = EVENTS_INIT.get_or_init(|| Mutex::new(HashSet::new()));
    {
        let initialized = init.lock().unwrap_or_else(|error| error.into_inner());
        if initialized.contains(&events_path) {
            return Ok(());
        }
    }

    ensure_events_db_path(&events_path, &resolve_patina_db_path(project_root))?;

    let mut initialized = init.lock().unwrap_or_else(|error| error.into_inner());
    initialized.insert(events_path);

    Ok(())
}

fn ensure_events_db_path_initialized(project_root: &Path) -> Result<PathBuf> {
    let events_path = resolve_events_db_path(project_root);
    ensure_events_db_for_root(project_root)?;
    Ok(events_path)
}

/// Open events.db with safety PRAGMAs. Creates and migrates if needed.
pub fn open_events_db() -> Result<Connection> {
    let project_root = std::env::current_dir()?;
    let events_path = ensure_events_db_path_initialized(&project_root)?;
    let conn = Connection::open(events_path)?;
    // synchronous = FULL is per-connection, must be set each time
    conn.execute_batch("PRAGMA synchronous = FULL;")?;
    Ok(conn)
}

/// Open events.db at a specific project root path.
///
/// Used by the broker to write to a destination project's events.db.
/// Same PRAGMAs and schema as open_events_db(), just parameterized path.
pub fn open_events_db_at(project_root: &Path) -> Result<Connection> {
    let events_path = resolve_events_db_path(project_root);
    ensure_events_db_path(&events_path, &resolve_patina_db_path(project_root))?;
    let conn = Connection::open(events_path)?;
    conn.execute_batch("PRAGMA synchronous = FULL;")?;

    Ok(conn)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DataMigrationReport {
    pub project_uid: String,
    pub events_rows: i64,
    pub patina_rows: Option<i64>,
    pub moved_events_db: bool,
    pub moved_patina_db: bool,
    pub deleted_legacy_events_db: bool,
    pub deleted_legacy_patina_db: bool,
    pub deleted_dead_cortex_db: bool,
    pub deleted_dead_graph_db: bool,
    pub marker_path: String,
}

struct MigrationLock {
    path: PathBuf,
}

impl Drop for MigrationLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn is_process_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn acquire_migration_lock(path: &Path) -> Result<MigrationLock> {
    if let Ok(existing) = std::fs::read_to_string(path) {
        if let Ok(pid) = existing.trim().parse::<u32>() {
            if is_process_alive(pid) {
                anyhow::bail!(
                    "migration lock already held by pid {} at {}",
                    pid,
                    path.display()
                );
            }
        }
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, std::process::id().to_string())?;
    Ok(MigrationLock {
        path: path.to_path_buf(),
    })
}

fn quick_integrity_check(path: &Path) -> Result<()> {
    let conn = Connection::open(path)?;
    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        anyhow::bail!(
            "integrity_check failed for {}: {}",
            path.display(),
            integrity
        );
    }
    let quick: String = conn.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if quick != "ok" {
        anyhow::bail!("quick_check failed for {}: {}", path.display(), quick);
    }
    Ok(())
}

fn table_row_count(path: &Path, table: &str) -> Result<i64> {
    let conn = Connection::open(path)?;
    let sql = format!("SELECT COUNT(*) FROM {}", table);
    Ok(conn.query_row(&sql, [], |row| row.get(0)).unwrap_or(0))
}

/// One-time migration: copy legacy project-local databases into Mother's project directory.
///
/// This prepares GMDP-G10 without running destructive commands elsewhere.
pub fn migrate_legacy_project_databases(project_root: &Path) -> Result<DataMigrationReport> {
    if std::env::var("PATINA_SKIP_MOTHER_PGREP").ok().as_deref() != Some("1") {
        let pgrep = std::process::Command::new("pgrep")
            .args(["-f", "patina mother"])
            .output();
        if let Ok(output) = pgrep {
            if output.status.success() && !String::from_utf8_lossy(&output.stdout).trim().is_empty()
            {
                anyhow::bail!("mother daemon appears to be running; stop it before migration");
            }
        }
    }

    let uid = crate::project::get_uid(project_root)
        .ok_or_else(|| anyhow::anyhow!("project uid missing at {}", project_root.display()))?;
    let project_dir =
        crate::paths::mother::projects::ensure_project_dir(&uid).map_err(anyhow::Error::msg)?;
    let lock_path = project_dir.join(".migration-lock");
    let _lock = acquire_migration_lock(&lock_path)?;

    let legacy_events = project_root.join(EVENTS_DB);
    let legacy_patina = project_root.join(PATINA_DB);
    for wal in [
        legacy_events.with_extension("db-wal"),
        legacy_events.with_extension("db-shm"),
    ] {
        if wal.exists() {
            anyhow::bail!(
                "WAL sidecar exists at {} — checkpoint before migration",
                wal.display()
            );
        }
    }

    let target_events =
        crate::paths::mother::projects::events_db(&uid).map_err(anyhow::Error::msg)?;
    let target_patina =
        crate::paths::mother::projects::patina_db(&uid).map_err(anyhow::Error::msg)?;

    let mut moved_events = false;
    let mut moved_patina = false;
    if legacy_events.exists() && !target_events.exists() {
        std::fs::copy(&legacy_events, &target_events)?;
        moved_events = true;
    }
    if legacy_patina.exists() && !target_patina.exists() {
        std::fs::copy(&legacy_patina, &target_patina)?;
        moved_patina = true;
    }

    if target_events.exists() {
        quick_integrity_check(&target_events)?;
    }
    if target_patina.exists() {
        quick_integrity_check(&target_patina)?;
    }

    let events_rows = if target_events.exists() {
        let target_rows = table_row_count(&target_events, "eventlog")?;
        if legacy_events.exists() {
            let source_rows = table_row_count(&legacy_events, "eventlog")?;
            if source_rows != target_rows {
                anyhow::bail!(
                    "events row count mismatch source={} target={}",
                    source_rows,
                    target_rows
                );
            }
        }
        target_rows
    } else {
        0
    };

    let patina_rows = if target_patina.exists() {
        Some(table_row_count(&target_patina, "eventlog")?)
    } else {
        None
    };

    let deleted_legacy_events_db = if legacy_events.exists() {
        std::fs::remove_file(&legacy_events)?;
        true
    } else {
        false
    };
    let deleted_legacy_patina_db = if legacy_patina.exists() {
        std::fs::remove_file(&legacy_patina)?;
        true
    } else {
        false
    };

    let legacy_cortex = project_root.join(".patina/local/data/cortex.db");
    let deleted_dead_cortex_db = if legacy_cortex.exists() {
        std::fs::remove_file(&legacy_cortex)?;
        true
    } else {
        false
    };
    let legacy_graph = project_root.join(".patina/local/data/graph.db");
    let deleted_dead_graph_db = if legacy_graph.exists() {
        std::fs::remove_file(&legacy_graph)?;
        true
    } else {
        false
    };

    let marker_path = project_dir.join(".migrated");
    let marker = serde_json::json!({
        "migrated_at": chrono::Utc::now().to_rfc3339(),
        "project_uid": uid,
        "events_rows": events_rows,
        "patina_rows": patina_rows,
    });
    std::fs::write(&marker_path, serde_json::to_string_pretty(&marker)?)?;

    Ok(DataMigrationReport {
        project_uid: crate::project::get_uid(project_root).unwrap_or_default(),
        events_rows,
        patina_rows,
        moved_events_db: moved_events,
        moved_patina_db: moved_patina,
        deleted_legacy_events_db,
        deleted_legacy_patina_db,
        deleted_dead_cortex_db,
        deleted_dead_graph_db,
        marker_path: marker_path.to_string_lossy().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn with_temp_patina_home<T>(f: impl FnOnce(PathBuf) -> T) -> T {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp = tempfile::TempDir::new().unwrap();
        let expected_home = temp.path().join("patina-home");
        std::fs::create_dir_all(&expected_home).unwrap();
        let old = std::env::var_os("PATINA_HOME");
        unsafe {
            std::env::set_var("PATINA_HOME", &expected_home);
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(expected_home)));
        match old {
            Some(value) => unsafe {
                std::env::set_var("PATINA_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("PATINA_HOME");
            },
        }
        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

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
    fn test_content_hash_dedup() -> Result<()> {
        let dir = tempdir()?;
        let conn = open_events_db_at(dir.path())?;

        // Insert event with content_hash
        conn.execute(
            "INSERT INTO eventlog (event_type, timestamp, source_id, data, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                "test.event",
                "2026-03-07T00:00:00Z",
                "child:test",
                r#"{"key":"val"}"#,
                "blake3:abc123"
            ],
        )?;

        // Duplicate content_hash should be rejected (INSERT OR IGNORE)
        let inserted = conn.execute(
            "INSERT OR IGNORE INTO eventlog (event_type, timestamp, source_id, data, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                "test.event",
                "2026-03-07T00:01:00Z",
                "child:test",
                r#"{"key":"val2"}"#,
                "blake3:abc123"
            ],
        )?;
        assert_eq!(inserted, 0, "duplicate content_hash should be ignored");

        // NULL content_hash should always succeed (partial index)
        conn.execute(
            "INSERT INTO eventlog (event_type, timestamp, source_id, data)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                "test.event",
                "2026-03-07T00:02:00Z",
                "local",
                r#"{"key":"val3"}"#,
            ],
        )?;
        conn.execute(
            "INSERT INTO eventlog (event_type, timestamp, source_id, data)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                "test.event",
                "2026-03-07T00:03:00Z",
                "local",
                r#"{"key":"val4"}"#,
            ],
        )?;

        assert_eq!(count_total_events(&conn)?, 3);
        Ok(())
    }

    #[test]
    fn test_broker_cursors() -> Result<()> {
        let dir = tempdir()?;
        let conn = open_events_db_at(dir.path())?;

        // Insert cursor
        conn.execute(
            "INSERT INTO broker_cursors (source_name, cursor_value, updated_at)
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["github", "2026-03-07T00:00:00Z", "2026-03-07T12:00:00Z"],
        )?;

        // Read cursor
        let cursor: String = conn.query_row(
            "SELECT cursor_value FROM broker_cursors WHERE source_name = ?1",
            ["github"],
            |row| row.get(0),
        )?;
        assert_eq!(cursor, "2026-03-07T00:00:00Z");

        // Update cursor (REPLACE)
        conn.execute(
            "INSERT OR REPLACE INTO broker_cursors (source_name, cursor_value, updated_at)
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["github", "2026-03-07T01:00:00Z", "2026-03-07T13:00:00Z"],
        )?;

        let cursor: String = conn.query_row(
            "SELECT cursor_value FROM broker_cursors WHERE source_name = ?1",
            ["github"],
            |row| row.get(0),
        )?;
        assert_eq!(cursor, "2026-03-07T01:00:00Z");

        // Cursor value length constraint (4096 max)
        let long_cursor = "x".repeat(4097);
        let result = conn.execute(
            "INSERT INTO broker_cursors (source_name, cursor_value, updated_at)
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["too-long", long_cursor, "2026-03-07T12:00:00Z"],
        );
        assert!(result.is_err(), "cursor > 4096 bytes should be rejected");

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

    #[test]
    fn test_open_events_db_at_uses_mother_project_path_when_uid_exists() -> Result<()> {
        with_temp_patina_home(|patina_home| {
            let dir = tempdir().unwrap();
            std::fs::create_dir_all(dir.path().join(".patina")).unwrap();
            std::fs::write(dir.path().join(".patina/uid"), "2bdc808e").unwrap();

            let conn = open_events_db_at(dir.path()).unwrap();
            insert_event(
                &conn,
                "test.event",
                "2026-03-30T00:00:00Z",
                "core",
                None,
                r#"{"ok":true}"#,
            )
            .unwrap();

            let expected = patina_home.join("mother/projects/2bdc808e/events.db");
            assert!(expected.exists());
        });
        Ok(())
    }

    #[test]
    fn test_migrate_legacy_project_databases_moves_and_marks() -> Result<()> {
        with_temp_patina_home(|patina_home| {
            let project = tempdir().unwrap();
            std::fs::create_dir_all(project.path().join(".patina/local/data")).unwrap();
            std::fs::write(project.path().join(".patina/uid"), "2bdc808e").unwrap();

            let legacy_events = project.path().join(EVENTS_DB);
            let conn = Connection::open(&legacy_events).unwrap();
            mother_crate::eventlog_schema::prepare_events_db(&conn).unwrap();
            conn.execute(
                "INSERT INTO eventlog (event_type, timestamp, source_id, data) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params!["test.event", "2026-03-30T00:00:00Z", "core", r#"{"ok":true}"#],
            )
            .unwrap();
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .unwrap();
            drop(conn);

            let legacy_patina = project.path().join(PATINA_DB);
            let pconn = initialize(&legacy_patina).unwrap();
            insert_event(
                &pconn,
                "code.symbol",
                "2026-03-30T00:00:00Z",
                "src/main.rs::main",
                Some("src/main.rs"),
                r#"{"content":"fn main() {}"}"#,
            )
            .unwrap();

            unsafe {
                std::env::set_var("PATINA_SKIP_MOTHER_PGREP", "1");
            }
            let report = migrate_legacy_project_databases(project.path()).unwrap();
            unsafe {
                std::env::remove_var("PATINA_SKIP_MOTHER_PGREP");
            }

            assert!(report.moved_events_db);
            assert!(report.moved_patina_db);
            assert!(report.deleted_legacy_events_db);
            assert!(report.deleted_legacy_patina_db);
            assert!(!legacy_events.exists());
            assert!(!legacy_patina.exists());

            let target_events = patina_home.join("mother/projects/2bdc808e/events.db");
            let target_patina = patina_home.join("mother/projects/2bdc808e/patina.db");
            assert!(target_events.exists());
            assert!(target_patina.exists());
            assert!(std::path::Path::new(&report.marker_path).exists());
        });
        Ok(())
    }

    #[test]
    fn test_migrate_legacy_project_databases_is_idempotent() -> Result<()> {
        with_temp_patina_home(|_patina_home| {
            let project = tempdir().unwrap();
            std::fs::create_dir_all(project.path().join(".patina/local/data")).unwrap();
            std::fs::write(project.path().join(".patina/uid"), "2bdc808e").unwrap();

            let legacy_events = project.path().join(EVENTS_DB);
            let conn = Connection::open(&legacy_events).unwrap();
            mother_crate::eventlog_schema::prepare_events_db(&conn).unwrap();
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .unwrap();
            drop(conn);

            unsafe {
                std::env::set_var("PATINA_SKIP_MOTHER_PGREP", "1");
            }
            let first = migrate_legacy_project_databases(project.path()).unwrap();
            let second = migrate_legacy_project_databases(project.path()).unwrap();
            unsafe {
                std::env::remove_var("PATINA_SKIP_MOTHER_PGREP");
            }

            assert!(first.moved_events_db || first.deleted_legacy_events_db);
            assert!(!second.moved_events_db);
        });
        Ok(())
    }
}
