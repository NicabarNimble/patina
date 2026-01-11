//! Internal implementation for forge sync.
//!
//! Contains pacing, backlog management, and resolution logic.
//! Not exposed in public interface.

use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use rusqlite::Connection;

use crate::forge::{ForgeReader, Issue, IssueState, PrState, PullRequest};

use super::SyncStats;

// ============================================================================
// Constants - visible, not configurable
// ============================================================================

/// Delay between API requests. GitHub recommends 1000ms for mutations,
/// we use 500ms for reads. Conservative but not glacial.
const DELAY_BETWEEN_REQUESTS: Duration = Duration::from_millis(500);

/// Maximum refs to resolve per sync run. Keeps each run bounded.
/// At 500ms delay, 50 refs = ~25 seconds.
const BATCH_SIZE: usize = 50;

// ============================================================================
// Public functions (called by mod.rs)
// ============================================================================

/// Main sync entry point.
pub(crate) fn sync_forge(
    conn: &Connection,
    reader: &dyn ForgeReader,
    repo: &str,
) -> Result<SyncStats> {
    // Step 1: Discover new refs (instant, local)
    let discovered = discover_refs(conn, repo)?;

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
