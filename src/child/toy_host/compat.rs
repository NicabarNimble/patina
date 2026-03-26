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

/// Legacy session toy dispatch routed through compatibility layer.
pub fn session_ensure_granted(session_granted: bool, plugin_name: &str) -> Result<(), String> {
    crate::child::toy_host::session::ensure_granted(session_granted, plugin_name)
}

pub fn session_get_session_id() -> String {
    crate::child::toy_host::session::get_session_id()
}

pub fn session_get_previous_session() -> Option<String> {
    crate::child::toy_host::session::get_previous_session()
}

pub fn session_get_previous_session_runtime_id() -> Option<String> {
    crate::child::toy_host::session::get_previous_session_runtime_id()
}

pub fn session_get_previous_session_handoff() -> Option<String> {
    crate::child::toy_host::session::get_previous_session_handoff()
}

pub fn session_write_artifact(section: &str, content: &str) -> Result<(), String> {
    crate::child::toy_host::session::write_artifact(section, content)
}

pub fn session_set_parent_session(runtime_id: &str) -> Result<(), String> {
    crate::child::toy_host::session::set_parent_session(runtime_id)
}

pub fn session_create_tag(name: &str) -> Result<(), String> {
    crate::child::toy_host::session::create_tag(name)
}

pub fn session_set_status(status: &str) -> Result<(), String> {
    crate::child::toy_host::session::set_status(status)
}

pub fn session_write_handoff(modified_files: &str, summary: &str) -> Result<(), String> {
    crate::child::toy_host::session::write_handoff(modified_files, summary)
}
