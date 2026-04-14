//! Child world — bindgen, engine, and WASM adapter.

use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use wasmtime::component::types::ComponentFunc;
use wasmtime::component::{Component, Linker};
use wasmtime::{AsContext, AsContextMut, Store};

use super::{wasm_engine, ChildKind, ChildManifest, GrantedCapabilities, QueryDispatchFn};
use crate::mother::{
    Child, ChildCallRequest, ChildHealth, ChildRequest, ChildResponse, MotherHost, PendingEvent,
    TaskIntent, TaskIntentKind,
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
        pub runtime: crate::mother::MotherRuntimeStore,
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
        path: "wit/child/",
        world: "child",
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

    impl patina::child::runtime_types::Host for HostState {}

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

    fn encode_state_value(value: &[u8]) -> Result<String, String> {
        serde_json::to_string(value).map_err(|error| error.to_string())
    }

    fn decode_state_value(raw: &str) -> Vec<u8> {
        serde_json::from_str::<Vec<u8>>(raw).unwrap_or_else(|_| raw.as_bytes().to_vec())
    }

    fn keyvalue_other(error: impl ToString) -> wasi::keyvalue::store::Error {
        wasi::keyvalue::store::Error::Other(error.to_string())
    }

    fn json_value_to_sql_data(value: serde_json::Value) -> wasi::sql::readwrite::DataType {
        match value {
            serde_json::Value::Null => wasi::sql::readwrite::DataType::Null,
            serde_json::Value::Bool(v) => wasi::sql::readwrite::DataType::Flag(v),
            serde_json::Value::Number(number) => {
                if let Some(v) = number.as_i64() {
                    if (i32::MIN as i64..=i32::MAX as i64).contains(&v) {
                        wasi::sql::readwrite::DataType::Int32(v as i32)
                    } else {
                        wasi::sql::readwrite::DataType::Int64(v)
                    }
                } else {
                    wasi::sql::readwrite::DataType::Float64(number.as_f64().unwrap_or_default())
                }
            }
            serde_json::Value::String(v) => wasi::sql::readwrite::DataType::Text(v),
            serde_json::Value::Array(values) => wasi::sql::readwrite::DataType::Text(
                serde_json::to_string(&values).unwrap_or_default(),
            ),
            serde_json::Value::Object(values) => wasi::sql::readwrite::DataType::Text(
                serde_json::to_string(&values).unwrap_or_default(),
            ),
        }
    }

    fn decode_sql_rows(result: &str) -> Vec<wasi::sql::readwrite::Row> {
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(result) else {
            return vec![wasi::sql::readwrite::Row {
                values: vec![wasi::sql::readwrite::DataType::Text(result.to_string())],
            }];
        };
        let Some(rows) = parsed.as_array() else {
            return vec![wasi::sql::readwrite::Row {
                values: vec![json_value_to_sql_data(parsed)],
            }];
        };
        rows.iter()
            .map(|row| {
                let mut values = Vec::new();
                if let Some(object) = row.as_object() {
                    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
                    keys.sort_unstable();
                    for key in keys {
                        if let Some(value) = object.get(key) {
                            values.push(json_value_to_sql_data(value.clone()));
                        }
                    }
                } else {
                    values.push(json_value_to_sql_data(row.clone()));
                }
                wasi::sql::readwrite::Row { values }
            })
            .collect()
    }

    impl wasi::keyvalue::store::Host for HostState {
        fn open(
            &mut self,
            identifier: String,
        ) -> Result<
            wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            wasi::keyvalue::store::Error,
        > {
            if !self.grants.state_enabled {
                return Err(wasi::keyvalue::store::Error::AccessDenied);
            }
            let handle = StateBucketHandle { identifier };
            let rep = self.wasi_table.push(handle).map_err(keyvalue_other)?;
            Ok(wasmtime::component::Resource::new_own(rep.rep()))
        }
    }

    impl wasi::keyvalue::store::HostBucket for HostState {
        fn get(
            &mut self,
            bucket: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            key: String,
        ) -> Result<Option<Vec<u8>>, wasi::keyvalue::store::Error> {
            let rep = wasmtime::component::Resource::<StateBucketHandle>::new_borrow(bucket.rep());
            let handle = self.wasi_table.get(&rep).map_err(keyvalue_other)?;
            let scoped = bucket_scoped_key(&handle.identifier, &key);
            let value =
                crate::child::toy_host::v2::state_get(&self.runtime, &self.plugin_name, &scoped)
                    .map_err(keyvalue_other)?;
            Ok(value.map(|raw| decode_state_value(&raw)))
        }

        fn set(
            &mut self,
            bucket: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            key: String,
            value: Vec<u8>,
        ) -> Result<(), wasi::keyvalue::store::Error> {
            let rep = wasmtime::component::Resource::<StateBucketHandle>::new_borrow(bucket.rep());
            let handle = self.wasi_table.get(&rep).map_err(keyvalue_other)?;
            let scoped = bucket_scoped_key(&handle.identifier, &key);
            let encoded = encode_state_value(&value).map_err(keyvalue_other)?;
            crate::child::toy_host::v2::state_set(
                &self.runtime,
                &self.plugin_name,
                &scoped,
                &encoded,
            )
            .map_err(keyvalue_other)
        }

        fn delete(
            &mut self,
            bucket: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            key: String,
        ) -> Result<(), wasi::keyvalue::store::Error> {
            let rep = wasmtime::component::Resource::<StateBucketHandle>::new_borrow(bucket.rep());
            let handle = self.wasi_table.get(&rep).map_err(keyvalue_other)?;
            let scoped = bucket_scoped_key(&handle.identifier, &key);
            crate::child::toy_host::v2::state_delete(&self.runtime, &self.plugin_name, &scoped)
                .map_err(keyvalue_other)
        }

        fn exists(
            &mut self,
            bucket: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            key: String,
        ) -> Result<bool, wasi::keyvalue::store::Error> {
            let rep = wasmtime::component::Resource::<StateBucketHandle>::new_borrow(bucket.rep());
            let handle = self.wasi_table.get(&rep).map_err(keyvalue_other)?;
            let scoped = bucket_scoped_key(&handle.identifier, &key);
            Ok(
                crate::child::toy_host::v2::state_get(&self.runtime, &self.plugin_name, &scoped)
                    .map_err(keyvalue_other)?
                    .is_some(),
            )
        }

        fn list_keys(
            &mut self,
            bucket: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            cursor: Option<u64>,
        ) -> Result<wasi::keyvalue::store::KeyResponse, wasi::keyvalue::store::Error> {
            let rep = wasmtime::component::Resource::<StateBucketHandle>::new_borrow(bucket.rep());
            let handle = self.wasi_table.get(&rep).map_err(keyvalue_other)?;
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
            let start = cursor.and_then(|c| usize::try_from(c).ok()).unwrap_or(0);
            let page_size = 200usize;
            let end = std::cmp::min(start + page_size, keys.len());
            let next = if end < keys.len() {
                Some(end as u64)
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
            if !self.grants.toys.sql {
                return Err(format!(
                    "sql toy not granted for child '{}'",
                    self.plugin_name
                ));
            }
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
            Ok(decode_sql_rows(&result))
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
            if !self.grants.toys.messaging {
                return Err(format!(
                    "messaging toy not granted for child '{}'",
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
            if !self.grants.toys.git {
                return Err(format!(
                    "git toy not granted for child '{}'",
                    self.plugin_name
                ));
            }
            crate::child::toy_host::v2::git_create_tag(&name)
        }

        fn delete_tag(&mut self, name: String) -> Result<(), String> {
            if !self.grants.toys.git {
                return Err(format!(
                    "git toy not granted for child '{}'",
                    self.plugin_name
                ));
            }
            crate::child::toy_host::v2::git_delete_tag(&name)
        }

        fn tag_exists(&mut self, name: String) -> Result<bool, String> {
            if !self.grants.toys.git {
                return Err(format!(
                    "git toy not granted for child '{}'",
                    self.plugin_name
                ));
            }
            crate::child::toy_host::v2::git_tag_exists(&name)
        }

        fn commit(&mut self, message: String) -> Result<String, String> {
            if !self.grants.toys.git {
                return Err(format!(
                    "git toy not granted for child '{}'",
                    self.plugin_name
                ));
            }
            crate::child::toy_host::v2::git_commit(&message)
        }

        fn log_oneline(&mut self, limit: u32) -> Result<Vec<String>, String> {
            if !self.grants.toys.git {
                return Err(format!(
                    "git toy not granted for child '{}'",
                    self.plugin_name
                ));
            }
            crate::child::toy_host::v2::git_log_oneline(limit)
        }

        fn diff_stat(&mut self) -> Result<String, String> {
            if !self.grants.toys.git {
                return Err(format!(
                    "git toy not granted for child '{}'",
                    self.plugin_name
                ));
            }
            crate::child::toy_host::v2::git_diff_stat()
        }
    }
}

use bindings::HostState;

pub struct ChildEngine {
    _unit: (),
}

#[derive(Debug, Clone)]
pub struct FilesystemPreopen {
    pub host_path: std::path::PathBuf,
    pub guest_path: String,
    pub mode: crate::child::internal::FilesystemAccessMode,
}

impl ChildEngine {
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
        if manifest.world != ChildKind::Child {
            anyhow::bail!(
                "child '{}' has world '{}', expected 'child'",
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
    ) -> Result<Box<dyn Child>> {
        self.instantiate_child_with_preopens(component, manifest, query_fn, &[])
    }

    pub fn instantiate_child_with_preopens(
        &self,
        component: &Component,
        manifest: &ChildManifest,
        query_fn: Option<QueryDispatchFn>,
        test_preopens: &[FilesystemPreopen],
    ) -> Result<Box<dyn Child>> {
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
            runtime: crate::mother::MotherRuntimeStore::default(),
            active_bindings: std::collections::HashMap::new(),
        };
        let mut store = Store::new(wasm_engine(), host_state);
        let instance_pre = linker.instantiate_pre(component)?;
        let component_instance = instance_pre.instantiate(&mut store)?;
        let instance = bindings::Child::new(&mut store, &component_instance)?;
        instance.call_init(&mut store)?;
        let name = instance.call_name(&mut store)?;
        let invocation_driver: Box<dyn InvocationDriver + Send + Sync> =
            if std::env::var("PATINA_TYPED_CALL_FAIL_CLOSED")
                .ok()
                .as_deref()
                == Some("1")
            {
                Box::new(FailClosedInvocationDriver)
            } else if std::env::var("PATINA_TYPED_CALL_DRIVER").ok().as_deref()
                == Some("handle-bridge")
            {
                Box::new(HandleBridgeInvocationDriver)
            } else {
                Box::new(TypedComponentInvocationDriver)
            };

        tracing::info!(
            child = %name,
            driver = if std::env::var("PATINA_TYPED_CALL_FAIL_CLOSED").ok().as_deref() == Some("1") {
                "fail-closed"
            } else if std::env::var("PATINA_TYPED_CALL_DRIVER").ok().as_deref() == Some("handle-bridge") {
                "handle-bridge"
            } else {
                "typed-component"
            },
            "typed invocation driver selected"
        );

        Ok(Box::new(WasmChild {
            name,
            inner: Mutex::new(WasmChildInner {
                store,
                instance,
                component_instance,
            }),
            invocation_driver,
        }))
    }
}

struct WasmChild {
    name: String,
    inner: Mutex<WasmChildInner>,
    invocation_driver: Box<dyn InvocationDriver + Send + Sync>,
}

struct WasmChildInner {
    store: Store<HostState>,
    instance: bindings::Child,
    component_instance: wasmtime::component::Instance,
}

#[derive(Debug, Clone, Copy)]
enum TypedInvocationErrorCode {
    InvalidOperationId,
    InvalidArgsShape,
    UnsupportedType,
    ChildReturnedError,
    ChildTrap,
    InvalidChildJson,
    NotImplemented,
}

impl TypedInvocationErrorCode {
    fn as_str(self) -> &'static str {
        match self {
            Self::InvalidOperationId => "invalid-operation-id",
            Self::InvalidArgsShape => "invalid-args-shape",
            Self::UnsupportedType => "unsupported-type",
            Self::ChildReturnedError => "child-returned-error",
            Self::ChildTrap => "child-trap",
            Self::InvalidChildJson => "invalid-child-json",
            Self::NotImplemented => "not-implemented",
        }
    }
}

fn typed_invocation_error(
    code: TypedInvocationErrorCode,
    message: impl std::fmt::Display,
) -> anyhow::Error {
    anyhow::anyhow!("typed invocation {}: {}", code.as_str(), message)
}

#[derive(Debug, Clone)]
struct ResolvedTypedOperation {
    interface: String,
    function: String,
    action: String,
}

fn resolve_typed_operation(operation_id: &str) -> Result<ResolvedTypedOperation> {
    let operation_id = operation_id.trim();
    let Some((interface, function)) = operation_id.rsplit_once('.') else {
        return Err(typed_invocation_error(
            TypedInvocationErrorCode::InvalidOperationId,
            format!(
                "expected '<package>:<interface>.<function>' operation id, got '{}'",
                operation_id
            ),
        ));
    };
    if interface.trim().is_empty() || function.trim().is_empty() {
        return Err(typed_invocation_error(
            TypedInvocationErrorCode::InvalidOperationId,
            format!(
                "operation id '{}' has empty interface/function",
                operation_id
            ),
        ));
    }
    if !function
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(typed_invocation_error(
            TypedInvocationErrorCode::InvalidOperationId,
            format!(
                "operation '{}' has unsupported function token '{}': only [A-Za-z0-9_-] allowed",
                operation_id, function
            ),
        ));
    }

    Ok(ResolvedTypedOperation {
        interface: interface.to_string(),
        function: function.to_string(),
        action: function.replace('_', "-"),
    })
}

fn encode_typed_args_for_handle(args: &serde_json::Value) -> Result<String> {
    let Some(list) = args.as_array() else {
        return Err(typed_invocation_error(
            TypedInvocationErrorCode::InvalidArgsShape,
            "typed call args must be a JSON array",
        ));
    };

    match list.len() {
        0 => Ok("{}".to_string()),
        1 => serde_json::to_string(&list[0])
            .map_err(|e| typed_invocation_error(TypedInvocationErrorCode::InvalidArgsShape, e)),
        _ => serde_json::to_string(args)
            .map_err(|e| typed_invocation_error(TypedInvocationErrorCode::InvalidArgsShape, e)),
    }
}

trait InvocationDriver {
    fn call(
        &self,
        child_name: &str,
        inner: &mut WasmChildInner,
        request: &ChildCallRequest,
    ) -> Result<ChildResponse>;
}

struct FailClosedInvocationDriver;

impl InvocationDriver for FailClosedInvocationDriver {
    fn call(
        &self,
        child_name: &str,
        _inner: &mut WasmChildInner,
        request: &ChildCallRequest,
    ) -> Result<ChildResponse> {
        Err(typed_invocation_error(
            TypedInvocationErrorCode::NotImplemented,
            format!(
                "typed call not implemented for child '{}' operation '{}'",
                child_name, request.operation_id
            ),
        ))
    }
}

struct HandleBridgeInvocationDriver;

impl InvocationDriver for HandleBridgeInvocationDriver {
    fn call(
        &self,
        child_name: &str,
        inner: &mut WasmChildInner,
        request: &ChildCallRequest,
    ) -> Result<ChildResponse> {
        let op = resolve_typed_operation(&request.operation_id)?;
        let payload_json = encode_typed_args_for_handle(&request.args)?;
        let WasmChildInner {
            store, instance, ..
        } = inner;

        match instance.call_handle(store, &op.action, &payload_json) {
            Ok(Ok(json)) => serde_json::from_str(&json)
                .map(|payload| ChildResponse { payload })
                .map_err(|error| {
                    typed_invocation_error(
                        TypedInvocationErrorCode::InvalidChildJson,
                        format!(
                            "child '{}' operation '{}' returned invalid JSON: {}",
                            child_name, request.operation_id, error
                        ),
                    )
                }),
            Ok(Err(error)) => Err(typed_invocation_error(
                TypedInvocationErrorCode::ChildReturnedError,
                format!(
                    "child '{}' operation '{}' ({}/{}) failed: {}",
                    child_name, request.operation_id, op.interface, op.function, error
                ),
            )),
            Err(error) => Err(typed_invocation_error(
                TypedInvocationErrorCode::ChildTrap,
                format!(
                    "child '{}' operation '{}' trapped: {}",
                    child_name, request.operation_id, error
                ),
            )),
        }
    }
}

struct TypedComponentInvocationDriver;

fn typed_interface_candidates(interface_name: &str) -> Vec<String> {
    if interface_name.contains('@') {
        vec![interface_name.to_string()]
    } else {
        vec![
            interface_name.to_string(),
            format!("{}@0.1.0", interface_name),
        ]
    }
}

fn typed_function_candidates(function_name: &str) -> Vec<String> {
    let mut out = vec![function_name.to_string()];
    let swapped_hyphen = function_name.replace('_', "-");
    if !out.iter().any(|candidate| candidate == &swapped_hyphen) {
        out.push(swapped_hyphen);
    }
    let swapped_underscore = function_name.replace('-', "_");
    if !out.iter().any(|candidate| candidate == &swapped_underscore) {
        out.push(swapped_underscore);
    }
    out
}

fn lookup_typed_component_func(
    inner: &mut WasmChildInner,
    op: &ResolvedTypedOperation,
) -> Result<wasmtime::component::Func> {
    let WasmChildInner {
        store,
        component_instance,
        ..
    } = inner;

    let mut attempted = Vec::new();
    for interface in typed_interface_candidates(&op.interface) {
        let Some(interface_idx) =
            component_instance.get_export_index(store.as_context_mut(), None, &interface)
        else {
            attempted.push(format!("{}.*", interface));
            continue;
        };

        for function in typed_function_candidates(&op.function) {
            attempted.push(format!("{}.{}", interface, function));
            let Some(function_idx) = component_instance.get_export_index(
                store.as_context_mut(),
                Some(&interface_idx),
                &function,
            ) else {
                continue;
            };

            if let Some(func) = component_instance.get_func(store.as_context_mut(), function_idx) {
                return Ok(func);
            }
        }
    }

    Err(typed_invocation_error(
        TypedInvocationErrorCode::InvalidOperationId,
        format!(
            "operation '{}.{}' not found in component exports; attempted [{}]",
            op.interface,
            op.function,
            attempted.join(", ")
        ),
    ))
}

fn json_to_component_val(
    value: &serde_json::Value,
    ty: &wasmtime::component::Type,
) -> Result<wasmtime::component::Val> {
    use wasmtime::component::Type;
    use wasmtime::component::Val;

    Ok(match ty {
        Type::Bool => Val::Bool(value.as_bool().ok_or_else(|| {
            typed_invocation_error(TypedInvocationErrorCode::InvalidArgsShape, "expected bool")
        })?),
        Type::S8 => Val::S8(
            i8::try_from(value.as_i64().ok_or_else(|| {
                typed_invocation_error(TypedInvocationErrorCode::InvalidArgsShape, "expected s8")
            })?)
            .map_err(|_| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "s8 out of range",
                )
            })?,
        ),
        Type::U8 => Val::U8(
            u8::try_from(value.as_u64().ok_or_else(|| {
                typed_invocation_error(TypedInvocationErrorCode::InvalidArgsShape, "expected u8")
            })?)
            .map_err(|_| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "u8 out of range",
                )
            })?,
        ),
        Type::S16 => Val::S16(
            i16::try_from(value.as_i64().ok_or_else(|| {
                typed_invocation_error(TypedInvocationErrorCode::InvalidArgsShape, "expected s16")
            })?)
            .map_err(|_| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "s16 out of range",
                )
            })?,
        ),
        Type::U16 => Val::U16(
            u16::try_from(value.as_u64().ok_or_else(|| {
                typed_invocation_error(TypedInvocationErrorCode::InvalidArgsShape, "expected u16")
            })?)
            .map_err(|_| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "u16 out of range",
                )
            })?,
        ),
        Type::S32 => Val::S32(
            i32::try_from(value.as_i64().ok_or_else(|| {
                typed_invocation_error(TypedInvocationErrorCode::InvalidArgsShape, "expected s32")
            })?)
            .map_err(|_| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "s32 out of range",
                )
            })?,
        ),
        Type::U32 => Val::U32(
            u32::try_from(value.as_u64().ok_or_else(|| {
                typed_invocation_error(TypedInvocationErrorCode::InvalidArgsShape, "expected u32")
            })?)
            .map_err(|_| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "u32 out of range",
                )
            })?,
        ),
        Type::S64 => Val::S64(value.as_i64().ok_or_else(|| {
            typed_invocation_error(TypedInvocationErrorCode::InvalidArgsShape, "expected s64")
        })?),
        Type::U64 => Val::U64(value.as_u64().ok_or_else(|| {
            typed_invocation_error(TypedInvocationErrorCode::InvalidArgsShape, "expected u64")
        })?),
        Type::Float32 => Val::Float32(value.as_f64().ok_or_else(|| {
            typed_invocation_error(
                TypedInvocationErrorCode::InvalidArgsShape,
                "expected float32",
            )
        })? as f32),
        Type::Float64 => Val::Float64(value.as_f64().ok_or_else(|| {
            typed_invocation_error(
                TypedInvocationErrorCode::InvalidArgsShape,
                "expected float64",
            )
        })?),
        Type::Char => {
            let as_str = value.as_str().ok_or_else(|| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "expected char string",
                )
            })?;
            let mut chars = as_str.chars();
            let ch = chars.next().ok_or_else(|| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "expected non-empty char string",
                )
            })?;
            if chars.next().is_some() {
                return Err(typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "char string must contain exactly one scalar",
                ));
            }
            Val::Char(ch)
        }
        Type::String => Val::String(
            value
                .as_str()
                .ok_or_else(|| {
                    typed_invocation_error(
                        TypedInvocationErrorCode::InvalidArgsShape,
                        "expected string",
                    )
                })?
                .to_string(),
        ),
        Type::List(list_ty) => {
            let values = value.as_array().ok_or_else(|| {
                typed_invocation_error(TypedInvocationErrorCode::InvalidArgsShape, "expected list")
            })?;
            let element_ty = list_ty.ty();
            let mut lowered = Vec::with_capacity(values.len());
            for item in values {
                lowered.push(json_to_component_val(item, &element_ty)?);
            }
            Val::List(lowered)
        }
        Type::Record(record_ty) => {
            let object = value.as_object().ok_or_else(|| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "expected record object",
                )
            })?;
            let mut lowered = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for field in record_ty.fields() {
                seen.insert(field.name.to_string());
                let field_value = object.get(field.name).ok_or_else(|| {
                    typed_invocation_error(
                        TypedInvocationErrorCode::InvalidArgsShape,
                        format!("missing record field '{}'", field.name),
                    )
                })?;
                lowered.push((
                    field.name.to_string(),
                    json_to_component_val(field_value, &field.ty)?,
                ));
            }
            for key in object.keys() {
                if !seen.contains(key) {
                    return Err(typed_invocation_error(
                        TypedInvocationErrorCode::InvalidArgsShape,
                        format!("unknown record field '{}'", key),
                    ));
                }
            }
            Val::Record(lowered)
        }
        Type::Tuple(tuple_ty) => {
            let values = value.as_array().ok_or_else(|| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "expected tuple array",
                )
            })?;
            let tuple_types = tuple_ty.types().collect::<Vec<_>>();
            if values.len() != tuple_types.len() {
                return Err(typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    format!(
                        "tuple arity mismatch: expected {}, got {}",
                        tuple_types.len(),
                        values.len()
                    ),
                ));
            }
            let mut lowered = Vec::with_capacity(values.len());
            for (item, ty) in values.iter().zip(tuple_types.iter()) {
                lowered.push(json_to_component_val(item, ty)?);
            }
            Val::Tuple(lowered)
        }
        Type::Variant(variant_ty) => {
            let object = value.as_object().ok_or_else(|| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "expected variant object",
                )
            })?;
            let case = object.get("case").and_then(|v| v.as_str()).ok_or_else(|| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "variant requires string field 'case'",
                )
            })?;
            let case_meta = variant_ty
                .cases()
                .find(|item| item.name == case)
                .ok_or_else(|| {
                    typed_invocation_error(
                        TypedInvocationErrorCode::InvalidArgsShape,
                        format!("unknown variant case '{}'", case),
                    )
                })?;
            let payload = match case_meta.ty {
                Some(case_ty) => {
                    let raw_payload = object.get("value").ok_or_else(|| {
                        typed_invocation_error(
                            TypedInvocationErrorCode::InvalidArgsShape,
                            format!("variant case '{}' requires 'value'", case),
                        )
                    })?;
                    Some(Box::new(json_to_component_val(raw_payload, &case_ty)?))
                }
                None => None,
            };
            Val::Variant(case.to_string(), payload)
        }
        Type::Enum(enum_ty) => {
            let case = value.as_str().ok_or_else(|| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "expected enum string",
                )
            })?;
            if !enum_ty.names().any(|name| name == case) {
                return Err(typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    format!("unknown enum case '{}'", case),
                ));
            }
            Val::Enum(case.to_string())
        }
        Type::Option(option_ty) => {
            if value.is_null() {
                Val::Option(None)
            } else {
                Val::Option(Some(Box::new(json_to_component_val(
                    value,
                    &option_ty.ty(),
                )?)))
            }
        }
        Type::Result(result_ty) => {
            let object = value.as_object().ok_or_else(|| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "expected result object",
                )
            })?;
            if let Some(ok_value) = object.get("ok") {
                let lowered = match result_ty.ok() {
                    Some(ok_ty) => {
                        if ok_value.is_null() {
                            None
                        } else {
                            Some(Box::new(json_to_component_val(ok_value, &ok_ty)?))
                        }
                    }
                    None => None,
                };
                Val::Result(Ok(lowered))
            } else if let Some(err_value) = object.get("err") {
                let lowered = match result_ty.err() {
                    Some(err_ty) => {
                        if err_value.is_null() {
                            None
                        } else {
                            Some(Box::new(json_to_component_val(err_value, &err_ty)?))
                        }
                    }
                    None => None,
                };
                Val::Result(Err(lowered))
            } else {
                return Err(typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "result object must contain either 'ok' or 'err'",
                ));
            }
        }
        Type::Flags(flags_ty) => {
            let values = value.as_array().ok_or_else(|| {
                typed_invocation_error(
                    TypedInvocationErrorCode::InvalidArgsShape,
                    "expected flags string array",
                )
            })?;
            let mut names = Vec::new();
            for item in values {
                let name = item.as_str().ok_or_else(|| {
                    typed_invocation_error(
                        TypedInvocationErrorCode::InvalidArgsShape,
                        "flags values must be strings",
                    )
                })?;
                if !flags_ty.names().any(|allowed| allowed == name) {
                    return Err(typed_invocation_error(
                        TypedInvocationErrorCode::InvalidArgsShape,
                        format!("unknown flag '{}'", name),
                    ));
                }
                names.push(name.to_string());
            }
            Val::Flags(names)
        }
        Type::Own(_) | Type::Borrow(_) | Type::Future(_) | Type::Stream(_) | Type::ErrorContext => {
            return Err(typed_invocation_error(
                TypedInvocationErrorCode::UnsupportedType,
                format!("unsupported component type for JSON lowering: {:?}", ty),
            ));
        }
    })
}

fn component_val_to_json(value: &wasmtime::component::Val) -> Result<serde_json::Value> {
    use wasmtime::component::Val;

    Ok(match value {
        Val::Bool(v) => serde_json::json!(v),
        Val::S8(v) => serde_json::json!(v),
        Val::U8(v) => serde_json::json!(v),
        Val::S16(v) => serde_json::json!(v),
        Val::U16(v) => serde_json::json!(v),
        Val::S32(v) => serde_json::json!(v),
        Val::U32(v) => serde_json::json!(v),
        Val::S64(v) => serde_json::json!(v),
        Val::U64(v) => serde_json::json!(v),
        Val::Float32(v) => serde_json::json!(v),
        Val::Float64(v) => serde_json::json!(v),
        Val::Char(v) => serde_json::json!(v.to_string()),
        Val::String(v) => serde_json::json!(v),
        Val::List(values) => serde_json::Value::Array(
            values
                .iter()
                .map(component_val_to_json)
                .collect::<Result<Vec<_>>>()?,
        ),
        Val::Record(fields) => {
            let mut object = serde_json::Map::new();
            for (name, value) in fields {
                object.insert(name.clone(), component_val_to_json(value)?);
            }
            serde_json::Value::Object(object)
        }
        Val::Tuple(values) => serde_json::Value::Array(
            values
                .iter()
                .map(component_val_to_json)
                .collect::<Result<Vec<_>>>()?,
        ),
        Val::Variant(case, payload) => {
            let mut object = serde_json::Map::new();
            object.insert("case".to_string(), serde_json::json!(case));
            object.insert(
                "value".to_string(),
                match payload {
                    Some(value) => component_val_to_json(value)?,
                    None => serde_json::Value::Null,
                },
            );
            serde_json::Value::Object(object)
        }
        Val::Enum(case) => serde_json::json!(case),
        Val::Option(payload) => match payload {
            Some(value) => component_val_to_json(value)?,
            None => serde_json::Value::Null,
        },
        Val::Result(result) => {
            let mut object = serde_json::Map::new();
            match result {
                Ok(value) => {
                    object.insert(
                        "ok".to_string(),
                        value
                            .as_ref()
                            .map(|v| component_val_to_json(v))
                            .transpose()?
                            .unwrap_or(serde_json::Value::Null),
                    );
                }
                Err(value) => {
                    object.insert(
                        "err".to_string(),
                        value
                            .as_ref()
                            .map(|v| component_val_to_json(v))
                            .transpose()?
                            .unwrap_or(serde_json::Value::Null),
                    );
                }
            }
            serde_json::Value::Object(object)
        }
        Val::Flags(flags) => serde_json::Value::Array(
            flags
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        ),
        Val::Resource(_) | Val::Future(_) | Val::Stream(_) | Val::ErrorContext(_) => {
            return Err(typed_invocation_error(
                TypedInvocationErrorCode::UnsupportedType,
                "unsupported component result type for JSON lift",
            ));
        }
    })
}

fn lower_typed_args_for_component(
    args: &serde_json::Value,
    ty: &ComponentFunc,
) -> Result<Vec<wasmtime::component::Val>> {
    let arg_values = args.as_array().ok_or_else(|| {
        typed_invocation_error(
            TypedInvocationErrorCode::InvalidArgsShape,
            "typed call args must be a JSON array",
        )
    })?;

    let params = ty.params().collect::<Vec<_>>();
    if arg_values.len() != params.len() {
        return Err(typed_invocation_error(
            TypedInvocationErrorCode::InvalidArgsShape,
            format!(
                "operation expects {} args, got {}",
                params.len(),
                arg_values.len()
            ),
        ));
    }

    let mut lowered = Vec::with_capacity(params.len());
    for ((param_name, param_ty), arg) in params.iter().zip(arg_values.iter()) {
        lowered.push(json_to_component_val(arg, param_ty).map_err(|error| {
            typed_invocation_error(
                TypedInvocationErrorCode::InvalidArgsShape,
                format!("arg '{}' failed to lower: {}", param_name, error),
            )
        })?);
    }
    Ok(lowered)
}

fn lift_component_results_to_json(
    values: &[wasmtime::component::Val],
    ty: &ComponentFunc,
) -> Result<serde_json::Value> {
    let result_types = ty.results().collect::<Vec<_>>();
    if values.len() != result_types.len() {
        return Err(typed_invocation_error(
            TypedInvocationErrorCode::InvalidChildJson,
            format!(
                "result arity mismatch: expected {}, got {}",
                result_types.len(),
                values.len()
            ),
        ));
    }

    let mut lifted = Vec::with_capacity(values.len());
    for value in values {
        lifted.push(component_val_to_json(value)?);
    }

    Ok(match lifted.len() {
        0 => serde_json::Value::Null,
        1 => lifted.into_iter().next().unwrap_or(serde_json::Value::Null),
        _ => serde_json::Value::Array(lifted),
    })
}

impl InvocationDriver for TypedComponentInvocationDriver {
    fn call(
        &self,
        child_name: &str,
        inner: &mut WasmChildInner,
        request: &ChildCallRequest,
    ) -> Result<ChildResponse> {
        let op = resolve_typed_operation(&request.operation_id)?;
        let func = lookup_typed_component_func(inner, &op)?;

        let WasmChildInner { store, .. } = inner;
        let func_ty = func.ty(store.as_context());
        let lowered_args = lower_typed_args_for_component(&request.args, &func_ty)?;

        let mut results = vec![wasmtime::component::Val::Bool(false); func_ty.results().len()];
        func.call(store.as_context_mut(), &lowered_args, &mut results)
            .map_err(|error| {
                typed_invocation_error(
                    TypedInvocationErrorCode::ChildTrap,
                    format!(
                        "child '{}' operation '{}' trapped: {}",
                        child_name, request.operation_id, error
                    ),
                )
            })?;
        func.post_return(store.as_context_mut()).map_err(|error| {
            typed_invocation_error(
                TypedInvocationErrorCode::ChildTrap,
                format!(
                    "child '{}' operation '{}' post-return failed: {}",
                    child_name, request.operation_id, error
                ),
            )
        })?;

        let payload = lift_component_results_to_json(&results, &func_ty)?;
        Ok(ChildResponse { payload })
    }
}

impl Child for WasmChild {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmChildInner {
            store, instance, ..
        } = &mut *inner;
        match instance.call_on_load(store)? {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("WASM on_load failed: {}", e)),
        }
    }

    fn on_unload(&mut self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmChildInner {
            store, instance, ..
        } = &mut *inner;
        let _ = instance.call_on_unload(store);
    }

    fn health(&self) -> ChildHealth {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmChildInner {
            store, instance, ..
        } = &mut *inner;
        match instance.call_health(store) {
            Ok(h) => {
                let reason = h.reason.unwrap_or_default();
                match h.status {
                    bindings::patina::child::runtime_types::HealthStatus::Healthy => {
                        ChildHealth::Healthy
                    }
                    bindings::patina::child::runtime_types::HealthStatus::Degraded => {
                        ChildHealth::Degraded(if reason.is_empty() {
                            "degraded".into()
                        } else {
                            reason
                        })
                    }
                    bindings::patina::child::runtime_types::HealthStatus::Unhealthy => {
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
        let WasmChildInner {
            store, instance, ..
        } = &mut *inner;
        let payload_json = serde_json::to_string(&request.payload)?;
        match instance.call_handle(store, &request.action, &payload_json)? {
            Ok(json) => Ok(ChildResponse {
                payload: serde_json::from_str(&json)?,
            }),
            Err(e) => Err(anyhow::anyhow!("{}", e)),
        }
    }

    fn call(&self, request: &ChildCallRequest) -> Result<ChildResponse> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        self.invocation_driver.call(&self.name, &mut inner, request)
    }

    fn drain(&mut self, limit: u32) -> Result<Vec<PendingEvent>> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let WasmChildInner {
            store, instance, ..
        } = &mut *inner;
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
        let WasmChildInner {
            store, instance, ..
        } = &mut *inner;
        match instance.call_tick(store) {
            Ok(intents) => intents
                .into_iter()
                .filter_map(|intent| {
                    Some(TaskIntent {
                        kind: match intent.kind {
                            bindings::patina::child::runtime_types::TaskIntentKind::FetchSource => {
                                crate::mother::TaskIntentKind::FetchSource
                            }
                            bindings::patina::child::runtime_types::TaskIntentKind::RunQuery => {
                                crate::mother::TaskIntentKind::RunQuery
                            }
                            bindings::patina::child::runtime_types::TaskIntentKind::EmitFacts => {
                                crate::mother::TaskIntentKind::EmitFacts
                            }
                            bindings::patina::child::runtime_types::TaskIntentKind::MaterializeIndex => {
                                crate::mother::TaskIntentKind::MaterializeIndex
                            }
                            bindings::patina::child::runtime_types::TaskIntentKind::VerifyBelief => {
                                crate::mother::TaskIntentKind::VerifyBelief
                            }
                            bindings::patina::child::runtime_types::TaskIntentKind::SyncGraph => {
                                crate::mother::TaskIntentKind::SyncGraph
                            }
                            bindings::patina::child::runtime_types::TaskIntentKind::RefreshCredential => {
                                crate::mother::TaskIntentKind::RefreshCredential
                            }
                            bindings::patina::child::runtime_types::TaskIntentKind::NativeJob => {
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

#[cfg(test)]
mod tests {
    use super::bindings::HostState;
    use super::{
        encode_typed_args_for_handle, resolve_typed_operation, GrantedCapabilities,
        TypedInvocationErrorCode,
    };

    fn host_state_with_grants(grants: GrantedCapabilities) -> HostState {
        HostState {
            plugin_name: "capability-test".to_string(),
            wasi: wasmtime_wasi::WasiCtxBuilder::new().build(),
            wasi_table: wasmtime::component::ResourceTable::new(),
            http: wasmtime_wasi_http::WasiHttpCtx::new(),
            project_root: None,
            grants,
            query_fn: None,
            http_client: reqwest::blocking::Client::new(),
            runtime: crate::mother::MotherRuntimeStore::default(),
            active_bindings: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn sql_open_denied_without_sql_toy_grant() {
        let mut state = host_state_with_grants(GrantedCapabilities::default());
        let err = <HostState as super::bindings::wasi::sql::readwrite::Host>::open(
            &mut state,
            "default".to_string(),
        )
        .unwrap_err();
        assert!(err.contains("sql toy not granted"), "got: {}", err);
    }

    #[test]
    fn keyvalue_open_denied_without_keyvalue_toy_grant() {
        let mut state = host_state_with_grants(GrantedCapabilities::default());
        let err = <HostState as super::bindings::wasi::keyvalue::store::Host>::open(
            &mut state,
            "default".to_string(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            super::bindings::wasi::keyvalue::store::Error::AccessDenied
        ));
    }

    #[test]
    fn git_calls_denied_without_git_toy_grant() {
        let mut state = host_state_with_grants(GrantedCapabilities::default());
        let err = <HostState as super::bindings::patina::git::git::Host>::diff_stat(&mut state)
            .unwrap_err();
        assert!(err.contains("git toy not granted"), "got: {}", err);
    }

    #[test]
    fn resolve_typed_operation_converts_function_name_to_action() {
        let resolved = resolve_typed_operation("patina:watch/control.scan_now")
            .expect("operation id should resolve");
        assert_eq!(resolved.interface, "patina:watch/control");
        assert_eq!(resolved.function, "scan_now");
        assert_eq!(resolved.action, "scan-now");
    }

    #[test]
    fn resolve_typed_operation_rejects_invalid_id_shape() {
        let err = resolve_typed_operation("patina:watch/control")
            .expect_err("missing function should fail closed");
        let message = err.to_string();
        assert!(
            message.contains(TypedInvocationErrorCode::InvalidOperationId.as_str()),
            "got: {}",
            message
        );
    }

    #[test]
    fn encode_typed_args_for_handle_requires_array() {
        let err = encode_typed_args_for_handle(&serde_json::json!({"foo": "bar"}))
            .expect_err("object args should be rejected");
        let message = err.to_string();
        assert!(
            message.contains(TypedInvocationErrorCode::InvalidArgsShape.as_str()),
            "got: {}",
            message
        );
    }

    #[test]
    fn encode_typed_args_for_handle_supports_zero_one_many_arity() {
        let zero = encode_typed_args_for_handle(&serde_json::json!([])).expect("zero-arity");
        assert_eq!(zero, "{}");

        let one = encode_typed_args_for_handle(&serde_json::json!([{"watch_path": "/input"}]))
            .expect("one-arity");
        assert_eq!(one, "{\"watch_path\":\"/input\"}");

        let many =
            encode_typed_args_for_handle(&serde_json::json!([{"watch_path": "/input"}, true]))
                .expect("many-arity");
        assert_eq!(many, "[{\"watch_path\":\"/input\"},true]");
    }
}
