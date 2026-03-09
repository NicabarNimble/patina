//! Schema-driven FTS5 indexing and event utilities.
//!
//! "Do X": Populate FTS5 search index from schema-declared projection tables.
//!
//! The generic projection engine (projection.rs) handles materialization
//! from events.db into schema-declared read model tables. This module
//! handles FTS5 indexing from those tables using `[[indexes]]` declarations.

use anyhow::Result;
use rusqlite::Connection;

// ============================================================================
// FTS5 indexing — schema-driven (Seam 2: contract model consumer)
// ============================================================================

/// Populate FTS5 index from schema-driven projection tables.
///
/// Reads table names, event types, and fts_fields from installed schemas.
/// Each projection table is indexed exactly once — if multiple schemas share
/// the same table, the first schema wins and subsequent ones are skipped.
pub fn populate_fts5_from_schema(conn: &Connection) -> Result<usize> {
    let mut schemas = match crate::commands::schema::load_all_installed() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: failed to load schemas for FTS5: {}", e);
            return Ok(0);
        }
    };

    schemas.sort_by(|a, b| a.schema.name.cmp(&b.schema.name));

    let mut total = 0;
    let mut indexed_tables = std::collections::HashSet::new();
    let mut skipped_event_types = Vec::new();

    for schema in &schemas {
        for fact in &schema.facts {
            let index = match schema.indexes.iter().find(|i| i.fact == fact.name) {
                Some(i) => i,
                None => continue,
            };

            if !indexed_tables.insert(index.table.clone()) {
                skipped_event_types.push(fact.event_type.clone());
                continue;
            }

            total +=
                populate_fts5_for_table(conn, &index.table, &fact.event_type, &index.fts_fields)?;
        }
    }

    for event_type in &skipped_event_types {
        conn.execute(
            "DELETE FROM code_fts WHERE event_type = ?1",
            [event_type.as_str()],
        )?;
    }

    Ok(total)
}

/// Populate FTS5 from a single projection table using declared fts_fields.
fn populate_fts5_for_table(
    conn: &Connection,
    table_name: &str,
    event_type: &str,
    fts_fields: &[String],
) -> Result<usize> {
    if !is_safe_identifier(table_name) {
        eprintln!(
            "Warning: skipping FTS5 for invalid table name: {}",
            table_name
        );
        return Ok(0);
    }

    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [table_name],
            |row| row.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !table_exists {
        return Ok(0);
    }

    let available_columns = get_table_columns(conn, table_name);
    let content_expr = build_fts_content_expr(fts_fields, &available_columns);

    conn.execute("DELETE FROM code_fts WHERE event_type = ?1", [event_type])?;

    let sql = format!(
        "INSERT INTO code_fts (symbol_name, file_path, content, event_type) \
         SELECT title, url, {content_expr}, ?1 FROM {table_name}"
    );

    let count = conn.execute(&sql, [event_type])?;
    Ok(count)
}

/// Check that a string is a safe SQL identifier (alphanumeric + underscore only).
fn is_safe_identifier(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Get column names from a table via PRAGMA table_info.
fn get_table_columns(conn: &Connection, table_name: &str) -> std::collections::HashSet<String> {
    let mut columns = std::collections::HashSet::new();
    if let Ok(mut stmt) = conn.prepare(&format!("PRAGMA table_info({table_name})")) {
        if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(1)) {
            for name in rows.flatten() {
                columns.insert(name);
            }
        }
    }
    columns
}

/// Build a SQL expression for FTS5 content from declared fts_fields.
fn build_fts_content_expr(
    fts_fields: &[String],
    available_columns: &std::collections::HashSet<String>,
) -> String {
    let parts: Vec<String> = fts_fields
        .iter()
        .filter(|f| available_columns.contains(f.as_str()) && is_safe_identifier(f))
        .map(|f| format!("COALESCE({f}, '')"))
        .collect();

    if parts.is_empty() {
        "COALESCE(body, '')".to_string()
    } else {
        parts.join(" || ' ' || ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Set up an in-memory database with code_fts and github projection tables.
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE VIRTUAL TABLE code_fts USING fts5(
                symbol_name, file_path, content, event_type
            );
            CREATE TABLE github_issues (
                number INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                body TEXT,
                url TEXT NOT NULL
            );
            CREATE TABLE github_prs (
                number INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                body TEXT,
                url TEXT NOT NULL,
                comments TEXT
            );",
        )
        .unwrap();
        conn
    }

    fn insert_issue(conn: &Connection, number: i64, title: &str, body: Option<&str>, url: &str) {
        conn.execute(
            "INSERT INTO github_issues (number, title, body, url) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![number, title, body, url],
        )
        .unwrap();
    }

    fn insert_pr(
        conn: &Connection,
        number: i64,
        title: &str,
        body: Option<&str>,
        url: &str,
        comments: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO github_prs (number, title, body, url, comments) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![number, title, body, url, comments],
        )
        .unwrap();
    }

    fn fts_count(conn: &Connection, event_type: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM code_fts WHERE event_type = ?1",
            [event_type],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn fts_content(conn: &Connection, event_type: &str) -> String {
        conn.query_row(
            "SELECT content FROM code_fts WHERE event_type = ?1",
            [event_type],
            |row| row.get(0),
        )
        .unwrap()
    }

    // --- populate_fts5_for_table tests ---

    #[test]
    fn test_fts5_basic_with_fts_fields() {
        let conn = setup_test_db();
        insert_issue(
            &conn,
            1,
            "Bug fix",
            Some("Fix the thing"),
            "https://ex.com/1",
        );
        insert_issue(&conn, 2, "Feature", None, "https://ex.com/2");

        let fts_fields = vec!["title".into(), "body".into()];
        let count =
            populate_fts5_for_table(&conn, "github_issues", "github.issue", &fts_fields).unwrap();
        assert_eq!(count, 2);
        assert_eq!(fts_count(&conn, "github.issue"), 2);
    }

    #[test]
    fn test_fts5_content_built_from_fts_fields() {
        let conn = setup_test_db();
        insert_issue(&conn, 1, "My Title", Some("My Body"), "https://ex.com/1");

        let fts_fields = vec!["title".into(), "body".into()];
        populate_fts5_for_table(&conn, "github_issues", "github.issue", &fts_fields).unwrap();

        let content = fts_content(&conn, "github.issue");
        assert_eq!(content, "My Title My Body");
    }

    #[test]
    fn test_fts5_unavailable_fields_skipped() {
        let conn = setup_test_db();
        insert_issue(&conn, 1, "Title", Some("Body"), "https://ex.com/1");

        let fts_fields = vec!["title".into(), "body".into(), "comments".into()];
        populate_fts5_for_table(&conn, "github_issues", "github.issue", &fts_fields).unwrap();

        let content = fts_content(&conn, "github.issue");
        assert_eq!(content, "Title Body");
    }

    #[test]
    fn test_fts5_comments_included_when_column_exists() {
        let conn = setup_test_db();
        insert_pr(
            &conn,
            1,
            "Add feature",
            Some("PR body"),
            "https://ex.com/1",
            Some("reviewer: looks good"),
        );

        let fts_fields = vec!["title".into(), "body".into(), "comments".into()];
        populate_fts5_for_table(&conn, "github_prs", "github.pr", &fts_fields).unwrap();

        let content = fts_content(&conn, "github.pr");
        assert_eq!(content, "Add feature PR body reviewer: looks good");
    }

    #[test]
    fn test_fts5_null_fields_coalesced() {
        let conn = setup_test_db();
        insert_issue(&conn, 1, "Title", None, "https://ex.com/1");

        let fts_fields = vec!["title".into(), "body".into()];
        populate_fts5_for_table(&conn, "github_issues", "github.issue", &fts_fields).unwrap();

        let content = fts_content(&conn, "github.issue");
        assert_eq!(content, "Title ");
    }

    #[test]
    fn test_fts5_replaces_existing() {
        let conn = setup_test_db();
        insert_issue(&conn, 1, "Old", Some("old body"), "https://ex.com/1");

        let fts_fields = vec!["title".into(), "body".into()];
        populate_fts5_for_table(&conn, "github_issues", "github.issue", &fts_fields).unwrap();
        assert_eq!(fts_count(&conn, "github.issue"), 1);

        conn.execute(
            "UPDATE github_issues SET title = 'New' WHERE number = 1",
            [],
        )
        .unwrap();
        populate_fts5_for_table(&conn, "github_issues", "github.issue", &fts_fields).unwrap();
        assert_eq!(fts_count(&conn, "github.issue"), 1);
    }

    #[test]
    fn test_fts5_missing_table() {
        let conn = setup_test_db();
        let fts_fields = vec!["title".into()];
        let count =
            populate_fts5_for_table(&conn, "nonexistent_table", "test.event", &fts_fields).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_fts5_invalid_table_name() {
        let conn = setup_test_db();
        let fts_fields = vec!["title".into()];
        let count =
            populate_fts5_for_table(&conn, "bad; DROP TABLE", "test.event", &fts_fields).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_fts5_empty_fts_fields_falls_back_to_body() {
        let conn = setup_test_db();
        insert_issue(&conn, 1, "Title", Some("Fallback body"), "https://ex.com/1");

        let fts_fields: Vec<String> = vec![];
        populate_fts5_for_table(&conn, "github_issues", "github.issue", &fts_fields).unwrap();

        let content = fts_content(&conn, "github.issue");
        assert_eq!(content, "Fallback body");
    }

    // --- build_fts_content_expr unit tests ---

    #[test]
    fn test_content_expr_from_declared_fields() {
        let available: std::collections::HashSet<String> = ["title", "body", "url"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let fts_fields = vec!["title".into(), "body".into()];

        let expr = build_fts_content_expr(&fts_fields, &available);
        assert_eq!(expr, "COALESCE(title, '') || ' ' || COALESCE(body, '')");
    }

    #[test]
    fn test_content_expr_filters_missing_columns() {
        let available: std::collections::HashSet<String> =
            ["title", "body"].iter().map(|s| s.to_string()).collect();
        let fts_fields = vec!["title".into(), "body".into(), "comments".into()];

        let expr = build_fts_content_expr(&fts_fields, &available);
        assert_eq!(expr, "COALESCE(title, '') || ' ' || COALESCE(body, '')");
    }

    #[test]
    fn test_content_expr_fallback_when_none_resolve() {
        let available: std::collections::HashSet<String> = std::collections::HashSet::new();
        let fts_fields = vec!["nonexistent".into()];

        let expr = build_fts_content_expr(&fts_fields, &available);
        assert_eq!(expr, "COALESCE(body, '')");
    }

    #[test]
    fn test_content_expr_rejects_unsafe_field_names() {
        let available: std::collections::HashSet<String> =
            ["title".into(), "body; DROP TABLE".into()]
                .into_iter()
                .collect();
        let fts_fields = vec!["title".into(), "body; DROP TABLE".into()];

        let expr = build_fts_content_expr(&fts_fields, &available);
        assert_eq!(expr, "COALESCE(title, '')");
    }
}
