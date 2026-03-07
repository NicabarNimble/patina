//! JSON-RPC 2.0 protocol types for pipe protocol.
//!
//! Same shape as `src/mcp/protocol.rs`, different vocabulary.
//! Pipe protocol methods: pipe/initialize, pipe/fetch, pipe/health,
//! pipe/shutdown, pipe/fact (notification).

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Serialize)]
pub struct Response {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}

#[derive(Serialize)]
pub struct Error {
    pub code: i32,
    pub message: String,
}

/// JSON-RPC notification (no `id` field). Used for pipe/fact streaming.
#[derive(Serialize)]
pub struct Notification {
    pub jsonrpc: &'static str,
    pub method: &'static str,
    pub params: serde_json::Value,
}

impl Response {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<serde_json::Value>, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(Error {
                code,
                message: message.to_string(),
            }),
        }
    }
}

impl Notification {
    pub fn fact(params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            method: "pipe/fact",
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_success_serializes() {
        let resp = Response::success(Some(serde_json::json!(1)), serde_json::json!({"ok": true}));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"ok\":true"));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn response_error_serializes() {
        let resp = Response::error(Some(serde_json::json!(2)), -32601, "Method not found");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"code\":-32601"));
        assert!(json.contains("Method not found"));
        assert!(!json.contains("\"result\""));
    }

    #[test]
    fn notification_fact_no_id() {
        let notif = Notification::fact(serde_json::json!({"schema": "github", "data": {}}));
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("\"method\":\"pipe/fact\""));
        assert!(!json.contains("\"id\""));
    }

    #[test]
    fn request_deserializes() {
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"pipe/initialize","params":{}}"#;
        let req: Request = serde_json::from_str(input).unwrap();
        assert_eq!(req.method, "pipe/initialize");
        assert_eq!(req.id, Some(serde_json::json!(1)));
    }

    #[test]
    fn request_no_params() {
        let input = r#"{"jsonrpc":"2.0","id":1,"method":"pipe/shutdown"}"#;
        let req: Request = serde_json::from_str(input).unwrap();
        assert_eq!(req.method, "pipe/shutdown");
        assert!(req.params.is_null());
    }
}
