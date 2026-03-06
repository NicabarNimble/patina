//! GitHub REST API client using host_http.
//!
//! Replaces the gh CLI wrapper in src/forge/github/internal.rs.
//! All HTTP calls go through host_http which enforces domain allowlist
//! and injects credentials (Bearer token from vault).

use patina_sdk::mother_child::{host_emit, host_http, host_log};
use serde::Deserialize;

use crate::{rate_limit_sleep, PER_PAGE};

// ============================================================================
// GitHub REST API JSON types
// ============================================================================

// GitHub REST API uses snake_case (unlike gh CLI which uses camelCase).
// Fields may appear unused but are required for deserialization.

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
    /// Present when this "issue" is actually a PR.
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
// Sync result
// ============================================================================

pub struct SyncResult {
    pub fetched: usize,
    pub emitted: usize,
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

    /// Base URL for this repo's API.
    fn api_base(&self) -> String {
        format!("https://api.github.com/repos/{}/{}", self.owner, self.repo)
    }

    /// GET with error checking. Returns response body on success.
    fn get(&self, url: &str) -> Result<String, String> {
        let response = host_http::get(url)?;
        if response.status == 200 {
            Ok(response.body)
        } else if response.status == 404 {
            Err(format!("not found (404): {}", url))
        } else if response.status == 403 {
            Err(format!(
                "rate limited or forbidden ({}): {}",
                response.status, url
            ))
        } else {
            Err(format!(
                "GitHub API error ({}): {}",
                response.status,
                truncate(&response.body, 200)
            ))
        }
    }

    // ========================================================================
    // Issues
    // ========================================================================

    /// Fetch issues and emit them as forge.issue facts.
    /// Paginates through all pages up to `limit` total items.
    /// Filters out pull requests (GitHub's issues endpoint includes PRs).
    pub fn fetch_and_emit_issues(
        &self,
        limit: usize,
        since: Option<&str>,
    ) -> Result<SyncResult, String> {
        let mut fetched = 0;
        let mut emitted = 0;
        let mut page = 1;

        loop {
            if fetched >= limit {
                break;
            }

            let url = self.issues_url(page, since);
            let body = self.get(&url)?;
            let items: Vec<GhApiIssue> =
                serde_json::from_str(&body).map_err(|e| format!("JSON parse error: {}", e))?;

            if items.is_empty() {
                break;
            }

            let page_count = items.len();

            for item in items {
                if fetched >= limit {
                    break;
                }

                // Skip PRs — GitHub's issues endpoint includes them
                if item.pull_request.is_some() {
                    continue;
                }

                let event_data = issue_to_event_json(&item);
                match host_emit::emit_fact("forge", "issue", &event_data) {
                    Ok(_seq) => emitted += 1,
                    Err(e) => {
                        host_log::log(
                            host_log::LogLevel::Warn,
                            &format!("emit issue #{} failed: {}", item.number, e),
                        );
                    }
                }
                fetched += 1;
            }

            // If we got fewer items than per_page, we've reached the last page
            if page_count < PER_PAGE {
                break;
            }

            page += 1;
            rate_limit_sleep();
        }

        Ok(SyncResult { fetched, emitted })
    }

    /// Fetch a single issue and emit it.
    pub fn fetch_single_issue(&self, number: i64) -> Result<(), String> {
        let url = format!("{}/issues/{}", self.api_base(), number);
        let body = self.get(&url)?;
        let item: GhApiIssue =
            serde_json::from_str(&body).map_err(|e| format!("JSON parse error: {}", e))?;

        // Verify it's actually an issue, not a PR
        if item.pull_request.is_some() {
            return Err(format!("#{} is a pull request, not an issue", number));
        }

        let event_data = issue_to_event_json(&item);
        host_emit::emit_fact("forge", "issue", &event_data)?;
        Ok(())
    }

    fn issues_url(&self, page: usize, since: Option<&str>) -> String {
        let mut url = format!(
            "{}/issues?state=all&per_page={}&page={}&sort=updated&direction=desc",
            self.api_base(),
            PER_PAGE,
            page,
        );
        if let Some(ts) = since {
            // GitHub accepts ISO 8601 for the since parameter
            url.push_str(&format!("&since={}", ts));
        }
        url
    }

    // ========================================================================
    // Pull Requests
    // ========================================================================

    /// Fetch PRs and emit them as forge.pr facts.
    /// For each PR, also fetches comments and reviews for full detail.
    pub fn fetch_and_emit_prs(
        &self,
        limit: usize,
        since: Option<&str>,
    ) -> Result<SyncResult, String> {
        let mut fetched = 0;
        let mut emitted = 0;
        let mut page = 1;

        loop {
            if fetched >= limit {
                break;
            }

            let url = self.prs_url(page, since);
            let body = self.get(&url)?;
            let items: Vec<GhApiPullRequest> =
                serde_json::from_str(&body).map_err(|e| format!("JSON parse error: {}", e))?;

            if items.is_empty() {
                break;
            }

            let page_count = items.len();

            for item in items {
                if fetched >= limit {
                    break;
                }

                // Fetch comments and reviews for full detail
                rate_limit_sleep();
                let comments = self.fetch_pr_comments(item.number).unwrap_or_default();
                rate_limit_sleep();
                let reviews = self.fetch_pr_reviews(item.number).unwrap_or_default();

                let event_data = pr_to_event_json(&item, &comments, &reviews);
                match host_emit::emit_fact("forge", "pull-request", &event_data) {
                    Ok(_seq) => emitted += 1,
                    Err(e) => {
                        host_log::log(
                            host_log::LogLevel::Warn,
                            &format!("emit PR #{} failed: {}", item.number, e),
                        );
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

        Ok(SyncResult { fetched, emitted })
    }

    /// Fetch a single PR with full details and emit it.
    pub fn fetch_single_pr(&self, number: i64) -> Result<(), String> {
        let url = format!("{}/pulls/{}", self.api_base(), number);
        let body = self.get(&url)?;
        let item: GhApiPullRequest =
            serde_json::from_str(&body).map_err(|e| format!("JSON parse error: {}", e))?;

        rate_limit_sleep();
        let comments = self.fetch_pr_comments(number).unwrap_or_default();
        rate_limit_sleep();
        let reviews = self.fetch_pr_reviews(number).unwrap_or_default();

        let event_data = pr_to_event_json(&item, &comments, &reviews);
        host_emit::emit_fact("forge", "pull-request", &event_data)?;
        Ok(())
    }

    /// Fetch comments on a PR (issue comments endpoint).
    fn fetch_pr_comments(&self, number: i64) -> Result<Vec<GhApiComment>, String> {
        let url = format!(
            "{}/issues/{}/comments?per_page=100",
            self.api_base(),
            number
        );
        let body = self.get(&url)?;
        serde_json::from_str(&body).map_err(|e| format!("JSON parse error: {}", e))
    }

    /// Fetch reviews on a PR.
    fn fetch_pr_reviews(&self, number: i64) -> Result<Vec<GhApiReview>, String> {
        let url = format!("{}/pulls/{}/reviews?per_page=100", self.api_base(), number);
        let body = self.get(&url)?;
        serde_json::from_str(&body).map_err(|e| format!("JSON parse error: {}", e))
    }

    fn prs_url(&self, page: usize, since: Option<&str>) -> String {
        // GitHub's pulls endpoint doesn't have a `since` parameter,
        // so we sort by updated desc and stop early when needed.
        let _ = since;
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

/// Convert a GitHub API issue to the event JSON format stored in events.db.
/// Must match the format in src/commands/scrape/forge/mod.rs insert_issues().
fn issue_to_event_json(item: &GhApiIssue) -> String {
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
    .to_string()
}

/// Convert a GitHub API PR to the event JSON format stored in events.db.
/// Must match the format in src/commands/scrape/forge/mod.rs insert_prs().
fn pr_to_event_json(
    item: &GhApiPullRequest,
    comments: &[GhApiComment],
    reviews: &[GhApiReview],
) -> String {
    let state = if item.merged_at.is_some() {
        "merged"
    } else {
        match item.state.as_str() {
            "open" => "open",
            _ => "closed",
        }
    };

    let labels: Vec<&str> = item.labels.iter().map(|l| l.name.as_str()).collect();

    // Structured comments matching WIT schema: list<comment>
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

    // Flat text for FTS/search convenience
    let comments_text: String = structured_comments
        .iter()
        .filter_map(|c| {
            let author = c["author"].as_str()?;
            let body = c["body"].as_str()?;
            Some(format!("{}: {}", author, body))
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Count approvals
    let approvals = reviews.iter().filter(|r| r.state == "APPROVED").count() as i32;

    // Note: linked_issues not available via REST API without GraphQL.
    // The existing code used gh's closingIssuesReferences which is GraphQL.
    // We emit an empty list; this can be enriched later.
    let linked_issues: Vec<i64> = Vec::new();

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
    .to_string()
}

/// Truncate a string for error messages (UTF-8 safe).
fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        s.get(..max).unwrap_or(s)
    }
}
