use anyhow::Result;
use patina_protocol::{BuiltinChild, BuiltinChildAction, BuiltinChildRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::http_daemon::{json_error, HttpRequest, HttpResponse};
use crate::http_routes::RouteTable;

const MAX_LIMIT: usize = 1000;

#[derive(Debug, Clone)]
pub struct ScryHit {
    pub content: String,
    pub score: f32,
    pub event_type: String,
    pub source_id: String,
    pub timestamp: String,
}

pub trait ApiRuntime {
    fn version(&self) -> String;
    fn uptime_secs(&self) -> u64;
    fn health_all(&self) -> Vec<(String, crate::ChildHealth)>;
    fn health_details(&self) -> Result<HealthDetails>;
    fn child_health(&self, child_name: &str) -> Result<crate::ChildHealth>;
    fn child_handle(
        &self,
        child_name: &str,
        action: String,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value>;
    fn scry_query(
        &self,
        query: &str,
        limit: usize,
        repo: Option<String>,
        all_repos: bool,
    ) -> Result<Vec<ScryHit>>;
    fn secrets_get(&self) -> Result<serde_json::Value>;
    fn secrets_cache(&self, payload: serde_json::Value) -> Result<serde_json::Value>;
    fn secrets_lock(&self) -> Result<serde_json::Value>;
    fn pando_registry_init(
        &self,
        request: patina_protocol::PandoRegistryInit,
    ) -> Result<patina_protocol::PandoRegistryState>;
    fn pando_list(&self) -> Result<patina_protocol::PandoRegistryState>;
    fn builtin_spec_dispatch(
        &self,
        request: patina_protocol::SpecDispatchRequest,
    ) -> Result<serde_json::Value>;
    fn builtin_lake_dispatch(
        &self,
        request: patina_protocol::LakeDispatchRequest,
    ) -> Result<serde_json::Value>;
    fn builtin_doctor_run(&self) -> Result<patina_protocol::DoctorRunResult>;
    fn builtin_secrets_dispatch(&self, payload: serde_json::Value) -> HttpResponse;
}

#[derive(Debug, Clone)]
pub struct HealthDetails {
    pub registered_projects: usize,
    pub active_project_uid: Option<String>,
    pub active_project_databases: Option<ProjectDatabases>,
    pub state_db_bytes: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ProjectDatabases {
    pub events_db_bytes: Option<u64>,
    pub patina_db_bytes: Option<u64>,
    pub runtime_db_bytes: Option<u64>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_secs: u64,
    children: Vec<ChildHealthJson>,
    child_count: usize,
    registered_projects: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_project_uid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_project_databases: Option<ProjectDatabasesJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_db_bytes: Option<u64>,
}

#[derive(Serialize)]
struct ProjectDatabasesJson {
    events_db_bytes: Option<u64>,
    patina_db_bytes: Option<u64>,
    runtime_db_bytes: Option<u64>,
}

#[derive(Serialize)]
struct ChildHealthJson {
    name: String,
    status: String,
}

#[derive(Deserialize)]
struct ScryRequest {
    query: String,
    repo: Option<String>,
    #[serde(default)]
    all_repos: bool,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    10
}

#[derive(Serialize)]
struct ScryResponse {
    results: Vec<ScryResultJson>,
    count: usize,
}

#[derive(Serialize)]
struct ScryResultJson {
    id: i64,
    content: String,
    score: f32,
    event_type: String,
    source_id: String,
    timestamp: String,
}

pub fn handle_health(runtime: &dyn ApiRuntime) -> HttpResponse {
    let children: Vec<ChildHealthJson> = runtime
        .health_all()
        .into_iter()
        .map(|(name, health)| ChildHealthJson {
            name,
            status: health.to_string(),
        })
        .collect();

    let details = runtime.health_details().unwrap_or(HealthDetails {
        registered_projects: 0,
        active_project_uid: None,
        active_project_databases: None,
        state_db_bytes: None,
    });

    let active_project_databases =
        details
            .active_project_databases
            .map(|db| ProjectDatabasesJson {
                events_db_bytes: db.events_db_bytes,
                patina_db_bytes: db.patina_db_bytes,
                runtime_db_bytes: db.runtime_db_bytes,
            });

    HttpResponse::json(
        200,
        &HealthResponse {
            status: "ok".to_string(),
            version: runtime.version(),
            uptime_secs: runtime.uptime_secs(),
            child_count: children.len(),
            children,
            registered_projects: details.registered_projects,
            active_project_uid: details.active_project_uid,
            active_project_databases,
            state_db_bytes: details.state_db_bytes,
        },
    )
}

pub fn handle_version(runtime: &dyn ApiRuntime) -> HttpResponse {
    HttpResponse::json(
        200,
        &serde_json::json!({
            "version": runtime.version(),
            "name": "patina-mother"
        }),
    )
}

pub fn handle_scry(request: &HttpRequest, runtime: &dyn ApiRuntime) -> HttpResponse {
    if request.body.is_empty() {
        return json_error(400, "Missing request body");
    }

    let mut body: ScryRequest = match serde_json::from_slice(&request.body) {
        Ok(req) => req,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
    };

    body.limit = body.limit.min(MAX_LIMIT);

    match runtime.scry_query(&body.query, body.limit, body.repo, body.all_repos) {
        Ok(results) => {
            let json_results: Vec<ScryResultJson> = results
                .into_iter()
                .map(|r| ScryResultJson {
                    id: 0,
                    content: r.content,
                    score: r.score,
                    event_type: r.event_type,
                    source_id: r.source_id,
                    timestamp: r.timestamp,
                })
                .collect();

            let response = ScryResponse {
                count: json_results.len(),
                results: json_results,
            };

            HttpResponse::json(200, &response)
        }
        Err(e) => json_error(500, &format!("Scry failed: {}", e)),
    }
}

pub fn handle_secrets_get(runtime: &dyn ApiRuntime) -> HttpResponse {
    match runtime.secrets_get() {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(_) => json_error(404, "No cached secrets"),
    }
}

pub fn handle_secrets_cache(request: &HttpRequest, runtime: &dyn ApiRuntime) -> HttpResponse {
    if request.body.is_empty() {
        return json_error(400, "Missing request body");
    }

    let payload: serde_json::Value = match serde_json::from_slice(&request.body) {
        Ok(v) => v,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
    };

    match runtime.secrets_cache(payload) {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(e) => json_error(500, &format!("Cache failed: {}", e)),
    }
}

pub fn handle_secrets_lock(runtime: &dyn ApiRuntime) -> HttpResponse {
    match runtime.secrets_lock() {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(e) => json_error(500, &format!("Lock failed: {}", e)),
    }
}

pub fn handle_pando_registry_init(request: &HttpRequest, runtime: &dyn ApiRuntime) -> HttpResponse {
    if request.body.is_empty() {
        return json_error(400, "Missing request body");
    }

    let init: patina_protocol::PandoRegistryInit = match serde_json::from_slice(&request.body) {
        Ok(v) => v,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
    };

    match runtime.pando_registry_init(init) {
        Ok(state) => HttpResponse::json(200, &state),
        Err(e) => json_error(400, &e.to_string()),
    }
}

pub fn handle_pando_list(runtime: &dyn ApiRuntime) -> HttpResponse {
    match runtime.pando_list() {
        Ok(state) => HttpResponse::json(200, &state),
        Err(e) => json_error(500, &e.to_string()),
    }
}

pub fn handle_child_request(request: &HttpRequest, runtime: &dyn ApiRuntime) -> HttpResponse {
    let parts: Vec<&str> = request.path[1..].split('/').collect();
    if parts.len() != 3 {
        return json_error(400, "Expected /child/{name}/{action}");
    }
    let child_name = parts[1];
    let action = parts[2];

    if BuiltinChild::from_route(child_name).is_some() {
        match BuiltinChildRequest::from_http_parts(child_name, action, &request.body) {
            Ok(builtin) => {
                return match builtin.action {
                    BuiltinChildAction::Health => {
                        HttpResponse::json(200, &serde_json::json!({ "status": "healthy" }))
                    }
                    BuiltinChildAction::SpecDispatch(dispatch) => {
                        match runtime.builtin_spec_dispatch(dispatch) {
                            Ok(payload) => HttpResponse::json(200, &payload),
                            Err(e) => json_error(400, &e.to_string()),
                        }
                    }
                    BuiltinChildAction::LakeDispatch(dispatch) => {
                        match runtime.builtin_lake_dispatch(dispatch) {
                            Ok(payload) => HttpResponse::json(200, &payload),
                            Err(e) => json_error(400, &e.to_string()),
                        }
                    }
                    BuiltinChildAction::DoctorRun(_) => match runtime.builtin_doctor_run() {
                        Ok(result) => HttpResponse::json(
                            200,
                            &serde_json::json!({
                                "child": "doctor",
                                "text": "",
                                "data": result.data,
                                "exit_code": result.exit_code,
                            }),
                        ),
                        Err(e) => json_error(400, &e.to_string()),
                    },
                    BuiltinChildAction::SecretsDispatch(dispatch) => {
                        runtime.builtin_secrets_dispatch(dispatch.operation.into_payload())
                    }
                };
            }
            Err(message) if message.starts_with("Unsupported action") => {}
            Err(message) => return json_error(400, &message),
        }
    }

    if action == "health" {
        return match runtime.child_health(child_name) {
            Ok(health) => {
                let status = match health {
                    crate::ChildHealth::Healthy => "healthy",
                    crate::ChildHealth::Degraded(_) => "degraded",
                    crate::ChildHealth::Unhealthy(_) => "unhealthy",
                };
                HttpResponse::json(200, &serde_json::json!({ "status": status }))
            }
            Err(e) => json_error(404, &e.to_string()),
        };
    }

    let payload = if request.body.is_empty() {
        serde_json::Value::Null
    } else {
        match serde_json::from_slice(&request.body) {
            Ok(v) => v,
            Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
        }
    };

    match runtime.child_handle(child_name, action.to_string(), payload) {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(e) => json_error(404, &e.to_string()),
    }
}

pub fn build_route_table(runtime: Arc<dyn ApiRuntime + Send + Sync>) -> RouteTable {
    let health_runtime = Arc::clone(&runtime);
    let version_runtime = Arc::clone(&runtime);
    let scry_runtime = Arc::clone(&runtime);
    let secrets_get_runtime = Arc::clone(&runtime);
    let secrets_cache_runtime = Arc::clone(&runtime);
    let secrets_lock_runtime = Arc::clone(&runtime);
    let pando_registry_runtime = Arc::clone(&runtime);
    let pando_list_runtime = Arc::clone(&runtime);
    let child_runtime = Arc::clone(&runtime);

    RouteTable {
        get_health: Arc::new(move |_request| handle_health(&*health_runtime)),
        get_version: Arc::new(move |_request| handle_version(&*version_runtime)),
        post_scry: Arc::new(move |request| handle_scry(request, &*scry_runtime)),
        get_secrets_cache: Arc::new(move |_request| handle_secrets_get(&*secrets_get_runtime)),
        post_secrets_cache: Arc::new(move |request| {
            handle_secrets_cache(request, &*secrets_cache_runtime)
        }),
        post_secrets_lock: Arc::new(move |_request| handle_secrets_lock(&*secrets_lock_runtime)),
        post_pando_registry_init: Arc::new(move |request| {
            handle_pando_registry_init(request, &*pando_registry_runtime)
        }),
        get_pando_list: Arc::new(move |_request| handle_pando_list(&*pando_list_runtime)),
        child_request: Arc::new(move |request| handle_child_request(request, &*child_runtime)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubRuntime;

    impl ApiRuntime for StubRuntime {
        fn version(&self) -> String {
            "0.0.0-test".to_string()
        }

        fn uptime_secs(&self) -> u64 {
            42
        }

        fn health_all(&self) -> Vec<(String, crate::ChildHealth)> {
            vec![("ducklake".to_string(), crate::ChildHealth::Healthy)]
        }

        fn health_details(&self) -> Result<HealthDetails> {
            Ok(HealthDetails {
                registered_projects: 2,
                active_project_uid: Some("2bdc808e".to_string()),
                active_project_databases: Some(ProjectDatabases {
                    events_db_bytes: Some(1024),
                    patina_db_bytes: Some(2048),
                    runtime_db_bytes: Some(512),
                }),
                state_db_bytes: Some(256),
            })
        }

        fn child_health(&self, _child_name: &str) -> Result<crate::ChildHealth> {
            Ok(crate::ChildHealth::Healthy)
        }

        fn child_handle(
            &self,
            _child_name: &str,
            _action: String,
            payload: serde_json::Value,
        ) -> Result<serde_json::Value> {
            Ok(payload)
        }

        fn scry_query(
            &self,
            _query: &str,
            _limit: usize,
            _repo: Option<String>,
            _all_repos: bool,
        ) -> Result<Vec<ScryHit>> {
            Ok(vec![])
        }

        fn secrets_get(&self) -> Result<serde_json::Value> {
            Ok(serde_json::json!({}))
        }

        fn secrets_cache(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
            Ok(payload)
        }

        fn secrets_lock(&self) -> Result<serde_json::Value> {
            Ok(serde_json::json!({"locked": true}))
        }

        fn pando_registry_init(
            &self,
            request: patina_protocol::PandoRegistryInit,
        ) -> Result<patina_protocol::PandoRegistryState> {
            Ok(patina_protocol::PandoRegistryState {
                protocol_version: request.protocol_version,
                pandos: vec![],
            })
        }

        fn pando_list(&self) -> Result<patina_protocol::PandoRegistryState> {
            Ok(patina_protocol::PandoRegistryState {
                protocol_version: patina_protocol::PANDO_REGISTRY_PROTOCOL_VERSION,
                pandos: vec![],
            })
        }

        fn builtin_spec_dispatch(
            &self,
            _request: patina_protocol::SpecDispatchRequest,
        ) -> Result<serde_json::Value> {
            Ok(serde_json::json!({}))
        }

        fn builtin_lake_dispatch(
            &self,
            _request: patina_protocol::LakeDispatchRequest,
        ) -> Result<serde_json::Value> {
            Ok(serde_json::json!({}))
        }

        fn builtin_doctor_run(&self) -> Result<patina_protocol::DoctorRunResult> {
            Ok(patina_protocol::DoctorRunResult {
                data: serde_json::json!({}),
                exit_code: 0,
            })
        }

        fn builtin_secrets_dispatch(&self, _payload: serde_json::Value) -> HttpResponse {
            HttpResponse::json(200, &serde_json::json!({}))
        }
    }

    #[test]
    fn health_response_includes_additive_deep_fields() {
        let response = handle_health(&StubRuntime);
        assert_eq!(response.status, 200);
        let json: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(
            json.get("version").and_then(|v| v.as_str()),
            Some("0.0.0-test")
        );
        assert_eq!(json.get("uptime_secs").and_then(|v| v.as_u64()), Some(42));
        assert_eq!(json.get("child_count").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(
            json.get("registered_projects").and_then(|v| v.as_u64()),
            Some(2)
        );
        assert_eq!(
            json.get("active_project_uid").and_then(|v| v.as_str()),
            Some("2bdc808e")
        );
        assert_eq!(
            json.get("state_db_bytes").and_then(|v| v.as_u64()),
            Some(256)
        );
        assert_eq!(
            json.get("active_project_databases")
                .and_then(|v| v.get("events_db_bytes"))
                .and_then(|v| v.as_u64()),
            Some(1024)
        );
    }
}
