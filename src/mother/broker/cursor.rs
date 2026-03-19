//! Cursor management for broker sources.

use anyhow::Result;
use rusqlite::Connection;

/// Get the stored cursor for a source.
pub fn get_cursor(conn: &Connection, source: &str) -> Result<Option<String>> {
    let result = conn.query_row(
        "SELECT cursor_value FROM broker_cursors WHERE source_name = ?1",
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
        let conn = crate::eventlog::open_events_db_at(dir.path()).unwrap();
        let cursor = get_cursor(&conn, "nonexistent").unwrap();
        assert!(cursor.is_none());
    }
}
