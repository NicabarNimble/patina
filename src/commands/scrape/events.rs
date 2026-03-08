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
// Dedup helpers
// ============================================================================

/// Check if we already have this issue at this updated_at timestamp.
fn issue_event_exists(events_conn: &Connection, number: i64, updated_at: &str) -> Result<bool> {
    let count: i64 = events_conn.query_row(
        "SELECT COUNT(*) FROM eventlog
         WHERE event_type LIKE '%.issue'
           AND json_extract(data, '$.number') = ?1
           AND json_extract(data, '$.updated_at') = ?2",
        rusqlite::params![number, updated_at],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Check if we already have this PR at this updated_at timestamp.
fn pr_event_exists(events_conn: &Connection, number: i64, updated_at: &str) -> Result<bool> {
    let count: i64 = events_conn.query_row(
        "SELECT COUNT(*) FROM eventlog
         WHERE event_type LIKE '%.pr'
           AND json_extract(data, '$.number') = ?1
           AND json_extract(data, '$.updated_at') = ?2",
        rusqlite::params![number, updated_at],
        |row| row.get(0),
    )?;
    Ok(count > 0)
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
pub fn insert_issues(
    patina_conn: &Connection,
    events_conn: &Connection,
    issues: &[Issue],
) -> Result<InsertStats> {
    let mut inserted = 0;
    let mut skipped = 0;

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

        let seq = if issue_event_exists(events_conn, issue.number, &issue.updated_at)? {
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
                "forge.issue",
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
pub fn insert_prs(
    patina_conn: &Connection,
    events_conn: &Connection,
    prs: &[PullRequest],
) -> Result<InsertStats> {
    let mut inserted = 0;
    let mut skipped = 0;

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

        let seq = if pr_event_exists(events_conn, pr.number, updated_at)? {
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
                "forge.pr",
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
// FTS5 indexing
// ============================================================================

/// Populate FTS5 index with forge issues.
///
/// DELETE and INSERT use the same label ('forge.issue') so they stay consistent
/// regardless of which schemas are installed. FTS5 reads from the materialized
/// view table, not from the eventlog — the label is a display tag, not a filter.
pub fn populate_fts5_issues(conn: &Connection) -> Result<usize> {
    conn.execute("DELETE FROM code_fts WHERE event_type = 'forge.issue'", [])?;

    let count = conn.execute(
        r#"
        INSERT INTO code_fts (symbol_name, file_path, content, event_type)
        SELECT
            title as symbol_name,
            url as file_path,
            COALESCE(body, '') as content,
            'forge.issue' as event_type
        FROM forge_issues
        "#,
        [],
    )?;

    Ok(count)
}

/// Populate FTS5 index with forge PRs.
///
/// See populate_fts5_issues for DELETE/INSERT consistency rationale.
pub fn populate_fts5_prs(conn: &Connection) -> Result<usize> {
    conn.execute("DELETE FROM code_fts WHERE event_type = 'forge.pr'", [])?;

    let count = conn.execute(
        r#"
        INSERT INTO code_fts (symbol_name, file_path, content, event_type)
        SELECT
            title as symbol_name,
            url as file_path,
            COALESCE(body, '') as content,
            'forge.pr' as event_type
        FROM forge_prs
        "#,
        [],
    )?;

    Ok(count)
}
