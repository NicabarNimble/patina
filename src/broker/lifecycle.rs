//! BrokerChild trait and adapters for unified child lifecycle.
//!
//! BrokerChild is Mother-side — it wraps the communication channel.
//! NativeChild wraps a subprocess (ChildConnection from patina-pipe).
//! WasmBrokerChild is stubbed (WASM routing via legacy host_emit, §5).

use anyhow::{bail, Context, Result};
use patina_pipe::harness::ChildConnection;
use std::collections::HashMap;

/// Parameters for a fetch operation.
#[derive(Debug, Clone)]
pub struct FetchParams {
    pub types: Vec<String>,
    pub since: Option<String>,
    pub params: HashMap<String, toml::Value>,
    pub limit: Option<u64>,
}

impl FetchParams {
    pub fn to_json(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();

        if !self.types.is_empty() {
            obj.insert(
                "types".to_string(),
                serde_json::Value::Array(
                    self.types
                        .iter()
                        .map(|t| serde_json::Value::String(t.clone()))
                        .collect(),
                ),
            );
        }

        if let Some(ref since) = self.since {
            obj.insert(
                "since".to_string(),
                serde_json::Value::String(since.clone()),
            );
        }

        if !self.params.is_empty() {
            let params_obj: serde_json::Map<String, serde_json::Value> = self
                .params
                .iter()
                .filter_map(|(k, v)| {
                    serde_json::to_value(v).ok().map(|jv| (k.clone(), jv))
                })
                .collect();
            obj.insert("params".to_string(), serde_json::Value::Object(params_obj));
        }

        if let Some(limit) = self.limit {
            obj.insert(
                "limit".to_string(),
                serde_json::Value::Number(serde_json::Number::from(limit)),
            );
        }

        serde_json::Value::Object(obj)
    }
}

/// A fact emitted by a child during fetch.
#[derive(Debug, Clone)]
pub struct BrokerFact {
    pub schema: String,
    pub fact_type: String,
    pub data: serde_json::Value,
    pub content_hash: Option<String>,
}

/// Result of a fetch operation.
#[derive(Debug, Clone)]
pub struct FetchResult {
    pub emitted: u64,
    pub cursor: Option<String>,
}

/// Health status of a child.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub status: String,
}

/// Maximum batch size before aborting (safety net for memory, §13).
pub const DEFAULT_MAX_BATCH_SIZE: usize = 10_000;

/// Unified trait for broker-managed children (native + WASM).
pub trait BrokerChild {
    fn name(&self) -> &str;
    fn fetch(
        &mut self,
        params: &FetchParams,
        on_fact: &mut dyn FnMut(BrokerFact) -> Result<()>,
    ) -> Result<FetchResult>;
    fn health(&self) -> Result<HealthStatus>;
    fn shutdown(&mut self) -> Result<()>;
}

/// Native child adapter wrapping ChildConnection.
pub struct NativeChild {
    child_name: String,
    conn: ChildConnection,
    max_batch_size: usize,
}

impl NativeChild {
    pub fn new(child_name: String, conn: ChildConnection) -> Self {
        Self {
            child_name,
            conn,
            max_batch_size: DEFAULT_MAX_BATCH_SIZE,
        }
    }

    pub fn with_max_batch_size(mut self, size: usize) -> Self {
        self.max_batch_size = size;
        self
    }
}

/// Parse a pipe/fact notification into a BrokerFact.
fn parse_pipe_fact(notif: &serde_json::Value) -> Result<BrokerFact> {
    let params = notif
        .get("params")
        .with_context(|| "pipe/fact missing params")?;

    let schema = params
        .get("schema")
        .and_then(|s| s.as_str())
        .with_context(|| "pipe/fact missing schema")?
        .to_string();

    let fact_type = params
        .get("fact_type")
        .and_then(|s| s.as_str())
        .with_context(|| "pipe/fact missing fact_type")?
        .to_string();

    let data = params
        .get("data")
        .cloned()
        .with_context(|| "pipe/fact missing data")?;

    let content_hash = params
        .get("content_hash")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    Ok(BrokerFact {
        schema,
        fact_type,
        data,
        content_hash,
    })
}

impl BrokerChild for NativeChild {
    fn name(&self) -> &str {
        &self.child_name
    }

    fn fetch(
        &mut self,
        params: &FetchParams,
        on_fact: &mut dyn FnMut(BrokerFact) -> Result<()>,
    ) -> Result<FetchResult> {
        let (notifications, response) = self
            .conn
            .request("pipe/fetch", params.to_json())
            .map_err(|e| anyhow::anyhow!("pipe/fetch failed: {}", e))?;

        // Check batch size limit (§13 safety net)
        if notifications.len() > self.max_batch_size {
            bail!(
                "{}: batch limit exceeded ({} > {}). Configure smaller page size via source params or increase max_batch_size.",
                self.child_name,
                notifications.len(),
                self.max_batch_size
            );
        }

        // Process facts with error attribution tracing (§13)
        for (i, notif) in notifications.iter().enumerate() {
            let fact = parse_pipe_fact(notif).with_context(|| {
                format!(
                    "child '{}': malformed pipe/fact at position {}",
                    self.child_name, i
                )
            })?;
            on_fact(fact).with_context(|| {
                format!(
                    "broker: validation/write error for fact {} from child '{}'",
                    i, self.child_name
                )
            })?;
        }

        // Parse fetch result
        let result = response
            .get("result")
            .with_context(|| "pipe/fetch response missing result")?;

        let emitted = result
            .get("emitted")
            .and_then(|e| e.as_u64())
            .unwrap_or(0);

        let cursor = result
            .get("cursor")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        Ok(FetchResult { emitted, cursor })
    }

    fn health(&self) -> Result<HealthStatus> {
        // Health check requires mutable access for request(), but trait says &self.
        // For now return unknown — health checks happen via separate call.
        Ok(HealthStatus {
            status: "unknown".to_string(),
        })
    }

    fn shutdown(&mut self) -> Result<()> {
        let (_notifs, response) = self
            .conn
            .request("pipe/shutdown", serde_json::json!({}))
            .map_err(|e| anyhow::anyhow!("pipe/shutdown failed: {}", e))?;

        if response.get("error").is_some() {
            bail!(
                "child '{}' shutdown error: {}",
                self.child_name,
                response["error"]
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_params_to_json_full() {
        let params = FetchParams {
            types: vec!["issues".to_string(), "prs".to_string()],
            since: Some("2026-03-07T00:00:00Z".to_string()),
            params: HashMap::from([
                ("owner".to_string(), toml::Value::String("NicabarNimble".to_string())),
                ("repo".to_string(), toml::Value::String("patina".to_string())),
            ]),
            limit: Some(100),
        };

        let json = params.to_json();
        assert_eq!(json["types"], serde_json::json!(["issues", "prs"]));
        assert_eq!(json["since"], "2026-03-07T00:00:00Z");
        assert_eq!(json["limit"], 100);
        assert_eq!(json["params"]["owner"], "NicabarNimble");
    }

    #[test]
    fn fetch_params_to_json_minimal() {
        let params = FetchParams {
            types: vec![],
            since: None,
            params: HashMap::new(),
            limit: None,
        };

        let json = params.to_json();
        assert!(json.get("types").is_none());
        assert!(json.get("since").is_none());
        assert!(json.get("limit").is_none());
    }

    #[test]
    fn parse_pipe_fact_valid() {
        let notif = serde_json::json!({
            "method": "pipe/fact",
            "params": {
                "schema": "github",
                "fact_type": "issue",
                "data": {"title": "test"},
                "content_hash": "blake3:abc123"
            }
        });

        let fact = parse_pipe_fact(&notif).unwrap();
        assert_eq!(fact.schema, "github");
        assert_eq!(fact.fact_type, "issue");
        assert_eq!(fact.content_hash.as_deref(), Some("blake3:abc123"));
    }

    #[test]
    fn parse_pipe_fact_missing_schema() {
        let notif = serde_json::json!({
            "method": "pipe/fact",
            "params": {
                "fact_type": "issue",
                "data": {"title": "test"}
            }
        });

        assert!(parse_pipe_fact(&notif).is_err());
    }
}
