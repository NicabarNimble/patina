//! Core SDK primitives for Patina knowledge children.

use std::cell::UnsafeCell;

use serde::{Deserialize, Serialize};

pub struct WasmCell<T>(pub UnsafeCell<T>);

#[cfg(not(target_feature = "atomics"))]
unsafe impl<T> Sync for WasmCell<T> {}

#[cfg(target_feature = "atomics")]
compile_error!("WasmCell assumes single-threaded WASM. Use thread_local! with atomics.");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildHealth {
    pub status: HealthStatus,
    pub reason: Option<String>,
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

pub trait StateBackend {
    fn get(key: &str) -> Option<String>;
    fn put(key: &str, value_json: &str) -> Result<(), String>;
    fn delete(key: &str) -> Result<(), String>;
    fn list_prefix(prefix: &str) -> Vec<String>;
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

    pub fn delete(&self, key: &str) -> Result<(), String> {
        B::delete(key)
    }

    pub fn list_prefix(&self, prefix: &str) -> Vec<String> {
        B::list_prefix(prefix)
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

    fn drain(&mut self, _limit: u32) -> Result<Vec<PendingEvent>, String> {
        Ok(vec![])
    }

    fn tick(&mut self) -> Vec<TaskIntent> {
        vec![]
    }
}

static PLUGIN: WasmCell<Option<Box<dyn KnowledgeChildPlugin>>> = WasmCell(UnsafeCell::new(None));

#[doc(hidden)]
pub fn __register_plugin(plugin: Box<dyn KnowledgeChildPlugin>) {
    unsafe {
        *PLUGIN.0.get() = Some(plugin);
    }
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
