use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ProjectUid(pub String);

impl ProjectUid {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct PersonaUid(pub String);

impl PersonaUid {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct InterfaceKindId(pub String);

impl InterfaceKindId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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
    /// Caller identity (interface/runtime host).
    pub agent: String,
    /// Stable project identity from `.patina/uid`.
    pub project_uid: ProjectUid,
    /// Interface runtime kind (`opencode`, `claude`, `gemini`, ...).
    pub interface_kind: InterfaceKindId,
    /// Optional human/debug path. Not authoritative for identity.
    #[serde(default, alias = "project")]
    pub project_root: Option<String>,
    /// Persona namespace selector.
    ///
    /// Pre-v1 this is an opaque UID/string. Future phases will bind this to
    /// cryptographic persona identity and capability verification.
    pub persona: Option<PersonaUid>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScryPayload {
    pub query: String,
    pub dimension: Option<String>,
    pub repo: Option<String>,
    pub all_repos: bool,
    pub include_issues: bool,
    pub include_persona: bool,
    pub limit: usize,
    pub min_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FederationStatusPayload {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FederationRefreshPayload {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationQueryPayload {
    pub sql: String,
    #[serde(default)]
    pub params: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}
