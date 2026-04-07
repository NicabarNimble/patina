use std::sync::Arc;
use subtle::ConstantTimeEq;

use crate::http_daemon::{json_error, with_security_headers, HttpRequest, HttpResponse};

type RouteHandler = Arc<dyn Fn(&HttpRequest) -> HttpResponse + Send + Sync>;

pub struct RouteTable {
    pub get_health: RouteHandler,
    pub get_version: RouteHandler,
    pub post_scry: RouteHandler,
    pub get_secrets_cache: RouteHandler,
    pub post_secrets_cache: RouteHandler,
    pub post_secrets_lock: RouteHandler,
    pub post_pando_registry_init: RouteHandler,
    pub get_pando_list: RouteHandler,
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
            ("GET", "/version") => (self.routes.get_version)(request),
            ("POST", "/api/scry") => {
                if self.require_auth && !self.check_auth(request) {
                    json_error(401, "Unauthorized")
                } else {
                    (self.routes.post_scry)(request)
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
