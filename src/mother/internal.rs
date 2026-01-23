//! Internal HTTP client implementation for mother

use anyhow::{Context, Result};
use reqwest::blocking::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Mother client
pub struct Client {
    base_url: String,
    http: HttpClient,
}

impl Client {
    /// Create a new client for the given address (host:port or just host)
    pub fn new(address: String) -> Self {
        let base_url = if address.starts_with("http://") || address.starts_with("https://") {
            address
        } else {
            format!("http://{}", address)
        };

        let http = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { base_url, http }
    }

    /// Health check - returns Ok if mother is reachable
    pub fn health(&self) -> Result<HealthResponse> {
        let url = format!("{}/health", self.base_url);
        let response = self
            .http
            .get(&url)
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
        let url = format!("{}/api/scry", self.base_url);
        let response = self
            .http
            .post(&url)
            .json(&request)
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
