//! Child registry — loads, iterates, and provides access to children.
//!
//! Children can be registered during startup warmup while the daemon
//! is already accepting control-plane connections. Individual children
//! use per-child RwLock for concurrent handle() vs exclusive tick().

use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::{
    Child, ChildHealth, ChildRequest, ChildResponse, KnowledgeRuntimeStore, MotherHost, RunStatus,
};

/// Registry of Mother's children.
pub struct ChildRegistry {
    children: RwLock<Vec<Arc<RwLock<Box<dyn Child>>>>>,
}

#[derive(Debug, Clone)]
pub struct ChildActivationResult {
    pub name: String,
    pub duration_ms: u64,
    pub error: Option<String>,
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

    fn children_snapshot(&self) -> Vec<Arc<RwLock<Box<dyn Child>>>> {
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

    fn emit_mother_metric(
        child_name: &str,
        action: &str,
        name: &str,
        kind: &str,
        value: f64,
    ) -> Result<()> {
        let conn = Self::open_registry_events_connection()?;
        let timestamp = Utc::now().to_rfc3339();
        let data = serde_json::json!({
            "name": name,
            "kind": kind,
            "value": value,
            "labels": [
                ["child", child_name],
                ["action", action],
            ],
            "source": "mother",
            "scope": "child-handle-boundary",
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

    pub fn register_knowledge(&self, child: Box<dyn Child>) -> Result<()> {
        let name = child.name().to_string();
        if self.child_name_exists(&name) {
            anyhow::bail!("duplicate child name: {}", name);
        }
        self.children
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(Arc::new(RwLock::new(child)));
        Ok(())
    }

    fn child_name_exists(&self, name: &str) -> bool {
        self.children
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|child| child.read().unwrap_or_else(|e| e.into_inner()).name() == name)
    }

    /// Load all children — calls on_load() for each in order.
    /// Fails fast if any child fails to load.
    pub fn load_all(&self, host: &dyn MotherHost) -> Result<()> {
        for entry in self.children_snapshot() {
            let mut child = entry.write().unwrap_or_else(|e| e.into_inner());
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
            let mut child = entry.write().unwrap_or_else(|e| e.into_inner());
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
        runtime: &KnowledgeRuntimeStore,
        lease_owner: &str,
    ) -> Result<()> {
        for entry in self.children_snapshot() {
            let mut child = entry.write().unwrap_or_else(|e| e.into_inner());
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
            let child = match child.read() {
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
        if let Some(child) = children
            .iter()
            .find(|child| child.read().unwrap_or_else(|e| e.into_inner()).name() == child_name)
        {
            let child = child.read().unwrap_or_else(|e| e.into_inner());
            return Self::invoke_handle_observed(child_name, child.as_ref(), request);
        }

        Err(anyhow::anyhow!("unknown child: {}", child_name))
    }

    pub fn health(&self, child_name: &str) -> Result<ChildHealth> {
        let children = self.children_snapshot();
        if let Some(child) = children
            .iter()
            .find(|child| child.read().unwrap_or_else(|e| e.into_inner()).name() == child_name)
        {
            let child = child.read().unwrap_or_else(|e| e.into_inner());
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
}
