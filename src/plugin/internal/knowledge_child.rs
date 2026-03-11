//! Knowledge-child world — bindgen, engine, and WASM adapter.

use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::Store;

use super::{wasm_engine, GrantedCapabilities, PluginManifest, QueryDispatchFn};
use crate::mother::{
    ChildHealth, ChildRequest, ChildResponse, KnowledgeChild, MotherHost, PendingEvent, TaskIntent,
    TaskIntentKind,
};

mod bindings {
    pub struct HostState {
        pub plugin_name: String,
        pub wasi: wasmtime_wasi::WasiCtx,
        pub wasi_table: wasmtime::component::ResourceTable,
        pub project_root: Option<std::path::PathBuf>,
        pub grants: super::GrantedCapabilities,
        pub query_fn: Option<super::QueryDispatchFn>,
        pub http_client: reqwest::blocking::Client,
        pub runtime: crate::mother::KnowledgeRuntimeStore,
    }

    impl wasmtime_wasi::WasiView for HostState {
        fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
            wasmtime_wasi::WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.wasi_table,
            }
        }
    }

    wasmtime::component::bindgen!({
        path: "wit/knowledge-child/",
        world: "knowledge-child",
    });

    impl patina::host::log::Host for HostState {
        fn log(&mut self, level: patina::host::log::LogLevel, message: String) {
            let level_str = match level {
                patina::host::log::LogLevel::Debug => "DEBUG",
                patina::host::log::LogLevel::Info => "INFO",
                patina::host::log::LogLevel::Warn => "WARN",
                patina::host::log::LogLevel::Error => "ERROR",
            };
            super::super::host_support::log(&self.plugin_name, level_str, &message);
        }
    }

    impl patina::host::types::Host for HostState {}

    impl patina::host::query::Host for HostState {
        fn query(&mut self, kind: String, params: String) -> Result<String, String> {
            super::super::host_support::query(
                &self.plugin_name,
                &self.grants,
                &mut self.query_fn,
                &kind,
                &params,
            )
        }
    }

    impl patina::host::http::Host for HostState {
        fn http_post(
            &mut self,
            url: String,
            body: String,
            content_type: String,
        ) -> Result<patina::host::http::HttpResponse, String> {
            let r = super::super::host_support::http_post(
                &self.http_client,
                &self.grants,
                &self.plugin_name,
                &url,
                &body,
                &content_type,
            )?;
            Ok(patina::host::http::HttpResponse {
                status: r.status,
                body: r.body,
            })
        }

        fn http_get(&mut self, url: String) -> Result<patina::host::http::HttpResponse, String> {
            let r = super::super::host_support::http_get(
                &self.http_client,
                &self.grants,
                &self.plugin_name,
                &url,
            )?;
            Ok(patina::host::http::HttpResponse {
                status: r.status,
                body: r.body,
            })
        }
    }

    impl patina::host::emit::Host for HostState {
        fn emit_fact(
            &mut self,
            schema: String,
            fact_type: String,
            data: String,
        ) -> Result<u64, String> {
            if !self.grants.host_emit {
                return Err(format!(
                    "host_emit not granted for plugin '{}'",
                    self.plugin_name
                ));
            }
            super::super::host_support::emit_fact(
                &self.grants.schema_facts,
                &self.plugin_name,
                &schema,
                &fact_type,
                &data,
            )
        }
    }

    impl patina::host::measure::Host for HostState {
        fn record_measurement(
            &mut self,
            verb: String,
            tool: String,
            mode: String,
            metrics_json: String,
        ) -> Result<(), String> {
            super::super::host_support::record_measurement(
                &self.project_root,
                &self.plugin_name,
                &verb,
                &tool,
                &mode,
                &metrics_json,
            )
        }
    }

    impl patina::host::state::Host for HostState {
        fn get(&mut self, key: String) -> Option<String> {
            self.runtime.get_state(&self.plugin_name, &key).ok().flatten()
        }

        fn put(&mut self, key: String, value_json: String) -> Result<(), String> {
            if !self.grants.state_enabled {
                return Err(format!("state not granted for plugin '{}'", self.plugin_name));
            }
            self.runtime
                .put_state(&self.plugin_name, &key, &value_json)
                .map_err(|e| e.to_string())
        }

        fn delete(&mut self, key: String) -> Result<(), String> {
            if !self.grants.state_enabled {
                return Err(format!("state not granted for plugin '{}'", self.plugin_name));
            }
            self.runtime
                .delete_state(&self.plugin_name, &key)
                .map_err(|e| e.to_string())
        }

        fn list_prefix(&mut self, prefix: String) -> Vec<String> {
            self.runtime
                .list_state_prefix(&self.plugin_name, &prefix)
                .unwrap_or_default()
        }
    }

    impl patina::host::checkpoint::Host for HostState {
        fn load(&mut self, stream: String) -> Option<String> {
            self.runtime
                .load_checkpoint(&self.plugin_name, &stream)
                .ok()
                .flatten()
        }

        fn save(&mut self, stream: String, checkpoint_json: String) -> Result<(), String> {
            if !self.grants.checkpoint_streams.contains(&stream) {
                return Err(format!(
                    "checkpoint stream '{}' not granted for plugin '{}'",
                    stream, self.plugin_name
                ));
            }
            self.runtime
                .save_checkpoint(&self.plugin_name, &stream, &checkpoint_json)
                .map_err(|e| e.to_string())
        }
    }

    impl patina::host::lake::Host for HostState {
        fn ensure_lake(&mut self, name: String) -> Result<String, String> {
            if !self.grants.lake_names.contains(&name) {
                return Err(format!("lake '{}' not granted for '{}'", name, self.plugin_name));
            }
            crate::mother::lake_host::ensure_lake(&name).map_err(|e| e.to_string())
        }

        fn load_cursor(
            &mut self,
            lake: String,
            source: String,
            data_type: String,
        ) -> Option<String> {
            if !self.grants.lake_names.contains(&lake) {
                return None;
            }
            crate::mother::lake_host::load_cursor(&self.runtime, &lake, &source, &data_type)
                .ok()
                .flatten()
        }

        fn save_cursor(
            &mut self,
            lake: String,
            source: String,
            data_type: String,
            cursor: Option<String>,
            written: u64,
            status: String,
            last_error: Option<String>,
        ) -> Result<(), String> {
            if !self.grants.lake_names.contains(&lake) {
                return Err(format!("lake '{}' not granted for '{}'", lake, self.plugin_name));
            }
            crate::mother::lake_host::save_cursor(
                &self.runtime,
                &lake,
                &source,
                &data_type,
                cursor.as_deref(),
                written,
                &status,
                last_error.as_deref(),
            )
            .map_err(|e| e.to_string())
        }

        fn ensure_table(&mut self, lake: String, table: String) -> Result<(), String> {
            if !self.grants.lake_names.contains(&lake) {
                return Err(format!("lake '{}' not granted for '{}'", lake, self.plugin_name));
            }
            crate::mother::lake_host::ensure_table(&lake, &table).map_err(|e| e.to_string())
        }

        fn append_json_batch(
            &mut self,
            lake: String,
            table: String,
            source: String,
            rows_json: Vec<String>,
        ) -> Result<u64, String> {
            if !self.grants.lake_names.contains(&lake) {
                return Err(format!("lake '{}' not granted for '{}'", lake, self.plugin_name));
            }
            crate::mother::lake_host::append_json_batch(&lake, &table, &source, &rows_json)
                .map_err(|e| e.to_string())
        }

        fn query_json(&mut self, lake: String, sql: String) -> Result<String, String> {
            if !self.grants.lake_names.contains(&lake) {
                return Err(format!("lake '{}' not granted for '{}'", lake, self.plugin_name));
            }
            crate::mother::lake_host::query_json(&lake, &sql).map_err(|e| e.to_string())
        }
    }

    impl patina::host::events::Host for HostState {
        fn pull(
            &mut self,
            stream: String,
            after_offset: Option<u64>,
            limit: u32,
        ) -> Result<Vec<patina::host::events::PendingEvent>, String> {
            if !self.grants.subscribed_streams.contains(&stream) {
                return Err(format!(
                    "event stream '{}' not granted for plugin '{}'",
                    stream, self.plugin_name
                ));
            }
            let events = crate::mother::events::pull(&stream, after_offset, limit)
                .map_err(|e| e.to_string())?;
            Ok(events
                .into_iter()
                .map(|event| patina::host::events::PendingEvent {
                    stream_name: event.stream,
                    offset: event.offset,
                    event_type: event.event_type,
                    payload_json: serde_json::to_string(&event.payload).unwrap_or_else(|_| "null".into()),
                    occurred_at: event.occurred_at,
                })
                .collect())
        }

        fn ack_through(&mut self, stream: String, offset: u64) -> Result<(), String> {
            if !self.grants.subscribed_streams.contains(&stream) {
                return Err(format!(
                    "event stream '{}' not granted for plugin '{}'",
                    stream, self.plugin_name
                ));
            }
            crate::mother::events::ack_through(&self.runtime, &self.plugin_name, &stream, offset)
                .map_err(|e| e.to_string())
        }

        fn list_streams(&mut self) -> Vec<String> {
            crate::mother::events::list_streams()
        }
    }

    impl patina::host::task::Host for HostState {
        fn enqueue(&mut self, intent: patina::host::task::TaskIntent) -> Result<String, String> {
            let kind = match intent.kind {
                patina::host::task::TaskIntentKind::FetchSource => crate::mother::TaskIntentKind::FetchSource,
                patina::host::task::TaskIntentKind::RunQuery => crate::mother::TaskIntentKind::RunQuery,
                patina::host::task::TaskIntentKind::EmitFacts => crate::mother::TaskIntentKind::EmitFacts,
                patina::host::task::TaskIntentKind::MaterializeIndex => crate::mother::TaskIntentKind::MaterializeIndex,
                patina::host::task::TaskIntentKind::VerifyBelief => crate::mother::TaskIntentKind::VerifyBelief,
                patina::host::task::TaskIntentKind::SyncGraph => crate::mother::TaskIntentKind::SyncGraph,
                patina::host::task::TaskIntentKind::RefreshCredential => crate::mother::TaskIntentKind::RefreshCredential,
                patina::host::task::TaskIntentKind::NativeJob => crate::mother::TaskIntentKind::NativeJob,
            };
            if !self.grants.task_intents.contains(&kind) {
                return Err(format!("task intent '{}' not granted for '{}'", kind, self.plugin_name));
            }
            let payload = serde_json::from_str(&intent.payload_json)
                .map_err(|e| format!("invalid task payload json: {}", e))?;
            self.runtime
                .enqueue_task(
                    &self.plugin_name,
                    &crate::mother::TaskIntent {
                        kind,
                        payload,
                        dedupe_key: intent.dedupe_key,
                    },
                )
                .map_err(|e| e.to_string())
        }
    }

    impl patina::host::graph::Host for HostState {
        fn query(&mut self, kind: String, params_json: String) -> Result<String, String> {
            if !self.grants.graph_read {
                return Err(format!("graph read not granted for '{}'", self.plugin_name));
            }
            crate::mother::graph_host::query(&kind, &params_json).map_err(|e| e.to_string())
        }

        fn mutate(&mut self, action: String, payload_json: String) -> Result<(), String> {
            if !self.grants.graph_write_actions.contains(&action) {
                return Err(format!(
                    "graph action '{}' not granted for '{}'",
                    action, self.plugin_name
                ));
            }
            crate::mother::graph_host::mutate(&self.runtime, &self.plugin_name, &action, &payload_json)
                .map_err(|e| e.to_string())
        }
    }

    impl patina::host::belief::Host for HostState {
        fn query(&mut self, kind: String, params_json: String) -> Result<String, String> {
            if !self.grants.belief_read {
                return Err(format!("belief read not granted for '{}'", self.plugin_name));
            }
            crate::mother::belief_host::query(&kind, &params_json).map_err(|e| e.to_string())
        }

        fn mutate(&mut self, action: String, payload_json: String) -> Result<(), String> {
            if !self.grants.belief_write_actions.contains(&action) {
                return Err(format!(
                    "belief action '{}' not granted for '{}'",
                    action, self.plugin_name
                ));
            }
            crate::mother::belief_host::mutate(
                &self.runtime,
                &self.plugin_name,
                &action,
                &payload_json,
            )
            .map_err(|e| e.to_string())
        }
    }
}

use bindings::HostState;

pub struct KnowledgeChildEngine {
    linker: Linker<HostState>,
}

impl KnowledgeChildEngine {
    pub fn new() -> Result<Self> {
        let mut linker = Linker::new(wasm_engine());
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        bindings::KnowledgeChild::add_to_linker::<HostState, wasmtime::component::HasSelf<HostState>>(
            &mut linker,
            |s| s,
        )?;
        Ok(Self { linker })
    }

    pub fn load_manifest(path: &Path) -> Result<PluginManifest> {
        PluginManifest::from_path(path)
    }

    pub fn load_component(&self, wasm: &[u8]) -> Result<Component> {
        PluginManifest::load_component(wasm)
    }

    pub fn check_capabilities(manifest: &PluginManifest) -> Result<()> {
        if manifest.world != super::PluginWorld::KnowledgeChild {
            anyhow::bail!(
                "plugin '{}' has world '{}', expected 'knowledge-child'",
                manifest.name,
                manifest.world
            );
        }

        let allowed = manifest.world.allowed_capabilities();
        let world_denied: Vec<&str> = manifest
            .capabilities
            .iter()
            .filter(|cap| !allowed.contains(&cap.as_str()))
            .map(|s| s.as_str())
            .collect();
        if !world_denied.is_empty() {
            anyhow::bail!(
                "plugin '{}' (world '{}') requests capabilities not allowed for this world: {}",
                manifest.name,
                manifest.world,
                world_denied.join(", ")
            );
        }

        const AUTO_GRANTED: &[&str] = &[
            "host_log",
            "host_measure",
            "host_emit",
            "host_http",
            "host_query",
        ];
        let denied: Vec<&str> = manifest
            .capabilities
            .iter()
            .filter(|cap| !AUTO_GRANTED.contains(&cap.as_str()))
            .map(|s| s.as_str())
            .collect();
        if !denied.is_empty() {
            anyhow::bail!(
                "plugin '{}' requests capabilities not granted: {}",
                manifest.name,
                denied.join(", ")
            );
        }

        const KNOWN_STREAMS: &[&str] = &[
            "belief.changed",
            "graph.changed",
            "fact.ingested",
            "session.completed",
            "repo.synced",
        ];
        for stream in &manifest.subscribed_streams {
            if !KNOWN_STREAMS.contains(&stream.as_str()) {
                anyhow::bail!(
                    "plugin '{}' requests unknown event stream '{}'",
                    manifest.name,
                    stream
                );
            }
        }
        let unknown_intents: Vec<&str> = manifest
            .task_intent_names
            .iter()
            .filter(|intent| TaskIntentKind::parse(intent).is_none())
            .map(|s| s.as_str())
            .collect();
        if !unknown_intents.is_empty() {
            anyhow::bail!(
                "plugin '{}' requests unknown task intents: {}",
                manifest.name,
                unknown_intents.join(", ")
            );
        }

        const GRAPH_ACTIONS: &[&str] = &["link", "unlink", "weight", "tag"];
        let unknown_graph: Vec<&str> = manifest
            .graph_write_actions
            .iter()
            .filter(|action| !GRAPH_ACTIONS.contains(&action.as_str()))
            .map(|s| s.as_str())
            .collect();
        if !unknown_graph.is_empty() {
            anyhow::bail!(
                "plugin '{}' requests unknown graph write actions: {}",
                manifest.name,
                unknown_graph.join(", ")
            );
        }

        const BELIEF_ACTIONS: &[&str] =
            &["attach-evidence", "record-verification", "link-related", "supersede"];
        let unknown_belief: Vec<&str> = manifest
            .belief_write_actions
            .iter()
            .filter(|action| !BELIEF_ACTIONS.contains(&action.as_str()))
            .map(|s| s.as_str())
            .collect();
        if !unknown_belief.is_empty() {
            anyhow::bail!(
                "plugin '{}' requests unknown belief write actions: {}",
                manifest.name,
                unknown_belief.join(", ")
            );
        }

        if manifest.capabilities.contains(&"host_emit".to_string()) && manifest.schemas.is_empty() {
            anyhow::bail!(
                "plugin '{}' declares host_emit but has no [schemas.*] entries",
                manifest.name
            );
        }

        Ok(())
    }

    pub fn instantiate_child(
        &self,
        component: &Component,
        manifest: &PluginManifest,
        query_fn: Option<QueryDispatchFn>,
    ) -> Result<Box<dyn KnowledgeChild>> {
        Self::check_capabilities(manifest)?;

        let grants = manifest.granted_capabilities();
        let http_client = super::host_support::build_http_client()?;
        let wasi = wasmtime_wasi::WasiCtxBuilder::new().build();
        let project_root = crate::session::SessionManager::find_project_root().ok();
        let host_state = HostState {
            plugin_name: manifest.name.clone(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
            project_root,
            grants,
            query_fn,
            http_client,
            runtime: crate::mother::KnowledgeRuntimeStore::default(),
        };
        let mut store = Store::new(wasm_engine(), host_state);
        let instance = bindings::KnowledgeChild::instantiate(&mut store, component, &self.linker)?;
        instance.call_init(&mut store)?;
        let name = instance.call_name(&mut store)?;
        Ok(Box::new(WasmKnowledgeChild {
            name,
            inner: Mutex::new(WasmKnowledgeChildInner { store, instance }),
        }))
    }
}

struct WasmKnowledgeChild {
    name: String,
    inner: Mutex<WasmKnowledgeChildInner>,
}

struct WasmKnowledgeChildInner {
    store: Store<HostState>,
    instance: bindings::KnowledgeChild,
}

impl KnowledgeChild for WasmKnowledgeChild {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmKnowledgeChildInner { store, instance } = &mut *inner;
        match instance.call_on_load(store)? {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("WASM on_load failed: {}", e)),
        }
    }

    fn on_unload(&mut self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmKnowledgeChildInner { store, instance } = &mut *inner;
        let _ = instance.call_on_unload(store);
    }

    fn health(&self) -> ChildHealth {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmKnowledgeChildInner { store, instance } = &mut *inner;
        match instance.call_health(store) {
            Ok(h) => {
                let reason = h.reason.unwrap_or_default();
                match h.status {
                    bindings::patina::host::types::HealthStatus::Healthy => ChildHealth::Healthy,
                    bindings::patina::host::types::HealthStatus::Degraded => {
                        ChildHealth::Degraded(if reason.is_empty() { "degraded".into() } else { reason })
                    }
                    bindings::patina::host::types::HealthStatus::Unhealthy => {
                        ChildHealth::Unhealthy(if reason.is_empty() { "unhealthy".into() } else { reason })
                    }
                }
            }
            Err(e) => ChildHealth::Unhealthy(format!("WASM call failed: {}", e)),
        }
    }

    fn handle(&self, request: &ChildRequest) -> Result<ChildResponse> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmKnowledgeChildInner { store, instance } = &mut *inner;
        let payload_json = serde_json::to_string(&request.payload)?;
        match instance.call_handle(store, &request.action, &payload_json)? {
            Ok(json) => Ok(ChildResponse {
                payload: serde_json::from_str(&json)?,
            }),
            Err(e) => Err(anyhow::anyhow!("{}", e)),
        }
    }

    fn drain(&mut self, limit: u32) -> Result<Vec<PendingEvent>> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmKnowledgeChildInner { store, instance } = &mut *inner;
        match instance.call_drain(store, limit)? {
            Ok(events) => Ok(events
                .into_iter()
                .map(|event| PendingEvent {
                    stream: event.stream_name,
                    offset: event.offset,
                    event_type: event.event_type,
                    payload: serde_json::from_str(&event.payload_json)
                        .unwrap_or(serde_json::Value::Null),
                    occurred_at: event.occurred_at,
                })
                .collect()),
            Err(e) => Err(anyhow::anyhow!("{}", e)),
        }
    }

    fn tick(&mut self) -> Vec<TaskIntent> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmKnowledgeChildInner { store, instance } = &mut *inner;
        match instance.call_tick(store) {
            Ok(intents) => intents
                .into_iter()
                .filter_map(|intent| {
                    Some(TaskIntent {
                        kind: match intent.kind {
                            bindings::patina::host::task::TaskIntentKind::FetchSource => crate::mother::TaskIntentKind::FetchSource,
                            bindings::patina::host::task::TaskIntentKind::RunQuery => crate::mother::TaskIntentKind::RunQuery,
                            bindings::patina::host::task::TaskIntentKind::EmitFacts => crate::mother::TaskIntentKind::EmitFacts,
                            bindings::patina::host::task::TaskIntentKind::MaterializeIndex => crate::mother::TaskIntentKind::MaterializeIndex,
                            bindings::patina::host::task::TaskIntentKind::VerifyBelief => crate::mother::TaskIntentKind::VerifyBelief,
                            bindings::patina::host::task::TaskIntentKind::SyncGraph => crate::mother::TaskIntentKind::SyncGraph,
                            bindings::patina::host::task::TaskIntentKind::RefreshCredential => crate::mother::TaskIntentKind::RefreshCredential,
                            bindings::patina::host::task::TaskIntentKind::NativeJob => crate::mother::TaskIntentKind::NativeJob,
                        },
                        payload: serde_json::from_str(&intent.payload_json).ok()?,
                        dedupe_key: intent.dedupe_key,
                    })
                })
                .collect(),
            Err(e) => {
                eprintln!("[plugin:{}] tick failed: {}", self.name, e);
                vec![]
            }
        }
    }
}
