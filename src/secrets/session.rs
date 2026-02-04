//! Session caching for secrets.
//!
//! Caches decrypted secrets in the `patina serve` daemon to avoid
//! repeated Touch ID prompts within a session.
//!
//! Transport: UDS first (no auth needed), HTTP+token fallback.
//!
//! Flow:
//! 1. `secrets run` checks serve for cached values
//! 2. Cache hit → use cached values (no Touch ID)
//! 3. Cache miss → decrypt locally (Touch ID), cache in serve
//! 4. serve not running → decrypt locally (Touch ID), no caching

use anyhow::Result;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::time::Duration;

use crate::paths;

/// Default cache TTL in seconds (10 minutes).
const DEFAULT_TTL_SECS: u64 = 600;

// === UDS client ===
// Small HTTP-over-UDS client. No reqwest needed for local path.

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
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;

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
    // Find status code in first line: "HTTP/1.1 200 OK\r\n..."
    let status_end = response.iter().position(|&b| b == b'\r')?;
    let first_line = std::str::from_utf8(&response[..status_end]).ok()?;
    let status: u16 = first_line.split_whitespace().nth(1)?.parse().ok()?;
    if !(200..300).contains(&status) {
        return None;
    }

    // Find body separator
    let separator = b"\r\n\r\n";
    let body_start = response
        .windows(4)
        .position(|w| w == separator)
        .map(|p| p + 4)?;

    Some(response[body_start..].to_vec())
}

// === HTTP+token fallback ===

/// Serve daemon base URL (TCP fallback).
fn serve_url() -> String {
    std::env::var("PATINA_SERVE_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string())
}

/// Read bearer token from file or env.
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

// === Public API ===

/// Check if the serve daemon is running (UDS first, then TCP).
pub fn is_serve_running() -> bool {
    // Try UDS
    if uds_get("/health").is_some() {
        return true;
    }

    // Fall back to TCP
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    let mut req = client.get(format!("{}/health", serve_url()));
    if let Some(token) = serve_token() {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    req.send().map(|r| r.status().is_success()).unwrap_or(false)
}

/// Try to get cached secrets from serve.
///
/// Returns None if serve is not running or cache is empty/expired.
pub fn get_cached_secrets() -> Option<HashMap<String, String>> {
    // Try UDS
    if let Some(body) = uds_get("/secrets/cache") {
        if let Ok(secrets) = serde_json::from_slice(&body) {
            return Some(secrets);
        }
    }

    // Fall back to TCP
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;

    let mut req = client.get(format!("{}/secrets/cache", serve_url()));
    if let Some(token) = serve_token() {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let response = req.send().ok()?;
    if !response.status().is_success() {
        return None;
    }

    response.json().ok()
}

/// Store secrets in the serve cache.
///
/// Returns Ok(true) if cached, Ok(false) if serve not running.
pub fn cache_secrets(secrets: &HashMap<String, String>) -> Result<bool> {
    let cache_req = CacheRequest {
        secrets: secrets.clone(),
        ttl_secs: DEFAULT_TTL_SECS,
    };
    let json_body = serde_json::to_vec(&cache_req)?;

    // Try UDS
    if uds_post("/secrets/cache", &json_body).is_some() {
        return Ok(true);
    }

    // Fall back to TCP
    if !is_serve_running() {
        return Ok(false);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let mut req = client
        .post(format!("{}/secrets/cache", serve_url()))
        .header("Content-Type", "application/json")
        .body(json_body);
    if let Some(token) = serve_token() {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let response = req.send()?;
    Ok(response.status().is_success())
}

/// Clear the secrets cache (lock).
///
/// Returns Ok(true) if cleared, Ok(false) if serve not running.
pub fn clear_cache() -> Result<bool> {
    // Try UDS
    if uds_post("/secrets/lock", b"{}").is_some() {
        return Ok(true);
    }

    // Fall back to TCP
    if !is_serve_running() {
        return Ok(false);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let mut req = client.post(format!("{}/secrets/lock", serve_url()));
    if let Some(token) = serve_token() {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let response = req.send()?;
    Ok(response.status().is_success())
}

/// Request body for caching secrets.
#[derive(Debug, serde::Serialize)]
struct CacheRequest {
    secrets: HashMap<String, String>,
    ttl_secs: u64,
}

/// Get secrets with session caching.
///
/// 1. Try cache first (no Touch ID)
/// 2. On miss, call decrypt_fn (may trigger Touch ID)
/// 3. Cache result for next time
pub fn get_secrets_with_cache<F>(decrypt_fn: F) -> Result<HashMap<String, String>>
where
    F: FnOnce() -> Result<HashMap<String, String>>,
{
    // Try cache first
    if let Some(cached) = get_cached_secrets() {
        return Ok(cached);
    }

    // Decrypt (may trigger Touch ID)
    let secrets = decrypt_fn()?;

    // Cache for next time (ignore errors - caching is optional)
    let _ = cache_secrets(&secrets);

    Ok(secrets)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
