//! Event insertion for pipeline-produced issues and PRs.
//!
//! "Do X": Insert typed events from pipeline plugins into events.db.
//!
//! Pipeline plugins (WASM) produce typed Issue/PullRequest structs. This
//! module writes them to events.db (write side). Event types are resolved
//! at runtime from installed schema declarations — no hardcoded connector
//! knowledge.
//!
//! The generic projection engine (projection.rs) handles materialization
//! from events.db into schema-declared read model tables.

use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::database;

// ============================================================================
// Domain types (platform-agnostic connector data)
// ============================================================================

/// Issue from any connector platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: IssueState,
    pub author: String,
    pub labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub url: String,
}

/// Pull/Merge Request from any connector platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: PrState,
    pub author: String,
    pub labels: Vec<String>,
    pub created_at: String,
    pub merged_at: Option<String>,
    pub url: String,
    pub linked_issues: Vec<i64>,
    pub comments: Vec<Comment>,
    pub approvals: i32,
}

/// Comment on an issue or PR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub author: String,
    pub body: String,
    pub created_at: String,
}

/// Issue state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    Open,
    Closed,
}

/// Pull request state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrState {
    Open,
    Merged,
    Closed,
}

// ============================================================================
// Dedup helper
// ============================================================================

/// Check if we already have this event (by number + updated_at).
fn event_exists(
    events_conn: &Connection,
    event_type: &str,
    number: i64,
    updated_at: &str,
) -> Result<bool> {
    let count: i64 = events_conn.query_row(
        "SELECT COUNT(*) FROM eventlog
         WHERE event_type = ?1
           AND json_extract(data, '$.number') = ?2
           AND json_extract(data, '$.updated_at') = ?3",
        rusqlite::params![event_type, number, updated_at],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

// ============================================================================
// Insert: typed structs → events.db (projection engine handles read model)
// ============================================================================

/// Stats returned from insert operations.
pub struct InsertStats {
    pub inserted: usize,
    pub skipped: usize,
}

/// Resolve event_type for a fact name from installed schemas.
fn resolve_event_type_for_fact(fact_name: &str) -> Result<String> {
    let schemas = crate::commands::schema::load_all_installed()?;
    for schema in &schemas {
        for fact in &schema.facts {
            if fact.name == fact_name {
                return Ok(fact.event_type.clone());
            }
        }
    }
    anyhow::bail!("no installed schema declares fact '{}'", fact_name)
}

/// Insert issues into events.db eventlog.
///
/// Event type is resolved from installed schemas (fact name "issue").
/// Read model tables are rebuilt by the generic projection engine.
pub fn insert_issues(events_conn: &Connection, issues: &[Issue]) -> Result<InsertStats> {
    let event_type = resolve_event_type_for_fact("issue")?;
    let mut inserted = 0;
    let mut skipped = 0;

    for issue in issues {
        let state_str = match issue.state {
            IssueState::Open => "open",
            IssueState::Closed => "closed",
        };

        if event_exists(events_conn, &event_type, issue.number, &issue.updated_at)? {
            skipped += 1;
            continue;
        }

        let event_data = json!({
            "number": issue.number,
            "title": &issue.title,
            "body": &issue.body,
            "state": state_str,
            "labels": &issue.labels,
            "author": &issue.author,
            "url": &issue.url,
            "updated_at": &issue.updated_at,
        });

        database::insert_event(
            events_conn,
            &event_type,
            &issue.created_at,
            &issue.number.to_string(),
            Some(&issue.url),
            &event_data.to_string(),
        )?;
        inserted += 1;
    }

    Ok(InsertStats { inserted, skipped })
}

/// Insert PRs into events.db eventlog.
///
/// Event type is resolved from installed schemas (fact name "pull-request").
/// Read model tables are rebuilt by the generic projection engine.
pub fn insert_prs(events_conn: &Connection, prs: &[PullRequest]) -> Result<InsertStats> {
    let event_type = resolve_event_type_for_fact("pull-request")?;
    let mut inserted = 0;
    let mut skipped = 0;

    for pr in prs {
        let updated_at = &pr.created_at;

        if event_exists(events_conn, &event_type, pr.number, updated_at)? {
            skipped += 1;
            continue;
        }

        let state_str = match pr.state {
            PrState::Open => "open",
            PrState::Merged => "merged",
            PrState::Closed => "closed",
        };

        let comments_text: String = pr
            .comments
            .iter()
            .map(|c| format!("{}: {}", c.author, c.body))
            .collect::<Vec<_>>()
            .join("\n");

        let event_data = json!({
            "number": pr.number,
            "title": &pr.title,
            "body": &pr.body,
            "state": state_str,
            "labels": &pr.labels,
            "author": &pr.author,
            "url": &pr.url,
            "linked_issues": &pr.linked_issues,
            "comments": &comments_text,
            "approvals": pr.approvals,
            "updated_at": updated_at,
        });

        database::insert_event(
            events_conn,
            &event_type,
            &pr.created_at,
            &pr.number.to_string(),
            Some(&pr.url),
            &event_data.to_string(),
        )?;
        inserted += 1;
    }

    Ok(InsertStats { inserted, skipped })
}

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

    // --- event_exists dedup test ---

    #[test]
    fn test_event_exists_exact_match() {
        let events_conn = Connection::open_in_memory().unwrap();
        events_conn
            .execute_batch(
                "CREATE TABLE eventlog (
                    seq INTEGER PRIMARY KEY AUTOINCREMENT,
                    event_type TEXT NOT NULL,
                    timestamp TEXT NOT NULL,
                    source_id TEXT NOT NULL,
                    source_file TEXT,
                    data TEXT NOT NULL,
                    provenance TEXT NOT NULL DEFAULT 'local',
                    CHECK(json_valid(data))
                );",
            )
            .unwrap();

        events_conn
            .execute(
                "INSERT INTO eventlog (event_type, timestamp, source_id, data)
                 VALUES ('github.issue', '2026-01-01', '42', ?1)",
                [r#"{"number": 42, "updated_at": "2026-01-01T12:00:00Z"}"#],
            )
            .unwrap();

        assert!(event_exists(&events_conn, "github.issue", 42, "2026-01-01T12:00:00Z").unwrap());
        assert!(!event_exists(&events_conn, "github.issue", 42, "2026-01-02T12:00:00Z").unwrap());
        assert!(!event_exists(&events_conn, "github.issue", 99, "2026-01-01T12:00:00Z").unwrap());
    }
}
