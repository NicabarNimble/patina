//! Git history scraper - extracts commits, files changed, and co-change relationships

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use super::ScrapeStats;

const DB_PATH: &str = ".patina/data/git.db";

/// Parsed commit from git log
#[derive(Debug)]
struct GitCommit {
    sha: String,
    message: String,
    author_name: String,
    author_email: String,
    timestamp: String,
    files: Vec<FileChange>,
}

/// File change within a commit
#[derive(Debug)]
struct FileChange {
    path: String,
    change_type: String,
    lines_added: i32,
    lines_removed: i32,
}

/// Initialize the git database schema
pub fn initialize(db_path: &Path) -> Result<Connection> {
    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(db_path)?;

    conn.execute_batch(
        r#"
        -- Commits table
        CREATE TABLE IF NOT EXISTS commits (
            sha TEXT PRIMARY KEY,
            message TEXT,
            author_name TEXT,
            author_email TEXT,
            timestamp TEXT,
            branch TEXT
        );

        -- Files changed per commit
        CREATE TABLE IF NOT EXISTS commit_files (
            sha TEXT,
            file_path TEXT,
            change_type TEXT,
            lines_added INTEGER,
            lines_removed INTEGER,
            PRIMARY KEY (sha, file_path)
        );

        -- Co-change relationships (derived)
        CREATE TABLE IF NOT EXISTS co_changes (
            file_a TEXT,
            file_b TEXT,
            count INTEGER,
            PRIMARY KEY (file_a, file_b)
        );

        -- Scrape metadata
        CREATE TABLE IF NOT EXISTS scrape_meta (
            key TEXT PRIMARY KEY,
            value TEXT
        );

        -- Indexes for common queries
        CREATE INDEX IF NOT EXISTS idx_commits_timestamp ON commits(timestamp);
        CREATE INDEX IF NOT EXISTS idx_commits_author ON commits(author_email);
        CREATE INDEX IF NOT EXISTS idx_commit_files_path ON commit_files(file_path);
        CREATE INDEX IF NOT EXISTS idx_co_changes_count ON co_changes(count DESC);
        "#,
    )?;

    Ok(conn)
}

/// Parse git log output into commits
fn parse_git_log(since_sha: Option<&str>) -> Result<Vec<GitCommit>> {
    // Build git log command
    // Format: SHA|message|author_name|author_email|timestamp
    let mut cmd = Command::new("git");
    cmd.args([
        "log",
        "--pretty=format:%H|%s|%an|%ae|%aI",
        "--numstat",
        "--no-merges",
    ]);

    if let Some(sha) = since_sha {
        cmd.arg(format!("{}..HEAD", sha));
    }

    let output = cmd.output().context("Failed to run git log")?;

    if !output.status.success() {
        anyhow::bail!(
            "git log failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_git_log_output(&stdout)
}

/// Parse the git log output format
fn parse_git_log_output(output: &str) -> Result<Vec<GitCommit>> {
    let mut commits = Vec::new();
    let mut current_commit: Option<GitCommit> = None;

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }

        // Check if this is a commit line (contains 5 pipe-separated fields starting with sha)
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() == 5
            && parts[0].len() == 40
            && parts[0].chars().all(|c| c.is_ascii_hexdigit())
        {
            // Save previous commit if exists
            if let Some(commit) = current_commit.take() {
                commits.push(commit);
            }

            current_commit = Some(GitCommit {
                sha: parts[0].to_string(),
                message: parts[1].to_string(),
                author_name: parts[2].to_string(),
                author_email: parts[3].to_string(),
                timestamp: parts[4].to_string(),
                files: Vec::new(),
            });
        } else if let Some(ref mut commit) = current_commit {
            // This is a numstat line: additions\tdeletions\tfilename
            let stat_parts: Vec<&str> = line.split('\t').collect();
            if stat_parts.len() >= 3 {
                let lines_added = stat_parts[0].parse().unwrap_or(0);
                let lines_removed = stat_parts[1].parse().unwrap_or(0);
                let path = stat_parts[2].to_string();

                // Determine change type based on lines
                let change_type = if lines_added > 0 && lines_removed == 0 {
                    "added"
                } else if lines_added == 0 && lines_removed > 0 {
                    "deleted"
                } else {
                    "modified"
                };

                commit.files.push(FileChange {
                    path,
                    change_type: change_type.to_string(),
                    lines_added,
                    lines_removed,
                });
            }
        }
    }

    // Don't forget the last commit
    if let Some(commit) = current_commit {
        commits.push(commit);
    }

    Ok(commits)
}

/// Insert commits into the database
fn insert_commits(conn: &Connection, commits: &[GitCommit]) -> Result<usize> {
    let mut count = 0;

    let mut commit_stmt = conn.prepare(
        "INSERT OR REPLACE INTO commits (sha, message, author_name, author_email, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;

    let mut file_stmt = conn.prepare(
        "INSERT OR REPLACE INTO commit_files (sha, file_path, change_type, lines_added, lines_removed) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;

    for commit in commits {
        commit_stmt.execute([
            &commit.sha,
            &commit.message,
            &commit.author_name,
            &commit.author_email,
            &commit.timestamp,
        ])?;

        for file in &commit.files {
            file_stmt.execute(rusqlite::params![
                &commit.sha,
                &file.path,
                &file.change_type,
                file.lines_added,
                file.lines_removed,
            ])?;
        }

        count += 1;
    }

    Ok(count)
}

/// Rebuild co-change relationships from commit_files
fn rebuild_co_changes(conn: &Connection) -> Result<usize> {
    // Clear existing co-changes
    conn.execute("DELETE FROM co_changes", [])?;

    // Build co-change map: for each commit, every pair of files changed together
    let mut co_change_counts: HashMap<(String, String), i32> = HashMap::new();

    let mut stmt = conn.prepare("SELECT sha, file_path FROM commit_files ORDER BY sha")?;
    let mut rows = stmt.query([])?;

    let mut current_sha: Option<String> = None;
    let mut current_files: Vec<String> = Vec::new();

    while let Some(row) = rows.next()? {
        let sha: String = row.get(0)?;
        let file_path: String = row.get(1)?;

        if Some(&sha) != current_sha.as_ref() {
            // Process previous commit's files
            if current_files.len() > 1 {
                for i in 0..current_files.len() {
                    for j in (i + 1)..current_files.len() {
                        let (a, b) = if current_files[i] < current_files[j] {
                            (current_files[i].clone(), current_files[j].clone())
                        } else {
                            (current_files[j].clone(), current_files[i].clone())
                        };
                        *co_change_counts.entry((a, b)).or_insert(0) += 1;
                    }
                }
            }

            current_sha = Some(sha);
            current_files.clear();
        }

        current_files.push(file_path);
    }

    // Process last commit
    if current_files.len() > 1 {
        for i in 0..current_files.len() {
            for j in (i + 1)..current_files.len() {
                let (a, b) = if current_files[i] < current_files[j] {
                    (current_files[i].clone(), current_files[j].clone())
                } else {
                    (current_files[j].clone(), current_files[i].clone())
                };
                *co_change_counts.entry((a, b)).or_insert(0) += 1;
            }
        }
    }

    // Insert co-changes
    let mut insert_stmt =
        conn.prepare("INSERT INTO co_changes (file_a, file_b, count) VALUES (?1, ?2, ?3)")?;

    let count = co_change_counts.len();
    for ((file_a, file_b), cnt) in co_change_counts {
        insert_stmt.execute([&file_a, &file_b, &cnt.to_string()])?;
    }

    Ok(count)
}

/// Get the last scraped SHA from metadata
fn get_last_sha(conn: &Connection) -> Result<Option<String>> {
    let result: Option<String> = conn
        .query_row(
            "SELECT value FROM scrape_meta WHERE key = 'last_sha'",
            [],
            |row| row.get(0),
        )
        .ok();
    Ok(result)
}

/// Update the last scraped SHA
fn update_last_sha(conn: &Connection, sha: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO scrape_meta (key, value) VALUES ('last_sha', ?1)",
        [sha],
    )?;
    Ok(())
}

/// Main entry point for git scraping
pub fn run(full: bool) -> Result<ScrapeStats> {
    let start = Instant::now();
    let db_path = Path::new(DB_PATH);

    let conn = initialize(db_path)?;

    // Get last SHA for incremental scraping
    let since_sha = if full { None } else { get_last_sha(&conn)? };

    if since_sha.is_some() {
        println!("📊 Incremental scrape from last known commit...");
    } else {
        println!("📊 Full git history scrape...");
    }

    // Parse git log
    let commits = parse_git_log(since_sha.as_deref())?;

    if commits.is_empty() {
        println!("  No new commits to process");
        return Ok(ScrapeStats {
            items_processed: 0,
            time_elapsed: start.elapsed(),
            database_size_kb: std::fs::metadata(db_path)
                .map(|m| m.len() / 1024)
                .unwrap_or(0),
        });
    }

    println!("  Found {} commits to process", commits.len());

    // Insert commits
    let commit_count = insert_commits(&conn, &commits)?;
    println!("  Inserted {} commits", commit_count);

    // Update last SHA
    if let Some(latest) = commits.first() {
        update_last_sha(&conn, &latest.sha)?;
    }

    // Rebuild co-changes
    let co_change_count = rebuild_co_changes(&conn)?;
    println!("  Built {} co-change relationships", co_change_count);

    let elapsed = start.elapsed();
    let db_size = std::fs::metadata(db_path)
        .map(|m| m.len() / 1024)
        .unwrap_or(0);

    Ok(ScrapeStats {
        items_processed: commit_count,
        time_elapsed: elapsed,
        database_size_kb: db_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_log_output() {
        // SHAs must be exactly 40 hex chars
        let sample = "abc123def456abc123def456abc123def456abc1|Fix bug in parser|John Doe|john@example.com|2025-01-15T10:30:00+00:00\n5\t2\tsrc/parser.rs\n10\t0\tsrc/new_file.rs\n\ndef456abc123def456abc123def456abc123def4|Add feature|Jane Smith|jane@example.com|2025-01-14T09:00:00+00:00\n20\t5\tsrc/feature.rs";

        let commits = parse_git_log_output(sample).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].sha, "abc123def456abc123def456abc123def456abc1");
        assert_eq!(commits[0].files.len(), 2);
        assert_eq!(commits[0].files[0].path, "src/parser.rs");
        assert_eq!(commits[0].files[0].lines_added, 5);
        assert_eq!(commits[0].files[0].lines_removed, 2);
    }
}
