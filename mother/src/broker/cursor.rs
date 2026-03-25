//! Cursor management for broker sources.

use anyhow::Result;
use rusqlite::Connection;

/// Get the stored cursor for a source.
pub fn get_cursor(conn: &Connection, source: &str) -> Result<Option<String>> {
    let result = conn.query_row(
        "SELECT cursor_value FROM mother_lake_cursors WHERE source_name = ?1 ORDER BY updated_at DESC LIMIT 1",
        [source],
        |row| row.get(0),
    );

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_cursor_none_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("events.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS mother_lake_cursors (
                lake_name TEXT NOT NULL,
                source_name TEXT NOT NULL,
                data_type TEXT NOT NULL,
                cursor_value TEXT,
                records_written INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL,
                last_error TEXT,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (lake_name, source_name, data_type)
             );",
        )
        .unwrap();
        let cursor = get_cursor(&conn, "nonexistent").unwrap();
        assert!(cursor.is_none());
    }
}
