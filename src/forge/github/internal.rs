//! Internal implementation for GitHub ForgeReader.
//!
//! Contains gh CLI calls and JSON parsing.
//! Not exposed in public interface.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::process::Command;

use crate::forge::{Comment, Issue, IssueState, PrState, PullRequest};

// ============================================================================
// gh CLI JSON types (internal, match gh output format)
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhIssue {
    number: i64,
    title: String,
    body: Option<String>,
    state: String,
    labels: Vec<GhLabel>,
    author: GhAuthor,
    created_at: String,
    updated_at: String,
    url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhPullRequest {
    number: i64,
    title: String,
    body: Option<String>,
    state: String,
    labels: Vec<GhLabel>,
    author: GhAuthor,
    created_at: String,
    merged_at: Option<String>,
    url: String,
    #[serde(default)]
    comments: Vec<GhComment>,
    #[serde(default)]
    reviews: Vec<GhReview>,
    #[serde(default, rename = "closingIssuesReferences")]
    closing_issues: Vec<GhIssueRef>,
}

#[derive(Debug, Deserialize)]
struct GhLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GhAuthor {
    login: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhComment {
    author: GhAuthor,
    body: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GhReview {
    author: GhAuthor,
    body: String,
    state: String,
    #[serde(default)]
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct GhIssueRef {
    number: i64,
}

// ============================================================================
// Public functions (called by GitHubReader)
// ============================================================================

/// Check if `gh` CLI is authenticated.
pub(crate) fn check_gh_auth() -> Result<bool> {
    let output = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .context("Failed to run `gh auth status`. Is `gh` CLI installed?")?;

    Ok(output.status.success())
}

/// Get total issue count via GitHub search API.
pub(crate) fn fetch_issue_count(repo: &str) -> Result<usize> {
    let query = format!("repo:{} is:issue", repo);
    let output = Command::new("gh")
        .args(["api", "search/issues", "-f", &format!("q={}", query), "--jq", ".total_count"])
        .output()
        .context("Failed to run `gh api search/issues`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh api search/issues failed: {}", stderr);
    }

    let count_str = String::from_utf8_lossy(&output.stdout);
    let count: usize = count_str.trim().parse().unwrap_or(0);
    Ok(count)
}

/// Get total PR count via GitHub search API.
pub(crate) fn fetch_pr_count(repo: &str) -> Result<usize> {
    let query = format!("repo:{} is:pr", repo);
    let output = Command::new("gh")
        .args(["api", "search/issues", "-f", &format!("q={}", query), "--jq", ".total_count"])
        .output()
        .context("Failed to run `gh api search/issues`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh api search/issues failed: {}", stderr);
    }

    let count_str = String::from_utf8_lossy(&output.stdout);
    let count: usize = count_str.trim().parse().unwrap_or(0);
    Ok(count)
}

/// Fetch issues via gh CLI.
pub(crate) fn fetch_issues(repo: &str, limit: usize, since: Option<&str>) -> Result<Vec<Issue>> {
    let mut cmd = Command::new("gh");
    cmd.args([
        "issue",
        "list",
        "--repo",
        repo,
        "--limit",
        &limit.to_string(),
        "--state",
        "all",
        "--json",
        "number,title,body,state,labels,author,createdAt,updatedAt,url",
    ]);

    // Add search filter for incremental updates
    if let Some(timestamp) = since {
        let date = &timestamp[..10.min(timestamp.len())]; // Extract YYYY-MM-DD
        cmd.args(["--search", &format!("updated:>={}", date)]);
    }

    let output = cmd.output().context("Failed to run `gh issue list`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh issue list failed: {}", stderr);
    }

    let gh_issues: Vec<GhIssue> =
        serde_json::from_slice(&output.stdout).context("Failed to parse GitHub issues JSON")?;

    Ok(gh_issues.into_iter().map(into_issue).collect())
}

/// Fetch pull requests via gh CLI.
pub(crate) fn fetch_pull_requests(
    repo: &str,
    limit: usize,
    since: Option<&str>,
) -> Result<Vec<PullRequest>> {
    let mut cmd = Command::new("gh");
    cmd.args([
        "pr",
        "list",
        "--repo",
        repo,
        "--limit",
        &limit.to_string(),
        "--state",
        "all",
        "--json",
        "number,title,body,state,labels,author,createdAt,mergedAt,url",
    ]);

    // Add search filter for incremental updates
    if let Some(timestamp) = since {
        let date = &timestamp[..10.min(timestamp.len())];
        cmd.args(["--search", &format!("updated:>={}", date)]);
    }

    let output = cmd.output().context("Failed to run `gh pr list`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh pr list failed: {}", stderr);
    }

    let gh_prs: Vec<GhPullRequest> =
        serde_json::from_slice(&output.stdout).context("Failed to parse GitHub PRs JSON")?;

    Ok(gh_prs.into_iter().map(into_pull_request).collect())
}

/// Fetch single issue by number.
pub(crate) fn fetch_issue(repo: &str, number: i64) -> Result<Issue> {
    let output = Command::new("gh")
        .args([
            "issue",
            "view",
            &number.to_string(),
            "--repo",
            repo,
            "--json",
            "number,title,body,state,labels,author,createdAt,updatedAt,url",
        ])
        .output()
        .context("Failed to run `gh issue view`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh issue view #{} failed: {}", number, stderr);
    }

    let gh_issue: GhIssue =
        serde_json::from_slice(&output.stdout).context("Failed to parse GitHub issue JSON")?;

    Ok(into_issue(gh_issue))
}

/// Fetch single PR with full details.
pub(crate) fn fetch_pull_request(repo: &str, number: i64) -> Result<PullRequest> {
    let output = Command::new("gh")
        .args([
            "pr",
            "view",
            &number.to_string(),
            "--repo",
            repo,
            "--json",
            "number,title,body,state,labels,author,createdAt,mergedAt,url,comments,reviews,closingIssuesReferences",
        ])
        .output()
        .context("Failed to run `gh pr view`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh pr view #{} failed: {}", number, stderr);
    }

    let gh_pr: GhPullRequest =
        serde_json::from_slice(&output.stdout).context("Failed to parse GitHub PR JSON")?;

    Ok(into_pull_request(gh_pr))
}

/// Fetch the highest issue number (for backlog population).
/// Returns 0 if no issues exist.
pub(crate) fn fetch_max_issue_number(repo: &str) -> Result<i64> {
    let output = Command::new("gh")
        .args([
            "issue", "list", "--repo", repo, "--limit", "1", "--state", "all", "--json", "number",
        ])
        .output()
        .context("Failed to run `gh issue list`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh issue list failed: {}", stderr);
    }

    #[derive(serde::Deserialize)]
    struct IssueNum {
        number: i64,
    }

    let issues: Vec<IssueNum> =
        serde_json::from_slice(&output.stdout).context("Failed to parse GitHub issues JSON")?;

    Ok(issues.first().map(|i| i.number).unwrap_or(0))
}

// ============================================================================
// Conversion functions
// ============================================================================

fn into_issue(gh: GhIssue) -> Issue {
    Issue {
        number: gh.number,
        title: gh.title,
        body: gh.body,
        state: parse_issue_state(&gh.state),
        author: gh.author.login,
        labels: gh.labels.into_iter().map(|l| l.name).collect(),
        created_at: gh.created_at,
        updated_at: gh.updated_at,
        url: gh.url,
    }
}

fn into_pull_request(gh: GhPullRequest) -> PullRequest {
    // Combine comments and reviews into unified comments list
    let mut comments: Vec<Comment> = gh
        .comments
        .into_iter()
        .map(|c| Comment {
            author: c.author.login,
            body: c.body,
            created_at: c.created_at,
        })
        .collect();

    // Add review comments (filter empty ones)
    for review in gh.reviews.iter() {
        if !review.body.is_empty() {
            comments.push(Comment {
                author: review.author.login.clone(),
                body: review.body.clone(),
                created_at: review.created_at.clone(),
            });
        }
    }

    // Count approvals
    let approvals = gh.reviews.iter().filter(|r| r.state == "APPROVED").count() as i32;

    PullRequest {
        number: gh.number,
        title: gh.title,
        body: gh.body,
        state: parse_pr_state(&gh.state, gh.merged_at.is_some()),
        author: gh.author.login,
        labels: gh.labels.into_iter().map(|l| l.name).collect(),
        created_at: gh.created_at,
        merged_at: gh.merged_at,
        url: gh.url,
        linked_issues: gh.closing_issues.into_iter().map(|i| i.number).collect(),
        comments,
        approvals,
    }
}

fn parse_issue_state(state: &str) -> IssueState {
    match state.to_uppercase().as_str() {
        "OPEN" => IssueState::Open,
        _ => IssueState::Closed,
    }
}

fn parse_pr_state(state: &str, has_merged_at: bool) -> PrState {
    if has_merged_at {
        PrState::Merged
    } else {
        match state.to_uppercase().as_str() {
            "OPEN" => PrState::Open,
            "MERGED" => PrState::Merged,
            _ => PrState::Closed,
        }
    }
}
