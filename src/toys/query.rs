//! Query toy host helpers.

use crate::plugin::{GrantedCapabilities, QueryDispatchFn};

pub fn dispatch(
    plugin_name: &str,
    grants: &GrantedCapabilities,
    query_fn: &mut Option<QueryDispatchFn>,
    kind: &str,
    params: &str,
) -> Result<String, String> {
    crate::plugin::internal::host_support::query(plugin_name, grants, query_fn, kind, params)
}
