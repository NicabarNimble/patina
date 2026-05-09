use anyhow::Result;
use std::sync::Arc;

use crate::http_daemon::{json_error, HttpRequest, HttpResponse};
use crate::http_routes::RouteTable;

mod bridge;
mod child;
mod federation;
mod health;
mod inspector;
mod interface_control;
mod lifecycle;
mod pando;
mod rivet;
mod scry;
mod secrets;
mod view_buffer;

pub use child::handle_child_request;
pub use health::{handle_health, handle_ready, handle_version};
pub use inspector::handle_inspector_typed_calls;
pub use interface_control::{
    handle_interface_control_call, InterfaceControlCallRequest, InterfaceControlCorrelation,
};
pub use lifecycle::{
    handle_lifecycle_load_pando, handle_lifecycle_refresh, handle_lifecycle_reload_child,
    handle_lifecycle_warmup_children, LifecycleError,
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
    fn ready_status(&self) -> Result<bool>;
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
    fn interface_control_call(
        &self,
        request: InterfaceControlCallRequest,
    ) -> Result<serde_json::Value>;
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
    fn view_shapes_list(&self) -> Result<Vec<crate::view_buffer::ViewShape>>;
    fn view_shape_get(&self, shape_id: &str) -> Result<Option<crate::view_buffer::ViewShape>>;
    fn view_shape_upsert(
        &self,
        shape: crate::view_buffer::ViewShape,
    ) -> Result<crate::view_buffer::ViewShape>;
    fn view_shape_deactivate(&self, shape_id: &str) -> Result<bool>;
    fn view_requests_list(&self) -> Result<Vec<crate::view_buffer::DisplayRequest>>;
    fn view_request_get(
        &self,
        request_id: &str,
    ) -> Result<Option<crate::view_buffer::DisplayRequest>>;
    fn view_request_details_list(&self) -> Result<Vec<crate::view_buffer::ViewRequestDetail>>;
    fn view_request_detail_get(
        &self,
        request_id: &str,
    ) -> Result<Option<crate::view_buffer::ViewRequestDetail>>;
    fn view_request_compose(
        &self,
        request: crate::view_buffer::ComposeViewRequest,
    ) -> Result<crate::view_buffer::ComposedViewRequest>;
    fn view_request_open_shape(
        &self,
        request: crate::view_buffer::OpenRequestShapeRequest,
    ) -> Result<Option<crate::view_buffer::OpenRequestShapeOutcome>>;
    fn view_buffers_list(&self) -> Result<Vec<crate::view_buffer::Buffer>>;
    fn view_buffer_open(
        &self,
        request: crate::view_buffer::OpenBufferRequest,
    ) -> Result<crate::view_buffer::OpenBufferOutcome>;
    fn view_buffer_connect_window(
        &self,
        request: crate::view_buffer::ConnectWindowRequest,
    ) -> Result<crate::view_buffer::Window>;
    fn view_buffer_disconnect_window(
        &self,
        request: crate::view_buffer::DisconnectWindowRequest,
    ) -> Result<crate::view_buffer::Window>;
    fn view_buffer_kill(
        &self,
        request: crate::view_buffer::KillBufferRequest,
    ) -> Result<crate::view_buffer::Buffer>;
    fn view_buffer_windows_list(&self) -> Result<Vec<crate::view_buffer::Window>>;
    fn view_buffer_gaps_list(&self) -> Result<Vec<crate::view_buffer::ObservabilityGap>>;
}

pub trait HealthApi {
    fn version(&self) -> String;
    fn uptime_secs(&self) -> u64;
    fn ready_status(&self) -> Result<bool>;
    fn health_all(&self) -> Vec<(String, crate::ChildHealth)>;
    fn health_details(&self) -> Result<HealthDetails>;
}

impl<T: ApiRuntime + ?Sized> HealthApi for T {
    fn version(&self) -> String {
        ApiRuntime::version(self)
    }

    fn uptime_secs(&self) -> u64 {
        ApiRuntime::uptime_secs(self)
    }

    fn ready_status(&self) -> Result<bool> {
        ApiRuntime::ready_status(self)
    }

    fn health_all(&self) -> Vec<(String, crate::ChildHealth)> {
        ApiRuntime::health_all(self)
    }

    fn health_details(&self) -> Result<HealthDetails> {
        ApiRuntime::health_details(self)
    }
}

pub trait BridgeApi {
    fn bridge_translate(
        &self,
        request: crate::bridge::BridgeRequest,
    ) -> Result<crate::bridge::BridgeResponse>;
}

impl<T: ApiRuntime + ?Sized> BridgeApi for T {
    fn bridge_translate(
        &self,
        request: crate::bridge::BridgeRequest,
    ) -> Result<crate::bridge::BridgeResponse> {
        ApiRuntime::bridge_translate(self, request)
    }
}

pub trait ScryApi {
    fn scry_query(
        &self,
        query: &str,
        limit: usize,
        repo: Option<String>,
        all_repos: bool,
    ) -> Result<Vec<ScryHit>>;
}

impl<T: ApiRuntime + ?Sized> ScryApi for T {
    fn scry_query(
        &self,
        query: &str,
        limit: usize,
        repo: Option<String>,
        all_repos: bool,
    ) -> Result<Vec<ScryHit>> {
        ApiRuntime::scry_query(self, query, limit, repo, all_repos)
    }
}

pub trait FederationApi {
    fn federation_status(&self) -> Result<serde_json::Value>;
    fn federation_refresh(&self) -> Result<serde_json::Value>;
    fn federation_query(
        &self,
        payload: crate::protocol::FederationQueryPayload,
    ) -> Result<serde_json::Value>;
}

impl<T: ApiRuntime + ?Sized> FederationApi for T {
    fn federation_status(&self) -> Result<serde_json::Value> {
        ApiRuntime::federation_status(self)
    }

    fn federation_refresh(&self) -> Result<serde_json::Value> {
        ApiRuntime::federation_refresh(self)
    }

    fn federation_query(
        &self,
        payload: crate::protocol::FederationQueryPayload,
    ) -> Result<serde_json::Value> {
        ApiRuntime::federation_query(self, payload)
    }
}

pub trait SecretsApi {
    fn secrets_get(&self) -> Result<serde_json::Value>;
    fn secrets_cache(&self, payload: serde_json::Value) -> Result<serde_json::Value>;
    fn secrets_lock(&self) -> Result<serde_json::Value>;
}

impl<T: ApiRuntime + ?Sized> SecretsApi for T {
    fn secrets_get(&self) -> Result<serde_json::Value> {
        ApiRuntime::secrets_get(self)
    }

    fn secrets_cache(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
        ApiRuntime::secrets_cache(self, payload)
    }

    fn secrets_lock(&self) -> Result<serde_json::Value> {
        ApiRuntime::secrets_lock(self)
    }
}

pub trait PandoRegistryApi {
    fn pando_registry_init(
        &self,
        request: patina_protocol::PandoRegistryInit,
    ) -> Result<patina_protocol::PandoRegistryState>;
    fn pando_list(&self) -> Result<patina_protocol::PandoRegistryState>;
}

impl<T: ApiRuntime + ?Sized> PandoRegistryApi for T {
    fn pando_registry_init(
        &self,
        request: patina_protocol::PandoRegistryInit,
    ) -> Result<patina_protocol::PandoRegistryState> {
        ApiRuntime::pando_registry_init(self, request)
    }

    fn pando_list(&self) -> Result<patina_protocol::PandoRegistryState> {
        ApiRuntime::pando_list(self)
    }
}

pub trait LifecycleApi {
    fn lifecycle_load_pando(&self, name: &str) -> Result<crate::PandoLoadResult>;
    fn lifecycle_refresh(&self) -> Result<crate::PandoRefreshResult>;
    fn lifecycle_reload_child(&self, name: &str) -> Result<crate::ChildReloadResult>;
    fn lifecycle_warmup_children(&self) -> Result<crate::ChildWarmupResult>;
}

impl<T: ApiRuntime + ?Sized> LifecycleApi for T {
    fn lifecycle_load_pando(&self, name: &str) -> Result<crate::PandoLoadResult> {
        ApiRuntime::lifecycle_load_pando(self, name)
    }

    fn lifecycle_refresh(&self) -> Result<crate::PandoRefreshResult> {
        ApiRuntime::lifecycle_refresh(self)
    }

    fn lifecycle_reload_child(&self, name: &str) -> Result<crate::ChildReloadResult> {
        ApiRuntime::lifecycle_reload_child(self, name)
    }

    fn lifecycle_warmup_children(&self) -> Result<crate::ChildWarmupResult> {
        ApiRuntime::lifecycle_warmup_children(self)
    }
}

pub trait RivetApi {
    fn rivet_dispatch(&self, request: RivetDispatchRequest) -> Result<serde_json::Value>;
}

impl<T: ApiRuntime + ?Sized> RivetApi for T {
    fn rivet_dispatch(&self, request: RivetDispatchRequest) -> Result<serde_json::Value> {
        ApiRuntime::rivet_dispatch(self, request)
    }
}

pub trait InterfaceControlApi {
    fn interface_control_call(
        &self,
        request: InterfaceControlCallRequest,
    ) -> Result<serde_json::Value>;
}

impl<T: ApiRuntime + ?Sized> InterfaceControlApi for T {
    fn interface_control_call(
        &self,
        request: InterfaceControlCallRequest,
    ) -> Result<serde_json::Value> {
        ApiRuntime::interface_control_call(self, request)
    }
}

pub trait InspectorApi {
    fn typed_call_history(&self, limit: usize) -> Result<serde_json::Value>;
}

impl<T: ApiRuntime + ?Sized> InspectorApi for T {
    fn typed_call_history(&self, limit: usize) -> Result<serde_json::Value> {
        ApiRuntime::typed_call_history(self, limit)
    }
}

pub trait ViewBufferApi {
    fn view_shapes_list(&self) -> Result<Vec<crate::view_buffer::ViewShape>>;
    fn view_shape_get(&self, shape_id: &str) -> Result<Option<crate::view_buffer::ViewShape>>;
    fn view_shape_upsert(
        &self,
        shape: crate::view_buffer::ViewShape,
    ) -> Result<crate::view_buffer::ViewShape>;
    fn view_shape_deactivate(&self, shape_id: &str) -> Result<bool>;
    fn view_requests_list(&self) -> Result<Vec<crate::view_buffer::DisplayRequest>>;
    fn view_request_get(
        &self,
        request_id: &str,
    ) -> Result<Option<crate::view_buffer::DisplayRequest>>;
    fn view_request_details_list(&self) -> Result<Vec<crate::view_buffer::ViewRequestDetail>>;
    fn view_request_detail_get(
        &self,
        request_id: &str,
    ) -> Result<Option<crate::view_buffer::ViewRequestDetail>>;
    fn view_request_compose(
        &self,
        request: crate::view_buffer::ComposeViewRequest,
    ) -> Result<crate::view_buffer::ComposedViewRequest>;
    fn view_request_open_shape(
        &self,
        request: crate::view_buffer::OpenRequestShapeRequest,
    ) -> Result<Option<crate::view_buffer::OpenRequestShapeOutcome>>;
    fn view_buffers_list(&self) -> Result<Vec<crate::view_buffer::Buffer>>;
    fn view_buffer_open(
        &self,
        request: crate::view_buffer::OpenBufferRequest,
    ) -> Result<crate::view_buffer::OpenBufferOutcome>;
    fn view_buffer_connect_window(
        &self,
        request: crate::view_buffer::ConnectWindowRequest,
    ) -> Result<crate::view_buffer::Window>;
    fn view_buffer_disconnect_window(
        &self,
        request: crate::view_buffer::DisconnectWindowRequest,
    ) -> Result<crate::view_buffer::Window>;
    fn view_buffer_kill(
        &self,
        request: crate::view_buffer::KillBufferRequest,
    ) -> Result<crate::view_buffer::Buffer>;
    fn view_buffer_windows_list(&self) -> Result<Vec<crate::view_buffer::Window>>;
    fn view_buffer_gaps_list(&self) -> Result<Vec<crate::view_buffer::ObservabilityGap>>;
}

impl<T: ApiRuntime + ?Sized> ViewBufferApi for T {
    fn view_shapes_list(&self) -> Result<Vec<crate::view_buffer::ViewShape>> {
        ApiRuntime::view_shapes_list(self)
    }

    fn view_shape_get(&self, shape_id: &str) -> Result<Option<crate::view_buffer::ViewShape>> {
        ApiRuntime::view_shape_get(self, shape_id)
    }

    fn view_shape_upsert(
        &self,
        shape: crate::view_buffer::ViewShape,
    ) -> Result<crate::view_buffer::ViewShape> {
        ApiRuntime::view_shape_upsert(self, shape)
    }

    fn view_shape_deactivate(&self, shape_id: &str) -> Result<bool> {
        ApiRuntime::view_shape_deactivate(self, shape_id)
    }

    fn view_requests_list(&self) -> Result<Vec<crate::view_buffer::DisplayRequest>> {
        ApiRuntime::view_requests_list(self)
    }

    fn view_request_get(
        &self,
        request_id: &str,
    ) -> Result<Option<crate::view_buffer::DisplayRequest>> {
        ApiRuntime::view_request_get(self, request_id)
    }

    fn view_request_details_list(&self) -> Result<Vec<crate::view_buffer::ViewRequestDetail>> {
        ApiRuntime::view_request_details_list(self)
    }

    fn view_request_detail_get(
        &self,
        request_id: &str,
    ) -> Result<Option<crate::view_buffer::ViewRequestDetail>> {
        ApiRuntime::view_request_detail_get(self, request_id)
    }

    fn view_request_compose(
        &self,
        request: crate::view_buffer::ComposeViewRequest,
    ) -> Result<crate::view_buffer::ComposedViewRequest> {
        ApiRuntime::view_request_compose(self, request)
    }

    fn view_request_open_shape(
        &self,
        request: crate::view_buffer::OpenRequestShapeRequest,
    ) -> Result<Option<crate::view_buffer::OpenRequestShapeOutcome>> {
        ApiRuntime::view_request_open_shape(self, request)
    }

    fn view_buffers_list(&self) -> Result<Vec<crate::view_buffer::Buffer>> {
        ApiRuntime::view_buffers_list(self)
    }

    fn view_buffer_open(
        &self,
        request: crate::view_buffer::OpenBufferRequest,
    ) -> Result<crate::view_buffer::OpenBufferOutcome> {
        ApiRuntime::view_buffer_open(self, request)
    }

    fn view_buffer_connect_window(
        &self,
        request: crate::view_buffer::ConnectWindowRequest,
    ) -> Result<crate::view_buffer::Window> {
        ApiRuntime::view_buffer_connect_window(self, request)
    }

    fn view_buffer_disconnect_window(
        &self,
        request: crate::view_buffer::DisconnectWindowRequest,
    ) -> Result<crate::view_buffer::Window> {
        ApiRuntime::view_buffer_disconnect_window(self, request)
    }

    fn view_buffer_kill(
        &self,
        request: crate::view_buffer::KillBufferRequest,
    ) -> Result<crate::view_buffer::Buffer> {
        ApiRuntime::view_buffer_kill(self, request)
    }

    fn view_buffer_windows_list(&self) -> Result<Vec<crate::view_buffer::Window>> {
        ApiRuntime::view_buffer_windows_list(self)
    }

    fn view_buffer_gaps_list(&self) -> Result<Vec<crate::view_buffer::ObservabilityGap>> {
        ApiRuntime::view_buffer_gaps_list(self)
    }
}

pub trait ChildApi {
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

impl<T: ApiRuntime + ?Sized> ChildApi for T {
    fn child_health(&self, child_name: &str) -> Result<crate::ChildHealth> {
        ApiRuntime::child_health(self, child_name)
    }

    fn child_handle(
        &self,
        child_name: &str,
        action: String,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        ApiRuntime::child_handle(self, child_name, action, payload)
    }

    fn child_call(
        &self,
        child_name: &str,
        operation_id: String,
        args: serde_json::Value,
        correlation: Option<crate::CallCorrelation>,
    ) -> Result<serde_json::Value> {
        ApiRuntime::child_call(self, child_name, operation_id, args, correlation)
    }

    fn builtin_spec_dispatch(
        &self,
        request: patina_protocol::SpecDispatchRequest,
    ) -> Result<serde_json::Value> {
        ApiRuntime::builtin_spec_dispatch(self, request)
    }

    fn builtin_lake_dispatch(
        &self,
        request: patina_protocol::LakeDispatchRequest,
    ) -> Result<serde_json::Value> {
        ApiRuntime::builtin_lake_dispatch(self, request)
    }

    fn builtin_doctor_run(&self) -> Result<patina_protocol::DoctorRunResult> {
        ApiRuntime::builtin_doctor_run(self)
    }

    fn builtin_secrets_dispatch(&self, payload: serde_json::Value) -> HttpResponse {
        ApiRuntime::builtin_secrets_dispatch(self, payload)
    }
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

pub fn build_route_table(runtime: Arc<dyn ApiRuntime + Send + Sync>) -> RouteTable {
    let health_runtime = Arc::clone(&runtime);
    let ready_runtime = Arc::clone(&runtime);
    let version_runtime = Arc::clone(&runtime);
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
    let interface_control_runtime = Arc::clone(&runtime);
    let rivet_dispatch_runtime = Arc::clone(&runtime);
    let inspector_typed_calls_runtime = Arc::clone(&runtime);
    let view_shapes_list_runtime = Arc::clone(&runtime);
    let view_shape_get_runtime = Arc::clone(&runtime);
    let view_shape_upsert_runtime = Arc::clone(&runtime);
    let view_shape_deactivate_runtime = Arc::clone(&runtime);
    let view_requests_list_runtime = Arc::clone(&runtime);
    let view_request_get_runtime = Arc::clone(&runtime);
    let view_request_details_list_runtime = Arc::clone(&runtime);
    let view_request_detail_get_runtime = Arc::clone(&runtime);
    let view_request_compose_runtime = Arc::clone(&runtime);
    let view_request_open_shape_runtime = Arc::clone(&runtime);
    let view_buffers_list_runtime = Arc::clone(&runtime);
    let view_buffer_open_runtime = Arc::clone(&runtime);
    let view_buffer_connect_runtime = Arc::clone(&runtime);
    let view_buffer_disconnect_runtime = Arc::clone(&runtime);
    let view_buffer_kill_runtime = Arc::clone(&runtime);
    let view_buffer_windows_runtime = Arc::clone(&runtime);
    let view_buffer_gaps_runtime = Arc::clone(&runtime);
    let child_runtime = Arc::clone(&runtime);

    RouteTable {
        get_health: Arc::new(move |_request| health::handle_health(&*health_runtime)),
        get_ready: Arc::new(move |_request| health::handle_ready(&*ready_runtime)),
        get_version: Arc::new(move |_request| health::handle_version(&*version_runtime)),
        post_bridge_translate: Arc::new(move |request| {
            bridge::handle_bridge_translate(request, &*bridge_runtime)
        }),
        post_scry: Arc::new(move |request| scry::handle_scry(request, &*scry_runtime)),
        post_federation_status: Arc::new(move |request| {
            federation::handle_federation_status(request, &*federation_status_runtime)
        }),
        post_federation_refresh: Arc::new(move |request| {
            federation::handle_federation_refresh(request, &*federation_refresh_runtime)
        }),
        post_federation_query: Arc::new(move |request| {
            federation::handle_federation_query(request, &*federation_query_runtime)
        }),
        get_secrets_cache: Arc::new(move |_request| {
            secrets::handle_secrets_get(&*secrets_get_runtime)
        }),
        post_secrets_cache: Arc::new(move |request| {
            secrets::handle_secrets_cache(request, &*secrets_cache_runtime)
        }),
        post_secrets_lock: Arc::new(move |_request| {
            secrets::handle_secrets_lock(&*secrets_lock_runtime)
        }),
        post_pando_registry_init: Arc::new(move |request| {
            pando::handle_pando_registry_init(request, &*pando_registry_runtime)
        }),
        get_pando_list: Arc::new(move |_request| pando::handle_pando_list(&*pando_list_runtime)),
        post_lifecycle_load_pando: Arc::new(move |request| {
            lifecycle::handle_lifecycle_load_pando(request, &*lifecycle_load_runtime)
        }),
        post_lifecycle_refresh: Arc::new(move |request| {
            lifecycle::handle_lifecycle_refresh(request, &*lifecycle_refresh_runtime)
        }),
        post_lifecycle_reload_child: Arc::new(move |request| {
            lifecycle::handle_lifecycle_reload_child(request, &*lifecycle_reload_runtime)
        }),
        post_lifecycle_warmup_children: Arc::new(move |request| {
            lifecycle::handle_lifecycle_warmup_children(request, &*lifecycle_warmup_runtime)
        }),
        post_interface_call: Arc::new(move |request| {
            interface_control::handle_interface_control_call(request, &*interface_control_runtime)
        }),
        post_rivet_dispatch: Arc::new(move |request| {
            rivet::handle_rivet_dispatch(request, &*rivet_dispatch_runtime)
        }),
        post_inspector_typed_calls: Arc::new(move |request| {
            inspector::handle_inspector_typed_calls(request, &*inspector_typed_calls_runtime)
        }),
        get_view_shapes: Arc::new(move |_request| {
            view_buffer::handle_list_view_shapes(&*view_shapes_list_runtime)
        }),
        get_view_shape: Arc::new(move |request| {
            view_buffer::handle_get_view_shape(request, &*view_shape_get_runtime)
        }),
        post_view_shape_upsert: Arc::new(move |request| {
            view_buffer::handle_upsert_view_shape(request, &*view_shape_upsert_runtime)
        }),
        post_view_shape_deactivate: Arc::new(move |request| {
            view_buffer::handle_deactivate_view_shape(request, &*view_shape_deactivate_runtime)
        }),
        get_view_requests: Arc::new(move |_request| {
            view_buffer::handle_list_view_requests(&*view_requests_list_runtime)
        }),
        get_view_request: Arc::new(move |request| {
            view_buffer::handle_get_view_request(request, &*view_request_get_runtime)
        }),
        get_view_request_details: Arc::new(move |_request| {
            view_buffer::handle_list_view_request_details(&*view_request_details_list_runtime)
        }),
        get_view_request_detail: Arc::new(move |request| {
            view_buffer::handle_get_view_request_detail(request, &*view_request_detail_get_runtime)
        }),
        post_view_request_compose: Arc::new(move |request| {
            view_buffer::handle_compose_view_request(request, &*view_request_compose_runtime)
        }),
        post_view_request_open_shape: Arc::new(move |request| {
            view_buffer::handle_open_view_request_shape(request, &*view_request_open_shape_runtime)
        }),
        get_view_buffers: Arc::new(move |_request| {
            view_buffer::handle_list_view_buffers(&*view_buffers_list_runtime)
        }),
        post_view_buffer_open: Arc::new(move |request| {
            view_buffer::handle_open_view_buffer(request, &*view_buffer_open_runtime)
        }),
        post_view_buffer_connect: Arc::new(move |request| {
            view_buffer::handle_connect_view_buffer_window(request, &*view_buffer_connect_runtime)
        }),
        post_view_buffer_disconnect: Arc::new(move |request| {
            view_buffer::handle_disconnect_view_buffer_window(
                request,
                &*view_buffer_disconnect_runtime,
            )
        }),
        post_view_buffer_kill: Arc::new(move |request| {
            view_buffer::handle_kill_view_buffer(request, &*view_buffer_kill_runtime)
        }),
        get_view_buffer_windows: Arc::new(move |_request| {
            view_buffer::handle_list_view_buffer_windows(&*view_buffer_windows_runtime)
        }),
        get_view_buffer_gaps: Arc::new(move |_request| {
            view_buffer::handle_list_view_buffer_gaps(&*view_buffer_gaps_runtime)
        }),
        child_request: Arc::new(move |request| {
            child::handle_child_request(request, &*child_runtime)
        }),
    }
}

#[cfg(test)]
mod tests;
