//! Null implementation for repos without a forge.
//!
//! Returns empty results for list operations, errors for specific fetches.
//! Simple enough - no internal.rs needed.

use anyhow::{bail, Result};

use super::{ForgeReader, Issue, PullRequest};

/// Null ForgeReader for repos without a forge connection.
pub struct NoneReader;

impl ForgeReader for NoneReader {
    fn list_issues(&self, _limit: usize, _since: Option<&str>) -> Result<Vec<Issue>> {
        Ok(vec![]) // No forge = no issues
    }

    fn list_pull_requests(&self, _limit: usize, _since: Option<&str>) -> Result<Vec<PullRequest>> {
        Ok(vec![]) // No forge = no PRs
    }

    fn get_pull_request(&self, number: i64) -> Result<PullRequest> {
        // Can't fetch what doesn't exist
        bail!("No forge configured, cannot fetch PR #{}", number)
    }
}
