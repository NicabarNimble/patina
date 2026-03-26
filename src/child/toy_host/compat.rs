//! Phase 2 compatibility adapters for legacy toy dispatch.

use crate::child::engine::GrantedCapabilities;
use crate::child::toy_host::github::{ListParams, Page};

/// Legacy ingress `fetch` routed through the HTTP toy helper.
pub fn ingress_fetch_via_http(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    endpoint: &str,
) -> Result<String, String> {
    crate::child::toy_host::http::get(http_client, grants, plugin_name, endpoint)
        .map(|response| response.body)
}

/// Legacy github toy dispatch routed through compatibility layer.
pub fn github_ensure_granted(github_granted: bool, plugin_name: &str) -> Result<(), String> {
    crate::child::toy_host::github::ensure_granted(github_granted, plugin_name)
}

pub fn github_list_issues(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    params: &ListParams,
) -> Result<Page, String> {
    crate::child::toy_host::github::list_issues(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        params,
    )
}

pub fn github_list_pulls(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    params: &ListParams,
) -> Result<Page, String> {
    crate::child::toy_host::github::list_pulls(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        params,
    )
}

pub fn github_list_issue_comments(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    issue_number: u32,
) -> Result<Page, String> {
    crate::child::toy_host::github::list_issue_comments(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        issue_number,
    )
}

pub fn github_list_issue_events(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    issue_number: u32,
) -> Result<Page, String> {
    crate::child::toy_host::github::list_issue_events(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        issue_number,
    )
}

pub fn github_list_pull_comments(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    pull_number: u32,
) -> Result<Page, String> {
    crate::child::toy_host::github::list_pull_comments(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        pull_number,
    )
}

pub fn github_list_reviews(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    pull_number: u32,
) -> Result<Page, String> {
    crate::child::toy_host::github::list_reviews(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        pull_number,
    )
}

pub fn github_list_review_comments(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    owner: &str,
    repo: &str,
    pull_number: u32,
    review_id: u64,
) -> Result<Page, String> {
    crate::child::toy_host::github::list_review_comments(
        http_client,
        grants,
        plugin_name,
        owner,
        repo,
        pull_number,
        review_id,
    )
}
