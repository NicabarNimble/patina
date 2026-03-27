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

#[cfg(feature = "toy-peer")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerEvent {
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

pub trait StateBackend {
    fn get(key: &str) -> Option<String>;
    fn put(key: &str, value_json: &str) -> Result<(), String>;
    fn delete(key: &str) -> Result<(), String>;
    fn list_prefix(prefix: &str) -> Vec<String>;
}

#[cfg(feature = "toy-layer-fs")]
pub trait LayerFsBackend {
    fn read_file(path: &str) -> Result<String, String>;
    fn write_file(path: &str, contents: &str) -> Result<(), String>;
    fn list_dir(path: &str) -> Result<Vec<String>, String>;
    fn delete_file(path: &str) -> Result<(), String>;
    fn move_path(from: &str, to: &str) -> Result<(), String>;
    fn exists(path: &str) -> Result<bool, String>;
}

#[cfg(feature = "toy-git")]
pub trait GitBackend {
    fn create_tag(name: &str) -> Result<(), String>;
    fn delete_tag(name: &str) -> Result<(), String>;
    fn tag_exists(name: &str) -> Result<bool, String>;
    fn commit(message: &str) -> Result<String, String>;
    fn log_oneline(limit: u32) -> Result<Vec<String>, String>;
    fn diff_stat() -> Result<String, String>;
}

pub trait QueryBackend {
    fn query(kind: &str, params_json: &str) -> Result<String, String>;
}

pub trait EmitBackend {
    fn emit(schema: &str, fact_type: &str, data: &str) -> Result<u64, String>;
}

pub trait SessionBackend {
    fn get_session_id() -> String;
    fn get_previous_session() -> Option<String>;
    fn get_previous_session_runtime_id() -> Option<String>;
    fn get_previous_session_handoff() -> Option<String>;
    fn write(section: &str, content: &str) -> Result<(), String>;
    fn set_parent_session(runtime_id: &str) -> Result<(), String>;
    fn create_tag(name: &str) -> Result<(), String>;
    fn set_status(status: &str) -> Result<(), String>;
    fn write_handoff(modified_files: &str, summary: &str) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantedLake {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LakeCursorRecord {
    pub source: String,
    pub data_type: String,
    pub cursor: Option<String>,
    pub written: u64,
    pub status: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantedConnectorBinding {
    pub binding_id: String,
    pub connection: String,
    pub owner: String,
    pub repo: String,
    pub types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorSyncResult {
    pub binding_id: String,
    pub data_type: String,
    pub cursor: Option<String>,
    pub rows_json: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubListParams {
    pub since: Option<String>,
    pub state: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubPage {
    pub items: String,
    pub has_next: bool,
    pub next_page: Option<u32>,
    pub rate_remaining: u32,
}

pub trait MeasureBackend {
    fn record(verb: &str, tool: &str, mode: &str, metrics_json: &str) -> Result<(), String>;
}

pub trait CheckpointBackend {
    fn load(stream: &str) -> Option<String>;
    fn save(stream: &str, checkpoint_json: &str) -> Result<(), String>;
}

pub trait LakeBackend {
    fn list_granted_lakes() -> Result<Vec<GrantedLake>, String>;
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

pub trait ConnectorBackend {
    fn list_bindings() -> Result<Vec<GrantedConnectorBinding>, String>;
    fn upsert_binding(binding: &GrantedConnectorBinding)
        -> Result<GrantedConnectorBinding, String>;
    fn remove_binding(binding_id: &str) -> Result<(), String>;
    fn sync_binding(
        binding_id: &str,
        data_type: &str,
        since: Option<&str>,
    ) -> Result<ConnectorSyncResult, String>;
}

pub trait GithubBackend {
    fn list_issues(
        owner: &str,
        repo: &str,
        params: &GithubListParams,
    ) -> Result<GithubPage, String>;
    fn list_pulls(owner: &str, repo: &str, params: &GithubListParams)
        -> Result<GithubPage, String>;
    fn list_issue_comments(
        owner: &str,
        repo: &str,
        issue_number: u32,
    ) -> Result<GithubPage, String>;
    fn list_issue_events(owner: &str, repo: &str, issue_number: u32) -> Result<GithubPage, String>;
    fn list_pull_comments(owner: &str, repo: &str, pull_number: u32) -> Result<GithubPage, String>;
    fn list_reviews(owner: &str, repo: &str, pull_number: u32) -> Result<GithubPage, String>;
    fn list_review_comments(
        owner: &str,
        repo: &str,
        pull_number: u32,
        review_id: u64,
    ) -> Result<GithubPage, String>;
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

    pub fn get_string(&self, key: &str) -> Option<String> {
        B::get(key)
    }

    pub fn set_string(&self, key: &str, value: &str) -> Result<(), String> {
        B::put(key, value)
    }
    pub fn delete(&self, key: &str) -> Result<(), String> {
        B::delete(key)
    }
    pub fn list_prefix(&self, prefix: &str) -> Vec<String> {
        B::list_prefix(prefix)
    }
}

#[cfg(feature = "toy-layer-fs")]
#[derive(Debug, Clone, Copy, Default)]
pub struct LayerFsToy<B>(std::marker::PhantomData<B>);
#[cfg(feature = "toy-layer-fs")]
impl<B> LayerFsToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
#[cfg(feature = "toy-layer-fs")]
impl<B: LayerFsBackend> LayerFsToy<B> {
    pub fn read_file(&self, path: &str) -> Result<String, String> {
        B::read_file(path)
    }
    pub fn write_file(&self, path: &str, contents: &str) -> Result<(), String> {
        B::write_file(path, contents)
    }
    pub fn list_dir(&self, path: &str) -> Result<Vec<String>, String> {
        B::list_dir(path)
    }
    pub fn delete_file(&self, path: &str) -> Result<(), String> {
        B::delete_file(path)
    }
    pub fn move_path(&self, from: &str, to: &str) -> Result<(), String> {
        B::move_path(from, to)
    }
    pub fn exists(&self, path: &str) -> Result<bool, String> {
        B::exists(path)
    }
}

#[cfg(feature = "toy-git")]
#[derive(Debug, Clone, Copy, Default)]
pub struct GitToy<B>(std::marker::PhantomData<B>);
#[cfg(feature = "toy-git")]
impl<B> GitToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
#[cfg(feature = "toy-git")]
impl<B: GitBackend> GitToy<B> {
    pub fn create_tag(&self, name: &str) -> Result<(), String> {
        B::create_tag(name)
    }
    pub fn delete_tag(&self, name: &str) -> Result<(), String> {
        B::delete_tag(name)
    }
    pub fn tag_exists(&self, name: &str) -> Result<bool, String> {
        B::tag_exists(name)
    }
    pub fn commit(&self, message: &str) -> Result<String, String> {
        B::commit(message)
    }
    pub fn log_oneline(&self, limit: u32) -> Result<Vec<String>, String> {
        B::log_oneline(limit)
    }
    pub fn diff_stat(&self) -> Result<String, String> {
        B::diff_stat()
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
pub struct SessionToy<B>(std::marker::PhantomData<B>);
impl<B> SessionToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: SessionBackend> SessionToy<B> {
    pub fn get_session_id(&self) -> String {
        B::get_session_id()
    }
    pub fn get_previous_session(&self) -> Option<String> {
        B::get_previous_session()
    }
    pub fn get_previous_session_runtime_id(&self) -> Option<String> {
        B::get_previous_session_runtime_id()
    }
    pub fn get_previous_session_handoff(&self) -> Option<String> {
        B::get_previous_session_handoff()
    }
    pub fn write(&self, section: &str, content: &str) -> Result<(), String> {
        B::write(section, content)
    }
    pub fn set_parent_session(&self, runtime_id: &str) -> Result<(), String> {
        B::set_parent_session(runtime_id)
    }
    pub fn create_tag(&self, name: &str) -> Result<(), String> {
        B::create_tag(name)
    }
    pub fn set_status(&self, status: &str) -> Result<(), String> {
        B::set_status(status)
    }
    pub fn write_handoff(&self, modified_files: &str, summary: &str) -> Result<(), String> {
        B::write_handoff(modified_files, summary)
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
    pub fn record(
        &self,
        verb: &str,
        tool: &str,
        mode: &str,
        metrics_json: &str,
    ) -> Result<(), String> {
        B::record(verb, tool, mode, metrics_json)
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

#[derive(Debug, Clone)]
pub struct LakeToy<B> {
    granted: GrantedLake,
    _marker: std::marker::PhantomData<B>,
}
impl<B> LakeToy<B> {
    pub fn new(granted: GrantedLake) -> Self {
        Self {
            granted,
            _marker: std::marker::PhantomData,
        }
    }
    pub fn grant(&self) -> &GrantedLake {
        &self.granted
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct LakeCatalog<B>(std::marker::PhantomData<B>);
impl<B> LakeCatalog<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: LakeBackend> LakeToy<B> {
    pub fn load_cursor(&self, source: &str, data_type: &str) -> Option<String> {
        B::load_cursor(&self.granted.name, source, data_type)
    }
    pub fn save_cursor(&self, record: &LakeCursorRecord) -> Result<(), String> {
        B::save_cursor(
            &self.granted.name,
            &record.source,
            &record.data_type,
            record.cursor.as_deref(),
            record.written,
            &record.status,
            record.last_error.as_deref(),
        )
    }
    pub fn ensure_table(&self, table: &str) -> Result<(), String> {
        B::ensure_table(&self.granted.name, table)
    }
    pub fn append_json_batch(
        &self,
        table: &str,
        source: &str,
        rows_json: &[String],
    ) -> Result<u64, String> {
        B::append_json_batch(&self.granted.name, table, source, rows_json)
    }
    pub fn query_json(&self, sql: &str) -> Result<String, String> {
        B::query_json(&self.granted.name, sql)
    }
}
impl<B: LakeBackend> LakeCatalog<B> {
    pub fn list(&self) -> Result<Vec<LakeToy<B>>, String> {
        Ok(B::list_granted_lakes()?
            .into_iter()
            .map(LakeToy::new)
            .collect())
    }
    pub fn require(&self, name: &str) -> Result<LakeToy<B>, String> {
        self.list()?
            .into_iter()
            .find(|lake| lake.grant().name == name)
            .ok_or_else(|| format!("lake '{}' not granted", name))
    }
}

#[derive(Debug, Clone)]
pub struct ConnectorBinding<B> {
    granted: GrantedConnectorBinding,
    _marker: std::marker::PhantomData<B>,
}
impl<B> ConnectorBinding<B> {
    pub fn new(granted: GrantedConnectorBinding) -> Self {
        Self {
            granted,
            _marker: std::marker::PhantomData,
        }
    }
    pub fn grant(&self) -> &GrantedConnectorBinding {
        &self.granted
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct ConnectorCatalog<B>(std::marker::PhantomData<B>);
impl<B> ConnectorCatalog<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: ConnectorBackend> ConnectorBinding<B> {
    pub fn sync(
        &self,
        data_type: &str,
        since: Option<&str>,
    ) -> Result<ConnectorSyncResult, String> {
        B::sync_binding(&self.granted.binding_id, data_type, since)
    }
}
impl<B: ConnectorBackend> ConnectorCatalog<B> {
    pub fn list(&self) -> Result<Vec<ConnectorBinding<B>>, String> {
        Ok(B::list_bindings()?
            .into_iter()
            .map(ConnectorBinding::new)
            .collect())
    }
    pub fn require(&self, binding_id: &str) -> Result<ConnectorBinding<B>, String> {
        self.list()?
            .into_iter()
            .find(|binding| binding.grant().binding_id == binding_id)
            .ok_or_else(|| format!("connector binding '{}' not granted", binding_id))
    }
    pub fn upsert(&self, binding: &GrantedConnectorBinding) -> Result<ConnectorBinding<B>, String> {
        B::upsert_binding(binding).map(ConnectorBinding::new)
    }
    pub fn remove(&self, binding_id: &str) -> Result<(), String> {
        B::remove_binding(binding_id)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GithubToy<B>(std::marker::PhantomData<B>);
impl<B> GithubToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
impl<B: GithubBackend> GithubToy<B> {
    pub fn list_issues(
        &self,
        owner: &str,
        repo: &str,
        params: &GithubListParams,
    ) -> Result<GithubPage, String> {
        B::list_issues(owner, repo, params)
    }
    pub fn list_pulls(
        &self,
        owner: &str,
        repo: &str,
        params: &GithubListParams,
    ) -> Result<GithubPage, String> {
        B::list_pulls(owner, repo, params)
    }
    pub fn list_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u32,
    ) -> Result<GithubPage, String> {
        B::list_issue_comments(owner, repo, issue_number)
    }
    pub fn list_issue_events(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u32,
    ) -> Result<GithubPage, String> {
        B::list_issue_events(owner, repo, issue_number)
    }
    pub fn list_pull_comments(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u32,
    ) -> Result<GithubPage, String> {
        B::list_pull_comments(owner, repo, pull_number)
    }
    pub fn list_reviews(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u32,
    ) -> Result<GithubPage, String> {
        B::list_reviews(owner, repo, pull_number)
    }
    pub fn list_review_comments(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u32,
        review_id: u64,
    ) -> Result<GithubPage, String> {
        B::list_review_comments(owner, repo, pull_number, review_id)
    }
}

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

#[cfg(feature = "toy-peer")]
pub trait PeerBackend {
    fn emit_event(event_type: &str, payload_json: &str) -> Result<(), String>;
    fn on_event(
        stream_name: &str,
        after_offset: Option<u64>,
        limit: u32,
    ) -> Result<Vec<PeerEvent>, String>;
}

#[cfg(feature = "toy-peer")]
#[derive(Debug, Clone, Copy, Default)]
pub struct PeerToy<B>(std::marker::PhantomData<B>);

#[cfg(feature = "toy-peer")]
impl<B> PeerToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

#[cfg(feature = "toy-peer")]
impl<B: PeerBackend> PeerToy<B> {
    pub fn emit_event(&self, event_type: &str, payload_json: &str) -> Result<(), String> {
        B::emit_event(event_type, payload_json)
    }

    pub fn on_event(
        &self,
        stream_name: &str,
        after_offset: Option<u64>,
        limit: u32,
    ) -> Result<Vec<PeerEvent>, String> {
        B::on_event(stream_name, after_offset, limit)
    }
}
