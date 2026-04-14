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
    fn child_call(
        &self,
        child_name: &str,
        operation_id: String,
        args: serde_json::Value,
    ) -> Result<serde_json::Value>;
    fn atlas_dashboard_html(&self) -> Result<String>;
    fn atlas_snapshot(&self) -> Result<serde_json::Value>;
    fn bridge_translate(
        &self,
        request: crate::bridge::BridgeRequest,
    ) -> Result<crate::bridge::BridgeResponse>;
    fn scry_query(
        &self,
        query: &str,
        limit: usize,
        repo: Option<String>,
        all_repos: bool,
    ) -> Result<Vec<ScryHit>>;
    fn federation_status(&self) -> Result<serde_json::Value>;
    fn federation_refresh(&self) -> Result<serde_json::Value>;
    fn federation_query(
        &self,
        payload: crate::protocol::FederationQueryPayload,
    ) -> Result<serde_json::Value>;
    fn secrets_get(&self) -> Result<serde_json::Value>;
    fn secrets_cache(&self, payload: serde_json::Value) -> Result<serde_json::Value>;
    fn secrets_lock(&self) -> Result<serde_json::Value>;
    fn pando_registry_init(
        &self,
        request: patina_protocol::PandoRegistryInit,
    ) -> Result<patina_protocol::PandoRegistryState>;
    fn pando_list(&self) -> Result<patina_protocol::PandoRegistryState>;
    fn lifecycle_load_pando(&self, name: &str) -> Result<crate::PandoLoadResult>;
    fn lifecycle_refresh(&self) -> Result<crate::PandoRefreshResult>;
    fn lifecycle_reload_child(&self, name: &str) -> Result<crate::ChildReloadResult>;
    fn typed_call_history(&self, limit: usize) -> Result<serde_json::Value>;
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
    pub federation_available: bool,
    pub federation_reason: Option<String>,
    pub federation_ducklake_loaded: bool,
    pub federation_projects_attached: usize,
    pub federation_projects_failed: usize,
    pub federation_projects_stale: usize,
    pub control_plane_ready: bool,
    pub children_ready_count: usize,
    pub children_total: usize,
    pub children_degraded: Vec<DegradedChild>,
}

#[derive(Debug, Clone)]
pub struct DegradedChild {
    pub name: String,
    pub reason: String,
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
    federation_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    federation_reason: Option<String>,
    federation_ducklake_loaded: bool,
    federation_projects_attached: usize,
    federation_projects_failed: usize,
    federation_projects_stale: usize,
    control_plane_ready: bool,
    children_ready_count: usize,
    children_total: usize,
    children_degraded: Vec<DegradedChildJson>,
}

#[derive(Serialize)]
struct DegradedChildJson {
    name: String,
    reason: String,
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

#[derive(Deserialize, Default)]
struct FederationNoopRequest {}

#[derive(Deserialize)]
struct LifecycleNameRequest {
    name: String,
}

#[derive(Deserialize, Default)]
struct LifecycleRefreshRequest {}

#[derive(Deserialize, Default)]
struct TypedCallHistoryRequest {
    #[serde(default = "default_typed_history_limit")]
    limit: usize,
}

fn default_typed_history_limit() -> usize {
    100
}

fn lifecycle_error(status: u16, code: &str, detail: &str) -> HttpResponse {
    HttpResponse::json(
        status,
        &serde_json::json!({
            "error": code,
            "code": status,
            "detail": detail,
        }),
    )
}

fn lifecycle_error_from_anyhow(error: &anyhow::Error) -> HttpResponse {
    let detail = error.to_string();
    if let Some(value) = detail.strip_prefix("invalid_request: ") {
        return lifecycle_error(400, "invalid_request", value);
    }
    if let Some(value) = detail.strip_prefix("child_not_found: ") {
        return lifecycle_error(404, "child_not_found", value);
    }
    if let Some(value) = detail.strip_prefix("pando_not_found: ") {
        return lifecycle_error(404, "pando_not_found", value);
    }
    if let Some(value) = detail.strip_prefix("operation_in_progress: ") {
        return lifecycle_error(409, "operation_in_progress", value);
    }
    if let Some(value) = detail.strip_prefix("internal_error: ") {
        return lifecycle_error(500, "internal_error", value);
    }
    lifecycle_error(500, "internal_error", &detail)
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
        federation_available: false,
        federation_reason: Some("federation status unavailable".to_string()),
        federation_ducklake_loaded: false,
        federation_projects_attached: 0,
        federation_projects_failed: 0,
        federation_projects_stale: 0,
        control_plane_ready: false,
        children_ready_count: 0,
        children_total: 0,
        children_degraded: Vec::new(),
    });

    let active_project_databases =
        details
            .active_project_databases
            .map(|db| ProjectDatabasesJson {
                events_db_bytes: db.events_db_bytes,
                patina_db_bytes: db.patina_db_bytes,
                runtime_db_bytes: db.runtime_db_bytes,
            });
    let children_degraded = details
        .children_degraded
        .iter()
        .map(|entry| DegradedChildJson {
            name: entry.name.clone(),
            reason: entry.reason.clone(),
        })
        .collect();

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
            federation_available: details.federation_available,
            federation_reason: details.federation_reason,
            federation_ducklake_loaded: details.federation_ducklake_loaded,
            federation_projects_attached: details.federation_projects_attached,
            federation_projects_failed: details.federation_projects_failed,
            federation_projects_stale: details.federation_projects_stale,
            control_plane_ready: details.control_plane_ready,
            children_ready_count: details.children_ready_count,
            children_total: details.children_total,
            children_degraded,
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

pub fn handle_atlas_dashboard(runtime: &dyn ApiRuntime) -> HttpResponse {
    match runtime.atlas_dashboard_html() {
        Ok(html) => HttpResponse {
            status: 200,
            headers: vec![(
                "Content-Type".to_string(),
                "text/html; charset=utf-8".to_string(),
            )],
            body: html.into_bytes(),
        },
        Err(error) => json_error(500, &format!("atlas dashboard render failed: {}", error)),
    }
}

pub fn handle_atlas_snapshot(runtime: &dyn ApiRuntime) -> HttpResponse {
    match runtime.atlas_snapshot() {
        Ok(snapshot) => HttpResponse::json(200, &snapshot),
        Err(error) => json_error(500, &format!("atlas snapshot failed: {}", error)),
    }
}

pub fn handle_bridge_translate(request: &HttpRequest, runtime: &dyn ApiRuntime) -> HttpResponse {
    if request.body.is_empty() {
        return json_error(400, "Missing request body");
    }

    let bridge_request: crate::bridge::BridgeRequest = match serde_json::from_slice(&request.body) {
        Ok(value) => value,
        Err(error) => return json_error(400, &format!("Invalid JSON: {}", error)),
    };

    match runtime.bridge_translate(bridge_request) {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(error) => json_error(500, &format!("bridge translate failed: {}", error)),
    }
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

pub fn handle_federation_status(request: &HttpRequest, runtime: &dyn ApiRuntime) -> HttpResponse {
    if !request.body.is_empty()
        && serde_json::from_slice::<FederationNoopRequest>(&request.body).is_err()
    {
        return json_error(400, "Invalid JSON");
    }
    match runtime.federation_status() {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(error) => json_error(500, &error.to_string()),
    }
}

pub fn handle_federation_refresh(request: &HttpRequest, runtime: &dyn ApiRuntime) -> HttpResponse {
    if !request.body.is_empty()
        && serde_json::from_slice::<FederationNoopRequest>(&request.body).is_err()
    {
        return json_error(400, "Invalid JSON");
    }
    match runtime.federation_refresh() {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(error) => json_error(500, &error.to_string()),
    }
}

pub fn handle_federation_query(request: &HttpRequest, runtime: &dyn ApiRuntime) -> HttpResponse {
    if request.body.is_empty() {
        return json_error(400, "Missing request body");
    }

    let payload: crate::protocol::FederationQueryPayload =
        match serde_json::from_slice(&request.body) {
            Ok(v) => v,
            Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
        };
    match runtime.federation_query(payload) {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(error) => json_error(500, &error.to_string()),
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

pub fn handle_lifecycle_load_pando(
    request: &HttpRequest,
    runtime: &dyn ApiRuntime,
) -> HttpResponse {
    if request.body.is_empty() {
        return lifecycle_error(400, "invalid_request", "missing request body");
    }
    let payload: LifecycleNameRequest = match serde_json::from_slice(&request.body) {
        Ok(v) => v,
        Err(e) => {
            return lifecycle_error(400, "invalid_request", &format!("invalid JSON: {}", e));
        }
    };
    if payload.name.trim().is_empty() {
        return lifecycle_error(400, "invalid_request", "name is required");
    }
    match runtime.lifecycle_load_pando(&payload.name) {
        Ok(response) => HttpResponse::json(200, &response),
        Err(e) => lifecycle_error_from_anyhow(&e),
    }
}

pub fn handle_lifecycle_refresh(request: &HttpRequest, runtime: &dyn ApiRuntime) -> HttpResponse {
    if !request.body.is_empty()
        && serde_json::from_slice::<LifecycleRefreshRequest>(&request.body).is_err()
    {
        return lifecycle_error(400, "invalid_request", "invalid JSON");
    }
    match runtime.lifecycle_refresh() {
        Ok(response) => HttpResponse::json(200, &response),
        Err(e) => lifecycle_error_from_anyhow(&e),
    }
}

pub fn handle_lifecycle_reload_child(
    request: &HttpRequest,
    runtime: &dyn ApiRuntime,
) -> HttpResponse {
    if request.body.is_empty() {
        return lifecycle_error(400, "invalid_request", "missing request body");
    }
    let payload: LifecycleNameRequest = match serde_json::from_slice(&request.body) {
        Ok(v) => v,
        Err(e) => {
            return lifecycle_error(400, "invalid_request", &format!("invalid JSON: {}", e));
        }
    };
    if payload.name.trim().is_empty() {
        return lifecycle_error(400, "invalid_request", "name is required");
    }
    match runtime.lifecycle_reload_child(&payload.name) {
        Ok(response) => HttpResponse::json(200, &response),
        Err(e) => lifecycle_error_from_anyhow(&e),
    }
}

pub fn handle_inspector_typed_calls(
    request: &HttpRequest,
    runtime: &dyn ApiRuntime,
) -> HttpResponse {
    let body = if request.body.is_empty() {
        TypedCallHistoryRequest::default()
    } else {
        match serde_json::from_slice::<TypedCallHistoryRequest>(&request.body) {
            Ok(value) => value,
            Err(error) => {
                return json_error(400, &format!("Invalid JSON: {}", error));
            }
        }
    };

    let limit = body.limit.min(MAX_LIMIT).max(1);
    match runtime.typed_call_history(limit) {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(error) => json_error(500, &format!("typed call history failed: {}", error)),
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

    if action == "call" {
        let operation_id = payload
            .get("operation_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();
        if operation_id.is_empty() {
            return json_error(400, "Missing operation_id for child typed call");
        }
        let args = payload
            .get("args")
            .cloned()
            .unwrap_or(serde_json::json!([]));
        return match runtime.child_call(child_name, operation_id, args) {
            Ok(payload) => HttpResponse::json(200, &payload),
            Err(e) => json_error(404, &e.to_string()),
        };
    }

    match runtime.child_handle(child_name, action.to_string(), payload) {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(e) => json_error(404, &e.to_string()),
    }
}

pub fn build_route_table(runtime: Arc<dyn ApiRuntime + Send + Sync>) -> RouteTable {
    let health_runtime = Arc::clone(&runtime);
    let version_runtime = Arc::clone(&runtime);
    let atlas_dashboard_runtime = Arc::clone(&runtime);
    let atlas_runtime = Arc::clone(&runtime);
    let bridge_runtime = Arc::clone(&runtime);
    let scry_runtime = Arc::clone(&runtime);
    let federation_status_runtime = Arc::clone(&runtime);
    let federation_refresh_runtime = Arc::clone(&runtime);
    let federation_query_runtime = Arc::clone(&runtime);
    let secrets_get_runtime = Arc::clone(&runtime);
    let secrets_cache_runtime = Arc::clone(&runtime);
    let secrets_lock_runtime = Arc::clone(&runtime);
    let pando_registry_runtime = Arc::clone(&runtime);
    let pando_list_runtime = Arc::clone(&runtime);
    let lifecycle_load_runtime = Arc::clone(&runtime);
    let lifecycle_refresh_runtime = Arc::clone(&runtime);
    let lifecycle_reload_runtime = Arc::clone(&runtime);
    let inspector_typed_calls_runtime = Arc::clone(&runtime);
    let child_runtime = Arc::clone(&runtime);

    RouteTable {
        get_health: Arc::new(move |_request| handle_health(&*health_runtime)),
        get_version: Arc::new(move |_request| handle_version(&*version_runtime)),
        get_atlas_dashboard: Arc::new(move |_request| {
            handle_atlas_dashboard(&*atlas_dashboard_runtime)
        }),
        get_atlas_snapshot: Arc::new(move |_request| handle_atlas_snapshot(&*atlas_runtime)),
        post_bridge_translate: Arc::new(move |request| {
            handle_bridge_translate(request, &*bridge_runtime)
        }),
        post_scry: Arc::new(move |request| handle_scry(request, &*scry_runtime)),
        post_federation_status: Arc::new(move |request| {
            handle_federation_status(request, &*federation_status_runtime)
        }),
        post_federation_refresh: Arc::new(move |request| {
            handle_federation_refresh(request, &*federation_refresh_runtime)
        }),
        post_federation_query: Arc::new(move |request| {
            handle_federation_query(request, &*federation_query_runtime)
        }),
        get_secrets_cache: Arc::new(move |_request| handle_secrets_get(&*secrets_get_runtime)),
        post_secrets_cache: Arc::new(move |request| {
            handle_secrets_cache(request, &*secrets_cache_runtime)
        }),
        post_secrets_lock: Arc::new(move |_request| handle_secrets_lock(&*secrets_lock_runtime)),
        post_pando_registry_init: Arc::new(move |request| {
            handle_pando_registry_init(request, &*pando_registry_runtime)
        }),
        get_pando_list: Arc::new(move |_request| handle_pando_list(&*pando_list_runtime)),
        post_lifecycle_load_pando: Arc::new(move |request| {
            handle_lifecycle_load_pando(request, &*lifecycle_load_runtime)
        }),
        post_lifecycle_refresh: Arc::new(move |request| {
            handle_lifecycle_refresh(request, &*lifecycle_refresh_runtime)
        }),
        post_lifecycle_reload_child: Arc::new(move |request| {
            handle_lifecycle_reload_child(request, &*lifecycle_reload_runtime)
        }),
        post_inspector_typed_calls: Arc::new(move |request| {
            handle_inspector_typed_calls(request, &*inspector_typed_calls_runtime)
        }),
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
                federation_available: true,
                federation_reason: None,
                federation_ducklake_loaded: true,
                federation_projects_attached: 2,
                federation_projects_failed: 1,
                federation_projects_stale: 1,
                control_plane_ready: true,
                children_ready_count: 1,
                children_total: 2,
                children_degraded: vec![DegradedChild {
                    name: "catalog".to_string(),
                    reason: "on_load failed".to_string(),
                }],
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

        fn child_call(
            &self,
            _child_name: &str,
            operation_id: String,
            args: serde_json::Value,
        ) -> Result<serde_json::Value> {
            Ok(serde_json::json!({
                "operation_id": operation_id,
                "args": args,
                "typed": true,
            }))
        }

        fn atlas_dashboard_html(&self) -> Result<String> {
            Ok("<html><body>atlas</body></html>".to_string())
        }

        fn atlas_snapshot(&self) -> Result<serde_json::Value> {
            Ok(serde_json::json!({"summary": {"spec_count": 0}}))
        }

        fn bridge_translate(
            &self,
            request: crate::bridge::BridgeRequest,
        ) -> Result<crate::bridge::BridgeResponse> {
            Ok(crate::bridge::evaluate_bridge_request(&request))
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

        fn federation_status(&self) -> Result<serde_json::Value> {
            Ok(serde_json::json!({"federation": "available"}))
        }

        fn federation_refresh(&self) -> Result<serde_json::Value> {
            Ok(serde_json::json!({"federation": "available"}))
        }

        fn federation_query(
            &self,
            _payload: crate::protocol::FederationQueryPayload,
        ) -> Result<serde_json::Value> {
            Ok(
                serde_json::json!({"columns":[], "rows":[], "row_count":0, "truncated":false, "elapsed_ms":1}),
            )
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

        fn lifecycle_load_pando(&self, name: &str) -> Result<crate::PandoLoadResult> {
            Ok(crate::PandoLoadResult {
                pando: name.to_string(),
                status: "loaded".to_string(),
                children_activated: 1,
            })
        }

        fn lifecycle_refresh(&self) -> Result<crate::PandoRefreshResult> {
            Ok(crate::PandoRefreshResult {
                pandos_loaded: 1,
                pandos_failed: 0,
                children_activated: 1,
                children_failed: 0,
                degraded: vec![],
            })
        }

        fn lifecycle_reload_child(&self, name: &str) -> Result<crate::ChildReloadResult> {
            Ok(crate::ChildReloadResult {
                child: name.to_string(),
                status: "reloaded".to_string(),
                previous_instance: "drained".to_string(),
                reason: None,
            })
        }

        fn typed_call_history(&self, limit: usize) -> Result<serde_json::Value> {
            Ok(serde_json::json!({
                "count": limit.min(1),
                "calls": [{
                    "child": "folder-watch-actor",
                    "operation_id": "patina:watch/control.status",
                    "outcome": "success"
                }]
            }))
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
        assert_eq!(
            json.get("federation_available").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            json.get("federation_projects_failed")
                .and_then(|v| v.as_u64()),
            Some(1)
        );
        assert_eq!(
            json.get("control_plane_ready").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            json.get("children_ready_count").and_then(|v| v.as_u64()),
            Some(1)
        );
        assert_eq!(json.get("children_total").and_then(|v| v.as_u64()), Some(2));
        assert_eq!(
            json.get("children_degraded")
                .and_then(|v| v.as_array())
                .and_then(|items| items.first())
                .and_then(|entry| entry.get("name"))
                .and_then(|v| v.as_str()),
            Some("catalog")
        );
    }

    #[test]
    fn atlas_snapshot_route_returns_json_payload() {
        let response = handle_atlas_snapshot(&StubRuntime);
        assert_eq!(response.status, 200);
        let json: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(
            json.get("summary")
                .and_then(|v| v.get("spec_count"))
                .and_then(|v| v.as_u64()),
            Some(0)
        );
    }

    #[test]
    fn atlas_dashboard_route_returns_html_payload() {
        let response = handle_atlas_dashboard(&StubRuntime);
        assert_eq!(response.status, 200);
        let body = String::from_utf8(response.body).unwrap();
        assert!(body.contains("atlas"));
    }

    #[test]
    fn child_call_route_dispatches_typed_operation() {
        let request = HttpRequest {
            method: "POST".to_string(),
            path: "/child/folder-watch-actor/call".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({
                "operation_id": "patina:watch/control.status",
                "args": [],
            }))
            .unwrap(),
        };

        let response = handle_child_request(&request, &StubRuntime);
        assert_eq!(response.status, 200);
        let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(
            payload.get("operation_id").and_then(|v| v.as_str()),
            Some("patina:watch/control.status")
        );
        assert_eq!(payload.get("typed"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn child_call_route_rejects_missing_operation_id() {
        let request = HttpRequest {
            method: "POST".to_string(),
            path: "/child/folder-watch-actor/call".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({"args": []})).unwrap(),
        };

        let response = handle_child_request(&request, &StubRuntime);
        assert_eq!(response.status, 400);
    }

    #[test]
    fn inspector_typed_calls_route_returns_history() {
        let request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/inspector/typed-calls".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({"limit": 10})).unwrap(),
        };

        let response = handle_inspector_typed_calls(&request, &StubRuntime);
        assert_eq!(response.status, 200);
        let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(payload.get("count").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(
            payload
                .get("calls")
                .and_then(|v| v.as_array())
                .and_then(|items| items.first())
                .and_then(|v| v.get("operation_id"))
                .and_then(|v| v.as_str()),
            Some("patina:watch/control.status")
        );
    }

    #[test]
    fn bridge_translate_route_returns_allow_and_deny_payloads() {
        let allow_request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/bridge/translate".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&crate::bridge::BridgeRequest {
                action: "dispatch".to_string(),
                legacy_toys: vec!["log".to_string(), "state".to_string()],
                payload: serde_json::Value::Null,
            })
            .unwrap(),
        };
        let allow_response = handle_bridge_translate(&allow_request, &StubRuntime);
        assert_eq!(allow_response.status, 200);
        let allow_json: serde_json::Value = serde_json::from_slice(&allow_response.body).unwrap();
        assert_eq!(
            allow_json.get("verdict").and_then(|v| v.as_str()),
            Some("allow")
        );

        let deny_request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/bridge/translate".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&crate::bridge::BridgeRequest {
                action: "dispatch".to_string(),
                legacy_toys: vec!["log".to_string(), "nope".to_string()],
                payload: serde_json::Value::Null,
            })
            .unwrap(),
        };
        let deny_response = handle_bridge_translate(&deny_request, &StubRuntime);
        assert_eq!(deny_response.status, 200);
        let deny_json: serde_json::Value = serde_json::from_slice(&deny_response.body).unwrap();
        assert_eq!(
            deny_json.get("verdict").and_then(|v| v.as_str()),
            Some("deny")
        );
    }

    #[test]
    fn bridge_translate_route_rejects_invalid_json() {
        let request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/bridge/translate".to_string(),
            headers: vec![],
            body: b"not-json".to_vec(),
        };

        let response = handle_bridge_translate(&request, &StubRuntime);
        assert_eq!(response.status, 400);
    }

    #[test]
    fn federation_routes_return_json_payloads() {
        let status_request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/federation/status".to_string(),
            headers: vec![],
            body: vec![],
        };
        let status_response = handle_federation_status(&status_request, &StubRuntime);
        assert_eq!(status_response.status, 200);

        let query_request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/federation/query".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({
                "sql": "SELECT 1",
                "params": [],
                "limit": 1,
                "timeout_ms": 100
            }))
            .unwrap(),
        };
        let query_response = handle_federation_query(&query_request, &StubRuntime);
        assert_eq!(query_response.status, 200);
    }

    #[test]
    fn lifecycle_reload_maps_operation_in_progress_to_409_envelope() {
        struct BusyRuntime;

        impl ApiRuntime for BusyRuntime {
            fn version(&self) -> String {
                "0.0.0-test".to_string()
            }
            fn uptime_secs(&self) -> u64 {
                0
            }
            fn health_all(&self) -> Vec<(String, crate::ChildHealth)> {
                vec![]
            }
            fn health_details(&self) -> Result<HealthDetails> {
                Ok(HealthDetails {
                    registered_projects: 0,
                    active_project_uid: None,
                    active_project_databases: None,
                    state_db_bytes: None,
                    federation_available: false,
                    federation_reason: None,
                    federation_ducklake_loaded: false,
                    federation_projects_attached: 0,
                    federation_projects_failed: 0,
                    federation_projects_stale: 0,
                    control_plane_ready: false,
                    children_ready_count: 0,
                    children_total: 0,
                    children_degraded: vec![],
                })
            }
            fn child_health(&self, _child_name: &str) -> Result<crate::ChildHealth> {
                Ok(crate::ChildHealth::Healthy)
            }
            fn child_handle(
                &self,
                _child_name: &str,
                _action: String,
                _payload: serde_json::Value,
            ) -> Result<serde_json::Value> {
                Ok(serde_json::Value::Null)
            }

            fn child_call(
                &self,
                _child_name: &str,
                operation_id: String,
                args: serde_json::Value,
            ) -> Result<serde_json::Value> {
                Ok(serde_json::json!({
                    "operation_id": operation_id,
                    "args": args,
                }))
            }
            fn atlas_dashboard_html(&self) -> Result<String> {
                Ok("<html><body>atlas</body></html>".to_string())
            }

            fn atlas_snapshot(&self) -> Result<serde_json::Value> {
                Ok(serde_json::json!({"summary": {"spec_count": 0}}))
            }

            fn bridge_translate(
                &self,
                request: crate::bridge::BridgeRequest,
            ) -> Result<crate::bridge::BridgeResponse> {
                Ok(crate::bridge::evaluate_bridge_request(&request))
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
            fn federation_status(&self) -> Result<serde_json::Value> {
                Ok(serde_json::json!({}))
            }
            fn federation_refresh(&self) -> Result<serde_json::Value> {
                Ok(serde_json::json!({}))
            }
            fn federation_query(
                &self,
                _payload: crate::protocol::FederationQueryPayload,
            ) -> Result<serde_json::Value> {
                Ok(serde_json::json!({}))
            }
            fn secrets_get(&self) -> Result<serde_json::Value> {
                Ok(serde_json::json!({}))
            }
            fn secrets_cache(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
                Ok(payload)
            }
            fn secrets_lock(&self) -> Result<serde_json::Value> {
                Ok(serde_json::json!({}))
            }
            fn pando_registry_init(
                &self,
                _request: patina_protocol::PandoRegistryInit,
            ) -> Result<patina_protocol::PandoRegistryState> {
                Ok(patina_protocol::PandoRegistryState {
                    protocol_version: patina_protocol::PANDO_REGISTRY_PROTOCOL_VERSION,
                    pandos: vec![],
                })
            }
            fn pando_list(&self) -> Result<patina_protocol::PandoRegistryState> {
                Ok(patina_protocol::PandoRegistryState {
                    protocol_version: patina_protocol::PANDO_REGISTRY_PROTOCOL_VERSION,
                    pandos: vec![],
                })
            }
            fn lifecycle_load_pando(&self, _name: &str) -> Result<crate::PandoLoadResult> {
                anyhow::bail!("operation_in_progress: load already running")
            }
            fn lifecycle_refresh(&self) -> Result<crate::PandoRefreshResult> {
                anyhow::bail!("operation_in_progress: refresh already running")
            }
            fn lifecycle_reload_child(&self, _name: &str) -> Result<crate::ChildReloadResult> {
                anyhow::bail!("operation_in_progress: reload already running")
            }
            fn typed_call_history(&self, _limit: usize) -> Result<serde_json::Value> {
                Ok(serde_json::json!({"count": 0, "calls": []}))
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

        let request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/lifecycle/reload-child".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({"name": "x"})).unwrap(),
        };
        let response = handle_lifecycle_reload_child(&request, &BusyRuntime);
        assert_eq!(response.status, 409);
        let json: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(
            json.get("error").and_then(|v| v.as_str()),
            Some("operation_in_progress")
        );
    }
}
