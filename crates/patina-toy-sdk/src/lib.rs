use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskIntent {
    pub kind: TaskIntentKind,
    pub payload_json: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedupe_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingEvent {
    pub stream_name: String,
    pub offset: u64,
    pub event_type: String,
    pub payload_json: String,
    pub occurred_at: String,
}

pub trait LogBackend {
    fn debug(message: &str);
    fn info(message: &str);
    fn warn(message: &str);
    fn error(message: &str);
}

pub trait MeasureBackend {
    fn record(verb: &str, tool: &str, mode: &str, metrics_json: &str) -> Result<(), String>;
}

pub trait QueryBackend {
    fn query(kind: &str, params_json: &str) -> Result<String, String>;
}

pub trait FetchBackend {
    fn get(url: &str) -> Result<String, String>;
    fn post(url: &str, body: &str, content_type: &str) -> Result<String, String>;
}

pub trait EmitBackend {
    fn emit(schema: &str, fact_type: &str, data: &str) -> Result<u64, String>;
}

pub trait StateBackend {
    fn get(key: &str) -> Option<String>;
    fn put(key: &str, value_json: &str) -> Result<(), String>;
    fn delete(key: &str) -> Result<(), String>;
    fn list_prefix(prefix: &str) -> Vec<String>;
}

pub trait CheckpointBackend {
    fn load(stream: &str) -> Option<String>;
    fn save(stream: &str, checkpoint_json: &str) -> Result<(), String>;
}

pub trait LakeBackend {
    fn ensure_lake(name: &str) -> Result<String, String>;
    fn load_cursor(lake: &str, source: &str, data_type: &str) -> Option<String>;
    fn save_cursor(
        lake: &str,
        source: &str,
        data_type: &str,
        cursor: Option<&str>,
        written: u64,
        status: &str,
        last_error: Option<&str>,
    ) -> Result<(), String>;
    fn ensure_table(lake: &str, table: &str) -> Result<(), String>;
    fn append_json_batch(
        lake: &str,
        table: &str,
        source: &str,
        rows_json: &[String],
    ) -> Result<u64, String>;
    fn query_json(lake: &str, sql: &str) -> Result<String, String>;
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
pub struct LogToy<B>(std::marker::PhantomData<B>);
impl<B> LogToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: LogBackend> LogToy<B> {
    pub fn debug(&self, message: &str) {
        B::debug(message);
    }
    pub fn info(&self, message: &str) {
        B::info(message);
    }
    pub fn warn(&self, message: &str) {
        B::warn(message);
    }
    pub fn error(&self, message: &str) {
        B::error(message);
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MeasureToy<B>(std::marker::PhantomData<B>);
impl<B> MeasureToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: MeasureBackend> MeasureToy<B> {
    pub fn record(&self, verb: &str, tool: &str, mode: &str, metrics_json: &str) -> Result<(), String> {
        B::record(verb, tool, mode, metrics_json)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct QueryToy<B>(std::marker::PhantomData<B>);
impl<B> QueryToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: QueryBackend> QueryToy<B> {
    pub fn query(&self, kind: &str, params_json: &str) -> Result<String, String> {
        B::query(kind, params_json)
    }
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

#[derive(Debug, Clone, Copy, Default)]
pub struct EmitToy<B>(std::marker::PhantomData<B>);
impl<B> EmitToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: EmitBackend> EmitToy<B> {
    pub fn emit(&self, schema: &str, fact_type: &str, data: &str) -> Result<u64, String> {
        B::emit(schema, fact_type, data)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StateToy<B>(std::marker::PhantomData<B>);
impl<B> StateToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: StateBackend> StateToy<B> {
    pub fn get(&self, key: &str) -> Option<String> {
        B::get(key)
    }
    pub fn put(&self, key: &str, value_json: &str) -> Result<(), String> {
        B::put(key, value_json)
    }
    pub fn delete(&self, key: &str) -> Result<(), String> {
        B::delete(key)
    }
    pub fn list_prefix(&self, prefix: &str) -> Vec<String> {
        B::list_prefix(prefix)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CheckpointToy<B>(std::marker::PhantomData<B>);
impl<B> CheckpointToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: CheckpointBackend> CheckpointToy<B> {
    pub fn load(&self, stream: &str) -> Option<String> {
        B::load(stream)
    }
    pub fn save(&self, stream: &str, checkpoint_json: &str) -> Result<(), String> {
        B::save(stream, checkpoint_json)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LakeToy<B>(std::marker::PhantomData<B>);
impl<B> LakeToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: LakeBackend> LakeToy<B> {
    pub fn ensure_lake(&self, name: &str) -> Result<String, String> {
        B::ensure_lake(name)
    }
    pub fn load_cursor(&self, lake: &str, source: &str, data_type: &str) -> Option<String> {
        B::load_cursor(lake, source, data_type)
    }
    pub fn save_cursor(
        &self,
        lake: &str,
        source: &str,
        data_type: &str,
        cursor: Option<&str>,
        written: u64,
        status: &str,
        last_error: Option<&str>,
    ) -> Result<(), String> {
        B::save_cursor(lake, source, data_type, cursor, written, status, last_error)
    }
    pub fn ensure_table(&self, lake: &str, table: &str) -> Result<(), String> {
        B::ensure_table(lake, table)
    }
    pub fn append_json_batch(
        &self,
        lake: &str,
        table: &str,
        source: &str,
        rows_json: &[String],
    ) -> Result<u64, String> {
        B::append_json_batch(lake, table, source, rows_json)
    }
    pub fn query_json(&self, lake: &str, sql: &str) -> Result<String, String> {
        B::query_json(lake, sql)
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
