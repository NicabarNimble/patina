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
    use std::collections::HashMap;

    #[allow(dead_code)]
    pub struct HostState {
        pub plugin_name: String,
        pub wasi: wasmtime_wasi::WasiCtx,
        pub wasi_table: wasmtime::component::ResourceTable,
        pub http: wasmtime_wasi_http::WasiHttpCtx,
        pub project_root: Option<std::path::PathBuf>,
        pub grants: super::GrantedCapabilities,
        pub query_fn: Option<super::QueryDispatchFn>,
        pub http_client: reqwest::blocking::Client,
        pub runtime: crate::mother::KnowledgeRuntimeStore,
        pub active_bindings: HashMap<u32, crate::child::toy_host::v2::ConnectionHandle>,
    }

    #[derive(Debug, Clone)]
    pub struct StateBucketHandle {
        pub identifier: String,
    }

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    pub struct SqlConnectionHandle {
        pub name: String,
        pub conn: crate::child::toy_host::v2::ConnectionHandle,
    }

    #[derive(Debug, Clone)]
    pub struct SqlStatementHandle {
        pub query: String,
        pub params: Vec<String>,
    }

    #[derive(Debug, Clone)]
    pub struct MessagingClientHandle {
        pub stream_name: String,
    }

    impl wasmtime_wasi::WasiView for HostState {
        fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
            wasmtime_wasi::WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.wasi_table,
            }
        }
    }

    impl wasmtime_wasi_http::WasiHttpView for HostState {
        fn ctx(&mut self) -> &mut wasmtime_wasi_http::WasiHttpCtx {
            &mut self.http
        }

        fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
            &mut self.wasi_table
        }

        fn send_request(
            &mut self,
            mut request: hyper::Request<wasmtime_wasi_http::body::HyperOutgoingBody>,
            config: wasmtime_wasi_http::types::OutgoingRequestConfig,
        ) -> wasmtime_wasi_http::HttpResult<wasmtime_wasi_http::types::HostFutureIncomingResponse>
        {
            let Some(host) = request.uri().host() else {
                return Ok(wasmtime_wasi_http::types::default_send_request(
                    request, config,
                ));
            };
            let host = crate::child::toy_host::v2::normalize_domain(host);

            let mut matched: Option<crate::child::toy_host::v2::ConnectionHandle> = None;
            for binding in self.active_bindings.values() {
                if crate::child::toy_host::v2::connect_matches_domain(binding, &host) {
                    if matched.is_some() {
                        return Err(
                            wasmtime_wasi_http::bindings::http::types::ErrorCode::ConfigurationError
                                .into(),
                        );
                    }
                    matched = Some(binding.clone());
                }
            }

            if let Some(binding) = matched {
                if let Some((name, value)) =
                    crate::child::toy_host::v2::connect_auth_header_for_domain(&binding, &host)
                        .map_err(|error| {
                            wasmtime_wasi_http::HttpError::trap(anyhow::anyhow!(error))
                        })?
                {
                    request.headers_mut().insert(
                        hyper::header::HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
                            wasmtime_wasi_http::bindings::http::types::ErrorCode::HttpRequestDenied
                        })?,
                        hyper::header::HeaderValue::from_str(&value).map_err(|_| {
                            wasmtime_wasi_http::bindings::http::types::ErrorCode::HttpRequestDenied
                        })?,
                    );
                }
            }

            Ok(wasmtime_wasi_http::types::default_send_request(
                request, config,
            ))
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

    fn bucket_scoped_key(bucket: &str, key: &str) -> String {
        if bucket == "default" {
            key.to_string()
        } else {
            format!("{}:{}", bucket, key)
        }
    }

    fn bucket_prefix(bucket: &str) -> String {
        if bucket == "default" {
            String::new()
        } else {
            format!("{}:", bucket)
        }
    }

    impl wasi::keyvalue::store::Host for HostState {
        fn open(
            &mut self,
            identifier: String,
        ) -> Result<wasmtime::component::Resource<wasi::keyvalue::store::Bucket>, String> {
            if !self.grants.state_enabled {
                return Err(format!(
                    "state not granted for child '{}'",
                    self.plugin_name
                ));
            }
            let handle = StateBucketHandle { identifier };
            let rep = self.wasi_table.push(handle).map_err(|e| e.to_string())?;
            Ok(wasmtime::component::Resource::new_own(rep.rep()))
        }
    }

    impl wasi::keyvalue::store::HostBucket for HostState {
        fn get(
            &mut self,
            bucket: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            key: String,
        ) -> Result<Option<Vec<u8>>, String> {
            let rep = wasmtime::component::Resource::<StateBucketHandle>::new_borrow(bucket.rep());
            let handle = self.wasi_table.get(&rep).map_err(|e| e.to_string())?;
            let scoped = bucket_scoped_key(&handle.identifier, &key);
            let value =
                crate::child::toy_host::v2::state_get(&self.runtime, &self.plugin_name, &scoped)?;
            Ok(value.map(|v| v.into_bytes()))
        }

        fn set(
            &mut self,
            bucket: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            key: String,
            value: Vec<u8>,
        ) -> Result<(), String> {
            let rep = wasmtime::component::Resource::<StateBucketHandle>::new_borrow(bucket.rep());
            let handle = self.wasi_table.get(&rep).map_err(|e| e.to_string())?;
            let scoped = bucket_scoped_key(&handle.identifier, &key);
            let value = String::from_utf8(value)
                .map_err(|e| format!("state value for '{}' is not valid UTF-8: {}", key, e))?;
            crate::child::toy_host::v2::state_set(&self.runtime, &self.plugin_name, &scoped, &value)
        }

        fn delete(
            &mut self,
            bucket: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            key: String,
        ) -> Result<(), String> {
            let rep = wasmtime::component::Resource::<StateBucketHandle>::new_borrow(bucket.rep());
            let handle = self.wasi_table.get(&rep).map_err(|e| e.to_string())?;
            let scoped = bucket_scoped_key(&handle.identifier, &key);
            crate::child::toy_host::v2::state_delete(&self.runtime, &self.plugin_name, &scoped)
        }

        fn exists(
            &mut self,
            bucket: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            key: String,
        ) -> Result<bool, String> {
            let rep = wasmtime::component::Resource::<StateBucketHandle>::new_borrow(bucket.rep());
            let handle = self.wasi_table.get(&rep).map_err(|e| e.to_string())?;
            let scoped = bucket_scoped_key(&handle.identifier, &key);
            Ok(
                crate::child::toy_host::v2::state_get(&self.runtime, &self.plugin_name, &scoped)?
                    .is_some(),
            )
        }

        fn list_keys(
            &mut self,
            bucket: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            cursor: Option<String>,
        ) -> Result<wasi::keyvalue::store::KeyResponse, String> {
            let rep = wasmtime::component::Resource::<StateBucketHandle>::new_borrow(bucket.rep());
            let handle = self.wasi_table.get(&rep).map_err(|e| e.to_string())?;
            let prefix = bucket_prefix(&handle.identifier);
            let mut keys = crate::child::toy_host::v2::state_list_prefix(
                &self.runtime,
                &self.plugin_name,
                &prefix,
            )
            .unwrap_or_default();
            if !prefix.is_empty() {
                keys = keys
                    .into_iter()
                    .filter_map(|k| k.strip_prefix(&prefix).map(ToString::to_string))
                    .collect();
            }
            keys.sort();
            let start = cursor
                .as_deref()
                .and_then(|c| c.parse::<usize>().ok())
                .unwrap_or(0);
            let page_size = 200usize;
            let end = std::cmp::min(start + page_size, keys.len());
            let next = if end < keys.len() {
                Some(end.to_string())
            } else {
                None
            };
            Ok(wasi::keyvalue::store::KeyResponse {
                keys: keys[start..end].to_vec(),
                cursor: next,
            })
        }

        fn drop(
            &mut self,
            rep: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
        ) -> wasmtime::Result<()> {
            let rep = wasmtime::component::Resource::<StateBucketHandle>::new_own(rep.rep());
            Ok(self.wasi_table.delete(rep).map(|_| ())?)
        }
    }

    impl patina::connect::connect::Host for HostState {
        fn resolve(
            &mut self,
            name: String,
        ) -> Result<wasmtime::component::Resource<patina::connect::connect::Binding>, String>
        {
            let binding = crate::child::toy_host::v2::connect_resolve(&name)?;
            let rep = self.wasi_table.push(binding).map_err(|e| e.to_string())?;
            let binding = self
                .wasi_table
                .get(&rep)
                .map_err(|e| e.to_string())?
                .clone();
            self.active_bindings.insert(rep.rep(), binding);
            Ok(wasmtime::component::Resource::new_own(rep.rep()))
        }
    }

    impl patina::connect::connect::HostBinding for HostState {
        fn drop(
            &mut self,
            rep: wasmtime::component::Resource<patina::connect::connect::Binding>,
        ) -> wasmtime::Result<()> {
            self.active_bindings.remove(&rep.rep());
            let rep = wasmtime::component::Resource::<crate::child::toy_host::v2::ConnectionHandle>::new_own(rep.rep());
            Ok(self.wasi_table.delete(rep).map(|_| ())?)
        }
    }

    impl wasi::sql::readwrite::Host for HostState {
        fn open(
            &mut self,
            name: String,
        ) -> Result<wasmtime::component::Resource<wasi::sql::readwrite::Connection>, String>
        {
            let conn = crate::child::toy_host::v2::connect_resolve(&name)?;
            let handle = SqlConnectionHandle { name, conn };
            let rep = self.wasi_table.push(handle).map_err(|e| e.to_string())?;
            Ok(wasmtime::component::Resource::new_own(rep.rep()))
        }

        fn prepare(
            &mut self,
            query: String,
            params: Vec<String>,
        ) -> Result<wasmtime::component::Resource<wasi::sql::readwrite::Statement>, String>
        {
            let stmt = SqlStatementHandle { query, params };
            let rep = self.wasi_table.push(stmt).map_err(|e| e.to_string())?;
            Ok(wasmtime::component::Resource::new_own(rep.rep()))
        }

        fn query(
            &mut self,
            c: wasmtime::component::Resource<wasi::sql::readwrite::Connection>,
            s: wasmtime::component::Resource<wasi::sql::readwrite::Statement>,
        ) -> Result<Vec<wasi::sql::readwrite::Row>, String> {
            let c_rep = wasmtime::component::Resource::<SqlConnectionHandle>::new_borrow(c.rep());
            let s_rep = wasmtime::component::Resource::<SqlStatementHandle>::new_borrow(s.rep());
            let conn = self.wasi_table.get(&c_rep).map_err(|e| e.to_string())?;
            let stmt = self.wasi_table.get(&s_rep).map_err(|e| e.to_string())?;

            let result = crate::child::toy_host::v2::store_query(&conn.conn, &stmt.query)?;
            Ok(vec![wasi::sql::readwrite::Row {
                values: vec![wasi::sql::readwrite::DataType::Text(result)],
            }])
        }

        fn exec(
            &mut self,
            c: wasmtime::component::Resource<wasi::sql::readwrite::Connection>,
            s: wasmtime::component::Resource<wasi::sql::readwrite::Statement>,
        ) -> Result<u32, String> {
            let c_rep = wasmtime::component::Resource::<SqlConnectionHandle>::new_borrow(c.rep());
            let s_rep = wasmtime::component::Resource::<SqlStatementHandle>::new_borrow(s.rep());
            let conn = self.wasi_table.get(&c_rep).map_err(|e| e.to_string())?;
            let stmt = self.wasi_table.get(&s_rep).map_err(|e| e.to_string())?;

            if stmt.query == "__patina_mutate__" {
                let action = stmt.params.first().cloned().unwrap_or_default();
                let payload = stmt.params.get(1).cloned().unwrap_or_default();
                let out = crate::child::toy_host::v2::store_mutate(&conn.conn, &action, &payload)?;
                return out.parse::<u32>().map_err(|e| e.to_string());
            }

            let out = crate::child::toy_host::v2::store_query(&conn.conn, &stmt.query)?;
            out.parse::<u32>().map_err(|e| e.to_string())
        }
    }

    impl wasi::sql::readwrite::HostConnection for HostState {
        fn drop(
            &mut self,
            rep: wasmtime::component::Resource<wasi::sql::readwrite::Connection>,
        ) -> wasmtime::Result<()> {
            let rep = wasmtime::component::Resource::<SqlConnectionHandle>::new_own(rep.rep());
            Ok(self.wasi_table.delete(rep).map(|_| ())?)
        }
    }

    impl wasi::sql::readwrite::HostStatement for HostState {
        fn query(
            &mut self,
            rep: wasmtime::component::Resource<wasi::sql::readwrite::Statement>,
        ) -> String {
            let rep = wasmtime::component::Resource::<SqlStatementHandle>::new_borrow(rep.rep());
            self.wasi_table
                .get(&rep)
                .map(|s| s.query.clone())
                .unwrap_or_default()
        }

        fn params(
            &mut self,
            rep: wasmtime::component::Resource<wasi::sql::readwrite::Statement>,
        ) -> Vec<String> {
            let rep = wasmtime::component::Resource::<SqlStatementHandle>::new_borrow(rep.rep());
            self.wasi_table
                .get(&rep)
                .map(|s| s.params.clone())
                .unwrap_or_default()
        }

        fn drop(
            &mut self,
            rep: wasmtime::component::Resource<wasi::sql::readwrite::Statement>,
        ) -> wasmtime::Result<()> {
            let rep = wasmtime::component::Resource::<SqlStatementHandle>::new_own(rep.rep());
            Ok(self.wasi_table.delete(rep).map(|_| ())?)
        }
    }

    impl wasi::messaging::producer::Host for HostState {
        fn connect(
            &mut self,
            name: String,
        ) -> Result<wasmtime::component::Resource<wasi::messaging::producer::Client>, String>
        {
            let handle = MessagingClientHandle { stream_name: name };
            let rep = self.wasi_table.push(handle).map_err(|e| e.to_string())?;
            Ok(wasmtime::component::Resource::new_own(rep.rep()))
        }

        fn send(
            &mut self,
            client: wasmtime::component::Resource<wasi::messaging::producer::Client>,
            message: wasi::messaging::types::Message,
        ) -> Result<u64, String> {
            if !self.grants.toys.events {
                return Err(format!(
                    "events toy not granted for child '{}'",
                    self.plugin_name
                ));
            }
            let rep =
                wasmtime::component::Resource::<MessagingClientHandle>::new_borrow(client.rep());
            let client = self.wasi_table.get(&rep).map_err(|e| e.to_string())?;
            let event_type = message.topic;
            let payload = String::from_utf8(message.data).map_err(|e| e.to_string())?;
            crate::child::toy_host::v2::events_publish(
                &self.runtime,
                &self.plugin_name,
                &client.stream_name,
                &event_type,
                &payload,
            )
        }
    }

    impl wasi::messaging::producer::HostClient for HostState {
        fn drop(
            &mut self,
            rep: wasmtime::component::Resource<wasi::messaging::producer::Client>,
        ) -> wasmtime::Result<()> {
            let rep = wasmtime::component::Resource::<MessagingClientHandle>::new_own(rep.rep());
            Ok(self.wasi_table.delete(rep).map(|_| ())?)
        }
    }

    impl patina::events_stream::events_stream::Host for HostState {
        fn subscribe(
            &mut self,
            stream_name: String,
            after: Option<u64>,
            limit: u32,
        ) -> Result<Vec<patina::events_stream::events_stream::Event>, String> {
            if !self.grants.subscribed_streams.contains(&stream_name) {
                return Err(format!(
                    "event stream '{}' not granted for child '{}'",
                    stream_name, self.plugin_name
                ));
            }
            crate::child::toy_host::v2::events_subscribe(&stream_name, after, limit).map(|events| {
                events
                    .into_iter()
                    .map(|event| patina::events_stream::events_stream::Event {
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

    impl patina::measure::measure::Host for HostState {
        fn emit(&mut self, metric: patina::measure::measure::Metric) -> Result<(), String> {
            if !self.grants.toys.measure {
                return Err(format!(
                    "measure toy not granted for child '{}'",
                    self.plugin_name
                ));
            }
            let metric_type = self
                .grants
                .declared_metrics
                .get(&metric.name)
                .map(|declared| declared.metric_type.clone())
                .unwrap_or(crate::child::internal::DeclaredMetricType::Gauge);
            crate::child::internal::host_support::record_declared_metric(
                &self.plugin_name,
                &self.grants.declared_metrics,
                &metric.name,
                metric_type,
                metric.value,
                &metric.labels,
            )
        }

        fn gauge(&mut self, name: String, value: f64) -> Result<(), String> {
            if !self.grants.toys.measure {
                return Err(format!(
                    "measure toy not granted for child '{}'",
                    self.plugin_name
                ));
            }
            crate::child::internal::host_support::record_declared_metric(
                &self.plugin_name,
                &self.grants.declared_metrics,
                &name,
                crate::child::internal::DeclaredMetricType::Gauge,
                value,
                &[],
            )
        }

        fn counter(&mut self, name: String, delta: f64) -> Result<(), String> {
            if !self.grants.toys.measure {
                return Err(format!(
                    "measure toy not granted for child '{}'",
                    self.plugin_name
                ));
            }
            crate::child::internal::host_support::record_declared_metric(
                &self.plugin_name,
                &self.grants.declared_metrics,
                &name,
                crate::child::internal::DeclaredMetricType::Counter,
                delta,
                &[],
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

#[derive(Debug, Clone)]
pub struct FilesystemPreopen {
    pub host_path: std::path::PathBuf,
    pub guest_path: String,
    pub mode: crate::child::internal::FilesystemAccessMode,
}

impl KnowledgeChildEngine {
    fn link_wasi(linker: &mut Linker<HostState>) -> Result<()> {
        wasmtime_wasi::p2::add_to_linker_sync(linker)?;
        wasmtime_wasi_http::add_only_http_to_linker_sync(linker)?;
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
        bindings::wasi::sql::readwrite::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        Ok(())
    }

    fn link_events(linker: &mut Linker<HostState>) -> Result<()> {
        bindings::wasi::messaging::producer::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        bindings::patina::events_stream::events_stream::add_to_linker::<
            HostState,
            wasmtime::component::HasSelf<HostState>,
        >(linker, |s| s)?;
        bindings::patina::measure::measure::add_to_linker::<
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
        bindings::wasi::keyvalue::store::add_to_linker::<
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

        let denied: Vec<&str> = manifest
            .capabilities
            .iter()
            .filter(|cap| !super::AUTO_GRANTED_CAPABILITIES.contains(&cap.as_str()))
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
            "file.found",
            "file.written",
            "record.extracted",
            "record.validated",
            "record.rejected",
            "record.ready",
            "record.duplicate",
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
        self.instantiate_child_with_preopens(component, manifest, query_fn, &[])
    }

    pub fn instantiate_child_with_preopens(
        &self,
        component: &Component,
        manifest: &ChildManifest,
        query_fn: Option<QueryDispatchFn>,
        test_preopens: &[FilesystemPreopen],
    ) -> Result<Box<dyn KnowledgeChild>> {
        Self::check_capabilities(manifest)?;

        let linker = Self::build_linker(manifest)?;

        let grants = manifest.granted_capabilities();
        let http_client = super::host_support::build_http_client()?;
        let mut wasi_builder = wasmtime_wasi::WasiCtxBuilder::new();
        for mount in &manifest.filesystem_preopens {
            let host_path = std::path::PathBuf::from(&mount.host_path);
            let (dir_perms, file_perms) = match mount.mode {
                crate::child::internal::FilesystemAccessMode::ReadOnly => (
                    wasmtime_wasi::DirPerms::READ,
                    wasmtime_wasi::FilePerms::READ,
                ),
                crate::child::internal::FilesystemAccessMode::ReadWrite => (
                    wasmtime_wasi::DirPerms::READ | wasmtime_wasi::DirPerms::MUTATE,
                    wasmtime_wasi::FilePerms::READ | wasmtime_wasi::FilePerms::WRITE,
                ),
            };
            wasi_builder.preopened_dir(&host_path, &mount.guest_path, dir_perms, file_perms)?;
        }
        for mount in test_preopens {
            let (dir_perms, file_perms) = match mount.mode {
                crate::child::internal::FilesystemAccessMode::ReadOnly => (
                    wasmtime_wasi::DirPerms::READ,
                    wasmtime_wasi::FilePerms::READ,
                ),
                crate::child::internal::FilesystemAccessMode::ReadWrite => (
                    wasmtime_wasi::DirPerms::READ | wasmtime_wasi::DirPerms::MUTATE,
                    wasmtime_wasi::FilePerms::READ | wasmtime_wasi::FilePerms::WRITE,
                ),
            };
            wasi_builder.preopened_dir(
                &mount.host_path,
                &mount.guest_path,
                dir_perms,
                file_perms,
            )?;
        }
        let wasi = wasi_builder.build();
        let project_root = crate::session::SessionManager::find_project_root().ok();
        let host_state = HostState {
            plugin_name: manifest.name.clone(),
            wasi,
            wasi_table: wasmtime::component::ResourceTable::new(),
            http: wasmtime_wasi_http::WasiHttpCtx::new(),
            project_root,
            grants,
            query_fn,
            http_client,
            runtime: crate::mother::KnowledgeRuntimeStore::default(),
            active_bindings: std::collections::HashMap::new(),
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
