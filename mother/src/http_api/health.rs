use serde::Serialize;

use super::*;

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
    startup_profile: String,
    rivet_integration: String,
    child_warmup: ChildWarmupStateJson,
    memory: MemoryStatusJson,
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
struct ChildWarmupStateJson {
    mode: String,
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_error: Option<String>,
}

#[derive(Serialize)]
struct MemoryStatusJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    rss_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_rss_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    soft_limit_bytes: Option<u64>,
    pressure: String,
}

#[derive(Serialize)]
struct ChildHealthJson {
    name: String,
    status: String,
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
        startup_profile: "unknown".to_string(),
        rivet_integration: "disabled".to_string(),
        child_warmup: ChildWarmupState {
            mode: "unknown".to_string(),
            state: "unknown".to_string(),
            last_error: Some("health details unavailable".to_string()),
        },
        memory: MemoryStatus {
            rss_bytes: None,
            max_rss_bytes: None,
            soft_limit_bytes: None,
            pressure: "unknown".to_string(),
        },
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
    let child_warmup = ChildWarmupStateJson {
        mode: details.child_warmup.mode.clone(),
        state: details.child_warmup.state.clone(),
        last_error: details.child_warmup.last_error.clone(),
    };
    let memory = MemoryStatusJson {
        rss_bytes: details.memory.rss_bytes,
        max_rss_bytes: details.memory.max_rss_bytes,
        soft_limit_bytes: details.memory.soft_limit_bytes,
        pressure: details.memory.pressure.clone(),
    };

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
            startup_profile: details.startup_profile,
            rivet_integration: details.rivet_integration,
            child_warmup,
            memory,
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
