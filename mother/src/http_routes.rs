use std::sync::Arc;
use subtle::ConstantTimeEq;

use crate::http_daemon::{json_error, with_security_headers, HttpRequest, HttpResponse};

type RouteHandler = Arc<dyn Fn(&HttpRequest) -> HttpResponse + Send + Sync>;

pub struct RouteTable {
    pub get_health: RouteHandler,
    pub get_ready: RouteHandler,
    pub get_version: RouteHandler,
    pub post_bridge_translate: RouteHandler,
    pub post_scry: RouteHandler,
    pub post_federation_status: RouteHandler,
    pub post_federation_refresh: RouteHandler,
    pub post_federation_query: RouteHandler,
    pub get_secrets_cache: RouteHandler,
    pub post_secrets_cache: RouteHandler,
    pub post_secrets_lock: RouteHandler,
    pub post_pando_registry_init: RouteHandler,
    pub get_pando_list: RouteHandler,
    pub post_lifecycle_load_pando: RouteHandler,
    pub post_lifecycle_refresh: RouteHandler,
    pub post_lifecycle_reload_child: RouteHandler,
    pub post_lifecycle_warmup_children: RouteHandler,
    pub post_interface_call: RouteHandler,
    pub post_rivet_dispatch: RouteHandler,
    pub post_inspector_typed_calls: RouteHandler,
    pub get_view_shapes: RouteHandler,
    pub get_view_shape: RouteHandler,
    pub post_view_shape_upsert: RouteHandler,
    pub post_view_shape_deactivate: RouteHandler,
    pub get_view_shape_revisions: RouteHandler,
    pub get_view_shape_revision: RouteHandler,
    pub post_view_shape_revise: RouteHandler,
    pub get_view_requests: RouteHandler,
    pub get_view_request: RouteHandler,
    pub get_view_request_details: RouteHandler,
    pub get_view_request_detail: RouteHandler,
    pub post_view_request_compose: RouteHandler,
    pub post_view_request_open_shape: RouteHandler,
    pub get_view_buffers: RouteHandler,
    pub post_view_buffer_open: RouteHandler,
    pub post_view_buffer_connect: RouteHandler,
    pub post_view_buffer_disconnect: RouteHandler,
    pub post_view_buffer_kill: RouteHandler,
    pub get_view_buffer_windows: RouteHandler,
    pub get_view_buffer_gaps: RouteHandler,
    pub child_request: RouteHandler,
}

pub struct Router {
    require_auth: bool,
    token: String,
    routes: RouteTable,
}

impl Router {
    pub fn new(require_auth: bool, token: String, routes: RouteTable) -> Self {
        Self {
            require_auth,
            token,
            routes,
        }
    }

    pub fn route(&self, request: &HttpRequest) -> HttpResponse {
        let response = match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/health") => (self.routes.get_health)(request),
            ("GET", "/ready") => (self.routes.get_ready)(request),
            ("GET", "/version") => (self.routes.get_version)(request),
            ("POST", "/api/bridge/translate") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_bridge_translate)(request)
                }
            }
            ("POST", "/api/scry") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_scry)(request)
                }
            }
            ("POST", "/api/federation/status") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_federation_status)(request)
                }
            }
            ("POST", "/api/federation/refresh") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_federation_refresh)(request)
                }
            }
            ("POST", "/api/federation/query") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_federation_query)(request)
                }
            }
            ("GET", "/secrets/cache") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_secrets_cache)(request)
                }
            }
            ("POST", "/secrets/cache") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_secrets_cache)(request)
                }
            }
            ("POST", "/secrets/lock") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_secrets_lock)(request)
                }
            }
            ("POST", "/api/pando/registry/init") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_pando_registry_init)(request)
                }
            }
            ("GET", "/api/pando/list") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_pando_list)(request)
                }
            }
            ("POST", "/api/lifecycle/load-pando") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_lifecycle_load_pando)(request)
                }
            }
            ("POST", "/api/lifecycle/refresh") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_lifecycle_refresh)(request)
                }
            }
            ("POST", "/api/lifecycle/reload-child") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_lifecycle_reload_child)(request)
                }
            }
            ("POST", "/api/lifecycle/warmup-children") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_lifecycle_warmup_children)(request)
                }
            }
            ("POST", "/api/interface/call") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_interface_call)(request)
                }
            }
            ("POST", "/api/rivet/dispatch") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_rivet_dispatch)(request)
                }
            }
            ("POST", "/api/inspector/typed-calls") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_inspector_typed_calls)(request)
                }
            }
            ("GET", "/api/view-shapes") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_view_shapes)(request)
                }
            }
            ("GET", path) if path.starts_with("/api/view-shapes/") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_view_shape)(request)
                }
            }
            ("POST", "/api/view-shapes/upsert") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_view_shape_upsert)(request)
                }
            }
            ("POST", "/api/view-shapes/deactivate") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_view_shape_deactivate)(request)
                }
            }
            ("POST", "/api/view-shapes/revise") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_view_shape_revise)(request)
                }
            }
            ("GET", "/api/view-shape-revisions") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_view_shape_revisions)(request)
                }
            }
            ("GET", path) if path.starts_with("/api/view-shape-revisions/") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_view_shape_revision)(request)
                }
            }
            ("GET", "/api/view-requests") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_view_requests)(request)
                }
            }
            ("GET", "/api/view-requests/details") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_view_request_details)(request)
                }
            }
            ("GET", path)
                if path.starts_with("/api/view-requests/") && path.ends_with("/detail") =>
            {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_view_request_detail)(request)
                }
            }
            ("GET", path) if path.starts_with("/api/view-requests/") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_view_request)(request)
                }
            }
            ("POST", "/api/view-requests/compose") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_view_request_compose)(request)
                }
            }
            ("POST", "/api/view-requests/open-shape") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_view_request_open_shape)(request)
                }
            }
            ("GET", "/api/view-buffers") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_view_buffers)(request)
                }
            }
            ("POST", "/api/view-buffers/open") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_view_buffer_open)(request)
                }
            }
            ("POST", "/api/view-buffers/connect") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_view_buffer_connect)(request)
                }
            }
            ("POST", "/api/view-buffers/disconnect") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_view_buffer_disconnect)(request)
                }
            }
            ("POST", "/api/view-buffers/kill") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_view_buffer_kill)(request)
                }
            }
            ("GET", "/api/view-buffers/windows") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_view_buffer_windows)(request)
                }
            }
            ("GET", "/api/view-buffers/gaps") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_view_buffer_gaps)(request)
                }
            }
            _ if request.path.starts_with("/child/") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.child_request)(request)
                }
            }
            _ => json_error(404, "Not found"),
        };

        with_security_headers(response)
    }

    fn check_auth(&self, request: &HttpRequest) -> bool {
        let expected = format!("Bearer {}", self.token);
        request
            .header("Authorization")
            .map(|header| header.as_bytes().ct_eq(expected.as_bytes()).into())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_json() -> HttpResponse {
        HttpResponse::json(200, &serde_json::json!({"ok": true}))
    }

    fn test_routes() -> RouteTable {
        RouteTable {
            get_health: Arc::new(|_| ok_json()),
            get_ready: Arc::new(|_| ok_json()),
            get_version: Arc::new(|_| ok_json()),
            post_bridge_translate: Arc::new(|_| ok_json()),
            post_scry: Arc::new(|_| ok_json()),
            post_federation_status: Arc::new(|_| ok_json()),
            post_federation_refresh: Arc::new(|_| ok_json()),
            post_federation_query: Arc::new(|_| ok_json()),
            get_secrets_cache: Arc::new(|_| ok_json()),
            post_secrets_cache: Arc::new(|_| ok_json()),
            post_secrets_lock: Arc::new(|_| ok_json()),
            post_pando_registry_init: Arc::new(|_| ok_json()),
            get_pando_list: Arc::new(|_| ok_json()),
            post_lifecycle_load_pando: Arc::new(|_| ok_json()),
            post_lifecycle_refresh: Arc::new(|_| ok_json()),
            post_lifecycle_reload_child: Arc::new(|_| ok_json()),
            post_lifecycle_warmup_children: Arc::new(|_| ok_json()),
            post_interface_call: Arc::new(|_| ok_json()),
            post_rivet_dispatch: Arc::new(|_| ok_json()),
            post_inspector_typed_calls: Arc::new(|_| ok_json()),
            get_view_shapes: Arc::new(|_| ok_json()),
            get_view_shape: Arc::new(|_| ok_json()),
            post_view_shape_upsert: Arc::new(|_| ok_json()),
            post_view_shape_deactivate: Arc::new(|_| ok_json()),
            get_view_shape_revisions: Arc::new(|_| ok_json()),
            get_view_shape_revision: Arc::new(|_| ok_json()),
            post_view_shape_revise: Arc::new(|_| ok_json()),
            get_view_requests: Arc::new(|_| ok_json()),
            get_view_request: Arc::new(|_| ok_json()),
            get_view_request_details: Arc::new(|_| ok_json()),
            get_view_request_detail: Arc::new(|_| ok_json()),
            post_view_request_compose: Arc::new(|_| ok_json()),
            post_view_request_open_shape: Arc::new(|_| ok_json()),
            get_view_buffers: Arc::new(|_| ok_json()),
            post_view_buffer_open: Arc::new(|_| ok_json()),
            post_view_buffer_connect: Arc::new(|_| ok_json()),
            post_view_buffer_disconnect: Arc::new(|_| ok_json()),
            post_view_buffer_kill: Arc::new(|_| ok_json()),
            get_view_buffer_windows: Arc::new(|_| ok_json()),
            get_view_buffer_gaps: Arc::new(|_| ok_json()),
            child_request: Arc::new(|_| ok_json()),
        }
    }

    fn request(method: &str, path: &str, auth: Option<&str>) -> HttpRequest {
        HttpRequest {
            method: method.to_string(),
            path: path.to_string(),
            headers: auth
                .map(|value| vec![("Authorization".to_string(), value.to_string())])
                .unwrap_or_default(),
            body: vec![],
        }
    }

    #[test]
    fn removed_atlas_routes_are_not_wired() {
        let router = Router::new(true, "token-123".to_string(), test_routes());
        let header = "Bearer token-123";

        let html = router.route(&request("GET", "/atlas", Some(header)));
        assert_eq!(html.status, 404);

        let json = router.route(&request("GET", "/api/atlas/snapshot", Some(header)));
        assert_eq!(json.status, 404);
    }

    #[test]
    fn lifecycle_warmup_route_is_wired() {
        let router = Router::new(false, "token-123".to_string(), test_routes());
        let response = router.route(&request("POST", "/api/lifecycle/warmup-children", None));
        assert_eq!(response.status, 200);
    }

    #[test]
    fn view_buffer_routes_are_wired_and_auth_guarded() {
        let router = Router::new(true, "token-123".to_string(), test_routes());
        let unauthorized = router.route(&request("GET", "/api/view-buffers", None));
        assert_eq!(unauthorized.status, 401);

        for (method, path) in [
            ("GET", "/api/view-shapes"),
            ("GET", "/api/view-shapes/mother.status.default"),
            ("POST", "/api/view-shapes/upsert"),
            ("POST", "/api/view-shapes/deactivate"),
            ("POST", "/api/view-shapes/revise"),
            ("GET", "/api/view-shape-revisions"),
            ("GET", "/api/view-shape-revisions/rev_1"),
            ("GET", "/api/view-requests"),
            ("GET", "/api/view-requests/req_1"),
            ("GET", "/api/view-requests/details"),
            ("GET", "/api/view-requests/req_1/detail"),
            ("POST", "/api/view-requests/compose"),
            ("POST", "/api/view-requests/open-shape"),
            ("GET", "/api/view-buffers"),
            ("POST", "/api/view-buffers/open"),
            ("POST", "/api/view-buffers/connect"),
            ("POST", "/api/view-buffers/disconnect"),
            ("POST", "/api/view-buffers/kill"),
            ("GET", "/api/view-buffers/windows"),
            ("GET", "/api/view-buffers/gaps"),
        ] {
            let authorized = router.route(&request(method, path, Some("Bearer token-123")));
            assert_eq!(authorized.status, 200, "{method} {path} should route");
        }
    }

    #[test]
    fn ready_route_is_wired() {
        let router = Router::new(false, "token-123".to_string(), test_routes());
        let response = router.route(&request("GET", "/ready", None));
        assert_eq!(response.status, 200);
    }

    #[test]
    fn interface_call_route_is_wired() {
        let router = Router::new(false, "token-123".to_string(), test_routes());
        let response = router.route(&request("POST", "/api/interface/call", None));
        assert_eq!(response.status, 200);
    }

    #[test]
    fn rivet_dispatch_route_is_wired() {
        let router = Router::new(false, "token-123".to_string(), test_routes());
        let response = router.route(&request("POST", "/api/rivet/dispatch", None));
        assert_eq!(response.status, 200);
    }
}
