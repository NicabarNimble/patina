use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::http_daemon::{json_error, HttpRequest, HttpResponse};
use crate::http_routes::RouteTable;

const MAX_LIMIT: usize = 1000;

pub type BuiltinChildHandler = dyn Fn(&str, &str, &[u8]) -> Option<HttpResponse> + Send + Sync;

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
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_secs: u64,
    children: Vec<ChildHealthJson>,
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

    HttpResponse::json(
        200,
        &HealthResponse {
            status: "ok".to_string(),
            version: runtime.version(),
            uptime_secs: runtime.uptime_secs(),
            children,
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
    match runtime.child_handle("secrets", "get".into(), serde_json::Value::Null) {
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

    match runtime.child_handle("secrets", "cache".into(), payload) {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(e) => json_error(500, &format!("Cache failed: {}", e)),
    }
}

pub fn handle_secrets_lock(runtime: &dyn ApiRuntime) -> HttpResponse {
    match runtime.child_handle("secrets", "lock".into(), serde_json::Value::Null) {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(e) => json_error(500, &format!("Lock failed: {}", e)),
    }
}

pub fn handle_child_request(
    request: &HttpRequest,
    runtime: &dyn ApiRuntime,
    builtin_handler: &BuiltinChildHandler,
) -> HttpResponse {
    let parts: Vec<&str> = request.path[1..].split('/').collect();
    if parts.len() != 3 {
        return json_error(400, "Expected /child/{name}/{action}");
    }
    let child_name = parts[1];
    let action = parts[2];

    if let Some(response) = builtin_handler(child_name, action, &request.body) {
        return response;
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

pub fn build_route_table(
    runtime: Arc<dyn ApiRuntime + Send + Sync>,
    builtin_handler: Arc<BuiltinChildHandler>,
) -> RouteTable {
    let health_runtime = Arc::clone(&runtime);
    let version_runtime = Arc::clone(&runtime);
    let scry_runtime = Arc::clone(&runtime);
    let secrets_get_runtime = Arc::clone(&runtime);
    let secrets_cache_runtime = Arc::clone(&runtime);
    let secrets_lock_runtime = Arc::clone(&runtime);
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
        child_request: Arc::new(move |request| {
            handle_child_request(request, &*child_runtime, builtin_handler.as_ref())
        }),
    }
}
