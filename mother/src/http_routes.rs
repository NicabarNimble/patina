use std::sync::Arc;
use subtle::ConstantTimeEq;

use crate::http_daemon::{json_error, with_security_headers, HttpRequest, HttpResponse};

type RouteHandler = Arc<dyn Fn(&HttpRequest) -> HttpResponse + Send + Sync>;

pub struct RouteTable {
    pub get_health: RouteHandler,
    pub get_ready: RouteHandler,
    pub get_version: RouteHandler,
    pub get_atlas_dashboard: RouteHandler,
    pub get_atlas_snapshot: RouteHandler,
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
            ("GET", "/atlas") | ("GET", "/atlas/index.html") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_atlas_dashboard)(request)
                }
            }
            ("GET", "/atlas/atlas.json") | ("GET", "/api/atlas/snapshot") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.get_atlas_snapshot)(request)
                }
            }
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
            get_atlas_dashboard: Arc::new(|_| HttpResponse {
                status: 200,
                headers: vec![("Content-Type".to_string(), "text/html".to_string())],
                body: b"<html>atlas</html>".to_vec(),
            }),
            get_atlas_snapshot: Arc::new(|_| ok_json()),
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
    fn atlas_routes_require_auth_when_router_requires_auth() {
        let router = Router::new(true, "token-123".to_string(), test_routes());

        let html = router.route(&request("GET", "/atlas", None));
        assert_eq!(html.status, 401);

        let json = router.route(&request("GET", "/atlas/atlas.json", None));
        assert_eq!(json.status, 401);
    }

    #[test]
    fn atlas_routes_allow_with_valid_auth_header() {
        let router = Router::new(true, "token-123".to_string(), test_routes());
        let header = "Bearer token-123";

        let html = router.route(&request("GET", "/atlas", Some(header)));
        assert_eq!(html.status, 200);

        let json = router.route(&request("GET", "/atlas/atlas.json", Some(header)));
        assert_eq!(json.status, 200);
    }

    #[test]
    fn lifecycle_warmup_route_is_wired() {
        let router = Router::new(false, "token-123".to_string(), test_routes());
        let response = router.route(&request("POST", "/api/lifecycle/warmup-children", None));
        assert_eq!(response.status, 200);
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
