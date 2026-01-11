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

use anyhow::Result;
use rusqlite::Connection;

use super::ForgeReader;

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
/// Safe to interrupt - progress is saved after each item.
pub fn run(conn: &Connection, reader: &dyn ForgeReader, repo: &str) -> Result<SyncStats> {
    internal::sync_forge(conn, reader, repo)
}

/// Check sync status without making changes.
pub fn status(conn: &Connection, repo: &str) -> Result<SyncStats> {
    internal::get_status(conn, repo)
}
