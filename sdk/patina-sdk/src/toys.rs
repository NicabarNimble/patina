use serde::{Deserialize, Serialize};

pub use patina_sdk_agent::{EmitBackend, EmitToy, QueryBackend, QueryToy};
pub use patina_sdk_core::{
    LogBackend, LogToy, PendingEvent, StateBackend, StateToy, TaskIntent, TaskIntentKind,
};
pub use patina_sdk_data::{
    CheckpointBackend, CheckpointToy, ConnectorBackend, ConnectorBinding, ConnectorCatalog,
    ConnectorSyncResult, GrantedConnectorBinding, GrantedLake, LakeBackend, LakeCatalog,
    LakeCursorRecord, LakeToy, MeasureBackend, MeasureToy,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantedIngressSource {
    pub name: String,
    pub endpoint: String,
}

pub trait FetchBackend {
    fn get(url: &str) -> Result<String, String>;
    fn post(url: &str, body: &str, content_type: &str) -> Result<String, String>;
}

pub trait IngressBackend {
    fn list_granted_sources() -> Vec<GrantedIngressSource>;
    fn fetch(source: &str) -> Result<String, String>;
}

pub trait EventBackend {
    fn pull(
        stream: &str,
        after_offset: Option<u64>,
        limit: u32,
    ) -> Result<Vec<PendingEvent>, String>;
    fn ack_through(stream: &str, offset: u64) -> Result<(), String>;
    fn list_streams() -> Vec<String>;
}

pub trait TaskBackend {
    fn enqueue(intent: &TaskIntent) -> Result<String, String>;
}

pub trait GraphBackend {
    fn query(kind: &str, params_json: &str) -> Result<String, String>;
    fn mutate(action: &str, payload_json: &str) -> Result<(), String>;
}

pub trait BeliefBackend {
    fn query(kind: &str, params_json: &str) -> Result<String, String>;
    fn mutate(action: &str, payload_json: &str) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FetchToy<B>(std::marker::PhantomData<B>);
impl<B> FetchToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: FetchBackend> FetchToy<B> {
    pub fn get(&self, url: &str) -> Result<String, String> {
        B::get(url)
    }
    pub fn post(&self, url: &str, body: &str, content_type: &str) -> Result<String, String> {
        B::post(url, body, content_type)
    }
}

#[derive(Debug, Clone)]
pub struct IngressToy<B> {
    granted: GrantedIngressSource,
    _marker: std::marker::PhantomData<B>,
}
impl<B> IngressToy<B> {
    pub fn new(granted: GrantedIngressSource) -> Self {
        Self {
            granted,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn grant(&self) -> &GrantedIngressSource {
        &self.granted
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct IngressCatalog<B>(std::marker::PhantomData<B>);
impl<B> IngressCatalog<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: IngressBackend> IngressToy<B> {
    pub fn fetch(&self) -> Result<String, String> {
        B::fetch(&self.granted.name)
    }
}
impl<B: IngressBackend> IngressCatalog<B> {
    pub fn list(&self) -> Vec<IngressToy<B>> {
        B::list_granted_sources()
            .into_iter()
            .map(IngressToy::new)
            .collect()
    }

    pub fn require(&self, name: &str) -> Result<IngressToy<B>, String> {
        self.list()
            .into_iter()
            .find(|source| source.grant().name == name)
            .ok_or_else(|| format!("ingress source '{}' not granted", name))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EventToy<B>(std::marker::PhantomData<B>);
impl<B> EventToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: EventBackend> EventToy<B> {
    pub fn pull(
        &self,
        stream: &str,
        after_offset: Option<u64>,
        limit: u32,
    ) -> Result<Vec<PendingEvent>, String> {
        B::pull(stream, after_offset, limit)
    }
    pub fn ack_through(&self, stream: &str, offset: u64) -> Result<(), String> {
        B::ack_through(stream, offset)
    }
    pub fn list_streams(&self) -> Vec<String> {
        B::list_streams()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TaskToy<B>(std::marker::PhantomData<B>);
impl<B> TaskToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: TaskBackend> TaskToy<B> {
    pub fn enqueue(&self, intent: &TaskIntent) -> Result<String, String> {
        B::enqueue(intent)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GraphToy<B>(std::marker::PhantomData<B>);
impl<B> GraphToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: GraphBackend> GraphToy<B> {
    pub fn query(&self, kind: &str, params_json: &str) -> Result<String, String> {
        B::query(kind, params_json)
    }
    pub fn mutate(&self, action: &str, payload_json: &str) -> Result<(), String> {
        B::mutate(action, payload_json)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BeliefToy<B>(std::marker::PhantomData<B>);
impl<B> BeliefToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: BeliefBackend> BeliefToy<B> {
    pub fn query(&self, kind: &str, params_json: &str) -> Result<String, String> {
        B::query(kind, params_json)
    }
    pub fn mutate(&self, action: &str, payload_json: &str) -> Result<(), String> {
        B::mutate(action, payload_json)
    }
}
