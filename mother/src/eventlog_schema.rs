use anyhow::Result;
use rusqlite::Connection;

/// Ensure canonical events.db schema, pragmas, and migrations.
pub fn prepare_events_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = FULL;
        PRAGMA busy_timeout = 5000;

        CREATE TABLE IF NOT EXISTS eventlog (
            seq INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            source_id TEXT NOT NULL,
            source_file TEXT,
            data TEXT NOT NULL,
            provenance TEXT NOT NULL DEFAULT 'local',
            content_hash TEXT,
            CHECK(json_valid(data))
        );

        CREATE INDEX IF NOT EXISTS idx_eventlog_type ON eventlog(event_type);
        CREATE INDEX IF NOT EXISTS idx_eventlog_timestamp ON eventlog(timestamp);
        CREATE INDEX IF NOT EXISTS idx_eventlog_source ON eventlog(source_id);
        CREATE INDEX IF NOT EXISTS idx_eventlog_type_time ON eventlog(event_type, timestamp);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_eventlog_content_hash
            ON eventlog(content_hash) WHERE content_hash IS NOT NULL;

        CREATE TABLE IF NOT EXISTS scrape_meta (
            key TEXT PRIMARY KEY,
            value TEXT
        );

        CREATE TABLE IF NOT EXISTS broker_cursors (
            source_name TEXT PRIMARY KEY,
            cursor_value TEXT NOT NULL CHECK(length(cursor_value) <= 4096),
            updated_at TEXT NOT NULL
        );

        PRAGMA user_version = 3;
        "#,
    )?;

    let has_provenance: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('eventlog') WHERE name = 'provenance'",
            [],
            |row| Ok(row.get::<_, i64>(0)? > 0),
        )
        .unwrap_or(false);

    if !has_provenance {
        conn.execute_batch(
            "ALTER TABLE eventlog ADD COLUMN provenance TEXT NOT NULL DEFAULT 'local';",
        )?;
    }

    let has_content_hash: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('eventlog') WHERE name = 'content_hash'",
            [],
            |row| Ok(row.get::<_, i64>(0)? > 0),
        )
        .unwrap_or(false);

    if !has_content_hash {
        conn.execute_batch(
            r#"
            ALTER TABLE eventlog ADD COLUMN content_hash TEXT;
            CREATE UNIQUE INDEX IF NOT EXISTS idx_eventlog_content_hash
                ON eventlog(content_hash) WHERE content_hash IS NOT NULL;
            "#,
        )?;
    }

    Ok(())
}
