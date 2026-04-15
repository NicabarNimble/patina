use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::http_daemon::{json_error, HttpRequest, HttpResponse};
use crate::http_routes::RouteTable;

#[path = "http_api/child.rs"]
mod child;
#[path = "http_api/health.rs"]
mod health;
#[path = "http_api/inspector.rs"]
mod inspector;
#[path = "http_api/lifecycle.rs"]
mod lifecycle;
#[path = "http_api/rivet.rs"]
mod rivet;

pub use child::handle_child_request;
pub use health::{handle_health, handle_version};
pub use inspector::handle_inspector_typed_calls;
pub use lifecycle::{
    handle_lifecycle_load_pando, handle_lifecycle_refresh, handle_lifecycle_reload_child,
    handle_lifecycle_warmup_children,
};
pub use rivet::{handle_rivet_dispatch, RivetDispatchDeadLetter, RivetDispatchRequest};

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
        correlation: Option<crate::CallCorrelation>,
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
    fn lifecycle_warmup_children(&self) -> Result<crate::ChildWarmupResult>;
    fn rivet_dispatch(&self, request: RivetDispatchRequest) -> Result<serde_json::Value>;
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
    pub startup_profile: String,
    pub rivet_integration: String,
    pub child_warmup: ChildWarmupState,
    pub memory: MemoryStatus,
    pub control_plane_ready: bool,
    pub children_ready_count: usize,
    pub children_total: usize,
    pub children_degraded: Vec<DegradedChild>,
}

#[derive(Debug, Clone)]
pub struct ChildWarmupState {
    pub mode: String,
    pub state: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MemoryStatus {
    pub rss_bytes: Option<u64>,
    pub max_rss_bytes: Option<u64>,
    pub soft_limit_bytes: Option<u64>,
    pub pressure: String,
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
    let lifecycle_warmup_runtime = Arc::clone(&runtime);
    let rivet_dispatch_runtime = Arc::clone(&runtime);
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
        post_lifecycle_warmup_children: Arc::new(move |request| {
            handle_lifecycle_warmup_children(request, &*lifecycle_warmup_runtime)
        }),
        post_rivet_dispatch: Arc::new(move |request| {
            handle_rivet_dispatch(request, &*rivet_dispatch_runtime)
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
                startup_profile: "full".to_string(),
                rivet_integration: "disabled".to_string(),
                child_warmup: ChildWarmupState {
                    mode: "auto".to_string(),
                    state: "complete".to_string(),
                    last_error: None,
                },
                memory: MemoryStatus {
                    rss_bytes: Some(8 * 1024 * 1024),
                    max_rss_bytes: Some(12 * 1024 * 1024),
                    soft_limit_bytes: Some(64 * 1024 * 1024),
                    pressure: "ok".to_string(),
                },
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
            correlation: Option<crate::CallCorrelation>,
        ) -> Result<serde_json::Value> {
            Ok(serde_json::json!({
                "operation_id": operation_id,
                "args": args,
                "correlation": correlation,
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

        fn lifecycle_warmup_children(&self) -> Result<crate::ChildWarmupResult> {
            Ok(crate::ChildWarmupResult {
                status: "warmed".to_string(),
                discovered: 2,
                activated: 2,
                failed: 0,
                degraded: vec![],
            })
        }

        fn rivet_dispatch(&self, request: RivetDispatchRequest) -> Result<serde_json::Value> {
            Ok(serde_json::json!({
                "child": request.child,
                "operation_id": request.operation_id,
                "args": request.args,
                "correlation": request.correlation,
                "delivery": request.delivery_policy(),
                "dead_letter": request.dead_letter,
                "adapter": "rivet"
            }))
        }

        fn typed_call_history(&self, limit: usize) -> Result<serde_json::Value> {
            Ok(serde_json::json!({
                "count": limit.min(1),
                "calls": [{
                    "child": "folder-watch-actor",
                    "operation_id": "patina:watch/control.status",
                    "outcome": "success",
                    "correlation": {
                        "rivet_run_id": "run-123",
                        "rivet_actor_id": "actor-a",
                        "rivet_workflow_id": "workflow-z",
                        "rivet_job_id": "job-9"
                    }
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
            json.get("startup_profile").and_then(|v| v.as_str()),
            Some("full")
        );
        assert_eq!(
            json.get("rivet_integration").and_then(|v| v.as_str()),
            Some("disabled")
        );
        assert_eq!(
            json.get("child_warmup")
                .and_then(|v| v.get("state"))
                .and_then(|v| v.as_str()),
            Some("complete")
        );
        assert_eq!(
            json.get("memory")
                .and_then(|v| v.get("pressure"))
                .and_then(|v| v.as_str()),
            Some("ok")
        );
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
                "correlation": {
                    "rivet_run_id": "run-123",
                    "rivet_actor_id": "actor-a"
                }
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
        assert_eq!(
            payload
                .get("correlation")
                .and_then(|v| v.get("rivet_run_id"))
                .and_then(|v| v.as_str()),
            Some("run-123")
        );
        assert_eq!(payload.get("typed"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn rivet_dispatch_route_translates_to_typed_call_shape() {
        let request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/rivet/dispatch".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({
                "child": "folder-watch-actor",
                "operation_id": "patina:watch/control.status",
                "args": [],
                "correlation": {
                    "rivet_run_id": "run-123"
                }
            }))
            .unwrap(),
        };

        let response = handle_rivet_dispatch(&request, &StubRuntime);
        assert_eq!(response.status, 200);
        let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(
            payload.get("adapter").and_then(|v| v.as_str()),
            Some("rivet")
        );
        assert_eq!(
            payload
                .get("correlation")
                .and_then(|v| v.get("rivet_run_id"))
                .and_then(|v| v.as_str()),
            Some("run-123")
        );
        assert_eq!(
            payload.get("delivery").and_then(|v| v.as_str()),
            Some("required")
        );
    }

    #[test]
    fn rivet_dispatch_route_rejects_dead_letter_without_target() {
        let request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/rivet/dispatch".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({
                "child": "folder-watch-actor",
                "operation_id": "patina:watch/control.status",
                "args": [],
                "delivery": "dead-letter"
            }))
            .unwrap(),
        };

        let response = handle_rivet_dispatch(&request, &StubRuntime);
        assert_eq!(response.status, 400);
        let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(
            payload.get("error").and_then(|v| v.as_str()),
            Some("invalid_request")
        );
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
    fn inspector_typed_calls_filters_by_rivet_run_id() {
        let request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/inspector/typed-calls".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({
                "limit": 10,
                "rivet_run_id": "run-123"
            }))
            .unwrap(),
        };

        let response = handle_inspector_typed_calls(&request, &StubRuntime);
        assert_eq!(response.status, 200);
        let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(payload.get("count").and_then(|v| v.as_u64()), Some(1));

        let miss_request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/inspector/typed-calls".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({
                "limit": 10,
                "rivet_run_id": "run-not-found"
            }))
            .unwrap(),
        };

        let miss_response = handle_inspector_typed_calls(&miss_request, &StubRuntime);
        assert_eq!(miss_response.status, 200);
        let miss_payload: serde_json::Value = serde_json::from_slice(&miss_response.body).unwrap();
        assert_eq!(miss_payload.get("count").and_then(|v| v.as_u64()), Some(0));
        assert_eq!(
            miss_payload
                .get("calls")
                .and_then(|v| v.as_array())
                .map(|items| items.len()),
            Some(0)
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
    fn route_table_wiring_preserves_handler_surface() {
        let routes = build_route_table(Arc::new(StubRuntime));

        let get = HttpRequest {
            method: "GET".to_string(),
            path: "/health".to_string(),
            headers: vec![],
            body: vec![],
        };
        assert_eq!((routes.get_health)(&get).status, 200);
        assert_eq!((routes.get_version)(&get).status, 200);

        let lifecycle_request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/lifecycle/load-pando".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({"name": "demo"})).unwrap(),
        };
        assert_eq!(
            (routes.post_lifecycle_load_pando)(&lifecycle_request).status,
            200
        );

        let rivet_request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/rivet/dispatch".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({
                "child": "folder-watch-actor",
                "operation_id": "patina:watch/control.status",
                "args": []
            }))
            .unwrap(),
        };
        assert_eq!((routes.post_rivet_dispatch)(&rivet_request).status, 200);

        let inspector_request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/inspector/typed-calls".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({"limit": 10})).unwrap(),
        };
        assert_eq!(
            (routes.post_inspector_typed_calls)(&inspector_request).status,
            200
        );

        let child_request = HttpRequest {
            method: "POST".to_string(),
            path: "/child/folder-watch-actor/call".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({
                "operation_id": "patina:watch/control.status",
                "args": []
            }))
            .unwrap(),
        };
        assert_eq!((routes.child_request)(&child_request).status, 200);
    }

    #[test]
    fn lifecycle_warmup_maps_resource_exhausted_to_429_envelope() {
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
                    startup_profile: "core".to_string(),
                    rivet_integration: "disabled".to_string(),
                    child_warmup: ChildWarmupState {
                        mode: "manual".to_string(),
                        state: "pending".to_string(),
                        last_error: None,
                    },
                    memory: MemoryStatus {
                        rss_bytes: Some(32 * 1024 * 1024),
                        max_rss_bytes: Some(64 * 1024 * 1024),
                        soft_limit_bytes: Some(16 * 1024 * 1024),
                        pressure: "high".to_string(),
                    },
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
                correlation: Option<crate::CallCorrelation>,
            ) -> Result<serde_json::Value> {
                Ok(serde_json::json!({
                    "operation_id": operation_id,
                    "args": args,
                    "correlation": correlation,
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
            fn lifecycle_warmup_children(&self) -> Result<crate::ChildWarmupResult> {
                anyhow::bail!("resource_exhausted: memory pressure high; warmup denied")
            }
            fn rivet_dispatch(&self, _request: RivetDispatchRequest) -> Result<serde_json::Value> {
                anyhow::bail!("invalid_request: rivet integration is disabled")
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

        let warmup_request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/lifecycle/warmup-children".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({})).unwrap(),
        };
        let warmup_response = handle_lifecycle_warmup_children(&warmup_request, &BusyRuntime);
        assert_eq!(warmup_response.status, 429);
        let warmup_json: serde_json::Value = serde_json::from_slice(&warmup_response.body).unwrap();
        assert_eq!(
            warmup_json.get("error").and_then(|v| v.as_str()),
            Some("resource_exhausted")
        );

        let rivet_request = HttpRequest {
            method: "POST".to_string(),
            path: "/api/rivet/dispatch".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({
                "child": "folder-watch-actor",
                "operation_id": "patina:watch/control.status",
                "args": []
            }))
            .unwrap(),
        };
        let rivet_response = handle_rivet_dispatch(&rivet_request, &BusyRuntime);
        assert_eq!(rivet_response.status, 400);
        let rivet_json: serde_json::Value = serde_json::from_slice(&rivet_response.body).unwrap();
        assert_eq!(
            rivet_json.get("error").and_then(|v| v.as_str()),
            Some("invalid_request")
        );
    }
}
