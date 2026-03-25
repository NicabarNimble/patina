use anyhow::Result;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::time::Duration;

use crate::secrets_paths as paths;

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

fn serve_url() -> String {
    std::env::var("PATINA_SERVE_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string())
}

fn serve_token() -> Option<String> {
    let token_path = paths::serve::token_path();
    if let Ok(token) = std::fs::read_to_string(&token_path) {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }
    std::env::var("PATINA_SERVE_TOKEN").ok()
}

pub fn is_serve_running() -> bool {
    if uds_get("/health").is_some() {
        return true;
    }

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

pub fn get_cached_secrets() -> Option<HashMap<String, String>> {
    if let Some(body) = uds_get("/secrets/cache") {
        if let Ok(secrets) = serde_json::from_slice(&body) {
            return Some(secrets);
        }
    }

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

pub fn clear_cache() -> Result<bool> {
    if uds_post("/secrets/lock", b"{}").is_some() {
        return Ok(true);
    }

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
