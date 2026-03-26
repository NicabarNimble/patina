//! Phase 2 compatibility adapters for legacy toy dispatch.

use crate::child::engine::GrantedCapabilities;
use crate::child::engine::QueryDispatchFn;
use crate::child::toy_host::connector::ConnectorBindingRecord;
use crate::child::toy_host::github::{ListParams, Page};
use crate::mother::state::LakeCursorUpdate;
use crate::mother::KnowledgeRuntimeStore;
use mother_crate::PendingEvent;

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

pub fn ingress_list_granted_sources(toys: &crate::mother::GrantedToys) -> Vec<(String, String)> {
    crate::child::toy_host::ingress::list_granted_sources(toys)
}

pub fn ingress_resolve_source_endpoint(
    toys: &crate::mother::GrantedToys,
    plugin_name: &str,
    source_name: &str,
) -> Result<String, String> {
    crate::child::toy_host::ingress::resolve_source_endpoint(toys, plugin_name, source_name)
}

pub fn query_dispatch(
    plugin_name: &str,
    grants: &GrantedCapabilities,
    query_fn: &mut Option<QueryDispatchFn>,
    kind: &str,
    params: &str,
) -> Result<String, String> {
    crate::child::toy_host::query::dispatch(plugin_name, grants, query_fn, kind, params)
}

pub fn http_post(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    url: &str,
    body: &str,
    content_type: &str,
) -> Result<crate::child::toy_host::http::HttpToyResponse, String> {
    crate::child::toy_host::http::post(http_client, grants, plugin_name, url, body, content_type)
}

pub fn http_get(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    url: &str,
) -> Result<crate::child::toy_host::http::HttpToyResponse, String> {
    crate::child::toy_host::http::get(http_client, grants, plugin_name, url)
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

/// Legacy connector toy dispatch routed through compatibility layer.
pub fn connector_ensure_granted(connector_granted: bool, plugin_name: &str) -> Result<(), String> {
    crate::child::toy_host::connector::ensure_granted(connector_granted, plugin_name)
}

pub fn connector_list_bindings(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
) -> Result<Vec<ConnectorBindingRecord>, String> {
    crate::child::toy_host::connector::list_bindings(runtime, plugin_name)
}

pub fn connector_upsert_binding(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    binding: ConnectorBindingRecord,
) -> Result<ConnectorBindingRecord, String> {
    crate::child::toy_host::connector::upsert_binding(runtime, plugin_name, binding)
}

pub fn connector_remove_binding(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    binding_id: &str,
) -> Result<(), String> {
    crate::child::toy_host::connector::remove_binding(runtime, plugin_name, binding_id)
}

pub fn connector_load_binding(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    binding_id: &str,
) -> Result<ConnectorBindingRecord, String> {
    crate::child::toy_host::connector::load_binding(runtime, plugin_name, binding_id)
}

pub fn connector_ensure_type_enabled(
    binding: &ConnectorBindingRecord,
    data_type: &str,
) -> Result<(), String> {
    crate::child::toy_host::connector::ensure_type_enabled(binding, data_type)
}

pub fn connector_endpoint_for(
    binding: &ConnectorBindingRecord,
    data_type: &str,
) -> Result<String, String> {
    crate::child::toy_host::connector::endpoint_for(binding, data_type)
}

/// Legacy lake toy dispatch routed through compatibility layer.
pub fn lake_ensure_lake(name: &str) -> Result<String, String> {
    crate::child::toy_host::lake::ensure_lake(name).map_err(|e| e.to_string())
}

pub fn lake_load_cursor(
    runtime: &KnowledgeRuntimeStore,
    lake: &str,
    source: &str,
    data_type: &str,
) -> Result<Option<String>, String> {
    crate::child::toy_host::lake::load_cursor(runtime, lake, source, data_type)
        .map_err(|e| e.to_string())
}

pub fn lake_save_cursor(
    runtime: &KnowledgeRuntimeStore,
    update: &LakeCursorUpdate<'_>,
) -> Result<(), String> {
    crate::child::toy_host::lake::save_cursor(runtime, update).map_err(|e| e.to_string())
}

pub fn lake_ensure_table(lake: &str, table: &str) -> Result<(), String> {
    crate::child::toy_host::lake::ensure_table(lake, table).map_err(|e| e.to_string())
}

pub fn lake_append_json_batch(
    lake: &str,
    table: &str,
    source: &str,
    rows_json: &[String],
) -> Result<u64, String> {
    crate::child::toy_host::lake::append_json_batch(lake, table, source, rows_json)
        .map_err(|e| e.to_string())
}

pub fn lake_query_json(lake: &str, sql: &str) -> Result<String, String> {
    crate::child::toy_host::lake::query_json(lake, sql).map_err(|e| e.to_string())
}

/// Legacy events toy dispatch routed through compatibility layer.
pub fn events_pull(
    stream: &str,
    after_offset: Option<u64>,
    limit: u32,
) -> Result<Vec<PendingEvent>, String> {
    crate::child::toy_host::events::pull(stream, after_offset, limit).map_err(|e| e.to_string())
}

pub fn events_ack_through(
    runtime: &KnowledgeRuntimeStore,
    plugin_name: &str,
    stream: &str,
    offset: u64,
) -> Result<(), String> {
    crate::child::toy_host::events::ack_through(runtime, plugin_name, stream, offset)
        .map_err(|e| e.to_string())
}

pub fn events_list_streams() -> Vec<String> {
    crate::child::toy_host::events::list_streams()
}
