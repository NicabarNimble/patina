//! Git history scraper - extracts commits, files changed, and co-change relationships
//!
//! Uses unified eventlog pattern:
//! - Inserts git.commit events into eventlog table
//! - Creates materialized views (commits, commit_files, co_changes) from eventlog
//!
//! Phase 1 of forge abstraction: includes conventional commit parsing to extract
//! type, scope, PR references, and issue references from commit messages.

pub mod commits;

use anyhow::{Context, Result};
use chrono;
use rusqlite::Connection;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use super::database;
use super::ScrapeStats;

// ============================================================================
// Session-Commit Linkage (Phase 3)
// ============================================================================

/// Session boundaries derived from git tags
#[derive(Debug)]
struct SessionBounds {
    session_id: String,
    start_time: String,       // ISO8601
    end_time: Option<String>, // None if session still active
}

/// Parse session tags to get session boundaries
///
/// Session tags follow pattern: session-YYYYMMDD-HHMMSS-{start|end}
fn parse_session_tags() -> Result<Vec<SessionBounds>> {
    // Get all session tags with their timestamps
    let output = Command::new("git")
        .args([
            "tag",
            "-l",
            "session-*",
            "--format",
            "%(refname:short)|%(creatordate:iso-strict)",
        ])
        .output()
        .context("Failed to run git tag")?;

    if !output.status.success() {
        // No tags or git error - return empty (commits won't be linked to sessions)
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Group tags by session ID
    let mut sessions: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 2 {
            continue;
        }

        let tag_name = parts[0];
        let timestamp = parts[1].to_string();

        // Parse tag name: session-YYYYMMDD-HHMMSS-{start|end}
        if let Some(rest) = tag_name.strip_prefix("session-") {
            if let Some(session_id) = rest.strip_suffix("-start") {
                let entry = sessions
                    .entry(session_id.to_string())
                    .or_insert((None, None));
                entry.0 = Some(timestamp);
            } else if let Some(session_id) = rest.strip_suffix("-end") {
                let entry = sessions
                    .entry(session_id.to_string())
                    .or_insert((None, None));
                entry.1 = Some(timestamp);
            }
        }
    }

    // Convert to SessionBounds
    let mut bounds: Vec<SessionBounds> = sessions
        .into_iter()
        .filter_map(|(session_id, (start, end))| {
            // Only include sessions with a start time
            start.map(|start_time| SessionBounds {
                session_id,
                start_time,
                end_time: end,
            })
        })
        .collect();

    // Sort by start time
    bounds.sort_by(|a, b| a.start_time.cmp(&b.start_time));

    Ok(bounds)
}

/// Maximum duration (in seconds) for a session without an end tag to be considered "active"
/// Sessions older than this with no end tag are treated as abandoned.
const MAX_SESSION_DURATION_SECS: i64 = 24 * 60 * 60; // 24 hours

/// Find which session a commit belongs to (if any)
fn find_session_for_commit(commit_time: &str, sessions: &[SessionBounds]) -> Option<String> {
    // Iterate in reverse (newest sessions first) for better matching
    for session in sessions.iter().rev() {
        // Check if commit is after session start
        if commit_time >= session.start_time.as_str() {
            match &session.end_time {
                Some(end_time) if commit_time <= end_time.as_str() => {
                    // Commit is within session bounds
                    return Some(session.session_id.clone());
                }
                None => {
                    // Session has no end tag - check if it's reasonably recent
                    // Parse timestamps to check duration
                    if let (Ok(commit_dt), Ok(start_dt)) = (
                        chrono::DateTime::parse_from_rfc3339(commit_time),
                        chrono::DateTime::parse_from_rfc3339(&session.start_time),
                    ) {
                        let duration = commit_dt.signed_duration_since(start_dt);
                        if duration.num_seconds() >= 0
                            && duration.num_seconds() <= MAX_SESSION_DURATION_SECS
                        {
                            return Some(session.session_id.clone());
                        }
                    }
                }
                _ => continue, // Commit is after session end, check next
            }
        }
    }
    None
}

/// Parsed commit from git log
#[derive(Debug)]
struct GitCommit {
    sha: String,
    message: String,
    author_name: String,
    author_email: String,
    timestamp: String,
    files: Vec<FileChange>,
    session_id: Option<String>, // Phase 3: Link commits to sessions
}

/// File change within a commit
#[derive(Debug)]
struct FileChange {
    path: String,
    change_type: String,
    lines_added: i32,
    lines_removed: i32,
}

/// Create materialized views for git events
///
/// Views are derived from eventlog WHERE event_type = 'git.commit'
fn create_materialized_views(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- Commits view (materialized from git.commit events)
        CREATE TABLE IF NOT EXISTS commits (
            sha TEXT PRIMARY KEY,
            message TEXT,
            author_name TEXT,
            author_email TEXT,
            timestamp TEXT,
            branch TEXT
        );

        -- Files changed per commit (from git.commit event data)
        CREATE TABLE IF NOT EXISTS commit_files (
            sha TEXT,
            file_path TEXT,
            change_type TEXT,
            lines_added INTEGER,
            lines_removed INTEGER,
            PRIMARY KEY (sha, file_path)
        );

        -- Co-change relationships (derived from commit_files)
        CREATE TABLE IF NOT EXISTS co_changes (
            file_a TEXT,
            file_b TEXT,
            count INTEGER,
            PRIMARY KEY (file_a, file_b)
        );

        -- Indexes for common queries
        CREATE INDEX IF NOT EXISTS idx_commits_timestamp ON commits(timestamp);
        CREATE INDEX IF NOT EXISTS idx_commits_author ON commits(author_email);
        CREATE INDEX IF NOT EXISTS idx_commit_files_path ON commit_files(file_path);
        CREATE INDEX IF NOT EXISTS idx_co_changes_count ON co_changes(count DESC);
        "#,
    )?;

    Ok(())
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
                session_id: None, // Will be populated later from session tags
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

/// Insert commits into eventlog and materialized views
///
/// Dual-write pattern (project repos):
/// 1. Insert git.commit event into eventlog (source of truth)
/// 2. Update materialized views (commits, commit_files) for fast queries
///
/// Direct-write pattern (ref repos, skip_eventlog=true):
/// - Skip eventlog (git IS the source of truth)
/// - Only update materialized views for queries
///
/// See: layer/surface/build/spec-ref-repo-storage.md
fn insert_commits(conn: &Connection, commits: &[GitCommit], skip_eventlog: bool) -> Result<usize> {
    let mut count = 0;

    let mut commit_stmt = conn.prepare(
        "INSERT OR REPLACE INTO commits (sha, message, author_name, author_email, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;

    let mut file_stmt = conn.prepare(
        "INSERT OR REPLACE INTO commit_files (sha, file_path, change_type, lines_added, lines_removed) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;

    for commit in commits {
        // Parse conventional commit format (Phase 1 forge abstraction)
        let parsed = commits::parse_conventional(&commit.message);

        // 1. Insert into eventlog (source of truth)
        // Phase 3: Include session_id for feedback loop
        let mut event_data = json!({
            "sha": &commit.sha,
            "message": &commit.message,
            "author_name": &commit.author_name,
            "author_email": &commit.author_email,
            "files": commit.files.iter().map(|f| json!({
                "path": &f.path,
                "change_type": &f.change_type,
                "lines_added": f.lines_added,
                "lines_removed": f.lines_removed,
            })).collect::<Vec<_>>(),
        });

        // Add parsed conventional commit fields if present
        if parsed.has_structure() {
            event_data["parsed"] = json!({
                "type": &parsed.commit_type,
                "scope": &parsed.scope,
                "breaking": parsed.breaking,
                "pr_ref": parsed.pr_ref,
                "issue_refs": &parsed.issue_refs,
            });
        }

        // Add session_id if commit was made during a session
        if let Some(ref session_id) = commit.session_id {
            event_data["session_id"] = json!(session_id);
        }

        // 1. Insert into eventlog (skip for ref repos - git IS the source)
        if !skip_eventlog {
            database::insert_event(
                conn,
                "git.commit",
                &commit.timestamp,
                &commit.sha,
                None, // source_file not applicable for commits
                &event_data.to_string(),
            )?;
        }

        // 2. Update materialized view (for fast queries)
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

/// Maximum files per commit to consider for co-change analysis
/// Commits with more files are skipped (likely bulk operations, not meaningful co-changes)
const MAX_FILES_PER_COMMIT: usize = 50;

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
    let mut skipped_commits = 0;

    while let Some(row) = rows.next()? {
        let sha: String = row.get(0)?;
        let file_path: String = row.get(1)?;

        if Some(&sha) != current_sha.as_ref() {
            // Process previous commit's files (skip if too many files)
            if current_files.len() > 1 && current_files.len() <= MAX_FILES_PER_COMMIT {
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
            } else if current_files.len() > MAX_FILES_PER_COMMIT {
                skipped_commits += 1;
            }

            current_sha = Some(sha);
            current_files.clear();
        }

        current_files.push(file_path);
    }

    // Process last commit (with same size limit)
    if current_files.len() > 1 && current_files.len() <= MAX_FILES_PER_COMMIT {
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
    } else if current_files.len() > MAX_FILES_PER_COMMIT {
        skipped_commits += 1;
    }

    if skipped_commits > 0 {
        println!(
            "  Skipped {} commits with >{} files",
            skipped_commits, MAX_FILES_PER_COMMIT
        );
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

/// Get the last scraped SHA from metadata (uses unified database module)
fn get_last_sha(conn: &Connection) -> Result<Option<String>> {
    database::get_last_processed(conn, "git")
}

/// Update the last scraped SHA (uses unified database module)
fn update_last_sha(conn: &Connection, sha: &str) -> Result<()> {
    database::set_last_processed(conn, "git", sha)
}

/// Check if this is a shallow clone (has .git/shallow file)
fn is_shallow_clone() -> bool {
    Path::new(".git/shallow").exists()
}

/// Main entry point for git scraping
pub fn run(full: bool) -> Result<ScrapeStats> {
    let start = Instant::now();
    let db_path = Path::new(database::PATINA_DB);

    // Ref repos use lean storage - skip eventlog for git data
    // Git IS the source of truth, no need to duplicate in eventlog
    let skip_eventlog = database::is_ref_repo(db_path);

    // Check for shallow clone - skip co-change analysis
    if is_shallow_clone() {
        println!("⚠️  Shallow clone detected - skipping git history analysis");
        println!("   (Co-change analysis requires full git history)");
        return Ok(ScrapeStats {
            items_processed: 0,
            time_elapsed: start.elapsed(),
            database_size_kb: std::fs::metadata(db_path)
                .map(|m| m.len() / 1024)
                .unwrap_or(0),
        });
    }

    // Initialize unified database with eventlog
    let conn = database::initialize(db_path)?;

    // Create materialized views for git events
    create_materialized_views(&conn)?;

    // Get last SHA for incremental scraping
    let since_sha = if full { None } else { get_last_sha(&conn)? };

    if since_sha.is_some() {
        println!("📊 Incremental scrape from last known commit...");
    } else {
        println!("📊 Full git history scrape...");
    }

    // Parse git log
    let mut commits = parse_git_log(since_sha.as_deref())?;

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

    // Phase 3: Parse session tags and link commits to sessions
    let session_bounds = parse_session_tags().unwrap_or_default();
    if !session_bounds.is_empty() {
        let mut linked_count = 0;
        for commit in &mut commits {
            if let Some(session_id) = find_session_for_commit(&commit.timestamp, &session_bounds) {
                commit.session_id = Some(session_id);
                linked_count += 1;
            }
        }
        if linked_count > 0 {
            println!("  Linked {} commits to sessions", linked_count);
        }
    }

    // Count commits with conventional format (Phase 1 measurement)
    let conventional_count = commits
        .iter()
        .filter(|c| commits::parse_conventional(&c.message).has_structure())
        .count();
    let pr_ref_count = commits
        .iter()
        .filter(|c| commits::parse_conventional(&c.message).pr_ref.is_some())
        .count();

    // Insert commits (skip eventlog for ref repos - git IS the source)
    let commit_count = insert_commits(&conn, &commits, skip_eventlog)?;
    if skip_eventlog {
        println!("  Inserted {} commits (direct, no eventlog)", commit_count);
    } else {
        println!("  Inserted {} commits", commit_count);
    }

    // Report conventional commit stats
    if conventional_count > 0 {
        let pct = (conventional_count * 100) / commits.len();
        println!(
            "  Parsed {} conventional commits ({}%), {} with PR refs",
            conventional_count, pct, pr_ref_count
        );
    }

    // Update last SHA
    if let Some(latest) = commits.first() {
        update_last_sha(&conn, &latest.sha)?;
    }

    // Rebuild co-changes
    let co_change_count = rebuild_co_changes(&conn)?;
    println!("  Built {} co-change relationships", co_change_count);

    // Populate commits FTS5 index for narrative search
    let fts_count = database::populate_commits_fts5(&conn)?;
    println!("  Indexed {} commit messages for search", fts_count);

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
        assert!(commits[0].session_id.is_none()); // Not linked yet
    }

    #[test]
    fn test_find_session_for_commit() {
        let sessions = vec![
            SessionBounds {
                session_id: "20251217-100000".to_string(),
                start_time: "2025-12-17T10:00:00+00:00".to_string(),
                end_time: Some("2025-12-17T12:00:00+00:00".to_string()),
            },
            SessionBounds {
                session_id: "20251217-140000".to_string(),
                start_time: "2025-12-17T14:00:00+00:00".to_string(),
                end_time: None, // Still active (within 24h)
            },
        ];

        // Commit during first session
        assert_eq!(
            find_session_for_commit("2025-12-17T11:00:00+00:00", &sessions),
            Some("20251217-100000".to_string())
        );

        // Commit between sessions (no match)
        assert_eq!(
            find_session_for_commit("2025-12-17T13:00:00+00:00", &sessions),
            None
        );

        // Commit during second (active) session - within 24h of start
        assert_eq!(
            find_session_for_commit("2025-12-17T15:00:00+00:00", &sessions),
            Some("20251217-140000".to_string())
        );

        // Commit before any session
        assert_eq!(
            find_session_for_commit("2025-12-17T09:00:00+00:00", &sessions),
            None
        );
    }

    #[test]
    fn test_find_session_for_commit_abandoned_session() {
        // Session started long ago with no end tag (abandoned)
        let sessions = vec![SessionBounds {
            session_id: "20250801-100000".to_string(),
            start_time: "2025-08-01T10:00:00+00:00".to_string(),
            end_time: None, // No end tag
        }];

        // Commit from months later should NOT match (session is abandoned)
        assert_eq!(
            find_session_for_commit("2025-12-17T11:00:00+00:00", &sessions),
            None
        );

        // Commit within 24h of session start should match
        assert_eq!(
            find_session_for_commit("2025-08-01T20:00:00+00:00", &sessions),
            Some("20250801-100000".to_string())
        );
    }

    #[test]
    fn test_chrono_parsing_real_timestamps() {
        // Test with actual timestamps from git log
        let commit_time = "2025-12-17T08:52:56-05:00";
        let session_start = "2025-08-19T10:51:24-04:00";

        let commit_dt = chrono::DateTime::parse_from_rfc3339(commit_time);
        let start_dt = chrono::DateTime::parse_from_rfc3339(session_start);

        assert!(commit_dt.is_ok(), "Failed to parse commit time");
        assert!(start_dt.is_ok(), "Failed to parse session start time");

        let duration = commit_dt.unwrap().signed_duration_since(start_dt.unwrap());

        // Should be ~120 days apart, way more than 24 hours
        assert!(
            duration.num_days() > 100,
            "Duration should be months, got {} days",
            duration.num_days()
        );
        assert!(
            duration.num_seconds() > MAX_SESSION_DURATION_SECS,
            "Duration should exceed 24h limit"
        );
    }
}
