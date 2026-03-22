use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Envelope {
    pub fn unsupported_version() -> Self {
        Self {
            v: PROTOCOL_VERSION,
            action: None,
            payload: None,
            result: None,
            error: Some("unsupported protocol version".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectPayload {
    pub agent: String,
    pub project: String,
    pub persona: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPayload {
    pub question: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LakeSyncPayload {
    pub lake: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurePayload {
    pub system: bool,
    pub json: bool,
    pub verb: Option<String>,
    pub full: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecPayload {
    pub subcommand: String,
    pub id: Option<String>,
    pub status: Option<String>,
    pub target: Option<String>,
    pub json: bool,
    pub handoff: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LakeManagePayload {
    pub op: String,
    pub name: Option<String>,
}
