//! GitHub issues scraper - extracts issues for bounty discovery and context
//!
//! Uses unified eventlog pattern:
//! - Inserts github.issue events into eventlog table
//! - Creates materialized views (github_issues) from eventlog
//! - Supports incremental updates via updated_at timestamp

use anyhow::{Context, Result};
use regex::Regex;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use super::database;
use super::ScrapeStats;

/// GitHub issue from `gh issue list --json`
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubIssue {
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub labels: Vec<Label>,
    pub author: Author,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub closed_at: Option<String>,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Label {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Author {
    pub login: String,
}

/// Bounty detection result
#[derive(Debug)]
pub struct BountyInfo {
    pub is_bounty: bool,
    pub amount: Option<String>,
}

/// Labels that indicate a bounty
const BOUNTY_LABELS: &[&str] = &["bounty", "onlydust", "reward", "paid", "💰"];

/// Create materialized views for GitHub events
fn create_materialized_views(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- GitHub issues view (materialized from github.issue events)
        CREATE TABLE IF NOT EXISTS github_issues (
            number INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            body TEXT,
            state TEXT NOT NULL,
            labels TEXT,           -- JSON array of label names
            author TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            closed_at TEXT,
            url TEXT NOT NULL,
            is_bounty INTEGER DEFAULT 0,
            bounty_amount TEXT,
            event_seq INTEGER,     -- Link back to eventlog
            FOREIGN KEY (event_seq) REFERENCES eventlog(seq)
        );

        -- Indexes for common queries
        CREATE INDEX IF NOT EXISTS idx_github_issues_state ON github_issues(state);
        CREATE INDEX IF NOT EXISTS idx_github_issues_updated ON github_issues(updated_at);
        CREATE INDEX IF NOT EXISTS idx_github_issues_bounty ON github_issues(is_bounty);
        "#,
    )?;

    Ok(())
}

/// Check if `gh` CLI is authenticated
pub fn check_gh_auth() -> Result<bool> {
    let output = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .context("Failed to run `gh auth status`. Is `gh` CLI installed?")?;

    Ok(output.status.success())
}

/// Fetch issues from GitHub using `gh` CLI
pub fn fetch_issues(repo: &str, limit: usize, since: Option<&str>) -> Result<Vec<GitHubIssue>> {
    let mut cmd = Command::new("gh");
    cmd.args([
        "issue",
        "list",
        "--repo",
        repo,
        "--limit",
        &limit.to_string(),
        "--state",
        "all",
        "--json",
        "number,title,body,state,labels,author,createdAt,updatedAt,closedAt,url",
    ]);

    // Add search filter for incremental updates
    if let Some(timestamp) = since {
        // Format: updated:>=2025-11-28
        let date = &timestamp[..10]; // Extract YYYY-MM-DD
        cmd.args(["--search", &format!("updated:>={}", date)]);
    }

    let output = cmd.output().context("Failed to run `gh issue list`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh issue list failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let issues: Vec<GitHubIssue> =
        serde_json::from_str(&stdout).context("Failed to parse GitHub issues JSON")?;

    Ok(issues)
}

/// Detect if an issue is a bounty based on labels and body content
pub fn detect_bounty(issue: &GitHubIssue) -> BountyInfo {
    // Check labels
    let label_match = issue.labels.iter().any(|l| {
        let lower = l.name.to_lowercase();
        BOUNTY_LABELS.iter().any(|b| lower.contains(b))
    });

    // Check body for bounty amount patterns
    let amount = issue
        .body
        .as_ref()
        .and_then(|body| extract_bounty_amount(body));

    BountyInfo {
        is_bounty: label_match || amount.is_some(),
        amount,
    }
}

/// Extract bounty amount from issue body
/// Patterns: "Bounty: 500 USDC", "Reward: $100", "💰 200 USD"
fn extract_bounty_amount(body: &str) -> Option<String> {
    // Try various patterns
    let patterns = [
        r"(?i)bounty[:\s]+\$?(\d+[\d,]*)\s*(usdc?|usd|\$)?",
        r"(?i)reward[:\s]+\$?(\d+[\d,]*)\s*(usdc?|usd|\$)?",
        r"💰\s*\$?(\d+[\d,]*)\s*(usdc?|usd|\$)?",
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(body) {
                let amount = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let currency = caps
                    .get(2)
                    .map(|m| m.as_str())
                    .unwrap_or("USD")
                    .to_uppercase();
                let currency = if currency == "$" { "USD" } else { &currency };
                return Some(format!("{} {}", amount, currency));
            }
        }
    }

    None
}

/// Insert issues into eventlog and materialized views
fn insert_issues(conn: &Connection, issues: &[GitHubIssue]) -> Result<usize> {
    let mut count = 0;

    let mut issue_stmt = conn.prepare(
        "INSERT OR REPLACE INTO github_issues
         (number, title, body, state, labels, author, created_at, updated_at, closed_at, url, is_bounty, bounty_amount, event_seq)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
    )?;

    for issue in issues {
        let bounty = detect_bounty(issue);
        let labels_json: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();
        let labels_str = serde_json::to_string(&labels_json)?;

        // 1. Insert into eventlog (source of truth)
        let event_data = json!({
            "number": issue.number,
            "title": &issue.title,
            "body": &issue.body,
            "state": &issue.state,
            "labels": &labels_json,
            "author": &issue.author.login,
            "url": &issue.url,
            "is_bounty": bounty.is_bounty,
            "bounty_amount": &bounty.amount,
        });

        let seq = database::insert_event(
            conn,
            "github.issue",
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
            &issue.state,
            &labels_str,
            &issue.author.login,
            &issue.created_at,
            &issue.updated_at,
            &issue.closed_at,
            &issue.url,
            bounty.is_bounty as i32,
            &bounty.amount,
            seq,
        ])?;

        count += 1;
    }

    Ok(count)
}

/// Populate FTS5 index with GitHub issues
pub fn populate_fts5_github(conn: &Connection) -> Result<usize> {
    // Insert GitHub issues into FTS5 (title + body as content)
    let count = conn.execute(
        r#"
        INSERT INTO code_fts (symbol_name, file_path, content, event_type)
        SELECT
            json_extract(data, '$.title') as symbol_name,
            json_extract(data, '$.url') as file_path,
            COALESCE(json_extract(data, '$.body'), '') as content,
            'github.issue' as event_type
        FROM eventlog
        WHERE event_type = 'github.issue'
        "#,
        [],
    )?;

    Ok(count)
}

/// Get the last scraped timestamp from metadata
fn get_last_scrape(conn: &Connection) -> Result<Option<String>> {
    database::get_last_processed(conn, "github")
}

/// Update the last scraped timestamp
fn update_last_scrape(conn: &Connection, timestamp: &str) -> Result<()> {
    database::set_last_processed(conn, "github", timestamp)
}

/// Scrape configuration for GitHub
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

/// Main entry point for GitHub scraping
pub fn run(config: GitHubScrapeConfig) -> Result<ScrapeStats> {
    let start = Instant::now();
    let db_path = Path::new(&config.db_path);

    // Check gh auth
    if !check_gh_auth()? {
        anyhow::bail!("GitHub CLI not authenticated. Run `gh auth login` first.");
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

    if since.is_some() {
        println!("📊 Incremental GitHub scrape since last update...");
    } else {
        println!("📊 Full GitHub issues scrape...");
    }

    // Fetch issues
    let issues = fetch_issues(&config.repo, config.limit, since.as_deref())?;

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

    // Count bounties
    let bounty_count = issues.iter().filter(|i| detect_bounty(i).is_bounty).count();
    if bounty_count > 0 {
        println!("  💰 Detected {} bounties", bounty_count);
    }

    // Insert issues
    let issue_count = insert_issues(&conn, &issues)?;
    println!("  Inserted {} issues", issue_count);

    // Update last scrape timestamp
    if let Some(latest) = issues.iter().max_by_key(|i| &i.updated_at) {
        update_last_scrape(&conn, &latest.updated_at)?;
    }

    // Populate FTS5 index
    let fts_count = populate_fts5_github(&conn)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_bounty_from_labels() {
        let issue = GitHubIssue {
            number: 1,
            title: "Test issue".to_string(),
            body: None,
            state: "open".to_string(),
            labels: vec![Label {
                name: "bounty".to_string(),
            }],
            author: Author {
                login: "test".to_string(),
            },
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
            closed_at: None,
            url: "https://github.com/test/test/issues/1".to_string(),
        };

        let bounty = detect_bounty(&issue);
        assert!(bounty.is_bounty);
    }

    #[test]
    fn test_detect_bounty_from_body() {
        let issue = GitHubIssue {
            number: 1,
            title: "Test issue".to_string(),
            body: Some("This is a Bounty: 500 USDC for completing this task.".to_string()),
            state: "open".to_string(),
            labels: vec![],
            author: Author {
                login: "test".to_string(),
            },
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
            closed_at: None,
            url: "https://github.com/test/test/issues/1".to_string(),
        };

        let bounty = detect_bounty(&issue);
        assert!(bounty.is_bounty);
        assert_eq!(bounty.amount, Some("500 USDC".to_string()));
    }

    #[test]
    fn test_extract_bounty_amount() {
        assert_eq!(
            extract_bounty_amount("Bounty: 500 USDC"),
            Some("500 USDC".to_string())
        );
        assert_eq!(
            extract_bounty_amount("Reward: $100"),
            Some("100 USD".to_string())
        );
        assert_eq!(
            extract_bounty_amount("💰 200 USD"),
            Some("200 USD".to_string())
        );
        assert_eq!(extract_bounty_amount("No bounty here"), None);
    }

    #[test]
    fn test_no_bounty() {
        let issue = GitHubIssue {
            number: 1,
            title: "Regular issue".to_string(),
            body: Some("Just a normal bug report.".to_string()),
            state: "open".to_string(),
            labels: vec![Label {
                name: "bug".to_string(),
            }],
            author: Author {
                login: "test".to_string(),
            },
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
            closed_at: None,
            url: "https://github.com/test/test/issues/1".to_string(),
        };

        let bounty = detect_bounty(&issue);
        assert!(!bounty.is_bounty);
        assert!(bounty.amount.is_none());
    }
}
