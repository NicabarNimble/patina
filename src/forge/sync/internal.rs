//! Internal implementation for forge sync.
//!
//! Contains pacing, backlog management, and resolution logic.
//! Not exposed in public interface.

use std::fs;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{bail, Result};
use rusqlite::Connection;

use crate::forge::{ForgeReader, Issue, IssueState, PrState, PullRequest};

use super::SyncStats;

// ============================================================================
// Constants - visible, not configurable
// ============================================================================

/// Delay between API requests.
///
/// GitHub allows 5,000/hour. At 750ms we do 4,800/hour max.
/// Conservative. Never hits limits. Works forever.
///
/// Eskil: "No adaptive logic. No rate limit API calls. Just works."
const DELAY_BETWEEN_REQUESTS: Duration = Duration::from_millis(750);

/// Maximum refs to resolve per sync run. Keeps each run bounded.
/// At 750ms delay, 50 refs = ~37 seconds per batch.
const BATCH_SIZE: usize = 50;

// ============================================================================
// PID file infrastructure - prevents multiple syncs per repo
// ============================================================================

/// Get path to PID file for a repo.
fn pid_file_path(repo: &str) -> PathBuf {
    let safe_name = repo.replace('/', "-");
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(format!(".patina/run/forge-sync-{}.pid", safe_name))
}

/// Get path to log file for a repo.
pub(crate) fn log_file_path(repo: &str) -> PathBuf {
    let safe_name = repo.replace('/', "-");
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(format!(".patina/logs/forge-sync-{}.log", safe_name))
}

/// Check if a process is running by PID.
fn process_is_running(pid: u32) -> bool {
    // Unix: kill -0 checks if process exists without sending signal
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as i32, 0) == 0
    }
    #[cfg(not(unix))]
    {
        // On non-Unix, assume not running (conservative)
        false
    }
}

/// RAII guard that manages PID file lifecycle.
/// Cleans up PID file on drop.
pub(crate) struct SyncGuard {
    pid_file: PathBuf,
}

impl Drop for SyncGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.pid_file);
    }
}

/// Check if we can start sync for a repo. Returns guard if yes.
/// Fails if another sync is already running.
pub(crate) fn can_start_sync(repo: &str) -> Result<SyncGuard> {
    let pid_file = pid_file_path(repo);

    // Ensure directory exists
    if let Some(parent) = pid_file.parent() {
        fs::create_dir_all(parent)?;
    }

    if pid_file.exists() {
        let content = fs::read_to_string(&pid_file)?;
        if let Ok(pid) = content.trim().parse::<u32>() {
            if process_is_running(pid) {
                bail!("Already syncing (PID {}). Check: --status", pid);
            }
        }
        // Stale PID file - process died, clean up
        fs::remove_file(&pid_file)?;
    }

    Ok(SyncGuard { pid_file })
}

/// Check if sync is currently running for a repo.
pub(crate) fn is_sync_running(repo: &str) -> Option<u32> {
    let pid_file = pid_file_path(repo);
    if pid_file.exists() {
        if let Ok(content) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                if process_is_running(pid) {
                    return Some(pid);
                }
            }
        }
        // Stale PID file - clean up
        let _ = fs::remove_file(&pid_file);
    }
    None
}

// ============================================================================
// Public functions (called by mod.rs)
// ============================================================================

/// Main sync entry point.
pub(crate) fn sync_forge(
    conn: &Connection,
    reader: &dyn ForgeReader,
    repo: &str,
) -> Result<SyncStats> {
    // Step 1a: Discover PR refs from commits (instant, local)
    let pr_discovered = discover_refs(conn, repo)?;

    // Step 1b: Discover all issues (one API call to get max number)
    let issue_discovered = discover_all_issues(conn, reader, repo)?;

    let discovered = pr_discovered + issue_discovered;

    // Step 2: Get pending refs to resolve (newest first - walk-back pattern)
    let pending_refs = get_pending_refs(conn, repo, BATCH_SIZE)?;
    let total_pending = count_pending_refs(conn, repo)?;

    if pending_refs.is_empty() {
        return Ok(SyncStats {
            discovered,
            resolved: 0,
            pending: 0,
            errors: 0,
            cache_hits: 0,
        });
    }

    println!(
        "  Forge sync: {} pending refs, processing batch of {}",
        total_pending,
        pending_refs.len()
    );

    // Step 3: Resolve with pacing
    let mut resolved = 0;
    let mut errors = 0;
    let mut cache_hits = 0;

    for ref_num in &pending_refs {
        // Always wait - simple, correct, unbreakable
        sleep(DELAY_BETWEEN_REQUESTS);

        match resolve_ref(conn, reader, repo, *ref_num) {
            Ok(was_cached) => {
                resolved += 1;
                if was_cached {
                    cache_hits += 1;
                }
                // Progress saved immediately - safe to interrupt
            }
            Err(e) => {
                errors += 1;
                eprintln!("  ⚠️  #{}: {}", ref_num, e);
                // Error recorded in DB - won't retry forever
            }
        }
    }

    Ok(SyncStats {
        discovered,
        resolved,
        pending: total_pending - resolved,
        errors,
        cache_hits,
    })
}

/// Get sync status without making changes.
pub(crate) fn get_status(conn: &Connection, repo: &str) -> Result<SyncStats> {
    let pending = count_pending_refs(conn, repo)?;
    let resolved = count_resolved_refs(conn, repo)?;
    let errors = count_failed_refs(conn, repo)?;

    Ok(SyncStats {
        discovered: 0,
        resolved,
        pending,
        errors,
        cache_hits: 0,
    })
}

/// Drain the backlog - keep syncing until all refs are resolved.
///
/// Respects rate limiting (750ms between API calls). Each batch
/// is bounded by BATCH_SIZE. Prints progress between batches.
pub(crate) fn drain_forge(
    conn: &Connection,
    reader: &dyn ForgeReader,
    repo: &str,
) -> Result<SyncStats> {
    let mut total = SyncStats::default();
    let mut batch_num = 0;

    loop {
        batch_num += 1;
        let stats = sync_forge(conn, reader, repo)?;

        total.discovered += stats.discovered;
        total.resolved += stats.resolved;
        total.errors += stats.errors;
        total.cache_hits += stats.cache_hits;
        total.pending = stats.pending;

        if stats.pending == 0 {
            break;
        }

        if stats.resolved == 0 && stats.errors == 0 {
            // No progress made - avoid infinite loop
            break;
        }

        println!(
            "  Batch {} complete. {} remaining...",
            batch_num, stats.pending
        );
    }

    Ok(total)
}

/// Sync with a limit - resolve up to N refs then stop.
/// Returns stats including how many remain.
pub(crate) fn sync_with_limit(
    conn: &Connection,
    reader: &dyn ForgeReader,
    repo: &str,
    limit: usize,
) -> Result<SyncStats> {
    let mut total = SyncStats::default();
    let mut resolved_count = 0;

    // Discover refs first
    let pr_discovered = discover_refs(conn, repo)?;
    let issue_discovered = discover_all_issues(conn, reader, repo)?;
    total.discovered = pr_discovered + issue_discovered;

    // Resolve in batches up to limit
    while resolved_count < limit {
        let batch_limit = std::cmp::min(BATCH_SIZE, limit - resolved_count);
        let pending_refs = get_pending_refs(conn, repo, batch_limit)?;

        if pending_refs.is_empty() {
            break;
        }

        for ref_num in &pending_refs {
            sleep(DELAY_BETWEEN_REQUESTS);

            match resolve_ref(conn, reader, repo, *ref_num) {
                Ok(was_cached) => {
                    total.resolved += 1;
                    resolved_count += 1;
                    if was_cached {
                        total.cache_hits += 1;
                    }
                }
                Err(e) => {
                    total.errors += 1;
                    eprintln!("  ⚠️  #{}: {}", ref_num, e);
                }
            }
        }
    }

    total.pending = count_pending_refs(conn, repo)?;
    Ok(total)
}

// ============================================================================
// Background sync - fork to detached process
// ============================================================================

/// Start background sync process.
///
/// Forks to background, writes PID file, syncs all pending refs.
/// Parent returns immediately. Child runs until complete or error.
#[cfg(unix)]
pub(crate) fn start_background_sync(
    db_path: &std::path::Path,
    repo: &str,
    detected: &crate::forge::Forge,
) -> Result<u32> {
    use std::os::unix::io::AsRawFd;

    // Check if already running
    let _guard = can_start_sync(repo)?;

    let log_path = log_file_path(repo);
    let pid_path = pid_file_path(repo);

    // Ensure log directory exists
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Fork
    match unsafe { libc::fork() } {
        -1 => bail!("Fork failed: {}", std::io::Error::last_os_error()),
        0 => {
            // === Child process ===

            // Detach from terminal (new session)
            unsafe { libc::setsid() };

            // Open log file for stdout/stderr redirection
            let log_file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
                .expect("Failed to open log file");

            let log_fd = log_file.as_raw_fd();

            // Redirect stdout and stderr to log file
            unsafe {
                libc::dup2(log_fd, libc::STDOUT_FILENO);
                libc::dup2(log_fd, libc::STDERR_FILENO);
            }

            // Write PID file
            if let Some(parent) = pid_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&pid_path, std::process::id().to_string());

            // Log start
            println!(
                "[{}] Background sync started for {}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                repo
            );

            // Do the work - reopen db and create reader
            let result = (|| -> Result<SyncStats> {
                let conn = rusqlite::Connection::open(db_path)?;
                let reader = crate::forge::reader(detected);
                drain_forge(&conn, reader.as_ref(), repo)
            })();

            // Log result
            match &result {
                Ok(stats) => {
                    println!(
                        "[{}] Sync complete: {} resolved, {} errors, {} pending",
                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                        stats.resolved,
                        stats.errors,
                        stats.pending
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[{}] Sync failed: {}",
                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                        e
                    );
                }
            }

            // Clean up PID file
            let _ = fs::remove_file(&pid_path);

            std::process::exit(if result.is_ok() { 0 } else { 1 });
        }
        child_pid => {
            // === Parent process ===
            Ok(child_pid as u32)
        }
    }
}

#[cfg(not(unix))]
pub(crate) fn start_background_sync(
    _db_path: &std::path::Path,
    _repo: &str,
    _detected: &crate::forge::Forge,
) -> Result<u32> {
    bail!("Background sync not supported on this platform. Use --limit instead.")
}

// ============================================================================
// Discovery - extract refs from commits
// ============================================================================

/// Find #N patterns in commit messages not already in forge_refs.
fn discover_refs(conn: &Connection, repo: &str) -> Result<usize> {
    let count = conn.execute(
        r#"
        INSERT OR IGNORE INTO forge_refs (repo, ref_number, discovered, source)
        SELECT
            ?1 as repo,
            CAST(json_extract(data, '$.parsed.pr_ref') AS INTEGER) as ref_number,
            datetime('now') as discovered,
            json_extract(data, '$.hash') as source
        FROM eventlog
        WHERE event_type = 'git.commit'
          AND json_extract(data, '$.parsed.pr_ref') IS NOT NULL
        "#,
        rusqlite::params![repo],
    )?;

    Ok(count)
}

/// Discover all issues by populating forge_refs with 1..max_issue_number.
/// Uses INSERT OR IGNORE - safe to call repeatedly.
fn discover_all_issues(conn: &Connection, reader: &dyn ForgeReader, repo: &str) -> Result<usize> {
    // Get max issue number from API (one call)
    let max_num = reader.get_max_issue_number()?;
    if max_num == 0 {
        return Ok(0);
    }

    // Count how many issue refs we already have
    let existing: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM forge_refs WHERE repo = ?1 AND ref_kind = 'issue'",
            rusqlite::params![repo],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // If we already have all issue refs, skip
    if existing >= max_num {
        return Ok(0);
    }

    // Insert all issue numbers 1..max_num (INSERT OR IGNORE handles duplicates)
    // Do it in a transaction for speed
    conn.execute("BEGIN TRANSACTION", [])?;

    let mut count = 0;
    for num in 1..=max_num {
        let inserted = conn.execute(
            "INSERT OR IGNORE INTO forge_refs (repo, ref_number, ref_kind, discovered)
             VALUES (?1, ?2, 'issue', datetime('now'))",
            rusqlite::params![repo, num],
        )?;
        count += inserted;
    }

    conn.execute("COMMIT", [])?;

    if count > 0 {
        println!("  Discovered {} issue refs (1..{})", count, max_num);
    }

    Ok(count)
}

// ============================================================================
// Backlog - get pending refs, newest first
// ============================================================================

/// Get pending refs to resolve (newest first for walk-back pattern).
fn get_pending_refs(conn: &Connection, repo: &str, limit: usize) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT ref_number
        FROM forge_refs
        WHERE repo = ?1 AND resolved IS NULL
        ORDER BY discovered DESC
        LIMIT ?2
        "#,
    )?;

    let refs: Vec<i64> = stmt
        .query_map(rusqlite::params![repo, limit], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(refs)
}

/// Count total pending refs.
fn count_pending_refs(conn: &Connection, repo: &str) -> Result<usize> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM forge_refs WHERE repo = ?1 AND resolved IS NULL",
        rusqlite::params![repo],
        |row| row.get(0),
    )?;
    Ok(count as usize)
}

/// Count resolved refs.
fn count_resolved_refs(conn: &Connection, repo: &str) -> Result<usize> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM forge_refs WHERE repo = ?1 AND resolved IS NOT NULL AND error IS NULL",
        rusqlite::params![repo],
        |row| row.get(0),
    )?;
    Ok(count as usize)
}

/// Count failed refs.
fn count_failed_refs(conn: &Connection, repo: &str) -> Result<usize> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM forge_refs WHERE repo = ?1 AND error IS NOT NULL",
        rusqlite::params![repo],
        |row| row.get(0),
    )?;
    Ok(count as usize)
}

// ============================================================================
// Resolution - fetch from API with pacing
// ============================================================================

/// Resolve a single ref. Returns true if it was a cache hit (no API call).
fn resolve_ref(
    conn: &Connection,
    reader: &dyn ForgeReader,
    repo: &str,
    ref_num: i64,
) -> Result<bool> {
    // Check if it's a known issue first (no API call needed)
    if is_known_issue(conn, ref_num)? {
        mark_resolved(conn, repo, ref_num, "issue")?;
        return Ok(true); // Cache hit
    }

    // Check if it's a known PR (no API call needed)
    if is_known_pr(conn, ref_num)? {
        mark_resolved(conn, repo, ref_num, "pr")?;
        return Ok(true); // Cache hit
    }

    // Try as PR first (more common in commit refs like "Merge PR #123")
    match reader.get_pull_request(ref_num) {
        Ok(pr) => {
            insert_pr(conn, &pr)?;
            mark_resolved(conn, repo, ref_num, "pr")?;
            return Ok(false);
        }
        Err(_) => {
            // Not a PR - might be an issue
        }
    }

    // Try as issue
    match reader.get_issue(ref_num) {
        Ok(issue) => {
            insert_issue(conn, &issue)?;
            mark_resolved(conn, repo, ref_num, "issue")?;
            Ok(false)
        }
        Err(e) => {
            // Neither PR nor issue - record error, move on
            mark_failed(conn, repo, ref_num, &e.to_string())?;
            anyhow::bail!("#{} is neither PR nor issue: {}", ref_num, e)
        }
    }
}

/// Check if ref is already in forge_issues table.
fn is_known_issue(conn: &Connection, ref_num: i64) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM forge_issues WHERE number = ?1",
        rusqlite::params![ref_num],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Check if ref is already in forge_prs table.
fn is_known_pr(conn: &Connection, ref_num: i64) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM forge_prs WHERE number = ?1",
        rusqlite::params![ref_num],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Mark ref as successfully resolved.
fn mark_resolved(conn: &Connection, repo: &str, ref_num: i64, kind: &str) -> Result<()> {
    conn.execute(
        r#"
        UPDATE forge_refs
        SET resolved = datetime('now'), ref_kind = ?3, error = NULL
        WHERE repo = ?1 AND ref_number = ?2
        "#,
        rusqlite::params![repo, ref_num, kind],
    )?;
    Ok(())
}

/// Mark ref as failed.
fn mark_failed(conn: &Connection, repo: &str, ref_num: i64, error: &str) -> Result<()> {
    conn.execute(
        r#"
        UPDATE forge_refs
        SET error = ?3
        WHERE repo = ?1 AND ref_number = ?2
        "#,
        rusqlite::params![repo, ref_num, error],
    )?;
    Ok(())
}

// ============================================================================
// Database insertion - store fetched PRs/issues
// ============================================================================

/// Insert a PR into forge_prs table.
fn insert_pr(conn: &Connection, pr: &PullRequest) -> Result<()> {
    let labels_json = serde_json::to_string(&pr.labels)?;
    let linked_json = serde_json::to_string(&pr.linked_issues)?;
    let state_str = match pr.state {
        PrState::Open => "open",
        PrState::Merged => "merged",
        PrState::Closed => "closed",
    };

    conn.execute(
        r#"
        INSERT OR REPLACE INTO forge_prs
        (number, title, body, state, labels, author, created_at, merged_at, url, linked_issues, approvals)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
        rusqlite::params![
            pr.number,
            &pr.title,
            &pr.body,
            state_str,
            &labels_json,
            &pr.author,
            &pr.created_at,
            &pr.merged_at,
            &pr.url,
            &linked_json,
            pr.approvals,
        ],
    )?;
    Ok(())
}

/// Insert an issue into forge_issues table.
fn insert_issue(conn: &Connection, issue: &Issue) -> Result<()> {
    let labels_json = serde_json::to_string(&issue.labels)?;
    let state_str = match issue.state {
        IssueState::Open => "open",
        IssueState::Closed => "closed",
    };

    conn.execute(
        r#"
        INSERT OR REPLACE INTO forge_issues
        (number, title, body, state, labels, author, created_at, updated_at, url)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        rusqlite::params![
            issue.number,
            &issue.title,
            &issue.body,
            state_str,
            &labels_json,
            &issue.author,
            &issue.created_at,
            &issue.updated_at,
            &issue.url,
        ],
    )?;
    Ok(())
}
