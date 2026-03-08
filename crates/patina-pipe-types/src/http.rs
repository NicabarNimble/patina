//! pipe/http protocol types — proxied HTTP for native children.
//!
//! Native children send pipe/http JSON-RPC requests over stdout to Mother.
//! Mother validates the domain against the manifest allowlist, executes
//! the HTTP request, and returns the response over stdin.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// pipe/http request from a native child to Mother.
///
/// Mirrors reqwest ergonomics: method, url, headers, optional body.
/// Mother validates the URL domain against the child's manifest before
/// executing the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeHttpRequest {
    /// HTTP method: "GET", "POST", "PUT", "PATCH", "DELETE".
    pub method: String,
    /// Full URL including scheme. Must be HTTPS.
    pub url: String,
    /// HTTP headers to include in the request.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Optional request body (for POST, PUT, PATCH).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// pipe/http response from Mother to a native child.
///
/// Contains the HTTP status, response headers, and body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipeHttpResponse {
    /// HTTP status code (200, 404, 500, etc.).
    pub status: u16,
    /// Response headers from the upstream server.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Response body as a string.
    pub body: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serializes() {
        let req = PipeHttpRequest {
            method: "GET".to_string(),
            url: "https://api.example.com/data".to_string(),
            headers: HashMap::from([("Accept".to_string(), "application/json".to_string())]),
            body: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"method\":\"GET\""));
        assert!(json.contains("api.example.com"));
        assert!(!json.contains("\"body\""));
    }

    #[test]
    fn request_with_body_serializes() {
        let req = PipeHttpRequest {
            method: "POST".to_string(),
            url: "https://api.example.com/data".to_string(),
            headers: HashMap::new(),
            body: Some("{\"key\":\"value\"}".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"body\""));
    }

    #[test]
    fn response_roundtrips() {
        let resp = PipeHttpResponse {
            status: 200,
            headers: HashMap::from([("content-type".to_string(), "application/json".to_string())]),
            body: "[{\"id\":1}]".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: PipeHttpResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, 200);
        assert_eq!(parsed.body, "[{\"id\":1}]");
    }

    #[test]
    fn request_deserializes_minimal() {
        let json = r#"{"method":"GET","url":"https://example.com"}"#;
        let req: PipeHttpRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "GET");
        assert!(req.headers.is_empty());
        assert!(req.body.is_none());
    }
}
