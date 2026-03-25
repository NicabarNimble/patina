use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

pub trait KnowledgeChild: Send + Sync {
    fn name(&self) -> &str;

    fn on_load(&mut self, host: &dyn MotherHost) -> Result<()>;

    fn on_unload(&mut self) {}

    fn health(&self) -> ChildHealth;

    fn handle(&self, request: &ChildRequest) -> Result<ChildResponse>;

    fn drain(&mut self, _limit: u32) -> Result<Vec<PendingEvent>> {
        Ok(vec![])
    }

    fn tick(&mut self) -> Vec<TaskIntent> {
        vec![]
    }
}

#[derive(Debug, Clone)]
pub enum ChildHealth {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

impl fmt::Display for ChildHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChildHealth::Healthy => write!(f, "healthy"),
            ChildHealth::Degraded(reason) => write!(f, "degraded: {}", reason),
            ChildHealth::Unhealthy(reason) => write!(f, "unhealthy: {}", reason),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChildRequest {
    pub action: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ChildResponse {
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskIntentKind {
    FetchSource,
    RunQuery,
    EmitFacts,
    MaterializeIndex,
    VerifyBelief,
    SyncGraph,
    RefreshCredential,
    NativeJob,
}

impl TaskIntentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FetchSource => "fetch-source",
            Self::RunQuery => "run-query",
            Self::EmitFacts => "emit-facts",
            Self::MaterializeIndex => "materialize-index",
            Self::VerifyBelief => "verify-belief",
            Self::SyncGraph => "sync-graph",
            Self::RefreshCredential => "refresh-credential",
            Self::NativeJob => "native-job",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "fetch-source" => Some(Self::FetchSource),
            "run-query" => Some(Self::RunQuery),
            "emit-facts" => Some(Self::EmitFacts),
            "materialize-index" => Some(Self::MaterializeIndex),
            "verify-belief" => Some(Self::VerifyBelief),
            "sync-graph" => Some(Self::SyncGraph),
            "refresh-credential" => Some(Self::RefreshCredential),
            "native-job" => Some(Self::NativeJob),
            _ => None,
        }
    }
}

impl fmt::Display for TaskIntentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskIntent {
    pub kind: TaskIntentKind,
    pub payload: serde_json::Value,
    pub dedupe_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingEvent {
    pub stream: String,
    pub offset: u64,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub occurred_at: String,
}

#[derive(Debug, Clone)]
pub struct Toy {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

pub trait MotherHost: Send + Sync {
    fn log(&self, child: &str, message: &str);
}
