mod wasm_cell;

pub use patina_toy_sdk as toys;
use patina_toy_sdk::{PendingEvent as ToyPendingEvent, TaskIntent as ToyTaskIntent};

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

pub mod host {
    use super::patina;
    use super::toys::{
        BeliefBackend, CheckpointBackend, EmitBackend, EventBackend, FetchBackend, GraphBackend,
        LakeBackend, LogBackend, MeasureBackend, PendingEvent, QueryBackend, StateBackend,
        TaskBackend, TaskIntent, TaskIntentKind,
    };

    #[derive(Debug, Clone, Copy, Default)]
    pub struct GuestHost;

    impl GuestHost {
        pub fn log() -> super::toys::LogToy<Self> {
            super::toys::LogToy::new()
        }
        pub fn measure() -> super::toys::MeasureToy<Self> {
            super::toys::MeasureToy::new()
        }
        pub fn query() -> super::toys::QueryToy<Self> {
            super::toys::QueryToy::new()
        }
        pub fn fetch() -> super::toys::FetchToy<Self> {
            super::toys::FetchToy::new()
        }
        pub fn emit() -> super::toys::EmitToy<Self> {
            super::toys::EmitToy::new()
        }
        pub fn state() -> super::toys::StateToy<Self> {
            super::toys::StateToy::new()
        }
        pub fn checkpoint() -> super::toys::CheckpointToy<Self> {
            super::toys::CheckpointToy::new()
        }
        pub fn lake() -> super::toys::LakeToy<Self> {
            super::toys::LakeToy::new()
        }
        pub fn events() -> super::toys::EventToy<Self> {
            super::toys::EventToy::new()
        }
        pub fn tasks() -> super::toys::TaskToy<Self> {
            super::toys::TaskToy::new()
        }
        pub fn graph() -> super::toys::GraphToy<Self> {
            super::toys::GraphToy::new()
        }
        pub fn belief() -> super::toys::BeliefToy<Self> {
            super::toys::BeliefToy::new()
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
        fn ensure_lake(name: &str) -> Result<String, String> {
            patina::host::lake::ensure_lake(name)
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
                        patina_toy_sdk::TaskIntentKind::FetchSource => {
                            patina::host::task::TaskIntentKind::FetchSource
                        }
                        patina_toy_sdk::TaskIntentKind::RunQuery => {
                            patina::host::task::TaskIntentKind::RunQuery
                        }
                        patina_toy_sdk::TaskIntentKind::EmitFacts => {
                            patina::host::task::TaskIntentKind::EmitFacts
                        }
                        patina_toy_sdk::TaskIntentKind::MaterializeIndex => {
                            patina::host::task::TaskIntentKind::MaterializeIndex
                        }
                        patina_toy_sdk::TaskIntentKind::VerifyBelief => {
                            patina::host::task::TaskIntentKind::VerifyBelief
                        }
                        patina_toy_sdk::TaskIntentKind::SyncGraph => {
                            patina::host::task::TaskIntentKind::SyncGraph
                        }
                        patina_toy_sdk::TaskIntentKind::RefreshCredential => {
                            patina::host::task::TaskIntentKind::RefreshCredential
                        }
                        patina_toy_sdk::TaskIntentKind::NativeJob => {
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
            $crate::__register_plugin(Box::new(<$plugin_type>::default()));
        }
    };
}
