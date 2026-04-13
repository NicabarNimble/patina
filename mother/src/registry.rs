//! Child registry — loads, iterates, and provides access to children.
//!
//! Children can be registered during startup warmup while the daemon
//! is already accepting control-plane connections. Individual children
//! use per-child RwLock for concurrent handle() vs exclusive tick().

use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use crate::{
    Child, ChildCallRequest, ChildHealth, ChildRequest, ChildResponse, MotherHost,
    MotherRuntimeStore, RunStatus,
};

/// Registry of Mother's children.
pub struct ChildRegistry {
    children: RwLock<Vec<Arc<ChildEntry>>>,
}

struct ChildEntry {
    child: Arc<RwLock<Box<dyn Child>>>,
    wasm_path: PathBuf,
    manifest_path: PathBuf,
    reload_lock: Arc<Mutex<()>>,
}

#[derive(Debug, Clone)]
pub struct ChildActivationResult {
    pub name: String,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildIngressMode {
    Handle,
    Hybrid,
    WitOnly,
}

#[derive(Debug, Clone)]
struct ChildIngressPolicy {
    mode: ChildIngressMode,
    allowed_operations: Vec<String>,
}

impl Default for ChildIngressPolicy {
    fn default() -> Self {
        Self {
            mode: ChildIngressMode::Handle,
            allowed_operations: vec![],
        }
    }
}

impl Default for ChildRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ChildRegistry {
    pub fn new() -> Self {
        Self {
            children: RwLock::new(vec![]),
        }
    }

    fn children_snapshot(&self) -> Vec<Arc<ChildEntry>> {
        self.children
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn read_project_uid(project_root: &std::path::Path) -> Option<String> {
        let uid = std::fs::read_to_string(project_root.join(".patina/uid")).ok()?;
        let uid = uid.trim();
        if uid.len() == 8
            && uid
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            Some(uid.to_string())
        } else {
            None
        }
    }

    fn resolve_events_db_path() -> PathBuf {
        let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if let Some(uid) = Self::read_project_uid(&project_root) {
            return crate::secrets_paths::patina_home()
                .join("mother")
                .join("projects")
                .join(uid)
                .join("events.db");
        }
        project_root
            .join(".patina")
            .join("local")
            .join("data")
            .join("events.db")
    }

    fn open_registry_events_connection() -> Result<Connection> {
        let path = Self::resolve_events_db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        crate::eventlog_schema::prepare_events_db(&conn)?;
        Ok(conn)
    }

    fn emit_mother_metric_with_labels(
        child_name: &str,
        name: &str,
        kind: &str,
        value: f64,
        labels: Vec<(&str, String)>,
        scope: &str,
    ) -> Result<()> {
        let conn = Self::open_registry_events_connection()?;
        let timestamp = Utc::now().to_rfc3339();

        let mut label_pairs = vec![serde_json::json!(["child", child_name])];
        for (k, v) in labels {
            label_pairs.push(serde_json::json!([k, v]));
        }

        let data = serde_json::json!({
            "name": name,
            "kind": kind,
            "value": value,
            "labels": label_pairs,
            "source": "mother",
            "scope": scope,
        })
        .to_string();

        conn.execute(
            "INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data, provenance)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "measure.metric",
                timestamp,
                format!("mother:{}:{}", child_name, name),
                Option::<String>::None,
                data,
                "local"
            ],
        )?;

        Ok(())
    }

    fn emit_mother_metric(
        child_name: &str,
        action: &str,
        name: &str,
        kind: &str,
        value: f64,
    ) -> Result<()> {
        Self::emit_mother_metric_with_labels(
            child_name,
            name,
            kind,
            value,
            vec![("action", action.to_string())],
            "child-handle-boundary",
        )
    }

    fn split_operation_id(operation_id: &str) -> (String, String) {
        if let Some((interface, function)) = operation_id.rsplit_once('.') {
            (interface.to_string(), function.to_string())
        } else {
            (operation_id.to_string(), "unknown".to_string())
        }
    }

    fn observe_handle(child_name: &str, action: &str, started_at: Instant, is_success: bool) {
        let latency_ms = started_at.elapsed().as_secs_f64() * 1000.0;
        let _ = Self::emit_mother_metric(
            child_name,
            action,
            "mother_handle_latency_ms",
            "gauge",
            latency_ms,
        )
        .map_err(|error| {
            tracing::warn!(
                child = child_name,
                %error,
                "failed to emit mother_handle_latency_ms"
            )
        });

        let _ = Self::emit_mother_metric(
            child_name,
            action,
            "mother_handle_throughput",
            "counter",
            1.0,
        )
        .map_err(|error| {
            tracing::warn!(
                child = child_name,
                %error,
                "failed to emit mother_handle_throughput"
            )
        });

        let metric_name = if is_success {
            "mother_handle_success"
        } else {
            "mother_handle_error"
        };
        let _ = Self::emit_mother_metric(child_name, action, metric_name, "counter", 1.0).map_err(
            |error| {
                tracing::warn!(child = child_name, metric = metric_name, %error, "failed to emit mother metric")
            },
        );
    }

    fn observe_call(child_name: &str, operation_id: &str, started_at: Instant, is_success: bool) {
        let (interface, function) = Self::split_operation_id(operation_id);
        let outcome = if is_success { "success" } else { "error" };
        let base_labels = vec![
            ("interface", interface.clone()),
            ("function", function.clone()),
            ("outcome", outcome.to_string()),
        ];

        let latency_ms = started_at.elapsed().as_secs_f64() * 1000.0;
        let _ = Self::emit_mother_metric_with_labels(
            child_name,
            "mother_wit_call_latency_ms",
            "gauge",
            latency_ms,
            base_labels.clone(),
            "child-wit-call-boundary",
        )
        .map_err(|error| {
            tracing::warn!(
                child = child_name,
                operation = operation_id,
                %error,
                "failed to emit mother_wit_call_latency_ms"
            )
        });

        let _ = Self::emit_mother_metric_with_labels(
            child_name,
            "mother_wit_call_throughput",
            "counter",
            1.0,
            base_labels.clone(),
            "child-wit-call-boundary",
        )
        .map_err(|error| {
            tracing::warn!(
                child = child_name,
                operation = operation_id,
                %error,
                "failed to emit mother_wit_call_throughput"
            )
        });

        let metric_name = if is_success {
            "mother_wit_call_success"
        } else {
            "mother_wit_call_error"
        };
        let _ = Self::emit_mother_metric_with_labels(
            child_name,
            metric_name,
            "counter",
            1.0,
            base_labels,
            "child-wit-call-boundary",
        )
        .map_err(|error| {
            tracing::warn!(
                child = child_name,
                metric = metric_name,
                operation = operation_id,
                %error,
                "failed to emit mother WIT call metric"
            )
        });
    }

    fn parse_ingress_policy(manifest_path: &std::path::Path) -> Result<ChildIngressPolicy> {
        if manifest_path.as_os_str().is_empty() || !manifest_path.exists() {
            return Ok(ChildIngressPolicy::default());
        }

        let content = std::fs::read_to_string(manifest_path)?;
        let table: toml::Table = content.parse()?;
        let child = table
            .get("child")
            .and_then(|v| v.as_table())
            .ok_or_else(|| anyhow::anyhow!("missing [child] section"))?;

        let mode = child
            .get("ingress")
            .and_then(|v| v.as_table())
            .and_then(|ingress| ingress.get("mode"))
            .and_then(|v| v.as_str())
            .unwrap_or("handle");

        let mode = match mode {
            "handle" => ChildIngressMode::Handle,
            "hybrid" => ChildIngressMode::Hybrid,
            "wit-only" => ChildIngressMode::WitOnly,
            other => anyhow::bail!("unknown child ingress mode: '{}'", other),
        };

        let allowed_operations = child
            .get("contract")
            .and_then(|v| v.as_table())
            .and_then(|contract| contract.get("allow"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(ChildIngressPolicy {
            mode,
            allowed_operations,
        })
    }

    fn is_lifecycle_action(action: &str) -> bool {
        matches!(
            action,
            "health" | "tick" | "drain" | "on-load" | "on-unload"
        )
    }

    fn enforce_handle_policy(
        child_name: &str,
        manifest_path: &std::path::Path,
        request: &ChildRequest,
    ) -> Result<()> {
        let policy = Self::parse_ingress_policy(manifest_path)?;
        if policy.mode == ChildIngressMode::WitOnly && !Self::is_lifecycle_action(&request.action) {
            anyhow::bail!(
                "child '{}' is wit-only: action '{}' is denied; use `patina child call {} <operation-id> '<json-args>'`",
                child_name,
                request.action,
                child_name,
            );
        }
        Ok(())
    }

    fn enforce_call_policy(
        child_name: &str,
        manifest_path: &std::path::Path,
        request: &ChildCallRequest,
    ) -> Result<()> {
        let policy = Self::parse_ingress_policy(manifest_path)?;

        match policy.mode {
            ChildIngressMode::Handle => anyhow::bail!(
                "child '{}' ingress mode is handle; typed call '{}' denied",
                child_name,
                request.operation_id
            ),
            ChildIngressMode::Hybrid | ChildIngressMode::WitOnly => {
                if policy.mode == ChildIngressMode::WitOnly && policy.allowed_operations.is_empty()
                {
                    anyhow::bail!(
                        "child '{}' is wit-only but has empty [child.contract].allow; deny by default",
                        child_name,
                    );
                }
                if !policy.allowed_operations.is_empty()
                    && !policy
                        .allowed_operations
                        .iter()
                        .any(|op| op == &request.operation_id)
                {
                    anyhow::bail!(
                        "child '{}' operation '{}' is not allowlisted in [child.contract].allow",
                        child_name,
                        request.operation_id
                    );
                }
            }
        }

        Ok(())
    }

    fn invoke_handle_observed(
        child_name: &str,
        child: &dyn Child,
        request: &ChildRequest,
    ) -> Result<ChildResponse> {
        let started_at = Instant::now();
        let response = child.handle(request);
        Self::observe_handle(child_name, &request.action, started_at, response.is_ok());
        response
    }

    fn invoke_call_observed(
        child_name: &str,
        child: &dyn Child,
        request: &ChildCallRequest,
    ) -> Result<ChildResponse> {
        let started_at = Instant::now();
        let response = child.call(request);
        Self::observe_call(
            child_name,
            &request.operation_id,
            started_at,
            response.is_ok(),
        );
        response
    }

    pub fn register_knowledge(&self, child: Box<dyn Child>) -> Result<()> {
        self.register_knowledge_with_paths(child, PathBuf::new(), PathBuf::new())
    }

    pub fn register_knowledge_with_paths(
        &self,
        child: Box<dyn Child>,
        wasm_path: PathBuf,
        manifest_path: PathBuf,
    ) -> Result<()> {
        let name = child.name().to_string();
        if self.child_name_exists(&name) {
            anyhow::bail!("duplicate child name: {}", name);
        }
        self.children
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(Arc::new(ChildEntry {
                child: Arc::new(RwLock::new(child)),
                wasm_path,
                manifest_path,
                reload_lock: Arc::new(Mutex::new(())),
            }));
        Ok(())
    }

    pub fn child_paths(&self, child_name: &str) -> Option<(PathBuf, PathBuf)> {
        let children = self.children_snapshot();
        children.iter().find_map(|entry| {
            let guard = entry.child.read().unwrap_or_else(|e| e.into_inner());
            if guard.name() == child_name {
                Some((entry.wasm_path.clone(), entry.manifest_path.clone()))
            } else {
                None
            }
        })
    }

    pub fn child_reload_lock(&self, child_name: &str) -> Option<Arc<Mutex<()>>> {
        let children = self.children_snapshot();
        children.iter().find_map(|entry| {
            let guard = entry.child.read().unwrap_or_else(|e| e.into_inner());
            if guard.name() == child_name {
                Some(Arc::clone(&entry.reload_lock))
            } else {
                None
            }
        })
    }

    pub fn swap_knowledge_child(
        &self,
        child_name: &str,
        replacement: Box<dyn Child>,
    ) -> Result<Box<dyn Child>> {
        let children = self.children_snapshot();
        for entry in children {
            let matches_name = {
                let guard = entry.child.read().unwrap_or_else(|e| e.into_inner());
                guard.name() == child_name
            };
            if matches_name {
                let mut slot = entry.child.write().unwrap_or_else(|e| e.into_inner());
                let previous = std::mem::replace(&mut *slot, replacement);
                return Ok(previous);
            }
        }

        Err(anyhow::anyhow!("unknown child: {}", child_name))
    }

    fn child_name_exists(&self, name: &str) -> bool {
        self.children
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|entry| entry.child.read().unwrap_or_else(|e| e.into_inner()).name() == name)
    }

    /// Load all children — calls on_load() for each in order.
    /// Fails fast if any child fails to load.
    pub fn load_all(&self, host: &dyn MotherHost) -> Result<()> {
        for entry in self.children_snapshot() {
            let mut child = entry.child.write().unwrap_or_else(|e| e.into_inner());
            let name = child.name().to_string();
            tracing::info!(event = "startup.child.onload.begin", child = %name, "mother child on_load begin");
            let started = Instant::now();
            host.log(&name, "loading");
            if let Err(error) = child.on_load(host) {
                tracing::warn!(
                    event = "startup.child.onload.failure",
                    child = %name,
                    duration_ms = started.elapsed().as_millis() as u64,
                    %error,
                    "mother child on_load failed"
                );
                return Err(error);
            }
            host.log(&name, "loaded");
            tracing::info!(
                event = "startup.child.onload.success",
                child = %name,
                duration_ms = started.elapsed().as_millis() as u64,
                "mother child on_load success"
            );
        }
        Ok(())
    }

    pub fn activate_all(&self, host: &dyn MotherHost) -> Vec<ChildActivationResult> {
        let mut results = Vec::new();
        for entry in self.children_snapshot() {
            let mut child = entry.child.write().unwrap_or_else(|e| e.into_inner());
            let name = child.name().to_string();
            tracing::info!(event = "startup.child.onload.begin", child = %name, "mother child on_load begin");
            let started = Instant::now();
            host.log(&name, "loading");
            let result = child.on_load(host);
            let duration_ms = started.elapsed().as_millis() as u64;
            match result {
                Ok(()) => {
                    host.log(&name, "loaded");
                    tracing::info!(
                        event = "startup.child.onload.success",
                        child = %name,
                        duration_ms,
                        "mother child on_load success"
                    );
                    results.push(ChildActivationResult {
                        name,
                        duration_ms,
                        error: None,
                    });
                }
                Err(error) => {
                    tracing::warn!(
                        event = "startup.child.onload.failure",
                        child = %name,
                        duration_ms,
                        %error,
                        "mother child on_load failed"
                    );
                    results.push(ChildActivationResult {
                        name,
                        duration_ms,
                        error: Some(error.to_string()),
                    });
                }
            }
        }
        results
    }

    pub fn run_knowledge_cycles(
        &self,
        runtime: &MotherRuntimeStore,
        lease_owner: &str,
    ) -> Result<()> {
        for entry in self.children_snapshot() {
            let mut child = entry.child.write().unwrap_or_else(|e| e.into_inner());
            let plugin_name = child.name().to_string();
            let run_id = runtime.record_run_start(&plugin_name)?;
            let mut metrics = serde_json::Map::new();
            let result = (|| -> Result<()> {
                let drained = child.drain(64)?;
                metrics.insert(
                    "drained_events".into(),
                    serde_json::Value::from(drained.len() as u64),
                );

                let tick_intents = child.tick();
                metrics.insert(
                    "tick_intents".into(),
                    serde_json::Value::from(tick_intents.len() as u64),
                );
                for intent in tick_intents {
                    runtime.enqueue_task(&plugin_name, &intent)?;
                }

                let mut executed = 0_u64;
                while let Some(task) = runtime.lease_next_task(&plugin_name, lease_owner)? {
                    runtime.mark_task_running(&task.id)?;
                    let request = ChildRequest {
                        action: task.kind.as_str().to_string(),
                        payload: serde_json::from_str(&task.payload_json)
                            .unwrap_or(serde_json::Value::Null),
                    };
                    if let Err(error) =
                        Self::enforce_handle_policy(&plugin_name, &entry.manifest_path, &request)
                    {
                        runtime.mark_task_failed(&task.id, task.attempts, &error.to_string())?;
                        executed += 1;
                        continue;
                    }

                    match Self::invoke_handle_observed(&plugin_name, child.as_ref(), &request) {
                        Ok(_) => runtime.mark_task_succeeded(&task.id)?,
                        Err(error) => {
                            runtime.mark_task_failed(&task.id, task.attempts, &error.to_string())?
                        }
                    }
                    executed += 1;
                }
                metrics.insert("executed_tasks".into(), serde_json::Value::from(executed));
                Ok(())
            })();

            match result {
                Ok(()) => runtime.finish_run(
                    run_id,
                    RunStatus::Succeeded,
                    Some(&serde_json::to_string(&metrics)?),
                    None,
                )?,
                Err(error) => runtime.finish_run(
                    run_id,
                    RunStatus::Failed,
                    Some(&serde_json::to_string(&metrics)?),
                    Some(&error.to_string()),
                )?,
            }
        }
        Ok(())
    }

    /// Health check all children.
    pub fn health_all(&self) -> Vec<(String, ChildHealth)> {
        let mut statuses = Vec::new();
        for child in self.children_snapshot() {
            let child = match child.child.read() {
                Ok(child) => child,
                Err(_) => continue,
            };
            statuses.push((child.name().to_string(), child.health()));
        }
        statuses
    }

    /// Route a request to a child by name.
    pub fn handle(&self, child_name: &str, request: &ChildRequest) -> Result<ChildResponse> {
        let children = self.children_snapshot();
        if let Some(entry) = children.iter().find(|entry| {
            entry.child.read().unwrap_or_else(|e| e.into_inner()).name() == child_name
        }) {
            Self::enforce_handle_policy(child_name, &entry.manifest_path, request)?;
            let child = entry.child.read().unwrap_or_else(|e| e.into_inner());
            return Self::invoke_handle_observed(child_name, child.as_ref(), request);
        }

        Err(anyhow::anyhow!("unknown child: {}", child_name))
    }

    /// Route a typed business call to a child by name.
    pub fn call(&self, child_name: &str, request: &ChildCallRequest) -> Result<ChildResponse> {
        let children = self.children_snapshot();
        if let Some(entry) = children.iter().find(|entry| {
            entry.child.read().unwrap_or_else(|e| e.into_inner()).name() == child_name
        }) {
            Self::enforce_call_policy(child_name, &entry.manifest_path, request)?;
            let child = entry.child.read().unwrap_or_else(|e| e.into_inner());
            return Self::invoke_call_observed(child_name, child.as_ref(), request);
        }

        Err(anyhow::anyhow!("unknown child: {}", child_name))
    }

    pub fn health(&self, child_name: &str) -> Result<ChildHealth> {
        let children = self.children_snapshot();
        if let Some(child) = children.iter().find(|entry| {
            entry.child.read().unwrap_or_else(|e| e.into_inner()).name() == child_name
        }) {
            let child = child.child.read().unwrap_or_else(|e| e.into_inner());
            return Ok(child.health());
        }

        Err(anyhow::anyhow!("unknown child: {}", child_name))
    }

    pub fn knowledge_len(&self) -> usize {
        self.children
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::io::Write;

    fn write_manifest(mode: &str, allow: &[&str]) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().expect("create manifest temp file");
        let allow_lines = if allow.is_empty() {
            String::new()
        } else {
            format!(
                "allow = [{}]",
                allow
                    .iter()
                    .map(|op| format!("\"{}\"", op))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        let content = format!(
            "[child]\nname = \"test-child\"\nkind = \"child\"\n\n[child.ingress]\nmode = \"{}\"\n\n[child.contract]\n{}\n\n[needs]\ntoys = [\"logging\"]\n",
            mode,
            allow_lines
        );

        file.write_all(content.as_bytes())
            .expect("write manifest content");
        file.flush().expect("flush manifest content");
        file
    }

    /// Minimal knowledge child for testing registry logic.
    struct StubChild {
        child_name: String,
    }

    impl StubChild {
        fn boxed(name: &str) -> Box<dyn Child> {
            Box::new(Self {
                child_name: name.to_string(),
            })
        }
    }

    impl Child for StubChild {
        fn name(&self) -> &str {
            &self.child_name
        }
        fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
            Ok(())
        }
        fn health(&self) -> ChildHealth {
            ChildHealth::Healthy
        }
        fn handle(&self, _request: &ChildRequest) -> Result<ChildResponse> {
            Ok(ChildResponse {
                payload: serde_json::json!({"stub": true}),
            })
        }
    }

    struct TypedStubChild {
        child_name: String,
    }

    impl TypedStubChild {
        fn boxed(name: &str) -> Box<dyn Child> {
            Box::new(Self {
                child_name: name.to_string(),
            })
        }
    }

    impl Child for TypedStubChild {
        fn name(&self) -> &str {
            &self.child_name
        }

        fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
            Ok(())
        }

        fn health(&self) -> ChildHealth {
            ChildHealth::Healthy
        }

        fn handle(&self, _request: &ChildRequest) -> Result<ChildResponse> {
            Ok(ChildResponse {
                payload: serde_json::json!({"stub": true}),
            })
        }

        fn call(&self, request: &ChildCallRequest) -> Result<ChildResponse> {
            Ok(ChildResponse {
                payload: serde_json::json!({
                    "operation": request.operation_id,
                    "args": request.args,
                    "typed": true,
                }),
            })
        }
    }

    struct ErrorStubChild {
        child_name: String,
    }

    impl ErrorStubChild {
        fn boxed(name: &str) -> Box<dyn Child> {
            Box::new(Self {
                child_name: name.to_string(),
            })
        }
    }

    impl Child for ErrorStubChild {
        fn name(&self) -> &str {
            &self.child_name
        }
        fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
            Ok(())
        }
        fn health(&self) -> ChildHealth {
            ChildHealth::Healthy
        }
        fn handle(&self, _request: &ChildRequest) -> Result<ChildResponse> {
            Err(anyhow::anyhow!("intentional failure"))
        }
    }

    fn metric_names_for_action(action: &str) -> Vec<String> {
        let conn = ChildRegistry::open_registry_events_connection()
            .expect("open events db for metric assertion");
        let mut stmt = conn
            .prepare("SELECT data FROM eventlog WHERE event_type = 'measure.metric' ORDER BY seq DESC LIMIT 256")
            .expect("prepare events query");
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query events")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("collect events");

        let mut names = Vec::new();
        for raw in rows {
            let Ok(value) = serde_json::from_str::<Value>(&raw) else {
                continue;
            };
            let labels = value
                .get("labels")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let mut action_match = false;
            for label in labels {
                let pair = label.as_array().cloned().unwrap_or_default();
                if pair.len() == 2
                    && pair.first().and_then(|v| v.as_str()) == Some("action")
                    && pair.get(1).and_then(|v| v.as_str()) == Some(action)
                {
                    action_match = true;
                    break;
                }
            }
            if !action_match {
                continue;
            }
            if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
                names.push(name.to_string());
            }
        }
        names
    }

    fn metric_names_for_wit_call(interface: &str, function: &str, outcome: &str) -> Vec<String> {
        let conn = ChildRegistry::open_registry_events_connection()
            .expect("open events db for metric assertion");
        let mut stmt = conn
            .prepare("SELECT data FROM eventlog WHERE event_type = 'measure.metric' ORDER BY seq DESC LIMIT 256")
            .expect("prepare events query");
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query events")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("collect events");

        let mut names = Vec::new();
        for raw in rows {
            let Ok(value) = serde_json::from_str::<Value>(&raw) else {
                continue;
            };
            let labels = value
                .get("labels")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            let mut interface_match = false;
            let mut function_match = false;
            let mut outcome_match = false;
            for label in labels {
                let pair = label.as_array().cloned().unwrap_or_default();
                if pair.len() != 2 {
                    continue;
                }
                let key = pair.first().and_then(|v| v.as_str()).unwrap_or_default();
                let value = pair.get(1).and_then(|v| v.as_str()).unwrap_or_default();
                if key == "interface" && value == interface {
                    interface_match = true;
                }
                if key == "function" && value == function {
                    function_match = true;
                }
                if key == "outcome" && value == outcome {
                    outcome_match = true;
                }
            }

            if interface_match && function_match && outcome_match {
                if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
                    names.push(name.to_string());
                }
            }
        }
        names
    }

    #[test]
    fn register_unique_names() {
        let registry = ChildRegistry::new();
        assert!(registry
            .register_knowledge(StubChild::boxed("alpha"))
            .is_ok());
        assert!(registry
            .register_knowledge(StubChild::boxed("beta"))
            .is_ok());
        assert_eq!(registry.knowledge_len(), 2);
    }

    #[test]
    fn register_duplicate_name_rejected() {
        let registry = ChildRegistry::new();
        assert!(registry
            .register_knowledge(StubChild::boxed("alpha"))
            .is_ok());
        let err = registry
            .register_knowledge(StubChild::boxed("alpha"))
            .unwrap_err();
        assert!(
            err.to_string().contains("duplicate child name: alpha"),
            "got: {}",
            err
        );
        assert_eq!(registry.knowledge_len(), 1);
    }

    #[test]
    fn observed_handle_emits_success_metrics() {
        let registry = ChildRegistry::new();
        registry
            .register_knowledge(StubChild::boxed("observed-success"))
            .unwrap();

        let action = format!("action-{}", uuid::Uuid::new_v4());
        let response = registry
            .handle(
                "observed-success",
                &ChildRequest {
                    action: action.clone(),
                    payload: serde_json::json!({}),
                },
            )
            .expect("registry handle should succeed");
        assert_eq!(response.payload.get("stub"), Some(&serde_json::json!(true)));

        let names = metric_names_for_action(&action);
        assert!(names.iter().any(|n| n == "mother_handle_latency_ms"));
        assert!(names.iter().any(|n| n == "mother_handle_throughput"));
        assert!(names.iter().any(|n| n == "mother_handle_success"));
    }

    #[test]
    fn observed_handle_emits_error_metrics() {
        let registry = ChildRegistry::new();
        registry
            .register_knowledge(ErrorStubChild::boxed("observed-error"))
            .unwrap();

        let action = format!("action-{}", uuid::Uuid::new_v4());
        let result = registry.handle(
            "observed-error",
            &ChildRequest {
                action: action.clone(),
                payload: serde_json::json!({}),
            },
        );
        assert!(result.is_err());

        let names = metric_names_for_action(&action);
        assert!(names.iter().any(|n| n == "mother_handle_latency_ms"));
        assert!(names.iter().any(|n| n == "mother_handle_throughput"));
        assert!(names.iter().any(|n| n == "mother_handle_error"));
    }

    #[test]
    fn observed_typed_call_emits_success_metrics() {
        let registry = ChildRegistry::new();
        let manifest = write_manifest("hybrid", &["patina:watch/control.status"]);
        registry
            .register_knowledge_with_paths(
                TypedStubChild::boxed("typed-success"),
                PathBuf::new(),
                manifest.path().to_path_buf(),
            )
            .unwrap();

        let response = registry
            .call(
                "typed-success",
                &ChildCallRequest {
                    operation_id: "patina:watch/control.status".to_string(),
                    args: serde_json::json!([]),
                },
            )
            .expect("typed call should succeed");
        assert_eq!(
            response.payload.get("typed"),
            Some(&serde_json::json!(true))
        );

        let names = metric_names_for_wit_call("patina:watch/control", "status", "success");
        assert!(names.iter().any(|n| n == "mother_wit_call_latency_ms"));
        assert!(names.iter().any(|n| n == "mother_wit_call_throughput"));
        assert!(names.iter().any(|n| n == "mother_wit_call_success"));
    }

    #[test]
    fn observed_typed_call_emits_error_metrics() {
        let registry = ChildRegistry::new();
        let manifest = write_manifest("hybrid", &["patina:watch/control.status"]);
        registry
            .register_knowledge_with_paths(
                StubChild::boxed("typed-error"),
                PathBuf::new(),
                manifest.path().to_path_buf(),
            )
            .unwrap();

        let result = registry.call(
            "typed-error",
            &ChildCallRequest {
                operation_id: "patina:watch/control.status".to_string(),
                args: serde_json::json!([]),
            },
        );
        assert!(result.is_err());

        let names = metric_names_for_wit_call("patina:watch/control", "status", "error");
        assert!(names.iter().any(|n| n == "mother_wit_call_latency_ms"));
        assert!(names.iter().any(|n| n == "mother_wit_call_throughput"));
        assert!(names.iter().any(|n| n == "mother_wit_call_error"));
    }

    #[test]
    fn wit_only_denies_business_handle_calls() {
        let registry = ChildRegistry::new();
        let manifest = write_manifest("wit-only", &["patina:watch/control.status"]);
        registry
            .register_knowledge_with_paths(
                StubChild::boxed("wit-only-child"),
                PathBuf::new(),
                manifest.path().to_path_buf(),
            )
            .unwrap();

        let err = registry
            .handle(
                "wit-only-child",
                &ChildRequest {
                    action: "status".to_string(),
                    payload: serde_json::json!({}),
                },
            )
            .unwrap_err();

        assert!(err.to_string().contains("is wit-only"), "got: {}", err);
    }

    #[test]
    fn handle_mode_denies_typed_call() {
        let registry = ChildRegistry::new();
        let manifest = write_manifest("handle", &["patina:watch/control.status"]);
        registry
            .register_knowledge_with_paths(
                TypedStubChild::boxed("handle-only-child"),
                PathBuf::new(),
                manifest.path().to_path_buf(),
            )
            .unwrap();

        let err = registry
            .call(
                "handle-only-child",
                &ChildCallRequest {
                    operation_id: "patina:watch/control.status".to_string(),
                    args: serde_json::json!([]),
                },
            )
            .unwrap_err();
        assert!(
            err.to_string().contains("ingress mode is handle"),
            "got: {}",
            err
        );
    }
}
