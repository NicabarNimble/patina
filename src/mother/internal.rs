//! Internal HTTP client implementation for mother
//!
//! Transport model (matches session.rs pattern):
//! - Local mother: UDS first (no auth needed — file permissions are auth)
//! - Remote mother: TCP with bearer token via reqwest

use anyhow::{Context, Result};
use mother_crate::bridge::{BridgeRequest, BridgeResponse};
use mother_crate::protocol::{
    FederationQueryPayload, LifecycleNamePayload, LifecycleRefreshPayload, LifecycleWarmupPayload,
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

    /// Fast readiness check for launcher preflight.
    pub fn ready(&self) -> Result<()> {
        if self.try_uds {
            if let Some((status, body)) = uds_request("GET", "/ready", None) {
                if status == 204 {
                    return Ok(());
                }
                let message = String::from_utf8_lossy(&body).trim().to_string();
                if message.is_empty() {
                    anyhow::bail!("Mother ready probe failed ({})", status);
                }
                anyhow::bail!("Mother ready probe failed ({}): {}", status, message);
            }
        }

        let url = format!("{}/ready", self.base_url);
        let mut req = self.http.get(&url);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req
            .send()
            .with_context(|| format!("Failed to connect to mother at {}", self.base_url))?;

        if response.status().as_u16() == 204 {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().unwrap_or_default();
        if body.trim().is_empty() {
            anyhow::bail!("Mother ready probe failed ({})", status);
        }
        anyhow::bail!("Mother ready probe failed ({}): {}", status, body);
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
                    return serde_json::from_slice(&resp_body).with_context(|| {
                        let preview = String::from_utf8_lossy(&resp_body)
                            .chars()
                            .take(200)
                            .collect::<String>();
                        format!(
                            "Failed to parse child response from UDS (status={}, bytes={}, preview={:?})",
                            status,
                            resp_body.len(),
                            preview
                        )
                    });
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

    pub fn child_call(&self, child: &str, operation_id: &str, args: &Value) -> Result<Value> {
        let payload = serde_json::json!({
            "operation_id": operation_id,
            "args": args,
        });
        self.child_action(child, "call", &payload)
    }

    pub fn interface_control_call(
        &self,
        operation_id: &str,
        args: Value,
        correlation: Option<Value>,
    ) -> Result<Value> {
        let payload = serde_json::json!({
            "operation_id": operation_id,
            "args": args,
            "correlation": correlation,
        });

        if self.try_uds {
            let body = serde_json::to_vec(&payload)?;
            if let Some((status, resp_body)) =
                uds_request("POST", "/api/interface/call", Some(&body))
            {
                if (200..300).contains(&status) {
                    return serde_json::from_slice(&resp_body)
                        .context("Failed to parse interface control response from UDS");
                }
                let msg = String::from_utf8_lossy(&resp_body).to_string();
                anyhow::bail!("interface control call failed ({}): {}", status, msg);
            }
        }

        let url = format!("{}/api/interface/call", self.base_url);
        let mut req = self.http.post(&url).json(&payload);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req.send().with_context(|| {
            format!(
                "Failed to send interface control call request to {}",
                self.base_url
            )
        })?;

        let status = response.status();
        let body = response.text().unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!("interface control call failed ({}): {}", status, body);
        }
        serde_json::from_str(&body).with_context(|| "Failed to parse interface control response")
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

    pub fn bridge_translate(&self, request: &BridgeRequest) -> Result<BridgeResponse> {
        if self.try_uds {
            let body = serde_json::to_vec(request)?;
            if let Some((status, resp_body)) =
                uds_request("POST", "/api/bridge/translate", Some(&body))
            {
                if (200..300).contains(&status) {
                    return serde_json::from_slice(&resp_body)
                        .context("Failed to parse bridge translate response from UDS");
                }
                let msg = String::from_utf8_lossy(&resp_body).to_string();
                anyhow::bail!("bridge translate failed ({}): {}", status, msg);
            }
        }

        let url = format!("{}/api/bridge/translate", self.base_url);
        let mut req = self.http.post(&url).json(request);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req.send().with_context(|| {
            format!("Failed to request bridge translate from {}", self.base_url)
        })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("bridge translate failed ({}): {}", status, body);
        }
        response
            .json::<BridgeResponse>()
            .with_context(|| "Failed to parse bridge translate response")
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

    pub fn lifecycle_warmup_children(&self, payload: LifecycleWarmupPayload) -> Result<Value> {
        if self.try_uds {
            let body = serde_json::to_vec(&payload)?;
            if let Some((status, resp_body)) =
                uds_request("POST", "/api/lifecycle/warmup-children", Some(&body))
            {
                if (200..300).contains(&status) {
                    return serde_json::from_slice(&resp_body)
                        .context("Failed to parse lifecycle warmup-children response from UDS");
                }
                let msg = String::from_utf8_lossy(&resp_body).to_string();
                anyhow::bail!("lifecycle warmup-children failed ({}): {}", status, msg);
            }
        }

        let url = format!("{}/api/lifecycle/warmup-children", self.base_url);
        let mut req = self.http.post(&url).json(&payload);
        if let Some(ref token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        let response = req.send().with_context(|| {
            format!(
                "Failed to send lifecycle warmup-children request to {}",
                self.base_url
            )
        })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("lifecycle warmup-children failed ({}): {}", status, body);
        }
        response
            .json::<Value>()
            .with_context(|| "Failed to parse lifecycle warmup-children response")
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

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        path
    );
    stream.write_all(request.as_bytes()).ok()?;

    let response_buf = read_http_response(&mut stream)?;

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
        "{} {} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        method, path, body_len
    )
    .into_bytes();
    if let Some(body) = json_body {
        request.extend_from_slice(body);
    }
    stream.write_all(&request).ok()?;

    let response_buf = read_http_response(&mut stream)?;

    parse_http_status_body(&response_buf)
}

fn read_http_response(stream: &mut std::os::unix::net::UnixStream) -> Option<Vec<u8>> {
    let mut response_buf = Vec::new();
    let mut chunk = [0u8; 8192];

    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                response_buf.extend_from_slice(&chunk[..n]);
                if let Some(expected) = expected_http_response_len(&response_buf) {
                    if response_buf.len() >= expected {
                        response_buf.truncate(expected);
                        return Some(response_buf);
                    }
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut =>
            {
                if let Some(expected) = expected_http_response_len(&response_buf) {
                    if response_buf.len() >= expected {
                        response_buf.truncate(expected);
                        return Some(response_buf);
                    }
                }
                if !response_buf.is_empty() {
                    break;
                }
                return None;
            }
            Err(_) => {
                if response_buf.is_empty() {
                    return None;
                }
                break;
            }
        }
    }

    if response_buf.is_empty() {
        None
    } else {
        Some(response_buf)
    }
}

fn expected_http_response_len(response: &[u8]) -> Option<usize> {
    let separator = b"\r\n\r\n";
    let headers_end = response.windows(4).position(|w| w == separator)?;
    let body_start = headers_end + 4;

    let header_text = std::str::from_utf8(&response[..headers_end]).ok()?;
    for line in header_text.split("\r\n").skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            let len = value.trim().parse::<usize>().ok()?;
            return Some(body_start + len);
        }
    }

    None
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
    let separator = b"\r\n\r\n";
    let headers_end = response.windows(4).position(|w| w == separator)?;
    let body_start = headers_end + 4;

    let header_text = std::str::from_utf8(&response[..headers_end]).ok()?;
    let mut lines = header_text.split("\r\n");

    let status_line = lines.next()?;
    let status: u16 = status_line.split_whitespace().nth(1)?.parse().ok()?;

    let mut content_length: Option<usize> = None;
    let mut chunked = false;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        if name == "content-length" {
            content_length = value.parse::<usize>().ok();
        } else if name == "transfer-encoding"
            && value
                .to_ascii_lowercase()
                .split(',')
                .any(|part| part.trim() == "chunked")
        {
            chunked = true;
        }
    }

    let raw_body = &response[body_start..];
    let body = if chunked {
        decode_chunked_body(raw_body)?
    } else if let Some(len) = content_length {
        if raw_body.len() < len {
            return None;
        }
        raw_body[..len].to_vec()
    } else {
        raw_body.to_vec()
    };

    Some((status, body))
}

fn decode_chunked_body(input: &[u8]) -> Option<Vec<u8>> {
    let mut cursor = 0usize;
    let mut out = Vec::new();

    loop {
        let size_line_end_rel = input[cursor..].windows(2).position(|w| w == b"\r\n")?;
        let size_line_end = cursor + size_line_end_rel;
        let size_line = std::str::from_utf8(&input[cursor..size_line_end]).ok()?;
        let size_hex = size_line.split(';').next()?.trim();
        let size = usize::from_str_radix(size_hex, 16).ok()?;
        cursor = size_line_end + 2;

        if size == 0 {
            return Some(out);
        }

        if input.len() < cursor + size + 2 {
            return None;
        }

        out.extend_from_slice(&input[cursor..cursor + size]);
        cursor += size;

        if &input[cursor..cursor + 2] != b"\r\n" {
            return None;
        }
        cursor += 2;
    }
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
    fn test_parse_http_body_respects_content_length() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}TRAILING";
        let body = parse_http_body(response);
        assert_eq!(body, Some(b"{}".to_vec()));
    }

    #[test]
    fn test_parse_http_body_chunked() {
        let response = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}\r\n0\r\n\r\n";
        let body = parse_http_body(response);
        assert_eq!(body, Some(b"{}".to_vec()));
    }

    #[test]
    fn test_expected_http_response_len_content_length() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}TRAIL";
        let expected = expected_http_response_len(response);
        assert_eq!(expected, Some(40));
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
