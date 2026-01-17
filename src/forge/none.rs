//! Null implementation for repos without a forge.
//!
//! Returns empty results for list operations, errors for specific fetches.
//! Simple enough - no internal.rs needed.

use anyhow::{bail, Result};

use super::{ForgeReader, Issue, PullRequest};

/// Null ForgeReader for repos without a forge connection.
pub struct NoneReader;

impl ForgeReader for NoneReader {
    fn get_issue_count(&self) -> Result<usize> {
        Ok(0) // No forge = no issues
    }

    fn get_pr_count(&self) -> Result<usize> {
        Ok(0) // No forge = no PRs
    }

    fn list_issues(&self, _limit: usize, _since: Option<&str>) -> Result<Vec<Issue>> {
        Ok(vec![]) // No forge = no issues
    }

    fn list_pull_requests(&self, _limit: usize, _since: Option<&str>) -> Result<Vec<PullRequest>> {
        Ok(vec![]) // No forge = no PRs
    }

    fn get_pull_request(&self, number: i64) -> Result<PullRequest> {
        bail!("No forge configured, cannot fetch PR #{}", number)
    }

    fn get_issue(&self, number: i64) -> Result<Issue> {
        bail!("No forge configured, cannot fetch issue #{}", number)
    }

    fn get_max_issue_number(&self) -> Result<i64> {
        Ok(0) // No forge = no issues
    }
}
