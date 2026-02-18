//! Forge sync engine - incremental resolution with rate limiting.
//!
//! "Do X": Sync forge data incrementally with pacing to avoid rate limits.
//!
//! Discovers #N refs from commits, resolves them via API with delays.
//! Safe to interrupt - progress saved after each item.
//!
//! # Example
//!
//! ```ignore
//! use patina::forge::sync;
//!
//! let stats = sync::run(&conn, &reader, "owner/repo")?;
//! println!("Resolved {}, {} pending", stats.resolved, stats.pending);
//! ```

mod internal;

use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

use super::{Forge, ForgeReader};

/// Stats returned from sync operation.
#[derive(Debug, Default)]
pub struct SyncStats {
    pub discovered: usize,
    pub resolved: usize,
    pub pending: usize,
    pub errors: usize,
    pub cache_hits: usize,
}

/// Sync forge data incrementally with rate limiting.
///
/// Discovers refs from commits, resolves them via API with pacing.
/// Resolved refs are written to staging files (`.forge-issue`/`.forge-pr`)
/// instead of direct DB insert — they flow through the pipeline on next `patina scrape`.
/// Safe to interrupt - progress is saved after each item.
pub fn run(
    conn: &Connection,
    reader: &dyn ForgeReader,
    repo: &str,
    staging_dir: &Path,
) -> Result<SyncStats> {
    internal::sync_forge(conn, reader, repo, staging_dir)
}

/// Check sync status without making changes.
pub fn status(conn: &Connection, repo: &str) -> Result<SyncStats> {
    internal::get_status(conn, repo)
}

/// Sync with a limit - resolve up to N refs then stop.
///
/// Useful for foreground sync when you want bounded execution.
/// Still respects rate limiting (750ms between API calls).
pub fn sync_limited(
    conn: &Connection,
    reader: &dyn ForgeReader,
    repo: &str,
    limit: usize,
    staging_dir: &Path,
) -> Result<SyncStats> {
    internal::sync_with_limit(conn, reader, repo, limit, staging_dir)
}

/// Start background sync process (fork to detached process).
///
/// Returns immediately with the child PID. Sync runs in background.
/// Use `is_running()` and `status()` to check progress.
/// Log output goes to `~/.patina/logs/forge-sync-{repo}.log`.
pub fn start_background(
    db_path: &Path,
    repo: &str,
    detected: &Forge,
    staging_dir: &Path,
) -> Result<u32> {
    internal::start_background_sync(db_path, repo, detected, staging_dir)
}

/// Check if sync is currently running for a repo.
/// Returns the PID if running, None otherwise.
pub fn is_running(repo: &str) -> Option<u32> {
    internal::is_sync_running(repo)
}

/// Get path to log file for a repo.
pub fn log_path(repo: &str) -> std::path::PathBuf {
    internal::log_file_path(repo)
}
