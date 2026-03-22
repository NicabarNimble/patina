//! Child host-support bridge during CV6 migration.

use crate::child::engine::{GrantedCapabilities, QueryDispatchFn};

pub(crate) fn query(
    child_name: &str,
    grants: &GrantedCapabilities,
    query_fn: &mut Option<QueryDispatchFn>,
    kind: &str,
    params: &str,
) -> Result<String, String> {
    crate::plugin::internal::host_support::query(child_name, grants, query_fn, kind, params)
}

pub(crate) fn http_get(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    child_name: &str,
    url: &str,
) -> Result<crate::plugin::internal::host_support::HttpResult, String> {
    crate::plugin::internal::host_support::http_get(http_client, grants, child_name, url)
}

pub(crate) fn http_post(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    child_name: &str,
    url: &str,
    body: &str,
    content_type: &str,
) -> Result<crate::plugin::internal::host_support::HttpResult, String> {
    crate::plugin::internal::host_support::http_post(
        http_client,
        grants,
        child_name,
        url,
        body,
        content_type,
    )
}
