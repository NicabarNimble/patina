//! GitHub REST API client using pipe/http.
//!
//! Migrated from plugins/forge/src/github.rs. All HTTP calls go
//! through Mother via io.get(url).send() — this module never opens
//! sockets directly. Mother injects Bearer auth and enforces domain
//! allowlist.

use std::io::{BufRead, Write};

use patina_pipe::{PipeError, PipeIo};
use patina_pipe_types::PipeHttpResponse;
use serde::Deserialize;

/// Maximum items per page for GitHub API pagination.
const PER_PAGE: usize = 100;

/// Delay between API calls (milliseconds). GitHub allows 5000/hour;
/// 750ms pacing is conservative and matches the existing sync engine.
const RATE_LIMIT_MS: u64 = 750;

// ============================================================================
// GitHub REST API JSON types
// ============================================================================

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GhApiIssue {
    number: i64,
    title: String,
    body: Option<String>,
    state: String,
    user: GhApiUser,
    labels: Vec<GhApiLabel>,
    created_at: String,
    updated_at: String,
    html_url: String,
    pull_request: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GhApiPullRequest {
    number: i64,
    title: String,
    body: Option<String>,
    state: String,
    user: GhApiUser,
    labels: Vec<GhApiLabel>,
    created_at: String,
    updated_at: String,
    merged_at: Option<String>,
    html_url: String,
}

#[derive(Debug, Deserialize)]
struct GhApiUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GhApiLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GhApiComment {
    user: GhApiUser,
    body: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GhApiReview {
    user: GhApiUser,
    body: Option<String>,
    state: String,
    submitted_at: Option<String>,
}

// ============================================================================
// Fetch result
// ============================================================================

pub struct SyncResult {
    pub fetched: usize,
    pub emitted: usize,
    pub latest_updated_at: Option<String>,
}

// ============================================================================
// Client
// ============================================================================

pub struct GitHubClient<'a> {
    owner: &'a str,
    repo: &'a str,
}

impl<'a> GitHubClient<'a> {
    pub fn new(owner: &'a str, repo: &'a str) -> Self {
        Self { owner, repo }
    }

    fn api_base(&self) -> String {
        format!("https://api.github.com/repos/{}/{}", self.owner, self.repo)
    }

    /// GET with error mapping per DESIGN.md §2.
    fn get(
        &self,
        url: &str,
        io: &mut PipeIo<impl Write, impl BufRead>,
    ) -> Result<PipeHttpResponse, PipeError> {
        let response = io
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()?;

        match response.status {
            200 => Ok(response),
            401 | 403 => {
                if response.body.contains("rate limit") {
                    Err(PipeError::RateLimited {
                        message: format!("GitHub rate limited ({}): {}", response.status, url),
                        retry_after_ms: 60_000,
                    })
                } else {
                    Err(PipeError::Fatal {
                        message: format!(
                            "GitHub auth error ({}): {}",
                            response.status,
                            truncate(&response.body, 200)
                        ),
                    })
                }
            }
            404 => Err(PipeError::Fatal {
                message: format!("not found (404): {}", url),
            }),
            429 => Err(PipeError::RateLimited {
                message: format!("GitHub rate limited (429): {}", url),
                retry_after_ms: 60_000,
            }),
            status if status >= 500 => Err(PipeError::Transient {
                message: format!(
                    "GitHub server error ({}): {}",
                    status,
                    truncate(&response.body, 200)
                ),
                retry_after_ms: Some(5_000),
            }),
            status => Err(PipeError::Transient {
                message: format!(
                    "GitHub API error ({}): {}",
                    status,
                    truncate(&response.body, 200)
                ),
                retry_after_ms: Some(5_000),
            }),
        }
    }

    // ========================================================================
    // Issues
    // ========================================================================

    /// Fetch issues and emit them as github.issue facts.
    pub fn fetch_and_emit_issues(
        &self,
        limit: usize,
        since: Option<&str>,
        io: &mut PipeIo<impl Write, impl BufRead>,
    ) -> Result<SyncResult, PipeError> {
        let mut fetched = 0;
        let mut emitted = 0;
        let mut latest_updated_at: Option<String> = None;
        let mut page = 1;

        loop {
            if fetched >= limit {
                break;
            }

            let url = self.issues_url(page, since);
            let response = self.get(&url, io)?;
            let items: Vec<GhApiIssue> =
                serde_json::from_str(&response.body).map_err(|e| PipeError::Fatal {
                    message: format!("JSON parse error: {}", e),
                })?;

            if items.is_empty() {
                break;
            }

            let page_count = items.len();

            for item in &items {
                if fetched >= limit {
                    break;
                }

                // Skip PRs — GitHub's issues endpoint includes them
                if item.pull_request.is_some() {
                    continue;
                }

                // Track latest updated_at for cursor
                track_latest(&mut latest_updated_at, &item.updated_at);

                let event_data = issue_to_event_json(item);
                match io.emit("github", "issue", &event_data) {
                    Ok(()) => emitted += 1,
                    Err(e) => {
                        eprintln!("[github] emit issue #{} failed: {}", item.number, e);
                    }
                }
                fetched += 1;
            }

            if page_count < PER_PAGE {
                break;
            }

            page += 1;
            rate_limit_sleep();
        }

        Ok(SyncResult {
            fetched,
            emitted,
            latest_updated_at,
        })
    }

    fn issues_url(&self, page: usize, since: Option<&str>) -> String {
        let mut url = format!(
            "{}/issues?state=all&per_page={}&page={}&sort=updated&direction=desc",
            self.api_base(),
            PER_PAGE,
            page,
        );
        if let Some(ts) = since {
            url.push_str(&format!("&since={}", ts));
        }
        url
    }

    // ========================================================================
    // Pull Requests
    // ========================================================================

    /// Fetch PRs and emit them as github.pr facts.
    ///
    /// GitHub's pulls endpoint doesn't support `since`, so we sort by
    /// `updated desc` and stop when we see an item older than `since`.
    pub fn fetch_and_emit_prs(
        &self,
        limit: usize,
        since: Option<&str>,
        io: &mut PipeIo<impl Write, impl BufRead>,
    ) -> Result<SyncResult, PipeError> {
        let mut fetched = 0;
        let mut emitted = 0;
        let mut latest_updated_at: Option<String> = None;
        let mut page = 1;
        let mut hit_since_cutoff = false;

        loop {
            if fetched >= limit || hit_since_cutoff {
                break;
            }

            let url = self.prs_url(page);
            let response = self.get(&url, io)?;
            let items: Vec<GhApiPullRequest> =
                serde_json::from_str(&response.body).map_err(|e| PipeError::Fatal {
                    message: format!("JSON parse error: {}", e),
                })?;

            if items.is_empty() {
                break;
            }

            let page_count = items.len();

            for item in &items {
                if fetched >= limit {
                    break;
                }

                // Client-side since filter: results are sorted by updated desc,
                // so once we see an older item, all remaining are older too.
                if let Some(cutoff) = since {
                    if item.updated_at.as_str() <= cutoff {
                        hit_since_cutoff = true;
                        break;
                    }
                }

                // Track latest updated_at for cursor
                track_latest(&mut latest_updated_at, &item.updated_at);

                // Fetch comments and reviews for full detail
                rate_limit_sleep();
                let comments = self.fetch_pr_comments(item.number, io).unwrap_or_default();
                rate_limit_sleep();
                let reviews = self.fetch_pr_reviews(item.number, io).unwrap_or_default();

                let event_data = pr_to_event_json(item, &comments, &reviews);
                match io.emit("github", "pr", &event_data) {
                    Ok(()) => emitted += 1,
                    Err(e) => {
                        eprintln!("[github] emit PR #{} failed: {}", item.number, e);
                    }
                }
                fetched += 1;
            }

            if page_count < PER_PAGE {
                break;
            }

            page += 1;
            rate_limit_sleep();
        }

        Ok(SyncResult {
            fetched,
            emitted,
            latest_updated_at,
        })
    }

    fn fetch_pr_comments(
        &self,
        number: i64,
        io: &mut PipeIo<impl Write, impl BufRead>,
    ) -> Result<Vec<GhApiComment>, PipeError> {
        let url = format!(
            "{}/issues/{}/comments?per_page=100",
            self.api_base(),
            number
        );
        let response = self.get(&url, io)?;
        serde_json::from_str(&response.body).map_err(|e| PipeError::Fatal {
            message: format!("JSON parse error: {}", e),
        })
    }

    fn fetch_pr_reviews(
        &self,
        number: i64,
        io: &mut PipeIo<impl Write, impl BufRead>,
    ) -> Result<Vec<GhApiReview>, PipeError> {
        let url = format!("{}/pulls/{}/reviews?per_page=100", self.api_base(), number);
        let response = self.get(&url, io)?;
        serde_json::from_str(&response.body).map_err(|e| PipeError::Fatal {
            message: format!("JSON parse error: {}", e),
        })
    }

    fn prs_url(&self, page: usize) -> String {
        format!(
            "{}/pulls?state=all&per_page={}&page={}&sort=updated&direction=desc",
            self.api_base(),
            PER_PAGE,
            page,
        )
    }
}

// ============================================================================
// JSON conversion — match existing eventlog format
// ============================================================================

fn issue_to_event_json(item: &GhApiIssue) -> serde_json::Value {
    let state = match item.state.as_str() {
        "open" => "open",
        _ => "closed",
    };

    let labels: Vec<&str> = item.labels.iter().map(|l| l.name.as_str()).collect();

    serde_json::json!({
        "number": item.number,
        "title": item.title,
        "body": item.body,
        "state": state,
        "labels": labels,
        "author": item.user.login,
        "url": item.html_url,
        "created_at": item.created_at,
        "updated_at": item.updated_at,
    })
}

fn pr_to_event_json(
    item: &GhApiPullRequest,
    comments: &[GhApiComment],
    reviews: &[GhApiReview],
) -> serde_json::Value {
    let state = if item.merged_at.is_some() {
        "merged"
    } else {
        match item.state.as_str() {
            "open" => "open",
            _ => "closed",
        }
    };

    let labels: Vec<&str> = item.labels.iter().map(|l| l.name.as_str()).collect();

    let mut structured_comments: Vec<serde_json::Value> = Vec::new();
    for c in comments {
        structured_comments.push(serde_json::json!({
            "author": c.user.login,
            "body": c.body,
            "created_at": c.created_at,
        }));
    }
    for r in reviews {
        if let Some(body) = &r.body {
            if !body.is_empty() {
                structured_comments.push(serde_json::json!({
                    "author": r.user.login,
                    "body": body,
                    "created_at": r.submitted_at,
                }));
            }
        }
    }

    let comments_text: String = structured_comments
        .iter()
        .filter_map(|c| {
            let author = c["author"].as_str()?;
            let body = c["body"].as_str()?;
            Some(format!("{}: {}", author, body))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let approvals = reviews.iter().filter(|r| r.state == "APPROVED").count() as i32;
    let linked_issues = extract_linked_issues(item.body.as_deref().unwrap_or(""));

    serde_json::json!({
        "number": item.number,
        "title": item.title,
        "body": item.body,
        "state": state,
        "labels": labels,
        "author": item.user.login,
        "url": item.html_url,
        "created_at": item.created_at,
        "updated_at": item.updated_at,
        "merged_at": item.merged_at,
        "linked_issues": linked_issues,
        "comments": structured_comments,
        "comments_text": comments_text,
        "approvals": approvals,
    })
}

/// Extract issue numbers from PR body text.
///
/// Looks for GitHub closing keywords followed by `#N`:
/// fixes, closes, resolves (and their conjugated forms).
fn extract_linked_issues(body: &str) -> Vec<i64> {
    const KEYWORDS: &[&str] = &[
        "fix ",
        "fixes ",
        "fixed ",
        "close ",
        "closes ",
        "closed ",
        "resolve ",
        "resolves ",
        "resolved ",
    ];
    let lower = body.to_lowercase();
    let mut issues = Vec::new();

    for keyword in KEYWORDS {
        let mut search_from = 0;
        while let Some(pos) = lower[search_from..].find(keyword) {
            let after = search_from + pos + keyword.len();
            if let Some(rest) = lower.get(after..) {
                let rest = rest.trim_start();
                if let Some(rest) = rest.strip_prefix('#') {
                    let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                    if let Ok(n) = num_str.parse::<i64>() {
                        if !issues.contains(&n) {
                            issues.push(n);
                        }
                    }
                }
            }
            search_from = after;
        }
    }

    issues.sort();
    issues
}

/// Track the latest updated_at timestamp for cursor.
fn track_latest(current: &mut Option<String>, candidate: &str) {
    match current {
        Some(existing) if candidate > existing.as_str() => {
            *current = Some(candidate.to_string());
        }
        None => {
            *current = Some(candidate.to_string());
        }
        _ => {}
    }
}

fn rate_limit_sleep() {
    std::thread::sleep(std::time::Duration::from_millis(RATE_LIMIT_MS));
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        s.get(..max).unwrap_or(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_linked_issues_fixes() {
        assert_eq!(extract_linked_issues("Fixes #42"), vec![42]);
        assert_eq!(extract_linked_issues("fixes #1 and fixes #2"), vec![1, 2]);
        assert_eq!(extract_linked_issues("Fixed #10"), vec![10]);
    }

    #[test]
    fn test_extract_linked_issues_closes() {
        assert_eq!(extract_linked_issues("Closes #7"), vec![7]);
        assert_eq!(extract_linked_issues("close #3"), vec![3]);
    }

    #[test]
    fn test_extract_linked_issues_resolves() {
        assert_eq!(extract_linked_issues("Resolves #99"), vec![99]);
    }

    #[test]
    fn test_extract_linked_issues_dedup() {
        assert_eq!(extract_linked_issues("fixes #5, closes #5"), vec![5]);
    }

    #[test]
    fn test_extract_linked_issues_none() {
        assert_eq!(extract_linked_issues("no issues here"), Vec::<i64>::new());
        assert_eq!(extract_linked_issues("see #123"), Vec::<i64>::new());
    }

    #[test]
    fn test_extract_linked_issues_sorted() {
        assert_eq!(
            extract_linked_issues("fixes #30, fixes #10, fixes #20"),
            vec![10, 20, 30]
        );
    }
}
