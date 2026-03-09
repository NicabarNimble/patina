//! Event projection and materialized views for forge data.
//!
//! "Do X": Project forge events from events.db into queryable tables.
//!
//! These functions handle the CQRS read-model: events.db is the write side,
//! patina.db tables (forge_issues, forge_prs) are the read side. Works with
//! events from any source (scrape, pipeline plugins, native connectors).

use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::database;

// ============================================================================
// Domain types (platform-agnostic forge data)
// ============================================================================

/// Issue from any forge platform.
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

/// Pull/Merge Request from any forge platform.
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

/// Issue state (platform-agnostic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    Open,
    Closed,
}

/// Pull request state (platform-agnostic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrState {
    Open,
    Merged,
    Closed,
}

// ============================================================================
// Schema registry — populated from installed schemas
// ============================================================================

/// Create schema_registry table in patina.db.
fn create_schema_registry(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_registry (
            schema_name TEXT NOT NULL,
            event_type  TEXT NOT NULL,
            fact_name   TEXT NOT NULL,
            table_name  TEXT NOT NULL,
            fts_fields  TEXT,
            corpus_query TEXT,
            offset_slot INTEGER,
            PRIMARY KEY (schema_name, event_type)
        );",
    )?;
    Ok(())
}

/// Populate schema_registry from installed schemas (idempotent — rebuilds each time).
fn populate_schema_registry(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM schema_registry", [])?;

    let schemas = match crate::commands::schema::load_all_installed() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: failed to load schemas for registry: {}", e);
            return Ok(());
        }
    };

    let mut stmt = conn.prepare(
        "INSERT OR REPLACE INTO schema_registry
         (schema_name, event_type, fact_name, table_name, fts_fields, corpus_query, offset_slot)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;

    for schema in &schemas {
        let corpus_query = schema.embedding.as_ref().map(|e| e.corpus_query.trim());
        let offset_slot = schema.embedding.as_ref().map(|e| e.offset_slot);

        for fact in &schema.facts {
            // Find matching index config for this fact
            let index = schema.indexes.iter().find(|i| i.fact == fact.name);
            let table_name = index.map(|i| i.table.as_str()).unwrap_or("");
            let fts_fields =
                index.map(|i| serde_json::to_string(&i.fts_fields).unwrap_or_default());

            stmt.execute(rusqlite::params![
                &schema.schema.name,
                &fact.event_type,
                &fact.name,
                table_name,
                fts_fields,
                corpus_query,
                offset_slot,
            ])?;
        }
    }

    Ok(())
}

// ============================================================================
// Schema-driven event type resolution
// ============================================================================

/// Resolve the write event_type for a given projection table from schema_registry.
///
/// Returns the event_type from the alphabetically-first schema (deterministic,
/// consistent with FTS5 dedup). Falls back to `fallback` if schema_registry
/// is not populated or the table is unknown.
pub fn resolve_event_type(patina_conn: &Connection, table_name: &str, fallback: &str) -> String {
    patina_conn
        .query_row(
            "SELECT event_type FROM schema_registry
             WHERE table_name = ?1
             ORDER BY schema_name
             LIMIT 1",
            [table_name],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| fallback.to_string())
}

/// Resolve all event_types that map to a given projection table.
///
/// Used for dedup: when checking if an event already exists, we must check
/// ALL event_types that could have written to this table (e.g., both
/// forge.issue and github.issue for forge_issues).
///
/// Falls back to vec![fallback] if schema_registry is not populated.
fn resolve_dedup_event_types(
    patina_conn: &Connection,
    table_name: &str,
    fallback: &str,
) -> Vec<String> {
    let result: Vec<String> = patina_conn
        .prepare("SELECT event_type FROM schema_registry WHERE table_name = ?1")
        .and_then(|mut stmt| {
            stmt.query_map([table_name], |row| row.get(0))
                .map(|rows| rows.flatten().collect())
        })
        .unwrap_or_default();

    if result.is_empty() {
        vec![fallback.to_string()]
    } else {
        result
    }
}

// ============================================================================
// Dedup helpers — exact event_type matching via schema registry
// ============================================================================

/// Check if we already have this event (by number + updated_at) under any
/// of the given event_types. Replaces suffix-based LIKE matching with
/// exact-match queries driven by the schema registry.
fn event_exists(
    events_conn: &Connection,
    event_types: &[String],
    number: i64,
    updated_at: &str,
) -> Result<bool> {
    for event_type in event_types {
        let count: i64 = events_conn.query_row(
            "SELECT COUNT(*) FROM eventlog
             WHERE event_type = ?1
               AND json_extract(data, '$.number') = ?2
               AND json_extract(data, '$.updated_at') = ?3",
            rusqlite::params![event_type, number, updated_at],
            |row| row.get(0),
        )?;
        if count > 0 {
            return Ok(true);
        }
    }
    Ok(false)
}

// ============================================================================
// Materialized views
// ============================================================================

/// Create materialized views for forge events.
pub fn create_materialized_views(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- Forge issues view (materialized from forge.issue events)
        CREATE TABLE IF NOT EXISTS forge_issues (
            number INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            body TEXT,
            state TEXT NOT NULL,
            labels TEXT,           -- JSON array of label names
            author TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            url TEXT NOT NULL,
            event_seq INTEGER,     -- Cross-db ref to events.db eventlog seq
            ingested_at TEXT       -- When event was inserted into events.db
        );

        -- Forge PRs view (materialized from forge.pr events)
        CREATE TABLE IF NOT EXISTS forge_prs (
            number INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            body TEXT,
            state TEXT NOT NULL,
            labels TEXT,           -- JSON array of label names
            author TEXT,
            created_at TEXT NOT NULL,
            merged_at TEXT,
            url TEXT NOT NULL,
            linked_issues TEXT,    -- JSON array of issue numbers
            approvals INTEGER DEFAULT 0,
            event_seq INTEGER,     -- Cross-db ref to events.db eventlog seq
            ingested_at TEXT       -- When event was inserted into events.db
        );

        -- Indexes for common queries
        CREATE INDEX IF NOT EXISTS idx_forge_issues_state ON forge_issues(state);
        CREATE INDEX IF NOT EXISTS idx_forge_issues_updated ON forge_issues(updated_at);
        CREATE INDEX IF NOT EXISTS idx_forge_prs_state ON forge_prs(state);
        CREATE INDEX IF NOT EXISTS idx_forge_prs_merged ON forge_prs(merged_at);

        -- Forge refs backlog (for incremental sync with pacing)
        CREATE TABLE IF NOT EXISTS forge_refs (
            repo        TEXT NOT NULL,
            ref_number  INTEGER NOT NULL,
            ref_kind    TEXT DEFAULT 'unknown',
            discovered  TEXT NOT NULL,
            source      TEXT,
            resolved    TEXT,
            error       TEXT,
            PRIMARY KEY (repo, ref_number)
        );

        CREATE INDEX IF NOT EXISTS idx_forge_refs_pending
        ON forge_refs(repo, discovered DESC) WHERE resolved IS NULL;
        "#,
    )?;

    // Migration: add ingested_at column to existing tables (idempotent)
    let _ = conn.execute("ALTER TABLE forge_issues ADD COLUMN ingested_at TEXT", []);
    let _ = conn.execute("ALTER TABLE forge_prs ADD COLUMN ingested_at TEXT", []);

    Ok(())
}

// ============================================================================
// Projection: events.db → patina.db
// ============================================================================

/// Stats from projection operation.
pub struct ProjectionStats {
    pub issues_projected: usize,
    pub prs_projected: usize,
}

/// Project forge events from events.db into patina.db materialized views.
///
/// Reads issue/PR events from events.db and upserts into forge_issues/forge_prs
/// tables in patina.db. Event types are discovered from the schema_registry —
/// no hardcoded strings. Idempotent.
pub fn project_from_events(patina_conn: &Connection) -> Result<ProjectionStats> {
    create_materialized_views(patina_conn)?;
    create_schema_registry(patina_conn)?;
    populate_schema_registry(patina_conn)?;

    patina::eventlog::ensure_events_db()?;
    let events_path = patina::eventlog::EVENTS_DB;
    patina_conn.execute("ATTACH DATABASE ?1 AS events_db", [events_path])?;

    let issue_count = patina_conn.execute(
        r#"INSERT OR REPLACE INTO forge_issues
           (number, title, body, state, labels, author, created_at, updated_at, url,
            event_seq, ingested_at)
           SELECT
               json_extract(e.data, '$.number'),
               json_extract(e.data, '$.title'),
               json_extract(e.data, '$.body'),
               json_extract(e.data, '$.state'),
               json_extract(e.data, '$.labels'),
               json_extract(e.data, '$.author'),
               COALESCE(json_extract(e.data, '$.created_at'), e.timestamp),
               COALESCE(json_extract(e.data, '$.updated_at'), e.timestamp),
               json_extract(e.data, '$.url'),
               e.seq,
               e.timestamp
           FROM events_db.eventlog e
           WHERE e.event_type IN (
               SELECT event_type FROM schema_registry WHERE table_name = 'forge_issues'
             )
             AND e.seq = (
               SELECT MAX(e2.seq) FROM events_db.eventlog e2
               WHERE e2.event_type IN (
                   SELECT event_type FROM schema_registry WHERE table_name = 'forge_issues'
                 )
                 AND json_extract(e2.data, '$.number') = json_extract(e.data, '$.number')
             )"#,
        [],
    )?;

    let pr_count = patina_conn.execute(
        r#"INSERT OR REPLACE INTO forge_prs
           (number, title, body, state, labels, author, created_at, merged_at, url,
            linked_issues, approvals, event_seq, ingested_at)
           SELECT
               json_extract(e.data, '$.number'),
               json_extract(e.data, '$.title'),
               json_extract(e.data, '$.body'),
               json_extract(e.data, '$.state'),
               json_extract(e.data, '$.labels'),
               json_extract(e.data, '$.author'),
               COALESCE(json_extract(e.data, '$.created_at'), e.timestamp),
               COALESCE(json_extract(e.data, '$.merged_at'),
                   CASE json_extract(e.data, '$.state')
                       WHEN 'merged' THEN e.timestamp
                       ELSE NULL
                   END),
               json_extract(e.data, '$.url'),
               COALESCE(json_extract(e.data, '$.linked_issues'), '[]'),
               COALESCE(json_extract(e.data, '$.approvals'), 0),
               e.seq,
               e.timestamp
           FROM events_db.eventlog e
           WHERE e.event_type IN (
               SELECT event_type FROM schema_registry WHERE table_name = 'forge_prs'
             )
             AND e.seq = (
               SELECT MAX(e2.seq) FROM events_db.eventlog e2
               WHERE e2.event_type IN (
                   SELECT event_type FROM schema_registry WHERE table_name = 'forge_prs'
                 )
                 AND json_extract(e2.data, '$.number') = json_extract(e.data, '$.number')
             )"#,
        [],
    )?;

    patina_conn.execute("DETACH DATABASE events_db", [])?;

    Ok(ProjectionStats {
        issues_projected: issue_count,
        prs_projected: pr_count,
    })
}

// ============================================================================
// Insert: typed structs → events.db + materialized views
// ============================================================================

/// Stats returned from insert operations.
pub struct InsertStats {
    pub inserted: usize,
    pub skipped: usize,
}

/// Insert issues into events.db eventlog and patina.db materialized views.
///
/// `event_type` is the schema-driven event type to write (e.g., "forge.issue").
/// Dedup checks all event_types that map to the same projection table via
/// schema_registry, so old forge.* events and new schema-driven events are
/// both considered when detecting duplicates.
pub fn insert_issues(
    patina_conn: &Connection,
    events_conn: &Connection,
    issues: &[Issue],
    event_type: &str,
) -> Result<InsertStats> {
    let mut inserted = 0;
    let mut skipped = 0;

    // Resolve all event_types for forge_issues table (dedup across schemas)
    let dedup_types = resolve_dedup_event_types(patina_conn, "forge_issues", event_type);

    let mut issue_stmt = patina_conn.prepare(
        "INSERT OR REPLACE INTO forge_issues
         (number, title, body, state, labels, author, created_at, updated_at, url, event_seq)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;

    for issue in issues {
        let labels_str = serde_json::to_string(&issue.labels)?;
        let state_str = match issue.state {
            IssueState::Open => "open",
            IssueState::Closed => "closed",
        };

        let seq = if event_exists(events_conn, &dedup_types, issue.number, &issue.updated_at)? {
            skipped += 1;
            None
        } else {
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

            let seq = database::insert_event(
                events_conn,
                event_type,
                &issue.created_at,
                &issue.number.to_string(),
                Some(&issue.url),
                &event_data.to_string(),
            )?;
            inserted += 1;
            Some(seq)
        };

        issue_stmt.execute(rusqlite::params![
            issue.number,
            &issue.title,
            &issue.body,
            state_str,
            &labels_str,
            &issue.author,
            &issue.created_at,
            &issue.updated_at,
            &issue.url,
            seq,
        ])?;
    }

    Ok(InsertStats { inserted, skipped })
}

/// Insert PRs into events.db eventlog and patina.db materialized views.
///
/// `event_type` is the schema-driven event type to write (e.g., "forge.pr").
/// Dedup checks all event_types that map to the same projection table via
/// schema_registry, so old forge.* events and new schema-driven events are
/// both considered when detecting duplicates.
pub fn insert_prs(
    patina_conn: &Connection,
    events_conn: &Connection,
    prs: &[PullRequest],
    event_type: &str,
) -> Result<InsertStats> {
    let mut inserted = 0;
    let mut skipped = 0;

    // Resolve all event_types for forge_prs table (dedup across schemas)
    let dedup_types = resolve_dedup_event_types(patina_conn, "forge_prs", event_type);

    let mut pr_stmt = patina_conn.prepare(
        "INSERT OR REPLACE INTO forge_prs
         (number, title, body, state, labels, author, created_at, merged_at, url, linked_issues, approvals, event_seq)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )?;

    for pr in prs {
        let labels_str = serde_json::to_string(&pr.labels)?;
        let linked_str = serde_json::to_string(&pr.linked_issues)?;
        let state_str = match pr.state {
            PrState::Open => "open",
            PrState::Merged => "merged",
            PrState::Closed => "closed",
        };

        let updated_at = &pr.created_at;

        let seq = if event_exists(events_conn, &dedup_types, pr.number, updated_at)? {
            skipped += 1;
            None
        } else {
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

            let seq = database::insert_event(
                events_conn,
                event_type,
                &pr.created_at,
                &pr.number.to_string(),
                Some(&pr.url),
                &event_data.to_string(),
            )?;
            inserted += 1;
            Some(seq)
        };

        pr_stmt.execute(rusqlite::params![
            pr.number,
            &pr.title,
            &pr.body,
            state_str,
            &labels_str,
            &pr.author,
            &pr.created_at,
            &pr.merged_at,
            &pr.url,
            &linked_str,
            pr.approvals,
            seq,
        ])?;
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
/// the same table (e.g., forge and github both use forge_issues), the first
/// schema wins and subsequent ones are skipped, because shared tables have
/// no per-row schema attribution.
///
/// Content is built from the declared `fts_fields` in `[[indexes]]`, filtered
/// to columns that actually exist in the projection table. This honors the
/// schema contract rather than hardcoding column names.
pub fn populate_fts5_from_schema(conn: &Connection) -> Result<usize> {
    let mut schemas = match crate::commands::schema::load_all_installed() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: failed to load schemas for FTS5: {}", e);
            return Ok(0);
        }
    };

    // Sort by schema name so the first-wins dedup is deterministic across
    // machines and runs. Without this, read_dir() order decides which
    // event_type label a shared table gets.
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

            // Each projection table is indexed once. Shared tables cannot
            // attribute rows to a specific schema — indexing twice produces
            // fabricated labels and duplicate FTS entries.
            if !indexed_tables.insert(index.table.clone()) {
                // Track skipped event_types so we can clean up stale FTS entries
                // from previous runs that may have indexed under this label.
                skipped_event_types.push(fact.event_type.clone());
                continue;
            }

            total +=
                populate_fts5_for_table(conn, &index.table, &fact.event_type, &index.fts_fields)?;
        }
    }

    // Clean up stale FTS entries from event_types that were skipped due to
    // table dedup. Without this, a previous run that indexed the same table
    // under a different event_type label would leave orphaned entries.
    for event_type in &skipped_event_types {
        conn.execute(
            "DELETE FROM code_fts WHERE event_type = ?1",
            [event_type.as_str()],
        )?;
    }

    Ok(total)
}

/// Populate FTS5 from a single projection table using declared fts_fields.
///
/// Content column is built from the intersection of declared fts_fields and
/// actual table columns. Fields that don't exist (e.g., "comments" when the
/// table has no comments column) are silently skipped. Falls back to body
/// if no declared fields resolve.
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
    // table_name is already validated by is_safe_identifier before this call
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
///
/// Only includes fields that (a) exist as columns in the projection table
/// and (b) pass identifier validation. Falls back to `COALESCE(body, '')`
/// if no declared fields resolve — preserves backward compatibility.
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

    /// Set up an in-memory database with code_fts and forge projection tables.
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE VIRTUAL TABLE code_fts USING fts5(
                symbol_name, file_path, content, event_type
            );
            CREATE TABLE forge_issues (
                number INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                body TEXT,
                url TEXT NOT NULL
            );
            CREATE TABLE forge_prs (
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
            "INSERT INTO forge_issues (number, title, body, url) VALUES (?1, ?2, ?3, ?4)",
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
            "INSERT INTO forge_prs (number, title, body, url, comments) VALUES (?1, ?2, ?3, ?4, ?5)",
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
            populate_fts5_for_table(&conn, "forge_issues", "forge.issue", &fts_fields).unwrap();
        assert_eq!(count, 2);
        assert_eq!(fts_count(&conn, "forge.issue"), 2);
    }

    #[test]
    fn test_fts5_content_built_from_fts_fields() {
        let conn = setup_test_db();
        insert_issue(&conn, 1, "My Title", Some("My Body"), "https://ex.com/1");

        let fts_fields = vec!["title".into(), "body".into()];
        populate_fts5_for_table(&conn, "forge_issues", "forge.issue", &fts_fields).unwrap();

        let content = fts_content(&conn, "forge.issue");
        assert_eq!(content, "My Title My Body");
    }

    #[test]
    fn test_fts5_unavailable_fields_skipped() {
        let conn = setup_test_db();
        insert_issue(&conn, 1, "Title", Some("Body"), "https://ex.com/1");

        // "comments" doesn't exist in forge_issues — should be silently skipped
        let fts_fields = vec!["title".into(), "body".into(), "comments".into()];
        populate_fts5_for_table(&conn, "forge_issues", "forge.issue", &fts_fields).unwrap();

        let content = fts_content(&conn, "forge.issue");
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

        // forge_prs HAS a comments column
        let fts_fields = vec!["title".into(), "body".into(), "comments".into()];
        populate_fts5_for_table(&conn, "forge_prs", "forge.pr", &fts_fields).unwrap();

        let content = fts_content(&conn, "forge.pr");
        assert_eq!(content, "Add feature PR body reviewer: looks good");
    }

    #[test]
    fn test_fts5_null_fields_coalesced() {
        let conn = setup_test_db();
        insert_issue(&conn, 1, "Title", None, "https://ex.com/1");

        let fts_fields = vec!["title".into(), "body".into()];
        populate_fts5_for_table(&conn, "forge_issues", "forge.issue", &fts_fields).unwrap();

        let content = fts_content(&conn, "forge.issue");
        assert_eq!(content, "Title ");
    }

    #[test]
    fn test_fts5_replaces_existing() {
        let conn = setup_test_db();
        insert_issue(&conn, 1, "Old", Some("old body"), "https://ex.com/1");

        let fts_fields = vec!["title".into(), "body".into()];
        populate_fts5_for_table(&conn, "forge_issues", "forge.issue", &fts_fields).unwrap();
        assert_eq!(fts_count(&conn, "forge.issue"), 1);

        conn.execute("UPDATE forge_issues SET title = 'New' WHERE number = 1", [])
            .unwrap();
        populate_fts5_for_table(&conn, "forge_issues", "forge.issue", &fts_fields).unwrap();
        assert_eq!(fts_count(&conn, "forge.issue"), 1);
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
        populate_fts5_for_table(&conn, "forge_issues", "forge.issue", &fts_fields).unwrap();

        let content = fts_content(&conn, "forge.issue");
        assert_eq!(content, "Fallback body");
    }

    // --- shared table dedup + stale cleanup tests ---

    #[test]
    fn test_helper_without_dedup_creates_duplicates() {
        let conn = setup_test_db();
        insert_issue(&conn, 1, "Issue 1", Some("body"), "https://ex.com/1");

        let fts_fields = vec!["title".into(), "body".into()];

        // Without the coordinator's dedup, the helper indexes the same table
        // under two event_types — proving the dedup is necessary.
        populate_fts5_for_table(&conn, "forge_issues", "forge.issue", &fts_fields).unwrap();
        populate_fts5_for_table(&conn, "forge_issues", "github.issue", &fts_fields).unwrap();

        assert_eq!(fts_count(&conn, "forge.issue"), 1);
        assert_eq!(fts_count(&conn, "github.issue"), 1);
        // 2 total — same row indexed under 2 labels = fabricated
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM code_fts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(total, 2);
    }

    #[test]
    fn test_stale_entries_cleaned_after_dedup() {
        let conn = setup_test_db();
        insert_issue(&conn, 1, "Issue 1", Some("body"), "https://ex.com/1");

        let fts_fields = vec!["title".into(), "body".into()];

        // Simulate a previous run that indexed under both labels (pre-dedup bug)
        populate_fts5_for_table(&conn, "forge_issues", "forge.issue", &fts_fields).unwrap();
        populate_fts5_for_table(&conn, "forge_issues", "github.issue", &fts_fields).unwrap();
        assert_eq!(fts_count(&conn, "forge.issue"), 1);
        assert_eq!(fts_count(&conn, "github.issue"), 1);

        // Now simulate the coordinator with dedup + stale cleanup.
        // Schema A wins, Schema B is skipped and its entries are deleted.
        let mut indexed_tables = std::collections::HashSet::new();
        let mut skipped_event_types = Vec::new();

        // Schema A: forge → indexes table
        indexed_tables.insert("forge_issues".to_string());
        populate_fts5_for_table(&conn, "forge_issues", "forge.issue", &fts_fields).unwrap();

        // Schema B: github → table already indexed, skip
        if !indexed_tables.insert("forge_issues".to_string()) {
            skipped_event_types.push("github.issue".to_string());
        }

        // Cleanup stale entries
        for et in &skipped_event_types {
            conn.execute("DELETE FROM code_fts WHERE event_type = ?1", [et.as_str()])
                .unwrap();
        }

        // Only forge.issue remains — github.issue stale entries cleaned up
        assert_eq!(fts_count(&conn, "forge.issue"), 1);
        assert_eq!(fts_count(&conn, "github.issue"), 0);
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM code_fts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(total, 1);
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
        // Only "title" passes identifier validation
        assert_eq!(expr, "COALESCE(title, '')");
    }

    // --- write-side event_type parameterization tests ---

    /// Set up an in-memory events.db with eventlog table (for dedup tests).
    fn setup_events_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE eventlog (
                seq INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                source_id TEXT NOT NULL,
                source_file TEXT,
                data TEXT NOT NULL,
                provenance TEXT NOT NULL DEFAULT 'local',
                CHECK(json_valid(data))
            );
            CREATE INDEX idx_eventlog_type ON eventlog(event_type);",
        )
        .unwrap();
        conn
    }

    /// Set up patina.db with schema_registry + materialized views (for insert tests).
    fn setup_patina_db_with_registry() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_registry (
                schema_name TEXT NOT NULL,
                event_type  TEXT NOT NULL,
                fact_name   TEXT NOT NULL,
                table_name  TEXT NOT NULL,
                fts_fields  TEXT,
                corpus_query TEXT,
                offset_slot INTEGER,
                PRIMARY KEY (schema_name, event_type)
            );
            CREATE TABLE forge_issues (
                number INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                body TEXT,
                state TEXT NOT NULL,
                labels TEXT,
                author TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                url TEXT NOT NULL,
                event_seq INTEGER
            );
            CREATE TABLE forge_prs (
                number INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                body TEXT,
                state TEXT NOT NULL,
                labels TEXT,
                author TEXT,
                created_at TEXT NOT NULL,
                merged_at TEXT,
                url TEXT NOT NULL,
                linked_issues TEXT,
                approvals INTEGER DEFAULT 0,
                event_seq INTEGER
            );",
        )
        .unwrap();

        // Populate with both forge and github schemas (shared tables)
        conn.execute_batch(
            "INSERT INTO schema_registry (schema_name, event_type, fact_name, table_name)
             VALUES ('forge', 'forge.issue', 'issue', 'forge_issues');
             INSERT INTO schema_registry (schema_name, event_type, fact_name, table_name)
             VALUES ('forge', 'forge.pr', 'pull-request', 'forge_prs');
             INSERT INTO schema_registry (schema_name, event_type, fact_name, table_name)
             VALUES ('github', 'github.issue', 'issue', 'forge_issues');
             INSERT INTO schema_registry (schema_name, event_type, fact_name, table_name)
             VALUES ('github', 'github.pr', 'pull-request', 'forge_prs');",
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_resolve_event_type_from_registry() {
        let conn = setup_patina_db_with_registry();

        // Alphabetically first schema wins: "forge" < "github"
        let et = resolve_event_type(&conn, "forge_issues", "fallback");
        assert_eq!(et, "forge.issue");

        let et = resolve_event_type(&conn, "forge_prs", "fallback");
        assert_eq!(et, "forge.pr");
    }

    #[test]
    fn test_resolve_event_type_fallback_no_registry() {
        let conn = Connection::open_in_memory().unwrap();
        // No schema_registry table — should use fallback
        let et = resolve_event_type(&conn, "forge_issues", "my.fallback");
        assert_eq!(et, "my.fallback");
    }

    #[test]
    fn test_resolve_dedup_event_types_returns_all() {
        let conn = setup_patina_db_with_registry();

        let types = resolve_dedup_event_types(&conn, "forge_issues", "fallback");
        assert_eq!(types.len(), 2);
        assert!(types.contains(&"forge.issue".to_string()));
        assert!(types.contains(&"github.issue".to_string()));
    }

    #[test]
    fn test_resolve_dedup_event_types_fallback() {
        let conn = Connection::open_in_memory().unwrap();
        let types = resolve_dedup_event_types(&conn, "forge_issues", "my.fallback");
        assert_eq!(types, vec!["my.fallback"]);
    }

    #[test]
    fn test_event_exists_exact_match() {
        let events_conn = setup_events_db();

        // Insert an event with forge.issue type
        events_conn
            .execute(
                "INSERT INTO eventlog (event_type, timestamp, source_id, data)
                 VALUES ('forge.issue', '2026-01-01', '42', ?1)",
                [r#"{"number": 42, "updated_at": "2026-01-01T00:00:00Z"}"#],
            )
            .unwrap();

        // Exact match: forge.issue should find it
        let types = vec!["forge.issue".to_string()];
        assert!(event_exists(&events_conn, &types, 42, "2026-01-01T00:00:00Z").unwrap());

        // Multi-type match: should also find via combined types
        let types = vec!["forge.issue".to_string(), "github.issue".to_string()];
        assert!(event_exists(&events_conn, &types, 42, "2026-01-01T00:00:00Z").unwrap());

        // Non-matching type should not find it
        let types = vec!["github.issue".to_string()];
        assert!(!event_exists(&events_conn, &types, 42, "2026-01-01T00:00:00Z").unwrap());
    }

    #[test]
    fn test_event_exists_cross_schema_dedup() {
        let events_conn = setup_events_db();

        // Old event written as forge.issue
        events_conn
            .execute(
                "INSERT INTO eventlog (event_type, timestamp, source_id, data)
                 VALUES ('forge.issue', '2026-01-01', '7', ?1)",
                [r#"{"number": 7, "updated_at": "2026-01-01T12:00:00Z"}"#],
            )
            .unwrap();

        // Dedup with both types should catch the old forge.issue event
        let types = vec!["forge.issue".to_string(), "github.issue".to_string()];
        assert!(event_exists(&events_conn, &types, 7, "2026-01-01T12:00:00Z").unwrap());

        // Different number should not match
        assert!(!event_exists(&events_conn, &types, 8, "2026-01-01T12:00:00Z").unwrap());

        // Different updated_at should not match
        assert!(!event_exists(&events_conn, &types, 7, "2026-02-01T00:00:00Z").unwrap());
    }

    #[test]
    fn test_insert_issues_uses_provided_event_type() {
        let patina_conn = setup_patina_db_with_registry();
        let events_conn = setup_events_db();

        let issues = vec![Issue {
            number: 1,
            title: "Test issue".to_string(),
            body: Some("Body".to_string()),
            state: IssueState::Open,
            author: "user".to_string(),
            labels: vec![],
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01T12:00:00Z".to_string(),
            url: "https://example.com/1".to_string(),
        }];

        let stats = insert_issues(&patina_conn, &events_conn, &issues, "forge.issue").unwrap();
        assert_eq!(stats.inserted, 1);
        assert_eq!(stats.skipped, 0);

        // Verify event was written with the provided event_type
        let stored_type: String = events_conn
            .query_row(
                "SELECT event_type FROM eventlog WHERE json_extract(data, '$.number') = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored_type, "forge.issue");
    }

    #[test]
    fn test_insert_issues_dedup_across_schemas() {
        let patina_conn = setup_patina_db_with_registry();
        let events_conn = setup_events_db();

        // Insert an event manually as "forge.issue" (simulating old data)
        events_conn
            .execute(
                "INSERT INTO eventlog (event_type, timestamp, source_id, data)
                 VALUES ('forge.issue', '2026-01-01', '1', ?1)",
                [r#"{"number": 1, "updated_at": "2026-01-01T12:00:00Z"}"#],
            )
            .unwrap();

        // Now try to insert the same issue via insert_issues with "github.issue"
        let issues = vec![Issue {
            number: 1,
            title: "Test issue".to_string(),
            body: Some("Body".to_string()),
            state: IssueState::Open,
            author: "user".to_string(),
            labels: vec![],
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01T12:00:00Z".to_string(),
            url: "https://example.com/1".to_string(),
        }];

        // Should skip because schema_registry maps both forge.issue and
        // github.issue to forge_issues — cross-schema dedup catches the old event
        let stats = insert_issues(&patina_conn, &events_conn, &issues, "github.issue").unwrap();
        assert_eq!(stats.inserted, 0);
        assert_eq!(stats.skipped, 1);
    }

    #[test]
    fn test_insert_prs_uses_provided_event_type() {
        let patina_conn = setup_patina_db_with_registry();
        let events_conn = setup_events_db();

        let prs = vec![PullRequest {
            number: 10,
            title: "Test PR".to_string(),
            body: Some("PR Body".to_string()),
            state: PrState::Open,
            author: "dev".to_string(),
            labels: vec![],
            created_at: "2026-01-01".to_string(),
            merged_at: None,
            url: "https://example.com/10".to_string(),
            linked_issues: vec![],
            comments: vec![],
            approvals: 0,
        }];

        let stats = insert_prs(&patina_conn, &events_conn, &prs, "forge.pr").unwrap();
        assert_eq!(stats.inserted, 1);

        let stored_type: String = events_conn
            .query_row(
                "SELECT event_type FROM eventlog WHERE json_extract(data, '$.number') = 10",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored_type, "forge.pr");
    }
}
