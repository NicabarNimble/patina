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
use std::path::Path;
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

/// Collect unique PR refs from git.commit events that haven't been fetched yet.
fn collect_pr_refs(conn: &Connection) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT DISTINCT json_extract(data, '$.parsed.pr_ref') as pr_ref
        FROM eventlog
        WHERE event_type = 'git.commit'
          AND json_extract(data, '$.parsed.pr_ref') IS NOT NULL
          AND json_extract(data, '$.parsed.pr_ref') NOT IN (
              SELECT number FROM forge_prs
          )
        ORDER BY pr_ref DESC
        "#,
    )?;

    let pr_refs: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(pr_refs)
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
        let linked_issues_str = serde_json::to_string(&pr.linked_issues)?;
        let state_str = match pr.state {
            PrState::Open => "open",
            PrState::Merged => "merged",
            PrState::Closed => "closed",
        };

        // Combine comments into single text for storage
        let comments_text: Vec<String> = pr
            .comments
            .iter()
            .map(|c| format!("{}: {}", c.author, c.body))
            .collect();

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
            "comments": comments_text,
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
            &linked_issues_str,
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
fn get_remote_url() -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .context("Failed to get git remote URL")?;

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
    pub limit: usize,    // max issues to fetch
    pub force: bool,     // full rebuild vs incremental
    pub db_path: String, // path to patina.db
}

impl Default for ForgeScrapeConfig {
    fn default() -> Self {
        Self {
            limit: 500,
            force: false,
            db_path: database::PATINA_DB.to_string(),
        }
    }
}

/// Main entry point for forge scraping.
pub fn run(config: ForgeScrapeConfig) -> Result<ScrapeStats> {
    let start = Instant::now();
    let db_path = Path::new(&config.db_path);

    // Detect forge from remote URL
    let remote_url = match get_remote_url()? {
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
    if since.is_some() {
        println!(
            "📊 Incremental forge scrape for {} since last update...",
            forge_name
        );
    } else {
        println!("📊 Full forge issues scrape for {}...", forge_name);
    }

    // Get reader and fetch issues
    let reader = forge::reader(&detected);
    let issues = reader.list_issues(config.limit, since.as_deref())?;

    if issues.is_empty() {
        println!("  No new issues to process");
        return Ok(ScrapeStats {
            items_processed: 0,
            time_elapsed: start.elapsed(),
            database_size_kb: std::fs::metadata(db_path)
                .map(|m| m.len() / 1024)
                .unwrap_or(0),
        });
    }

    println!("  Found {} issues to process", issues.len());

    // Insert issues
    let issue_count = insert_issues(&conn, &issues)?;
    println!("  Inserted {} issues", issue_count);

    // Update last scrape timestamp
    if let Some(latest) = issues.iter().max_by_key(|i| &i.updated_at) {
        update_last_scrape(&conn, &latest.updated_at)?;
    }

    // Populate FTS5 index for issues
    let issue_fts_count = populate_fts5_issues(&conn)?;
    println!("  Indexed {} issues in FTS5", issue_fts_count);

    // Phase 3: Fetch PRs referenced in commits
    let pr_refs = collect_pr_refs(&conn)?;
    let mut pr_count = 0;

    if !pr_refs.is_empty() {
        println!("  Found {} PR refs to fetch from commits", pr_refs.len());

        let mut prs = Vec::new();
        for pr_num in &pr_refs {
            match reader.get_pull_request(*pr_num) {
                Ok(pr) => prs.push(pr),
                Err(e) => {
                    // PR might be deleted or inaccessible - skip but don't fail
                    println!("  ⚠️  Could not fetch PR #{}: {}", pr_num, e);
                }
            }
        }

        if !prs.is_empty() {
            pr_count = insert_prs(&conn, &prs)?;
            println!("  Inserted {} PRs", pr_count);

            // Populate FTS5 index for PRs
            let pr_fts_count = populate_fts5_prs(&conn)?;
            println!("  Indexed {} PRs in FTS5", pr_fts_count);
        }
    }

    let elapsed = start.elapsed();
    let db_size = std::fs::metadata(db_path)
        .map(|m| m.len() / 1024)
        .unwrap_or(0);

    Ok(ScrapeStats {
        items_processed: issue_count + pr_count,
        time_elapsed: elapsed,
        database_size_kb: db_size,
    })
}

// ============================================================================
// Legacy compatibility - keeping old API for repo/internal.rs
// ============================================================================

/// Legacy config for backward compatibility with repo command.
pub struct GitHubScrapeConfig {
    pub repo: String,    // owner/repo format
    pub limit: usize,    // max issues to fetch
    pub force: bool,     // full rebuild vs incremental
    pub db_path: String, // path to patina.db
}

impl Default for GitHubScrapeConfig {
    fn default() -> Self {
        Self {
            repo: String::new(),
            limit: 500,
            force: false,
            db_path: database::PATINA_DB.to_string(),
        }
    }
}

/// Legacy entry point for backward compatibility.
///
/// Used by repo command which provides explicit owner/repo.
pub fn run_legacy(config: GitHubScrapeConfig) -> Result<ScrapeStats> {
    let start = Instant::now();
    let db_path = Path::new(&config.db_path);

    // Check authentication
    if !forge::github::is_authenticated()? {
        anyhow::bail!("GitHub CLI not authenticated. Run `gh auth login` first.");
    }

    // Build forge from explicit repo spec
    let parts: Vec<&str> = config.repo.splitn(2, '/').collect();
    if parts.len() != 2 {
        anyhow::bail!(
            "Invalid repo format. Expected owner/repo, got: {}",
            config.repo
        );
    }

    let detected = forge::Forge {
        kind: ForgeKind::GitHub,
        owner: parts[0].to_string(),
        repo: parts[1].to_string(),
        host: "github.com".to_string(),
    };

    // Initialize database
    let conn = database::initialize(db_path)?;
    create_materialized_views(&conn)?;

    // Get last scrape timestamp
    let since = if config.force {
        None
    } else {
        get_last_scrape(&conn)?
    };

    if since.is_some() {
        println!("📊 Incremental forge scrape since last update...");
    } else {
        println!("📊 Full forge issues scrape...");
    }

    // Fetch issues via trait
    let reader = forge::reader(&detected);
    let issues = reader.list_issues(config.limit, since.as_deref())?;

    if issues.is_empty() {
        println!("  No new issues to process");
        return Ok(ScrapeStats {
            items_processed: 0,
            time_elapsed: start.elapsed(),
            database_size_kb: std::fs::metadata(db_path)
                .map(|m| m.len() / 1024)
                .unwrap_or(0),
        });
    }

    println!("  Found {} issues to process", issues.len());

    // Insert issues
    let issue_count = insert_issues(&conn, &issues)?;
    println!("  Inserted {} issues", issue_count);

    // Update timestamp
    if let Some(latest) = issues.iter().max_by_key(|i| &i.updated_at) {
        update_last_scrape(&conn, &latest.updated_at)?;
    }

    // Populate FTS5
    let fts_count = populate_fts5_issues(&conn)?;
    println!("  Indexed {} issues in FTS5", fts_count);

    let elapsed = start.elapsed();
    let db_size = std::fs::metadata(db_path)
        .map(|m| m.len() / 1024)
        .unwrap_or(0);

    Ok(ScrapeStats {
        items_processed: issue_count,
        time_elapsed: elapsed,
        database_size_kb: db_size,
    })
}
