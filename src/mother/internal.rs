//! Internal HTTP client implementation for mother
//!
//! Transport model (matches session.rs pattern):
//! - Local mother: UDS first (no auth needed — file permissions are auth)
//! - Remote mother: TCP with bearer token via reqwest

use anyhow::{Context, Result};
use mother_crate::protocol::{
    FederationQueryPayload, LifecycleNamePayload, LifecycleRefreshPayload,
};
use patina_protocol::{
    BuiltinChildRequest, BuiltinChildResponse, PandoRegistryInit, PandoRegistryState,
};
use reqwest::blocking::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;

use crate::paths;

/// Mother client
///
/// Tries UDS first for localhost addresses (no auth needed).
/// Falls back to TCP with bearer token for remote or when UDS unavailable.
pub struct Client {
    base_url: String,
    http: HttpClient,
    token: Option<String>,
    try_uds: bool,
}

impl Client {
    /// Create a new client for the given address (host:port or just host)
    pub fn new(address: String) -> Self {
        let base_url = if address.starts_with("http://") || address.starts_with("https://") {
            address
        } else {
            format!("http://{}", address)
        };

        let try_uds = is_localhost(&base_url);
        let token = serve_token();

        let http = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url,
            http,
            token,
            try_uds,
        }
    }

    /// Health check - returns Ok if mother is reachable
    pub fn health(&self) -> Result<HealthResponse> {
        // Try UDS first for local mother
        if self.try_uds {
            if let Some(body) = uds_get("/health") {
                return serde_json::from_slice(&body)
                    .context("Failed to parse health response from UDS");
            }
        }

        // TCP fallback with auth
        let url = format!("{}/health", self.base_url);
        let mut req = self.http.get(&url);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req
            .send()
            .with_context(|| format!("Failed to connect to mother at {}", self.base_url))?;

        if !response.status().is_success() {
            anyhow::bail!("Mother returned status: {}", response.status());
        }

        response
            .json::<HealthResponse>()
            .with_context(|| "Failed to parse health response")
    }

    /// Execute a scry query against the mother
    pub fn scry(&self, request: ScryRequest) -> Result<ScryResponse> {
        // Try UDS first for local mother
        if self.try_uds {
            let json_body = serde_json::to_vec(&request)?;
            if let Some(body) = uds_post("/api/scry", &json_body) {
                return serde_json::from_slice(&body)
                    .context("Failed to parse scry response from UDS");
            }
        }

        // TCP fallback with auth
        let url = format!("{}/api/scry", self.base_url);
        let mut req = self.http.post(&url).json(&request);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req
            .send()
            .with_context(|| format!("Failed to send scry request to {}", self.base_url))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("Mother scry failed ({}): {}", status, body);
        }

        response
            .json::<ScryResponse>()
            .with_context(|| "Failed to parse scry response")
    }

    pub fn child_action(&self, child: &str, action: &str, payload: &Value) -> Result<Value> {
        let path = format!("/child/{}/{}", child, action);

        if self.try_uds {
            let body = serde_json::to_vec(payload)?;
            if let Some((status, resp_body)) = uds_request("POST", &path, Some(&body)) {
                if (200..300).contains(&status) {
                    return serde_json::from_slice(&resp_body)
                        .context("Failed to parse child response from UDS");
                }
                let msg = String::from_utf8_lossy(&resp_body).to_string();
                anyhow::bail!("child request failed ({}): {}", status, msg);
            }
        }

        let url = format!("{}{}", self.base_url, path);
        let mut req = self.http.post(&url).json(payload);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req
            .send()
            .with_context(|| format!("Failed to send child request to {}", self.base_url))?;

        let status = response.status();
        let body = response.text().unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("child request failed ({}): {}", status, body);
        }
        serde_json::from_str(&body).with_context(|| "Failed to parse child response")
    }

    pub fn child_action_typed(
        &self,
        request: &BuiltinChildRequest,
    ) -> Result<BuiltinChildResponse> {
        let (child, action, payload) = request.to_http_parts();
        let http_payload = self.child_action(child, action, &payload)?;
        BuiltinChildResponse::from_http_payload(request.child, &request.action, http_payload)
            .map_err(|e| anyhow::anyhow!("Failed to decode typed child response: {}", e))
    }

    pub fn pando_registry_init(&self, request: &PandoRegistryInit) -> Result<PandoRegistryState> {
        if self.try_uds {
            let body = serde_json::to_vec(request)?;
            if let Some((status, resp_body)) =
                uds_request("POST", "/api/pando/registry/init", Some(&body))
            {
                if (200..300).contains(&status) {
                    return serde_json::from_slice(&resp_body)
                        .context("Failed to parse pando registry init response from UDS");
                }
                let msg = String::from_utf8_lossy(&resp_body).to_string();
                anyhow::bail!("pando registry init failed ({}): {}", status, msg);
            }
        }

        let url = format!("{}/api/pando/registry/init", self.base_url);
        let mut req = self.http.post(&url).json(request);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req.send().with_context(|| {
            format!(
                "Failed to send pando registry init request to {}",
                self.base_url
            )
        })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("pando registry init failed ({}): {}", status, body);
        }
        response
            .json::<PandoRegistryState>()
            .with_context(|| "Failed to parse pando registry init response")
    }

    pub fn pando_list(&self) -> Result<PandoRegistryState> {
        if self.try_uds {
            if let Some((status, resp_body)) = uds_request("GET", "/api/pando/list", None) {
                if (200..300).contains(&status) {
                    return serde_json::from_slice(&resp_body)
                        .context("Failed to parse pando list response from UDS");
                }
                let msg = String::from_utf8_lossy(&resp_body).to_string();
                anyhow::bail!("pando list failed ({}): {}", status, msg);
            }
        }

        let url = format!("{}/api/pando/list", self.base_url);
        let mut req = self.http.get(&url);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req
            .send()
            .with_context(|| format!("Failed to request pando list from {}", self.base_url))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("pando list failed ({}): {}", status, body);
        }
        response
            .json::<PandoRegistryState>()
            .with_context(|| "Failed to parse pando list response")
    }

    pub fn atlas_snapshot(&self) -> Result<Value> {
        if self.try_uds {
            if let Some((status, resp_body)) = uds_request("GET", "/api/atlas/snapshot", None) {
                if (200..300).contains(&status) {
                    return serde_json::from_slice(&resp_body)
                        .context("Failed to parse atlas snapshot response from UDS");
                }
                let msg = String::from_utf8_lossy(&resp_body).to_string();
                anyhow::bail!("atlas snapshot failed ({}): {}", status, msg);
            }
        }

        let url = format!("{}/api/atlas/snapshot", self.base_url);
        let mut req = self.http.get(&url);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req
            .send()
            .with_context(|| format!("Failed to request atlas snapshot from {}", self.base_url))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("atlas snapshot failed ({}): {}", status, body);
        }
        response
            .json::<Value>()
            .with_context(|| "Failed to parse atlas snapshot response")
    }

    pub fn federation_status(&self) -> Result<Value> {
        let payload = serde_json::json!({});
        if self.try_uds {
            let body = serde_json::to_vec(&payload)?;
            if let Some((status, resp_body)) =
                uds_request("POST", "/api/federation/status", Some(&body))
            {
                if (200..300).contains(&status) {
                    return serde_json::from_slice(&resp_body)
                        .context("Failed to parse federation status response from UDS");
                }
                let msg = String::from_utf8_lossy(&resp_body).to_string();
                anyhow::bail!("federation status failed ({}): {}", status, msg);
            }
        }

        let url = format!("{}/api/federation/status", self.base_url);
        let mut req = self.http.post(&url).json(&payload);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req.send().with_context(|| {
            format!(
                "Failed to send federation status request to {}",
                self.base_url
            )
        })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("federation status failed ({}): {}", status, body);
        }
        response
            .json::<Value>()
            .with_context(|| "Failed to parse federation status response")
    }

    pub fn federation_refresh(&self) -> Result<Value> {
        let payload = serde_json::json!({});
        if self.try_uds {
            let body = serde_json::to_vec(&payload)?;
            if let Some((status, resp_body)) =
                uds_request("POST", "/api/federation/refresh", Some(&body))
            {
                if (200..300).contains(&status) {
                    return serde_json::from_slice(&resp_body)
                        .context("Failed to parse federation refresh response from UDS");
                }
                let msg = String::from_utf8_lossy(&resp_body).to_string();
                anyhow::bail!("federation refresh failed ({}): {}", status, msg);
            }
        }

        let url = format!("{}/api/federation/refresh", self.base_url);
        let mut req = self.http.post(&url).json(&payload);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req.send().with_context(|| {
            format!(
                "Failed to send federation refresh request to {}",
                self.base_url
            )
        })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("federation refresh failed ({}): {}", status, body);
        }
        response
            .json::<Value>()
            .with_context(|| "Failed to parse federation refresh response")
    }

    pub fn federation_query(&self, payload: FederationQueryPayload) -> Result<Value> {
        if self.try_uds {
            let body = serde_json::to_vec(&payload)?;
            if let Some((status, resp_body)) =
                uds_request("POST", "/api/federation/query", Some(&body))
            {
                if (200..300).contains(&status) {
                    return serde_json::from_slice(&resp_body)
                        .context("Failed to parse federation query response from UDS");
                }
                let msg = String::from_utf8_lossy(&resp_body).to_string();
                anyhow::bail!("federation query failed ({}): {}", status, msg);
            }
        }

        let url = format!("{}/api/federation/query", self.base_url);
        let mut req = self.http.post(&url).json(&payload);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req.send().with_context(|| {
            format!(
                "Failed to send federation query request to {}",
                self.base_url
            )
        })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("federation query failed ({}): {}", status, body);
        }
        response
            .json::<Value>()
            .with_context(|| "Failed to parse federation query response")
    }

    pub fn lifecycle_load_pando(&self, payload: LifecycleNamePayload) -> Result<Value> {
        if self.try_uds {
            let body = serde_json::to_vec(&payload)?;
            if let Some((status, resp_body)) =
                uds_request("POST", "/api/lifecycle/load-pando", Some(&body))
            {
                if (200..300).contains(&status) {
                    return serde_json::from_slice(&resp_body)
                        .context("Failed to parse lifecycle load-pando response from UDS");
                }
                let msg = String::from_utf8_lossy(&resp_body).to_string();
                anyhow::bail!("lifecycle load-pando failed ({}): {}", status, msg);
            }
        }

        let url = format!("{}/api/lifecycle/load-pando", self.base_url);
        let mut req = self.http.post(&url).json(&payload);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req.send().with_context(|| {
            format!(
                "Failed to send lifecycle load-pando request to {}",
                self.base_url
            )
        })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("lifecycle load-pando failed ({}): {}", status, body);
        }
        response
            .json::<Value>()
            .with_context(|| "Failed to parse lifecycle load-pando response")
    }

    pub fn lifecycle_refresh(&self, payload: LifecycleRefreshPayload) -> Result<Value> {
        if self.try_uds {
            let body = serde_json::to_vec(&payload)?;
            if let Some((status, resp_body)) =
                uds_request("POST", "/api/lifecycle/refresh", Some(&body))
            {
                if (200..300).contains(&status) {
                    return serde_json::from_slice(&resp_body)
                        .context("Failed to parse lifecycle refresh response from UDS");
                }
                let msg = String::from_utf8_lossy(&resp_body).to_string();
                anyhow::bail!("lifecycle refresh failed ({}): {}", status, msg);
            }
        }

        let url = format!("{}/api/lifecycle/refresh", self.base_url);
        let mut req = self.http.post(&url).json(&payload);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req.send().with_context(|| {
            format!(
                "Failed to send lifecycle refresh request to {}",
                self.base_url
            )
        })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("lifecycle refresh failed ({}): {}", status, body);
        }
        response
            .json::<Value>()
            .with_context(|| "Failed to parse lifecycle refresh response")
    }

    pub fn lifecycle_reload_child(&self, payload: LifecycleNamePayload) -> Result<Value> {
        if self.try_uds {
            let body = serde_json::to_vec(&payload)?;
            if let Some((status, resp_body)) =
                uds_request("POST", "/api/lifecycle/reload-child", Some(&body))
            {
                if (200..300).contains(&status) {
                    return serde_json::from_slice(&resp_body)
                        .context("Failed to parse lifecycle reload-child response from UDS");
                }
                let msg = String::from_utf8_lossy(&resp_body).to_string();
                anyhow::bail!("lifecycle reload-child failed ({}): {}", status, msg);
            }
        }

        let url = format!("{}/api/lifecycle/reload-child", self.base_url);
        let mut req = self.http.post(&url).json(&payload);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req.send().with_context(|| {
            format!(
                "Failed to send lifecycle reload-child request to {}",
                self.base_url
            )
        })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("lifecycle reload-child failed ({}): {}", status, body);
        }
        response
            .json::<Value>()
            .with_context(|| "Failed to parse lifecycle reload-child response")
    }
}

// === UDS client ===
// Small HTTP-over-UDS client — same pattern as secrets/session.rs.
// No reqwest needed for local path. File permissions are auth.

/// Send a GET request over UDS and return the response body.
fn uds_get(path: &str) -> Option<Vec<u8>> {
    let sock_path = mother_socket_path();
    let mut stream = std::os::unix::net::UnixStream::connect(&sock_path).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;

    let request = format!("GET {} HTTP/1.1\r\nHost: localhost\r\n\r\n", path);
    stream.write_all(request.as_bytes()).ok()?;

    let mut response_buf = Vec::new();
    stream.read_to_end(&mut response_buf).ok()?;

    parse_http_body(&response_buf)
}

/// Send a POST request with JSON body over UDS and return the response body.
fn uds_post(path: &str, json_body: &[u8]) -> Option<Vec<u8>> {
    uds_request("POST", path, Some(json_body)).and_then(|(status, body)| {
        if (200..300).contains(&status) {
            Some(body)
        } else {
            None
        }
    })
}

fn uds_request(method: &str, path: &str, json_body: Option<&[u8]>) -> Option<(u16, Vec<u8>)> {
    let sock_path = mother_socket_path();
    let mut stream = std::os::unix::net::UnixStream::connect(&sock_path).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .ok()?;

    let body_len = json_body.map(|b| b.len()).unwrap_or(0);
    let mut request = format!(
        "{} {} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        method, path, body_len
    )
    .into_bytes();
    if let Some(body) = json_body {
        request.extend_from_slice(body);
    }
    stream.write_all(&request).ok()?;

    let mut response_buf = Vec::new();
    stream.read_to_end(&mut response_buf).ok()?;

    parse_http_status_body(&response_buf)
}

/// Extract HTTP response body (everything after \r\n\r\n) if status is 2xx.
fn parse_http_body(response: &[u8]) -> Option<Vec<u8>> {
    let (status, body) = parse_http_status_body(response)?;
    if !(200..300).contains(&status) {
        return None;
    }

    Some(body)
}

fn parse_http_status_body(response: &[u8]) -> Option<(u16, Vec<u8>)> {
    let status_end = response.iter().position(|&b| b == b'\r')?;
    let first_line = std::str::from_utf8(&response[..status_end]).ok()?;
    let status: u16 = first_line.split_whitespace().nth(1)?.parse().ok()?;

    let separator = b"\r\n\r\n";
    let body_start = response
        .windows(4)
        .position(|w| w == separator)
        .map(|p| p + 4)?;

    Some((status, response[body_start..].to_vec()))
}

// === Token + localhost detection ===

/// Check if a URL points to localhost (eligible for UDS).
fn is_localhost(url: &str) -> bool {
    url.contains("://localhost") || url.contains("://127.0.0.1") || url.contains("://[::1]")
}

/// Read bearer token from file or env (same resolution as session.rs).
fn serve_token() -> Option<String> {
    // Try token file first
    let token_path = mother_token_path();
    if let Ok(token) = std::fs::read_to_string(&token_path) {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }
    // Fall back to env var
    std::env::var("PATINA_SERVE_TOKEN").ok()
}

fn mother_socket_path() -> PathBuf {
    std::env::var_os(super::ENV_MOTHER_SOCKET)
        .map(PathBuf::from)
        .unwrap_or_else(paths::serve::socket_path)
}

fn mother_token_path() -> PathBuf {
    std::env::var_os(super::ENV_MOTHER_TOKEN_FILE)
        .map(PathBuf::from)
        .unwrap_or_else(paths::serve::token_path)
}

/// Health check response
#[derive(Debug, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
}

/// Scry request to mother
#[derive(Debug, Serialize)]
pub struct ScryRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimension: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(default)]
    pub all_repos: bool,
    #[serde(default)]
    pub include_issues: bool,
    #[serde(default)]
    pub include_persona: bool,
    pub limit: usize,
    pub min_score: f32,
}

impl Default for ScryRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            dimension: None,
            repo: None,
            all_repos: false,
            include_issues: false,
            include_persona: true,
            limit: 10,
            min_score: 0.0,
        }
    }
}

/// Scry response from mother
#[derive(Debug, Deserialize)]
pub struct ScryResponse {
    pub results: Vec<ScryResultJson>,
    pub count: usize,
}

/// Single result in JSON format (matches server response)
#[derive(Debug, Deserialize)]
pub struct ScryResultJson {
    pub id: i64,
    pub content: String,
    pub score: f32,
    pub event_type: String,
    pub source_id: String,
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_url_normalization() {
        let client = Client::new("localhost:50051".to_string());
        assert_eq!(client.base_url, "http://localhost:50051");

        let client = Client::new("http://localhost:50051".to_string());
        assert_eq!(client.base_url, "http://localhost:50051");

        let client = Client::new("host.docker.internal:50051".to_string());
        assert_eq!(client.base_url, "http://host.docker.internal:50051");
    }

    #[test]
    fn test_client_uds_detection() {
        // Localhost addresses should try UDS
        let client = Client::new("localhost:50051".to_string());
        assert!(client.try_uds);

        let client = Client::new("127.0.0.1:50051".to_string());
        assert!(client.try_uds);

        let client = Client::new("http://localhost:50051".to_string());
        assert!(client.try_uds);

        // Remote addresses should not try UDS
        let client = Client::new("host.docker.internal:50051".to_string());
        assert!(!client.try_uds);

        let client = Client::new("192.168.1.100:50051".to_string());
        assert!(!client.try_uds);
    }

    #[test]
    fn test_is_localhost() {
        assert!(is_localhost("http://localhost:50051"));
        assert!(is_localhost("http://127.0.0.1:50051"));
        assert!(is_localhost("http://[::1]:50051"));
        assert!(!is_localhost("http://host.docker.internal:50051"));
        assert!(!is_localhost("http://192.168.1.100:50051"));
    }

    #[test]
    fn test_parse_http_body_success() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
        let body = parse_http_body(response);
        assert_eq!(body, Some(b"{}".to_vec()));
    }

    #[test]
    fn test_parse_http_body_error_status() {
        let response = b"HTTP/1.1 401 Unauthorized\r\n\r\n{\"error\":\"nope\"}";
        assert!(parse_http_body(response).is_none());
    }

    #[test]
    fn test_parse_http_body_empty() {
        assert!(parse_http_body(b"").is_none());
    }

    #[test]
    fn test_scry_request_serialization() {
        let request = ScryRequest {
            query: "test query".to_string(),
            limit: 5,
            ..Default::default()
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test query"));
        assert!(json.contains("\"limit\":5"));
    }
}
