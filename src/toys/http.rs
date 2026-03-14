//! HTTP toy host helpers.

use crate::plugin::GrantedCapabilities;

#[derive(Debug, Clone)]
pub struct HttpToyResponse {
    pub status: u16,
    pub body: String,
}

pub fn post(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    url: &str,
    body: &str,
    content_type: &str,
) -> Result<HttpToyResponse, String> {
    let response = crate::plugin::internal::host_support::http_post(
        http_client,
        grants,
        plugin_name,
        url,
        body,
        content_type,
    )?;
    Ok(HttpToyResponse {
        status: response.status,
        body: response.body,
    })
}

pub fn get(
    http_client: &reqwest::blocking::Client,
    grants: &GrantedCapabilities,
    plugin_name: &str,
    url: &str,
) -> Result<HttpToyResponse, String> {
    let response =
        crate::plugin::internal::host_support::http_get(http_client, grants, plugin_name, url)?;
    Ok(HttpToyResponse {
        status: response.status,
        body: response.body,
    })
}
