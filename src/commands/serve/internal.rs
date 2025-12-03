//! Internal implementation of the Mothership server

use anyhow::Result;
use rouille::{router, Request, Response};
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;

use super::ServeOptions;

/// Server state shared across request handlers
pub struct ServerState {
    start_time: Instant,
    version: String,
}

impl ServerState {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_secs: u64,
}

/// Run the Mothership HTTP server
pub fn run_server(options: ServeOptions) -> Result<()> {
    let addr = format!("{}:{}", options.host, options.port);
    let state = Arc::new(ServerState::new());

    println!("🚀 Mothership daemon starting...");
    println!("   Listening on http://{}", addr);
    println!("   Press Ctrl+C to stop\n");

    rouille::start_server(&addr, move |request| {
        let state = Arc::clone(&state);
        handle_request(request, &state)
    });
}

/// Route requests to handlers
fn handle_request(request: &Request, state: &ServerState) -> Response {
    router!(request,
        // Health check
        (GET) ["/health"] => {
            handle_health(state)
        },

        // Version info
        (GET) ["/version"] => {
            handle_version(state)
        },

        // Future: /api/scry, /api/embed, /api/repos
        // These will be added in subsequent phases

        // 404 for unknown routes
        _ => {
            Response::text("Not Found").with_status_code(404)
        }
    )
}

/// Handle GET /health
fn handle_health(state: &ServerState) -> Response {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: state.version.clone(),
        uptime_secs: state.uptime_secs(),
    };

    Response::json(&response)
}

/// Handle GET /version
fn handle_version(state: &ServerState) -> Response {
    Response::json(&serde_json::json!({
        "version": state.version,
        "name": "patina-mothership"
    }))
}
