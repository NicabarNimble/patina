//! GitHub ForgeReader implementation.
//!
//! "Do X": Fetch code review data from GitHub.
//!
//! Uses `gh` CLI for authentication, pagination, and rate limiting.
//! All CLI interaction hidden in internal.rs.

mod internal;

use anyhow::Result;

use super::{Forge, ForgeReader, Issue, PullRequest};

/// GitHub implementation of ForgeReader.
pub struct GitHubReader {
    owner: String,
    repo: String,
}

impl GitHubReader {
    pub fn new(forge: &Forge) -> Self {
        Self {
            owner: forge.owner.clone(),
            repo: forge.repo.clone(),
        }
    }

    fn repo_spec(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

impl ForgeReader for GitHubReader {
    fn list_issues(&self, limit: usize, since: Option<&str>) -> Result<Vec<Issue>> {
        internal::fetch_issues(&self.repo_spec(), limit, since)
    }

    fn list_pull_requests(&self, limit: usize, since: Option<&str>) -> Result<Vec<PullRequest>> {
        internal::fetch_pull_requests(&self.repo_spec(), limit, since)
    }

    fn get_pull_request(&self, number: i64) -> Result<PullRequest> {
        internal::fetch_pull_request(&self.repo_spec(), number)
    }
}

/// Check if `gh` CLI is authenticated.
pub fn is_authenticated() -> Result<bool> {
    internal::check_gh_auth()
}
