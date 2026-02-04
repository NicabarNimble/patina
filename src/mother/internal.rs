//! Internal HTTP client implementation for mother
//!
//! Transport model (matches session.rs pattern):
//! - Local mother: UDS first (no auth needed — file permissions are auth)
//! - Remote mother: TCP with bearer token via reqwest

use anyhow::{Context, Result};
use reqwest::blocking::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
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
}

// === UDS client ===
// Small HTTP-over-UDS client — same pattern as secrets/session.rs.
// No reqwest needed for local path. File permissions are auth.

/// Send a GET request over UDS and return the response body.
fn uds_get(path: &str) -> Option<Vec<u8>> {
    let sock_path = paths::serve::socket_path();
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
    let sock_path = paths::serve::socket_path();
    let mut stream = std::os::unix::net::UnixStream::connect(&sock_path).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .ok()?;

    let request = format!(
        "POST {} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        path,
        json_body.len()
    );
    stream.write_all(request.as_bytes()).ok()?;
    stream.write_all(json_body).ok()?;

    let mut response_buf = Vec::new();
    stream.read_to_end(&mut response_buf).ok()?;

    parse_http_body(&response_buf)
}

/// Extract HTTP response body (everything after \r\n\r\n) if status is 2xx.
fn parse_http_body(response: &[u8]) -> Option<Vec<u8>> {
    let status_end = response.iter().position(|&b| b == b'\r')?;
    let first_line = std::str::from_utf8(&response[..status_end]).ok()?;
    let status: u16 = first_line.split_whitespace().nth(1)?.parse().ok()?;
    if !(200..300).contains(&status) {
        return None;
    }

    let separator = b"\r\n\r\n";
    let body_start = response
        .windows(4)
        .position(|w| w == separator)
        .map(|p| p + 4)?;

    Some(response[body_start..].to_vec())
}

// === Token + localhost detection ===

/// Check if a URL points to localhost (eligible for UDS).
fn is_localhost(url: &str) -> bool {
    url.contains("://localhost") || url.contains("://127.0.0.1") || url.contains("://[::1]")
}

/// Read bearer token from file or env (same resolution as session.rs).
fn serve_token() -> Option<String> {
    // Try token file first
    let token_path = paths::serve::token_path();
    if let Ok(token) = std::fs::read_to_string(&token_path) {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }
    // Fall back to env var
    std::env::var("PATINA_SERVE_TOKEN").ok()
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
