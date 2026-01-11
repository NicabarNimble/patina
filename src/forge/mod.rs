//! Forge abstraction for code review data.
//!
//! "Do X": Fetch code review data from a forge platform.
//!
//! This module provides a trait-based abstraction over different forge platforms
//! (GitHub, Gitea/Codeberg, etc.) for read-only access to issues and PRs.
//!
//! # Design
//!
//! - **ForgeReader**: Read-only access (issues, PRs, reviews)
//! - **ForgeWriter**: Write operations live in `src/repo/` (different concern)
//!
//! # Example
//!
//! ```ignore
//! use patina::forge::{detect, reader};
//!
//! let forge = detect("git@github.com:owner/repo.git");
//! let reader = reader(&forge);
//! let issues = reader.list_issues(100, None)?;
//! ```

mod types;
pub mod writer;

pub mod github;
mod none;

pub use types::*;
pub use writer::{ForgeWriter, GitHubWriter, NoneWriter};

use anyhow::Result;

/// Read-only access to forge data (issues, PRs, reviews).
///
/// "Do X": Fetch code review data from a forge platform.
///
/// Implementations handle platform-specific CLI/API calls internally.
/// Results are cached in eventlog for offline access.
pub trait ForgeReader {
    /// Fetch issues (with optional since filter for incremental updates).
    fn list_issues(&self, limit: usize, since: Option<&str>) -> Result<Vec<Issue>>;

    /// Fetch pull requests.
    fn list_pull_requests(&self, limit: usize, since: Option<&str>) -> Result<Vec<PullRequest>>;

    /// Get single PR with full details (body, comments, reviews, linked issues).
    fn get_pull_request(&self, number: i64) -> Result<PullRequest>;

    /// Get single issue by number.
    fn get_issue(&self, number: i64) -> Result<Issue>;
}

/// Detect forge from git remote URL.
///
/// Parses URLs like:
/// - `git@github.com:owner/repo.git`
/// - `https://github.com/owner/repo`
/// - `https://codeberg.org/owner/repo`
pub fn detect(remote_url: &str) -> Forge {
    // Try to parse owner/repo from URL
    let (host, owner, repo) = parse_remote_url(remote_url);

    let kind = if host.contains("github.com") {
        ForgeKind::GitHub
    } else if is_gitea_host(&host) {
        ForgeKind::Gitea
    } else {
        ForgeKind::None
    };

    Forge {
        kind,
        owner,
        repo,
        host,
    }
}

/// Get a ForgeReader for the detected forge.
pub fn reader(forge: &Forge) -> Box<dyn ForgeReader> {
    match forge.kind {
        ForgeKind::GitHub => Box::new(github::GitHubReader::new(forge)),
        ForgeKind::Gitea => Box::new(none::NoneReader), // TODO: implement GiteaReader
        ForgeKind::None => Box::new(none::NoneReader),
    }
}

/// Parse remote URL into (host, owner, repo).
fn parse_remote_url(url: &str) -> (String, String, String) {
    // SSH format: git@github.com:owner/repo.git
    if let Some(rest) = url.strip_prefix("git@") {
        if let Some((host, path)) = rest.split_once(':') {
            let path = path.trim_end_matches(".git");
            if let Some((owner, repo)) = path.split_once('/') {
                return (host.to_string(), owner.to_string(), repo.to_string());
            }
        }
    }

    // HTTPS format: https://github.com/owner/repo
    if url.starts_with("https://") || url.starts_with("http://") {
        let without_proto = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);
        let without_git = without_proto.trim_end_matches(".git");
        let parts: Vec<&str> = without_git.splitn(3, '/').collect();
        if parts.len() >= 3 {
            return (
                parts[0].to_string(),
                parts[1].to_string(),
                parts[2].to_string(),
            );
        }
    }

    // Couldn't parse
    (String::new(), String::new(), String::new())
}

/// Check if host is a Gitea/Forgejo instance.
fn is_gitea_host(host: &str) -> bool {
    host.contains("codeberg.org")
        || host.contains("gitea.")
        || host.contains("forgejo.")
        || host.contains("gitea.io")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_github_ssh() {
        let forge = detect("git@github.com:anthropics/claude-code.git");
        assert_eq!(forge.kind, ForgeKind::GitHub);
        assert_eq!(forge.owner, "anthropics");
        assert_eq!(forge.repo, "claude-code");
        assert_eq!(forge.host, "github.com");
    }

    #[test]
    fn test_detect_github_https() {
        let forge = detect("https://github.com/anthropics/claude-code");
        assert_eq!(forge.kind, ForgeKind::GitHub);
        assert_eq!(forge.owner, "anthropics");
        assert_eq!(forge.repo, "claude-code");
    }

    #[test]
    fn test_detect_codeberg() {
        let forge = detect("https://codeberg.org/owner/repo");
        assert_eq!(forge.kind, ForgeKind::Gitea);
        assert_eq!(forge.owner, "owner");
        assert_eq!(forge.repo, "repo");
    }

    #[test]
    fn test_detect_unknown() {
        let forge = detect("https://gitlab.com/owner/repo");
        assert_eq!(forge.kind, ForgeKind::None);
    }

    #[test]
    fn test_detect_local() {
        let forge = detect("/path/to/local/repo");
        assert_eq!(forge.kind, ForgeKind::None);
    }
}
