//! Knowledge-child world — bindgen, engine, and WASM adapter.

use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use wasmtime::component::{Component, Linker};
use wasmtime::Store;

use super::{wasm_engine, ChildKind, ChildManifest, GrantedCapabilities, QueryDispatchFn};
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

    impl wasi::logging::logging::Host for HostState {
        fn log(&mut self, level: wasi::logging::logging::Level, context: String, message: String) {
            let level_str = match level {
                wasi::logging::logging::Level::Trace => "TRACE",
                wasi::logging::logging::Level::Debug => "DEBUG",
                wasi::logging::logging::Level::Info => "INFO",
                wasi::logging::logging::Level::Warn => "WARN",
                wasi::logging::logging::Level::Error => "ERROR",
                wasi::logging::logging::Level::Critical => "CRITICAL",
            };
            let source = if context.trim().is_empty() {
                self.plugin_name.clone()
            } else {
                format!("{}:{}", self.plugin_name, context)
            };
            crate::child::toy_host::v2::log_emit(&source, level_str, &message);
        }
    }

    impl patina::knowledge_child::runtime_types::Host for HostState {}

    impl patina::state::state::Host for HostState {
        fn get(&mut self, key: String) -> Option<String> {
            crate::child::toy_host::v2::state_get(&self.runtime, &self.plugin_name, &key)
                .ok()
                .flatten()
        }

        fn set(&mut self, key: String, value_json: String) -> Result<(), String> {
            if !self.grants.state_enabled {
                return Err(format!(
                    "state not granted for child '{}'",
                    self.plugin_name
                ));
            }
            crate::child::toy_host::v2::state_set(
                &self.runtime,
                &self.plugin_name,
                &key,
                &value_json,
            )
        }

        fn delete(&mut self, key: String) -> Result<(), String> {
            if !self.grants.state_enabled {
                return Err(format!(
                    "state not granted for child '{}'",
                    self.plugin_name
                ));
            }
            crate::child::toy_host::v2::state_delete(&self.runtime, &self.plugin_name, &key)
        }

        fn list_prefix(&mut self, prefix: String) -> Vec<String> {
            crate::child::toy_host::v2::state_list_prefix(&self.runtime, &self.plugin_name, &prefix)
                .unwrap_or_default()
        }
    }

    impl patina::connect::connect::Host for HostState {
        fn resolve(
            &mut self,
            name: String,
        ) -> Result<wasmtime::component::Resource<patina::connect::connect::Connection>, String>
        {
            let conn = crate::child::toy_host::v2::connect_resolve(&name)?;
            let rep = self.wasi_table.push(conn).map_err(|e| e.to_string())?;
            Ok(wasmtime::component::Resource::new_own(rep.rep()))
        }

        fn base_url(
            &mut self,
            conn: wasmtime::component::Resource<patina::connect::connect::Connection>,
        ) -> String {
            let rep = wasmtime::component::Resource::<crate::child::toy_host::v2::ConnectionHandle>::new_borrow(conn.rep());
            self.wasi_table
                .get(&rep)
                .map(crate::child::toy_host::v2::connect_base_url)
                .unwrap_or_default()
        }

        fn request(
            &mut self,
            conn: wasmtime::component::Resource<patina::connect::connect::Connection>,
            method: String,
            path: String,
            headers: Vec<patina::connect::connect::Header>,
            body: Option<Vec<u8>>,
        ) -> Result<patina::connect::connect::Response, String> {
            let headers = headers
                .into_iter()
                .map(|h| crate::child::toy_host::v2::Header {
                    name: h.name,
                    value: h.value,
                })
                .collect::<Vec<_>>();
            let rep = wasmtime::component::Resource::<crate::child::toy_host::v2::ConnectionHandle>::new_borrow(conn.rep());
            let conn = self.wasi_table.get(&rep).map_err(|e| e.to_string())?;
            let response = crate::child::toy_host::v2::connect_request(
                &self.http_client,
                conn,
                &method,
                &path,
                &headers,
                body.as_deref(),
            )?;
            Ok(patina::connect::connect::Response {
                status: response.status,
                headers: response
                    .headers
                    .into_iter()
                    .map(|h| patina::connect::connect::Header {
                        name: h.name,
                        value: h.value,
                    })
                    .collect(),
                body: response.body,
            })
        }
    }

    impl patina::connect::connect::HostConnection for HostState {
        fn drop(
            &mut self,
            rep: wasmtime::component::Resource<patina::connect::connect::Connection>,
        ) -> wasmtime::Result<()> {
            let rep = wasmtime::component::Resource::<crate::child::toy_host::v2::ConnectionHandle>::new_own(rep.rep());
            Ok(self.wasi_table.delete(rep).map(|_| ())?)
        }
    }

    impl patina::store::store::Host for HostState {
        fn query(
            &mut self,
            conn: wasmtime::component::Resource<patina::connect::connect::Connection>,
            query: String,
        ) -> Result<String, String> {
            let rep = wasmtime::component::Resource::<crate::child::toy_host::v2::ConnectionHandle>::new_borrow(conn.rep());
            let conn = self.wasi_table.get(&rep).map_err(|e| e.to_string())?;
            crate::child::toy_host::v2::store_query(conn, &query)
        }

        fn mutate(
            &mut self,
            conn: wasmtime::component::Resource<patina::connect::connect::Connection>,
            action: String,
            payload: String,
        ) -> Result<String, String> {
            let rep = wasmtime::component::Resource::<crate::child::toy_host::v2::ConnectionHandle>::new_borrow(conn.rep());
            let conn = self.wasi_table.get(&rep).map_err(|e| e.to_string())?;
            crate::child::toy_host::v2::store_mutate(conn, &action, &payload)
        }
    }

    impl patina::events::events::Host for HostState {
        fn publish(
            &mut self,
            stream_name: String,
            event_type: String,
            payload: String,
        ) -> Result<u64, String> {
            if !self.grants.host_emit {
                return Err(format!(
                    "host_emit not granted for child '{}'",
                    self.plugin_name
                ));
            }
            crate::child::toy_host::v2::events_publish(
                &self.runtime,
                &self.plugin_name,
                &stream_name,
                &event_type,
                &payload,
            )
        }

        fn subscribe(
            &mut self,
            stream_name: String,
            after: Option<u64>,
            limit: u32,
        ) -> Result<Vec<patina::events::events::Event>, String> {
            if !self.grants.subscribed_streams.contains(&stream_name) {
                return Err(format!(
                    "event stream '{}' not granted for child '{}'",
                    stream_name, self.plugin_name
                ));
            }
            crate::child::toy_host::v2::events_subscribe(&stream_name, after, limit).map(|events| {
                events
                    .into_iter()
                    .map(|event| patina::events::events::Event {
                        stream_name: event.stream_name,
                        offset: event.offset,
                        event_type: event.event_type,
                        payload: event.payload,
                        occurred_at: event.occurred_at,
                    })
                    .collect()
            })
        }

        fn ack(&mut self, stream_name: String, offset: u64) -> Result<(), String> {
            if !self.grants.subscribed_streams.contains(&stream_name) {
                return Err(format!(
                    "event stream '{}' not granted for child '{}'",
                    stream_name, self.plugin_name
                ));
            }
            crate::child::toy_host::v2::events_ack(
                &self.runtime,
                &self.plugin_name,
                &stream_name,
                offset,
            )
        }
    }

    impl patina::task::task::Host for HostState {
        fn enqueue(
            &mut self,
            kind: String,
            payload: String,
            dedupe_key: Option<String>,
        ) -> Result<String, String> {
            let resolved = crate::mother::TaskIntentKind::parse(&kind)
                .ok_or_else(|| format!("unknown task intent kind '{}'", kind))?;
            if !self.grants.task_intents.contains(&resolved) {
                return Err(format!(
                    "task intent '{}' not granted for '{}'",
                    resolved, self.plugin_name
                ));
            }
            crate::child::toy_host::v2::task_enqueue(
                &self.runtime,
                &self.plugin_name,
                &kind,
                &payload,
                dedupe_key,
            )
        }
    }

    impl patina::peer::peer::Host for HostState {
        fn call(
            &mut self,
            child: String,
            action: String,
            payload: String,
        ) -> Result<String, String> {
            crate::child::toy_host::v2::peer_call(
                &self.runtime,
                &self.plugin_name,
                &child,
                &action,
                &payload,
            )
        }
    }

    impl patina::git::git::Host for HostState {
        fn create_tag(&mut self, name: String) -> Result<(), String> {
            crate::child::toy_host::v2::git_create_tag(&name)
        }

        fn delete_tag(&mut self, name: String) -> Result<(), String> {
            crate::child::toy_host::v2::git_delete_tag(&name)
        }

        fn tag_exists(&mut self, name: String) -> Result<bool, String> {
            crate::child::toy_host::v2::git_tag_exists(&name)
        }

        fn commit(&mut self, message: String) -> Result<String, String> {
            crate::child::toy_host::v2::git_commit(&message)
        }

        fn log_oneline(&mut self, limit: u32) -> Result<Vec<String>, String> {
            crate::child::toy_host::v2::git_log_oneline(limit)
        }

        fn diff_stat(&mut self) -> Result<String, String> {
            crate::child::toy_host::v2::git_diff_stat()
        }
    }
}

use bindings::HostState;

pub struct KnowledgeChildEngine {
    _unit: (),
}

impl KnowledgeChildEngine {
    fn link_wasi(linker: &mut Linker<HostState>) -> Result<()> {
        wasmtime_wasi::p2::add_to_linker_sync(linker)?;
        Ok(())
    }

    fn link_log(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::wasi::logging::logging::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_connect(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::connect::connect::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_store(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::store::store::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_events(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::events::events::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_task(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::task::task::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_state(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::state::state::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_peer(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::peer::peer::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_git(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::patina::git::git::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn build_linker(manifest: &ChildManifest) -> Result<Linker<HostState>> {
        let mut linker = Linker::new(wasm_engine());
        Self::link_wasi(&mut linker)?;
        Self::link_log(&mut linker)?;
        Self::link_state(&mut linker)?;
        Self::link_connect(&mut linker)?;
        Self::link_store(&mut linker)?;
        Self::link_events(&mut linker)?;
        Self::link_task(&mut linker)?;
        Self::link_peer(&mut linker)?;
        Self::link_git(&mut linker)?;

        let _ = manifest;

        Ok(linker)
    }

    pub fn new() -> Result<Self> {
        Ok(Self { _unit: () })
    }

    pub fn load_manifest(path: &Path) -> Result<ChildManifest> {
        ChildManifest::from_path(path)
    }

    pub fn load_component(&self, wasm: &[u8]) -> Result<Component> {
        ChildManifest::load_component(wasm)
    }

    pub fn check_capabilities(manifest: &ChildManifest) -> Result<()> {
        if manifest.world != ChildKind::KnowledgeChild {
            anyhow::bail!(
                "child '{}' has world '{}', expected 'knowledge-child'",
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
                "child '{}' (world '{}') requests capabilities not allowed for this world: {}",
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
                "child '{}' requests capabilities not granted: {}",
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
                    "child '{}' requests unknown event stream '{}'",
                    manifest.name,
                    stream
                );
            }
        }
        for source in manifest.ingress_sources.values() {
            super::host_support::validate_http_url(&source.endpoint).map_err(|error| {
                anyhow::anyhow!(
                    "child '{}' declares invalid ingress source '{}' endpoint '{}': {}",
                    manifest.name,
                    source.name,
                    source.endpoint,
                    error
                )
            })?;
        }
        let unknown_intents: Vec<&str> = manifest
            .task_intent_names
            .iter()
            .filter(|intent| TaskIntentKind::parse(intent).is_none())
            .map(|s| s.as_str())
            .collect();
        if !unknown_intents.is_empty() {
            anyhow::bail!(
                "child '{}' requests unknown task intents: {}",
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
                "child '{}' requests unknown graph write actions: {}",
                manifest.name,
                unknown_graph.join(", ")
            );
        }

        const BELIEF_ACTIONS: &[&str] = &[
            "attach-evidence",
            "record-verification",
            "link-related",
            "supersede",
        ];
        let unknown_belief: Vec<&str> = manifest
            .belief_write_actions
            .iter()
            .filter(|action| !BELIEF_ACTIONS.contains(&action.as_str()))
            .map(|s| s.as_str())
            .collect();
        if !unknown_belief.is_empty() {
            anyhow::bail!(
                "child '{}' requests unknown belief write actions: {}",
                manifest.name,
                unknown_belief.join(", ")
            );
        }

        if manifest.capabilities.contains(&"host_emit".to_string()) && manifest.schemas.is_empty() {
            anyhow::bail!(
                "child '{}' declares host_emit but has no [schemas.*] entries",
                manifest.name
            );
        }

        Ok(())
    }

    pub fn instantiate_child(
        &self,
        component: &Component,
        manifest: &ChildManifest,
        query_fn: Option<QueryDispatchFn>,
    ) -> Result<Box<dyn KnowledgeChild>> {
        Self::check_capabilities(manifest)?;

        let linker = Self::build_linker(manifest)?;

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
        let instance = bindings::KnowledgeChild::instantiate(&mut store, component, &linker)?;
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
                    bindings::patina::knowledge_child::runtime_types::HealthStatus::Healthy => {
                        ChildHealth::Healthy
                    }
                    bindings::patina::knowledge_child::runtime_types::HealthStatus::Degraded => {
                        ChildHealth::Degraded(if reason.is_empty() {
                            "degraded".into()
                        } else {
                            reason
                        })
                    }
                    bindings::patina::knowledge_child::runtime_types::HealthStatus::Unhealthy => {
                        ChildHealth::Unhealthy(if reason.is_empty() {
                            "unhealthy".into()
                        } else {
                            reason
                        })
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
                            bindings::patina::knowledge_child::runtime_types::TaskIntentKind::FetchSource => {
                                crate::mother::TaskIntentKind::FetchSource
                            }
                            bindings::patina::knowledge_child::runtime_types::TaskIntentKind::RunQuery => {
                                crate::mother::TaskIntentKind::RunQuery
                            }
                            bindings::patina::knowledge_child::runtime_types::TaskIntentKind::EmitFacts => {
                                crate::mother::TaskIntentKind::EmitFacts
                            }
                            bindings::patina::knowledge_child::runtime_types::TaskIntentKind::MaterializeIndex => {
                                crate::mother::TaskIntentKind::MaterializeIndex
                            }
                            bindings::patina::knowledge_child::runtime_types::TaskIntentKind::VerifyBelief => {
                                crate::mother::TaskIntentKind::VerifyBelief
                            }
                            bindings::patina::knowledge_child::runtime_types::TaskIntentKind::SyncGraph => {
                                crate::mother::TaskIntentKind::SyncGraph
                            }
                            bindings::patina::knowledge_child::runtime_types::TaskIntentKind::RefreshCredential => {
                                crate::mother::TaskIntentKind::RefreshCredential
                            }
                            bindings::patina::knowledge_child::runtime_types::TaskIntentKind::NativeJob => {
                                crate::mother::TaskIntentKind::NativeJob
                            }
                        },
                        payload: serde_json::from_str(&intent.payload_json).ok()?,
                        dedupe_key: intent.dedupe_key,
                    })
                })
                .collect(),
            Err(e) => {
                eprintln!("[child:{}] tick failed: {}", self.name, e);
                vec![]
            }
        }
    }
}
