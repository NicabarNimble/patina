//! Forge scraper - extracts issues and PRs for project context.
//!
//! Uses ForgeReader trait for platform abstraction:
//! - Detects forge from git remote (GitHub, Gitea, etc.)
//! - Fetches issues/PRs via ForgeReader
//! - Stores in eventlog with materialized views
//!
//! Graceful degradation: works without forge access (returns empty).

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use super::database;
use super::ScrapeStats;
use patina::forge::{self, ForgeKind, Issue, IssueState, PrState, PullRequest};

/// Create materialized views for forge events.
fn create_materialized_views(conn: &Connection) -> Result<()> {
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
            event_seq INTEGER,     -- Link back to eventlog
            FOREIGN KEY (event_seq) REFERENCES eventlog(seq)
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
            event_seq INTEGER,     -- Link back to eventlog
            FOREIGN KEY (event_seq) REFERENCES eventlog(seq)
        );

        -- Indexes for common queries
        CREATE INDEX IF NOT EXISTS idx_forge_issues_state ON forge_issues(state);
        CREATE INDEX IF NOT EXISTS idx_forge_issues_updated ON forge_issues(updated_at);
        CREATE INDEX IF NOT EXISTS idx_forge_prs_state ON forge_prs(state);
        CREATE INDEX IF NOT EXISTS idx_forge_prs_merged ON forge_prs(merged_at);

        -- Forge refs backlog (for incremental sync with pacing)
        -- Tracks #N references found in commits, pending resolution
        CREATE TABLE IF NOT EXISTS forge_refs (
            repo        TEXT NOT NULL,       -- owner/repo
            ref_number  INTEGER NOT NULL,

            -- What we know
            ref_kind    TEXT DEFAULT 'unknown',  -- 'unknown', 'issue', 'pr'
            discovered  TEXT NOT NULL,       -- ISO timestamp when found
            source      TEXT,                -- Commit SHA where found

            -- Resolution status
            resolved    TEXT,                -- ISO timestamp when fetched (NULL = pending)
            error       TEXT,                -- Error message if failed

            PRIMARY KEY (repo, ref_number)
        );

        -- Index for efficient backlog queries (pending refs, newest first)
        CREATE INDEX IF NOT EXISTS idx_forge_refs_pending
        ON forge_refs(repo, discovered DESC) WHERE resolved IS NULL;
        "#,
    )?;

    Ok(())
}

/// Insert issues into eventlog and materialized views.
fn insert_issues(conn: &Connection, issues: &[Issue]) -> Result<usize> {
    let mut count = 0;

    let mut issue_stmt = conn.prepare(
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

        // 1. Insert into eventlog (source of truth)
        let event_data = json!({
            "number": issue.number,
            "title": &issue.title,
            "body": &issue.body,
            "state": state_str,
            "labels": &issue.labels,
            "author": &issue.author,
            "url": &issue.url,
        });

        let seq = database::insert_event(
            conn,
            "forge.issue",
            &issue.created_at,
            &issue.number.to_string(),
            Some(&issue.url),
            &event_data.to_string(),
        )?;

        // 2. Update materialized view
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

        count += 1;
    }

    Ok(count)
}

/// Insert PRs into eventlog and materialized views.
fn insert_prs(conn: &Connection, prs: &[PullRequest]) -> Result<usize> {
    let mut count = 0;

    let mut pr_stmt = conn.prepare(
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

        // Combine body and comments for searchable content
        let comments_text: String = pr
            .comments
            .iter()
            .map(|c| format!("{}: {}", c.author, c.body))
            .collect::<Vec<_>>()
            .join("\n");

        // 1. Insert into eventlog (source of truth)
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
        });

        let seq = database::insert_event(
            conn,
            "forge.pr",
            &pr.created_at,
            &pr.number.to_string(),
            Some(&pr.url),
            &event_data.to_string(),
        )?;

        // 2. Update materialized view
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

        count += 1;
    }

    Ok(count)
}

/// Populate FTS5 index with forge issues.
pub fn populate_fts5_issues(conn: &Connection) -> Result<usize> {
    // Clear existing forge.issue entries to avoid duplicates on re-run
    conn.execute("DELETE FROM code_fts WHERE event_type = 'forge.issue'", [])?;

    let count = conn.execute(
        r#"
        INSERT INTO code_fts (symbol_name, file_path, content, event_type)
        SELECT
            json_extract(data, '$.title') as symbol_name,
            json_extract(data, '$.url') as file_path,
            COALESCE(json_extract(data, '$.body'), '') as content,
            'forge.issue' as event_type
        FROM eventlog
        WHERE event_type = 'forge.issue'
        "#,
        [],
    )?;

    Ok(count)
}

/// Populate FTS5 index with forge PRs.
pub fn populate_fts5_prs(conn: &Connection) -> Result<usize> {
    // Clear existing forge.pr entries to avoid duplicates on re-run
    conn.execute("DELETE FROM code_fts WHERE event_type = 'forge.pr'", [])?;

    // Include PR body and comments for rich search
    let count = conn.execute(
        r#"
        INSERT INTO code_fts (symbol_name, file_path, content, event_type)
        SELECT
            json_extract(data, '$.title') as symbol_name,
            json_extract(data, '$.url') as file_path,
            COALESCE(json_extract(data, '$.body'), '') || ' ' ||
            COALESCE(json_extract(data, '$.comments'), '') as content,
            'forge.pr' as event_type
        FROM eventlog
        WHERE event_type = 'forge.pr'
        "#,
        [],
    )?;

    Ok(count)
}

/// Get the last scraped timestamp from metadata.
fn get_last_scrape(conn: &Connection) -> Result<Option<String>> {
    database::get_last_processed(conn, "forge")
}

/// Update the last scraped timestamp.
fn update_last_scrape(conn: &Connection, timestamp: &str) -> Result<()> {
    database::set_last_processed(conn, "forge", timestamp)
}

/// Detect git remote origin URL.
fn get_remote_url(working_dir: Option<&Path>) -> Result<Option<String>> {
    let mut cmd = Command::new("git");
    cmd.args(["remote", "get-url", "origin"]);
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }
    let output = cmd.output().context("Failed to get git remote URL")?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !url.is_empty() {
            return Ok(Some(url));
        }
    }

    Ok(None)
}

/// Scrape configuration for forge.
pub struct ForgeScrapeConfig {
    pub limit: usize,                 // max issues to fetch
    pub force: bool,                  // full rebuild vs incremental
    pub working_dir: Option<PathBuf>, // target directory (None = cwd)
}

impl Default for ForgeScrapeConfig {
    fn default() -> Self {
        Self {
            limit: 50000,
            force: false,
            working_dir: None,
        }
    }
}

/// Main entry point for forge scraping.
pub fn run(config: ForgeScrapeConfig) -> Result<ScrapeStats> {
    let start = Instant::now();

    // Compute db_path based on working_dir
    let db_path_buf: PathBuf;
    let db_path = match &config.working_dir {
        Some(dir) => {
            db_path_buf = dir.join(".patina/local/data/patina.db");
            db_path_buf.as_path()
        }
        None => Path::new(database::PATINA_DB),
    };

    // Detect forge from remote URL
    let remote_url = match get_remote_url(config.working_dir.as_deref())? {
        Some(url) => url,
        None => {
            println!("  No git remote configured, skipping forge scrape");
            return Ok(ScrapeStats {
                items_processed: 0,
                time_elapsed: start.elapsed(),
                database_size_kb: std::fs::metadata(db_path)
                    .map(|m| m.len() / 1024)
                    .unwrap_or(0),
            });
        }
    };

    let detected = forge::detect(&remote_url);

    // Handle forge detection result
    match detected.kind {
        ForgeKind::None => {
            // Silent - no forge is normal for local repos
            return Ok(ScrapeStats {
                items_processed: 0,
                time_elapsed: start.elapsed(),
                database_size_kb: std::fs::metadata(db_path)
                    .map(|m| m.len() / 1024)
                    .unwrap_or(0),
            });
        }
        ForgeKind::Gitea => {
            println!("  Gitea detected. Forge scraping not yet implemented.");
            return Ok(ScrapeStats {
                items_processed: 0,
                time_elapsed: start.elapsed(),
                database_size_kb: std::fs::metadata(db_path)
                    .map(|m| m.len() / 1024)
                    .unwrap_or(0),
            });
        }
        ForgeKind::GitHub => {
            // Check authentication
            if !forge::github::is_authenticated()? {
                println!("  GitHub detected but `gh` not authenticated. Skipping forge data.");
                println!("  Run `gh auth login` to enable issue/PR fetching.");
                return Ok(ScrapeStats {
                    items_processed: 0,
                    time_elapsed: start.elapsed(),
                    database_size_kb: std::fs::metadata(db_path)
                        .map(|m| m.len() / 1024)
                        .unwrap_or(0),
                });
            }
        }
    }

    // Initialize database
    let conn = database::initialize(db_path)?;
    create_materialized_views(&conn)?;

    // Get last scrape timestamp for incremental updates
    let since = if config.force {
        None
    } else {
        get_last_scrape(&conn)?
    };

    let forge_name = format!("{}/{}", detected.owner, detected.repo);

    // Get reader for bulk fetches
    let reader = forge::reader(&detected);

    // Query counts first for progress reporting
    let issue_count_expected = reader.get_issue_count().unwrap_or(0);
    let pr_count_expected = reader.get_pr_count().unwrap_or(0);

    if since.is_some() {
        println!(
            "📊 Incremental forge scrape for {} since last update...",
            forge_name
        );
    } else {
        println!(
            "📊 Full forge scrape for {} ({} issues, {} PRs)...",
            forge_name, issue_count_expected, pr_count_expected
        );
    }

    // Bulk fetch issues
    let issues = reader.list_issues(config.limit, since.as_deref())?;
    let issue_count = if issues.is_empty() {
        println!("  No new issues to process");
        0
    } else {
        println!(
            "  Fetched {}/{} issues",
            issues.len(),
            issue_count_expected
        );
        let count = insert_issues(&conn, &issues)?;
        println!("  Inserted {} issues", count);

        // Update last scrape timestamp from issues
        if let Some(latest) = issues.iter().max_by_key(|i| &i.updated_at) {
            update_last_scrape(&conn, &latest.updated_at)?;
        }

        // Populate FTS5 index for issues
        let issue_fts_count = populate_fts5_issues(&conn)?;
        println!("  Indexed {} issues in FTS5", issue_fts_count);
        count
    };

    // Bulk fetch PRs (same pattern as issues)
    let prs = reader.list_pull_requests(config.limit, since.as_deref())?;
    let pr_count = if prs.is_empty() {
        println!("  No new PRs to process");
        0
    } else {
        println!("  Fetched {}/{} PRs", prs.len(), pr_count_expected);
        let count = insert_prs(&conn, &prs)?;
        println!("  Inserted {} PRs", count);

        // Populate FTS5 index for PRs
        let pr_fts_count = populate_fts5_prs(&conn)?;
        println!("  Indexed {} PRs in FTS5", pr_fts_count);
        count
    };

    // Discover PR refs from commits (for numbers mentioned in commit messages)
    // These are PRs we know the number of but haven't fetched yet
    let repo_spec = format!("{}/{}", detected.owner, detected.repo);
    let sync_stats = forge::sync::run(&conn, reader.as_ref(), &repo_spec)?;

    if sync_stats.discovered > 0 || sync_stats.resolved > 0 {
        println!(
            "  Sync: {} PR refs from commits, {} resolved, {} pending",
            sync_stats.discovered, sync_stats.resolved, sync_stats.pending
        );
        if sync_stats.cache_hits > 0 {
            println!(
                "  ({} cache hits - already fetched via bulk)",
                sync_stats.cache_hits
            );
        }
        if sync_stats.errors > 0 {
            println!("  ({} refs failed - see warnings above)", sync_stats.errors);
        }
    }

    let elapsed = start.elapsed();
    let db_size = std::fs::metadata(db_path)
        .map(|m| m.len() / 1024)
        .unwrap_or(0);

    Ok(ScrapeStats {
        items_processed: issue_count + pr_count + sync_stats.resolved,
        time_elapsed: elapsed,
        database_size_kb: db_size,
    })
}
