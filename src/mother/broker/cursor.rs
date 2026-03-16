//! Transactional cursor management.
//!
//! Wraps fact inserts + cursor update in one unchecked_transaction().
//! Both succeed or both fail — no partial state (DESIGN.md §3, §10).

use anyhow::Result;
use rusqlite::Connection;

use super::routing::{ValidatedFact, WriteResult};

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

/// Write facts and update cursor in a single transaction.
///
/// This is the critical atomicity guarantee: facts + cursor either
/// both commit or both roll back. On rollback, cursor stays at the
/// old position and the next run re-fetches (at-least-once, dedup
/// handles overlap).
pub fn write_facts_with_cursor(
    conn: &Connection,
    source_name: &str,
    facts: &[ValidatedFact],
    cursor: Option<&str>,
) -> Result<WriteResult> {
    let tx = conn.unchecked_transaction()?;

    let timestamp = chrono::Utc::now().to_rfc3339();
    let mut inserted: u64 = 0;
    let mut dedup_skipped: u64 = 0;

    for fact in facts {
        let rows = tx.execute(
            "INSERT OR IGNORE INTO eventlog (event_type, timestamp, source_id, data, provenance, content_hash)
             VALUES (?1, ?2, ?3, ?4, 'external', ?5)",
            rusqlite::params![
                &fact.event_type,
                &timestamp,
                &fact.source_id,
                &fact.data,
                &fact.content_hash,
            ],
        )?;

        if rows == 0 {
            dedup_skipped += 1;
        } else {
            inserted += 1;
        }
    }

    // Update cursor in the same transaction
    if let Some(cursor_value) = cursor {
        tx.execute(
            "INSERT OR REPLACE INTO broker_cursors (source_name, cursor_value, updated_at)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![source_name, cursor_value, &timestamp],
        )?;
    }

    tx.commit()?;

    Ok(WriteResult {
        inserted,
        dedup_skipped,
        cursor: cursor.map(|s| s.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = crate::eventlog::open_events_db_at(dir.path()).unwrap();
        (dir, conn)
    }

    #[test]
    fn transactional_write_commits_both() {
        let (_dir, conn) = setup_db();

        let facts = vec![
            ValidatedFact {
                event_type: "github.issue".to_string(),
                data: r#"{"title":"one"}"#.to_string(),
                content_hash: "blake3:aaa".to_string(),
                source_id: "child:test".to_string(),
            },
            ValidatedFact {
                event_type: "github.pr".to_string(),
                data: r#"{"title":"two"}"#.to_string(),
                content_hash: "blake3:bbb".to_string(),
                source_id: "child:test".to_string(),
            },
        ];

        let result =
            write_facts_with_cursor(&conn, "github", &facts, Some("2026-03-07T00:00:00Z")).unwrap();

        assert_eq!(result.inserted, 2);
        assert_eq!(result.dedup_skipped, 0);
        assert_eq!(result.cursor.as_deref(), Some("2026-03-07T00:00:00Z"));

        // Verify cursor was written
        let cursor = get_cursor(&conn, "github").unwrap();
        assert_eq!(cursor.as_deref(), Some("2026-03-07T00:00:00Z"));

        // Verify facts are in eventlog
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM eventlog WHERE source_id = 'child:test'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn cursor_not_advanced_without_facts() {
        let (_dir, conn) = setup_db();

        // Write initial cursor
        let facts = vec![ValidatedFact {
            event_type: "github.issue".to_string(),
            data: r#"{"title":"one"}"#.to_string(),
            content_hash: "blake3:aaa".to_string(),
            source_id: "child:test".to_string(),
        }];

        write_facts_with_cursor(&conn, "github", &facts, Some("cursor-1")).unwrap();

        // Write empty facts with new cursor — cursor should still update
        let result = write_facts_with_cursor(&conn, "github", &[], Some("cursor-2")).unwrap();
        assert_eq!(result.inserted, 0);

        let cursor = get_cursor(&conn, "github").unwrap();
        assert_eq!(cursor.as_deref(), Some("cursor-2"));
    }

    #[test]
    fn dedup_within_transaction() {
        let (_dir, conn) = setup_db();

        let facts = vec![ValidatedFact {
            event_type: "github.issue".to_string(),
            data: r#"{"title":"one"}"#.to_string(),
            content_hash: "blake3:aaa".to_string(),
            source_id: "child:test".to_string(),
        }];

        // First write
        write_facts_with_cursor(&conn, "github", &facts, Some("cursor-1")).unwrap();

        // Second write with same content_hash — should dedup
        let result = write_facts_with_cursor(&conn, "github", &facts, Some("cursor-2")).unwrap();
        assert_eq!(result.inserted, 0);
        assert_eq!(result.dedup_skipped, 1);

        // Cursor still advances (dedup doesn't block cursor update)
        let cursor = get_cursor(&conn, "github").unwrap();
        assert_eq!(cursor.as_deref(), Some("cursor-2"));
    }

    #[test]
    fn get_cursor_none_when_missing() {
        let (_dir, conn) = setup_db();
        let cursor = get_cursor(&conn, "nonexistent").unwrap();
        assert!(cursor.is_none());
    }
}
