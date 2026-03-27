//! Core SDK primitives for Patina knowledge children.

use std::cell::UnsafeCell;

use serde::{Deserialize, Serialize};

pub const TOY_WIT_DIR: &str = env!("PATINA_SDK_CORE_WIT_DIR");

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

pub trait KnowledgeChild {
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

static PLUGIN: WasmCell<Option<Box<dyn KnowledgeChild>>> = WasmCell(UnsafeCell::new(None));

#[doc(hidden)]
pub fn __register_knowledge_child(child: Box<dyn KnowledgeChild>) {
    unsafe {
        *PLUGIN.0.get() = Some(child);
    }
}

#[doc(hidden)]
pub fn __register_plugin(plugin: Box<dyn KnowledgeChild>) {
    __register_knowledge_child(plugin);
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

pub use KnowledgeChild as KnowledgeChildPlugin;
