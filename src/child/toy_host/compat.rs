//! Phase 2 compatibility adapters for legacy toy dispatch.

use crate::child::engine::GrantedCapabilities;

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
