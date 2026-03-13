use crate::toys;
use crate::toys::{PendingEvent as ToyPendingEvent, TaskIntent as ToyTaskIntent};

#[cfg(target_arch = "wasm32")]
#[used]
#[link_section = ".patina_api_version"]
static API_VERSION: [u8; 3] = [0, 1, 0];

wit_bindgen::generate!({
    path: "wit/knowledge-child",
    world: "knowledge-child",
    skip: ["init"],
    generate_all,
});

pub use patina::host::types::HealthStatus;

use crate::wasm_cell::WasmCell;

pub mod granted {
    use super::host::GuestHost;
    use super::toys;

    pub type Log = toys::LogToy<GuestHost>;
    pub type Measure = toys::MeasureToy<GuestHost>;
    pub type Query = toys::QueryToy<GuestHost>;
    pub type Fetch = toys::FetchToy<GuestHost>;
    pub type Emit = toys::EmitToy<GuestHost>;
    pub type State = toys::StateToy<GuestHost>;
    pub type Checkpoint = toys::CheckpointToy<GuestHost>;
    pub type Lakes = toys::LakeCatalog<GuestHost>;
    pub type Lake = toys::LakeToy<GuestHost>;
    pub type IngressSources = toys::IngressCatalog<GuestHost>;
    pub type Ingress = toys::IngressToy<GuestHost>;
    pub type Connectors = toys::ConnectorCatalog<GuestHost>;
    pub type Connector = toys::ConnectorBinding<GuestHost>;
    pub type Events = toys::EventToy<GuestHost>;
    pub type Graph = toys::GraphToy<GuestHost>;
    pub type Belief = toys::BeliefToy<GuestHost>;

    pub trait Bundle {
        fn granted() -> Self;
    }

    pub fn log() -> Log {
        Log::new()
    }

    pub fn measure() -> Measure {
        Measure::new()
    }

    pub fn query() -> Query {
        Query::new()
    }

    pub fn fetch() -> Fetch {
        Fetch::new()
    }

    pub fn emit() -> Emit {
        Emit::new()
    }

    pub fn state() -> State {
        State::new()
    }

    pub fn checkpoint() -> Checkpoint {
        Checkpoint::new()
    }

    pub fn lakes() -> Lakes {
        Lakes::new()
    }

    pub fn lake(name: &str) -> Lake {
        lakes()
            .require(name)
            .unwrap_or_else(|error| panic!("missing granted lake '{}': {}", name, error))
    }

    pub fn ingress_sources() -> IngressSources {
        IngressSources::new()
    }

    pub fn ingress(name: &str) -> Ingress {
        ingress_sources()
            .require(name)
            .unwrap_or_else(|error| panic!("missing granted ingress source '{}': {}", name, error))
    }

    pub fn connectors() -> Connectors {
        Connectors::new()
    }

    pub fn connector(binding_id: &str) -> Connector {
        connectors().require(binding_id).unwrap_or_else(|error| {
            panic!(
                "missing granted connector binding '{}': {}",
                binding_id, error
            )
        })
    }

    pub fn events() -> Events {
        Events::new()
    }

    pub fn graph() -> Graph {
        Graph::new()
    }

    pub fn belief() -> Belief {
        Belief::new()
    }
}

pub mod substrate {
    pub use crate::toys::{PendingEvent, TaskIntent, TaskIntentKind};

    pub fn enqueue(intent: &TaskIntent) -> Result<String, String> {
        super::host::GuestHost::enqueue_task(intent)
    }
}

pub trait KnowledgeChildPlugin {
    fn name(&self) -> String;

    fn on_load(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn on_unload(&mut self) {}

    fn health(&self) -> ChildHealth {
        ChildHealth {
            status: HealthStatus::Healthy,
            reason: None,
        }
    }

    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String>;

    fn drain(&mut self, _limit: u32) -> Result<Vec<ToyPendingEvent>, String> {
        Ok(vec![])
    }

    fn tick(&mut self) -> Vec<ToyTaskIntent> {
        vec![]
    }
}

#[doc(hidden)]
pub mod host {
    use super::patina;
    use crate::toys::{
        BeliefBackend, CheckpointBackend, ConnectorBackend, EmitBackend, EventBackend,
        FetchBackend, GraphBackend, IngressBackend, LakeBackend, LogBackend, MeasureBackend,
        PendingEvent, QueryBackend, StateBackend, TaskBackend, TaskIntent, TaskIntentKind,
    };

    #[derive(Debug, Clone, Copy, Default)]
    pub struct GuestHost;

    impl GuestHost {
        pub fn log() -> crate::toys::LogToy<Self> {
            crate::toys::LogToy::new()
        }
        pub fn measure() -> crate::toys::MeasureToy<Self> {
            crate::toys::MeasureToy::new()
        }
        pub fn query() -> crate::toys::QueryToy<Self> {
            crate::toys::QueryToy::new()
        }
        pub fn fetch() -> crate::toys::FetchToy<Self> {
            crate::toys::FetchToy::new()
        }
        pub fn emit() -> crate::toys::EmitToy<Self> {
            crate::toys::EmitToy::new()
        }
        pub fn state() -> crate::toys::StateToy<Self> {
            crate::toys::StateToy::new()
        }
        pub fn checkpoint() -> crate::toys::CheckpointToy<Self> {
            crate::toys::CheckpointToy::new()
        }
        pub fn lake(name: &str) -> crate::toys::LakeToy<Self> {
            crate::toys::LakeCatalog::<Self>::new()
                .require(name)
                .unwrap_or_else(|error| panic!("missing granted lake '{}': {}", name, error))
        }
        pub fn ingress(name: &str) -> crate::toys::IngressToy<Self> {
            crate::toys::IngressCatalog::<Self>::new()
                .require(name)
                .unwrap_or_else(|error| {
                    panic!("missing granted ingress source '{}': {}", name, error)
                })
        }
        pub fn events() -> crate::toys::EventToy<Self> {
            crate::toys::EventToy::new()
        }
        pub fn graph() -> crate::toys::GraphToy<Self> {
            crate::toys::GraphToy::new()
        }
        pub fn belief() -> crate::toys::BeliefToy<Self> {
            crate::toys::BeliefToy::new()
        }
    }

    impl LogBackend for GuestHost {
        fn debug(message: &str) {
            patina::host::log::log(patina::host::log::LogLevel::Debug, message);
        }
        fn info(message: &str) {
            patina::host::log::log(patina::host::log::LogLevel::Info, message);
        }
        fn warn(message: &str) {
            patina::host::log::log(patina::host::log::LogLevel::Warn, message);
        }
        fn error(message: &str) {
            patina::host::log::log(patina::host::log::LogLevel::Error, message);
        }
    }

    impl MeasureBackend for GuestHost {
        fn record(verb: &str, tool: &str, mode: &str, metrics_json: &str) -> Result<(), String> {
            patina::host::measure::record_measurement(verb, tool, mode, metrics_json)
        }
    }

    impl QueryBackend for GuestHost {
        fn query(kind: &str, params_json: &str) -> Result<String, String> {
            patina::host::query::query(kind, params_json)
        }
    }

    impl FetchBackend for GuestHost {
        fn get(url: &str) -> Result<String, String> {
            patina::host::http::http_get(url).map(|r| r.body)
        }
        fn post(url: &str, body: &str, content_type: &str) -> Result<String, String> {
            patina::host::http::http_post(url, body, content_type).map(|r| r.body)
        }
    }

    impl EmitBackend for GuestHost {
        fn emit(schema: &str, fact_type: &str, data: &str) -> Result<u64, String> {
            patina::host::emit::emit_fact(schema, fact_type, data)
        }
    }

    impl StateBackend for GuestHost {
        fn get(key: &str) -> Option<String> {
            patina::host::state::get(key)
        }
        fn put(key: &str, value_json: &str) -> Result<(), String> {
            patina::host::state::put(key, value_json)
        }
        fn delete(key: &str) -> Result<(), String> {
            patina::host::state::delete(key)
        }
        fn list_prefix(prefix: &str) -> Vec<String> {
            patina::host::state::list_prefix(prefix)
        }
    }

    impl CheckpointBackend for GuestHost {
        fn load(stream: &str) -> Option<String> {
            patina::host::checkpoint::load(stream)
        }
        fn save(stream: &str, checkpoint_json: &str) -> Result<(), String> {
            patina::host::checkpoint::save(stream, checkpoint_json)
        }
    }

    impl LakeBackend for GuestHost {
        fn list_granted_lakes() -> Result<Vec<crate::toys::GrantedLake>, String> {
            patina::host::lake::list_granted_lakes().map(|lakes| {
                lakes
                    .into_iter()
                    .map(|lake| crate::toys::GrantedLake {
                        name: lake.name,
                        path: lake.path,
                    })
                    .collect()
            })
        }
        fn load_cursor(lake: &str, source: &str, data_type: &str) -> Option<String> {
            patina::host::lake::load_cursor(lake, source, data_type)
        }
        fn save_cursor(
            lake: &str,
            source: &str,
            data_type: &str,
            cursor: Option<&str>,
            written: u64,
            status: &str,
            last_error: Option<&str>,
        ) -> Result<(), String> {
            patina::host::lake::save_cursor(
                lake, source, data_type, cursor, written, status, last_error,
            )
        }
        fn ensure_table(lake: &str, table: &str) -> Result<(), String> {
            patina::host::lake::ensure_table(lake, table)
        }
        fn append_json_batch(
            lake: &str,
            table: &str,
            source: &str,
            rows_json: &[String],
        ) -> Result<u64, String> {
            patina::host::lake::append_json_batch(lake, table, source, rows_json)
        }
        fn query_json(lake: &str, sql: &str) -> Result<String, String> {
            patina::host::lake::query_json(lake, sql)
        }
    }

    impl IngressBackend for GuestHost {
        fn list_granted_sources() -> Vec<crate::toys::GrantedIngressSource> {
            patina::host::ingress::list_granted_sources()
                .into_iter()
                .map(|source| crate::toys::GrantedIngressSource {
                    name: source.name,
                    endpoint: source.endpoint,
                })
                .collect()
        }

        fn fetch(source: &str) -> Result<String, String> {
            patina::host::ingress::fetch(source)
        }
    }

    impl ConnectorBackend for GuestHost {
        fn list_bindings() -> Result<Vec<crate::toys::GrantedConnectorBinding>, String> {
            patina::host::connector::list_bindings().map(|bindings| {
                bindings
                    .into_iter()
                    .map(|binding| crate::toys::GrantedConnectorBinding {
                        binding_id: binding.binding_id,
                        connection: binding.connection,
                        owner: binding.owner,
                        repo: binding.repo,
                        types: binding.types,
                    })
                    .collect()
            })
        }

        fn upsert_binding(
            binding: &crate::toys::GrantedConnectorBinding,
        ) -> Result<crate::toys::GrantedConnectorBinding, String> {
            patina::host::connector::upsert_binding(&patina::host::connector::RepoBinding {
                binding_id: binding.binding_id.clone(),
                connection: binding.connection.clone(),
                owner: binding.owner.clone(),
                repo: binding.repo.clone(),
                types: binding.types.clone(),
            })
            .map(|saved| crate::toys::GrantedConnectorBinding {
                binding_id: saved.binding_id,
                connection: saved.connection,
                owner: saved.owner,
                repo: saved.repo,
                types: saved.types,
            })
        }

        fn remove_binding(binding_id: &str) -> Result<(), String> {
            patina::host::connector::remove_binding(binding_id)
        }

        fn sync_binding(
            binding_id: &str,
            data_type: &str,
            since: Option<&str>,
        ) -> Result<crate::toys::ConnectorSyncResult, String> {
            patina::host::connector::sync_binding(binding_id, data_type, since).map(|result| {
                crate::toys::ConnectorSyncResult {
                    binding_id: result.binding_id,
                    data_type: result.data_type,
                    cursor: result.cursor,
                    rows_json: result.rows_json,
                }
            })
        }
    }

    impl EventBackend for GuestHost {
        fn pull(
            stream: &str,
            after_offset: Option<u64>,
            limit: u32,
        ) -> Result<Vec<PendingEvent>, String> {
            patina::host::events::pull(stream, after_offset, limit).map(|events| {
                events
                    .into_iter()
                    .map(|event| PendingEvent {
                        stream_name: event.stream_name,
                        offset: event.offset,
                        event_type: event.event_type,
                        payload_json: event.payload_json,
                        occurred_at: event.occurred_at,
                    })
                    .collect()
            })
        }
        fn ack_through(stream: &str, offset: u64) -> Result<(), String> {
            patina::host::events::ack_through(stream, offset)
        }
        fn list_streams() -> Vec<String> {
            patina::host::events::list_streams()
        }
    }

    impl TaskBackend for GuestHost {
        fn enqueue(intent: &TaskIntent) -> Result<String, String> {
            let kind = match intent.kind {
                TaskIntentKind::FetchSource => patina::host::task::TaskIntentKind::FetchSource,
                TaskIntentKind::RunQuery => patina::host::task::TaskIntentKind::RunQuery,
                TaskIntentKind::EmitFacts => patina::host::task::TaskIntentKind::EmitFacts,
                TaskIntentKind::MaterializeIndex => {
                    patina::host::task::TaskIntentKind::MaterializeIndex
                }
                TaskIntentKind::VerifyBelief => patina::host::task::TaskIntentKind::VerifyBelief,
                TaskIntentKind::SyncGraph => patina::host::task::TaskIntentKind::SyncGraph,
                TaskIntentKind::RefreshCredential => {
                    patina::host::task::TaskIntentKind::RefreshCredential
                }
                TaskIntentKind::NativeJob => patina::host::task::TaskIntentKind::NativeJob,
            };
            patina::host::task::enqueue(&patina::host::task::TaskIntent {
                kind,
                payload_json: intent.payload_json.clone(),
                dedupe_key: intent.dedupe_key.clone(),
            })
        }
    }

    impl GuestHost {
        pub fn enqueue_task(intent: &TaskIntent) -> Result<String, String> {
            <Self as TaskBackend>::enqueue(intent)
        }
    }

    impl GraphBackend for GuestHost {
        fn query(kind: &str, params_json: &str) -> Result<String, String> {
            patina::host::graph::query(kind, params_json)
        }
        fn mutate(action: &str, payload_json: &str) -> Result<(), String> {
            patina::host::graph::mutate(action, payload_json)
        }
    }

    impl BeliefBackend for GuestHost {
        fn query(kind: &str, params_json: &str) -> Result<String, String> {
            patina::host::belief::query(kind, params_json)
        }
        fn mutate(action: &str, payload_json: &str) -> Result<(), String> {
            patina::host::belief::mutate(action, payload_json)
        }
    }
}

static PLUGIN: WasmCell<Option<Box<dyn KnowledgeChildPlugin>>> =
    WasmCell(std::cell::UnsafeCell::new(None));

#[doc(hidden)]
pub fn __register_plugin(plugin: Box<dyn KnowledgeChildPlugin>) {
    unsafe {
        *PLUGIN.0.get() = Some(plugin);
    }
}

#[cfg(target_arch = "wasm32")]
mod __wasm {
    use super::*;

    fn plugin() -> &'static mut dyn KnowledgeChildPlugin {
        unsafe {
            (*PLUGIN.0.get())
                .as_deref_mut()
                .expect("plugin not initialized — host must call init first")
        }
    }

    struct Component;

    impl Guest for Component {
        fn name() -> String {
            plugin().name()
        }

        fn on_load() -> Result<(), String> {
            plugin().on_load()
        }

        fn on_unload() {
            plugin().on_unload()
        }

        fn health() -> ChildHealth {
            plugin().health()
        }

        fn handle(action: String, payload: String) -> Result<String, String> {
            plugin().handle(&action, &payload)
        }

        fn drain(limit: u32) -> Result<Vec<PendingEvent>, String> {
            plugin().drain(limit).map(|events| {
                events
                    .into_iter()
                    .map(|event| patina::host::events::PendingEvent {
                        stream_name: event.stream_name,
                        offset: event.offset,
                        event_type: event.event_type,
                        payload_json: event.payload_json,
                        occurred_at: event.occurred_at,
                    })
                    .collect()
            })
        }

        fn tick() -> Vec<patina::host::task::TaskIntent> {
            plugin()
                .tick()
                .into_iter()
                .map(|intent| patina::host::task::TaskIntent {
                    kind: match intent.kind {
                        crate::toys::TaskIntentKind::FetchSource => {
                            patina::host::task::TaskIntentKind::FetchSource
                        }
                        crate::toys::TaskIntentKind::RunQuery => {
                            patina::host::task::TaskIntentKind::RunQuery
                        }
                        crate::toys::TaskIntentKind::EmitFacts => {
                            patina::host::task::TaskIntentKind::EmitFacts
                        }
                        crate::toys::TaskIntentKind::MaterializeIndex => {
                            patina::host::task::TaskIntentKind::MaterializeIndex
                        }
                        crate::toys::TaskIntentKind::VerifyBelief => {
                            patina::host::task::TaskIntentKind::VerifyBelief
                        }
                        crate::toys::TaskIntentKind::SyncGraph => {
                            patina::host::task::TaskIntentKind::SyncGraph
                        }
                        crate::toys::TaskIntentKind::RefreshCredential => {
                            patina::host::task::TaskIntentKind::RefreshCredential
                        }
                        crate::toys::TaskIntentKind::NativeJob => {
                            patina::host::task::TaskIntentKind::NativeJob
                        }
                    },
                    payload_json: intent.payload_json,
                    dedupe_key: intent.dedupe_key,
                })
                .collect()
        }
    }

    export!(Component);
}

#[macro_export]
macro_rules! register_knowledge_child {
    ($plugin_type:ty) => {
        #[export_name = "init"]
        extern "C" fn __patina_plugin_init() {
            $crate::knowledge_child::__register_plugin(Box::new(<$plugin_type>::default()));
        }
    };
}
