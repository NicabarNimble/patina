//! Mother daemon server implementation
//!
//! Provides HTTP server for:
//! - Container queries to Mac mother
//! - Hot model caching (E5 embeddings)
//! - Cross-project knowledge access
//!
//! Design: Blocking HTTP microserver (no async/tokio)
//!
//! Transport model:
//! - Default: Unix domain socket at ~/.patina/run/serve.sock
//! - Opt-in: TCP at --host/--port (bearer token required)

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::microserver;
use crate::retrieval::{QueryEngine, QueryOptions};

/// Maximum request body size (1 MB)
const MAX_BODY_SIZE: usize = 1_048_576;

/// Maximum results per query
const MAX_LIMIT: usize = 1000;

// === Transport-free request/response types ===

/// HTTP request independent of transport
struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// HTTP response independent of transport
struct HttpResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpRequest {
    /// Get header value by name (case-insensitive)
    fn header(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.as_str())
    }
}

impl HttpResponse {
    /// Create a JSON response
    fn json(status: u16, value: &impl Serialize) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: serde_json::to_vec(value).unwrap_or_default(),
        }
    }

    /// Add a header
    fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
}

// === Server state ===

/// Cached secrets entry with expiry
struct SecretsCacheEntry {
    secrets: HashMap<String, String>,
    expires_at: Instant,
}

/// Server state shared across request handlers
pub struct ServerState {
    start_time: Instant,
    version: String,
    token: String,
    secrets_cache: Mutex<Option<SecretsCacheEntry>>,
}

impl ServerState {
    fn new(token: String) -> Self {
        Self {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            token,
            secrets_cache: Mutex::new(None),
        }
    }

    fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

// === API types ===

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_secs: u64,
}

/// Scry API request
#[derive(Deserialize)]
struct ScryRequest {
    query: String,
    repo: Option<String>,
    #[serde(default)]
    all_repos: bool,
    #[serde(default)]
    include_issues: bool,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    10
}

/// Scry API response
#[derive(Serialize)]
struct ScryResponse {
    results: Vec<ScryResultJson>,
    count: usize,
}

/// Single result in JSON format
#[derive(Serialize)]
struct ScryResultJson {
    id: i64,
    content: String,
    score: f32,
    event_type: String,
    source_id: String,
    timestamp: String,
}

// === Helpers ===

/// Generate a random 32-byte hex token
fn generate_token() -> String {
    (0..32)
        .map(|_| format!("{:02x}", fastrand::u8(..)))
        .collect()
}

/// Check bearer token authorization
fn check_auth(request: &HttpRequest, token: &str) -> bool {
    request
        .header("Authorization")
        .map(|h| h == format!("Bearer {}", token))
        .unwrap_or(false)
}

/// Add security headers to response
fn with_security_headers(response: HttpResponse) -> HttpResponse {
    response
        .with_header("X-Content-Type-Options", "nosniff")
        .with_header("X-Frame-Options", "DENY")
}

/// Consistent JSON error response
fn json_error(status: u16, message: &str) -> HttpResponse {
    HttpResponse::json(status, &serde_json::json!({"error": message}))
}

// === Transport-free handlers ===

/// Route request to handler
fn route_request(request: &HttpRequest, state: &ServerState, require_auth: bool) -> HttpResponse {
    let response = match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => handle_health(state),
        ("GET", "/version") => handle_version(state),
        ("POST", "/api/scry") => handle_scry(request, state, require_auth),
        ("GET", "/secrets/cache") => handle_secrets_get(request, state, require_auth),
        ("POST", "/secrets/cache") => handle_secrets_cache(request, state, require_auth),
        ("POST", "/secrets/lock") => handle_secrets_lock(request, state, require_auth),
        _ => json_error(404, "Not found"),
    };
    with_security_headers(response)
}

/// Handle GET /health
fn handle_health(state: &ServerState) -> HttpResponse {
    HttpResponse::json(
        200,
        &HealthResponse {
            status: "ok".to_string(),
            version: state.version.clone(),
            uptime_secs: state.uptime_secs(),
        },
    )
}

/// Handle GET /version
fn handle_version(state: &ServerState) -> HttpResponse {
    HttpResponse::json(
        200,
        &serde_json::json!({
            "version": state.version,
            "name": "patina-mother"
        }),
    )
}

/// Handle POST /api/scry
fn handle_scry(request: &HttpRequest, state: &ServerState, require_auth: bool) -> HttpResponse {
    if require_auth && !check_auth(request, &state.token) {
        return json_error(401, "Unauthorized");
    }

    if request.body.is_empty() {
        return json_error(400, "Missing request body");
    }

    let mut body: ScryRequest = match serde_json::from_slice(&request.body) {
        Ok(req) => req,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
    };

    body.limit = body.limit.min(MAX_LIMIT);

    let engine = QueryEngine::new();
    let query_opts = QueryOptions {
        repo: body.repo,
        all_repos: body.all_repos,
        include_issues: body.include_issues,
    };

    match engine.query_with_options(&body.query, body.limit, &query_opts) {
        Ok(results) => {
            let json_results: Vec<ScryResultJson> = results
                .into_iter()
                .map(|r| ScryResultJson {
                    id: 0,
                    content: r.content,
                    score: r.fused_score,
                    event_type: r.sources.join("+"),
                    source_id: r.doc_id,
                    timestamp: r.metadata.timestamp.unwrap_or_default(),
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

// === Secrets cache handlers ===

#[derive(Deserialize)]
struct SecretsCacheRequest {
    secrets: HashMap<String, String>,
    #[serde(default = "default_ttl_secs")]
    ttl_secs: u64,
}

fn default_ttl_secs() -> u64 {
    600
}

fn handle_secrets_get(
    request: &HttpRequest,
    state: &ServerState,
    require_auth: bool,
) -> HttpResponse {
    if require_auth && !check_auth(request, &state.token) {
        return json_error(401, "Unauthorized");
    }

    let cache = state
        .secrets_cache
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    match cache.as_ref() {
        Some(entry) if entry.expires_at > Instant::now() => HttpResponse::json(200, &entry.secrets),
        _ => json_error(404, "No cached secrets"),
    }
}

fn handle_secrets_cache(
    request: &HttpRequest,
    state: &ServerState,
    require_auth: bool,
) -> HttpResponse {
    if require_auth && !check_auth(request, &state.token) {
        return json_error(401, "Unauthorized");
    }

    if request.body.is_empty() {
        return json_error(400, "Missing request body");
    }

    let body: SecretsCacheRequest = match serde_json::from_slice(&request.body) {
        Ok(req) => req,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
    };

    let ttl = std::time::Duration::from_secs(body.ttl_secs);
    let mut cache = state
        .secrets_cache
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    *cache = Some(SecretsCacheEntry {
        secrets: body.secrets,
        expires_at: Instant::now() + ttl,
    });

    HttpResponse::json(200, &serde_json::json!({"status": "cached"}))
}

fn handle_secrets_lock(
    request: &HttpRequest,
    state: &ServerState,
    require_auth: bool,
) -> HttpResponse {
    if require_auth && !check_auth(request, &state.token) {
        return json_error(401, "Unauthorized");
    }

    let mut cache = state
        .secrets_cache
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    *cache = None;

    HttpResponse::json(200, &serde_json::json!({"status": "locked"}))
}

// === Transport: microserver accept loop ===

fn from_micro(req: microserver::HttpRequest) -> HttpRequest {
    HttpRequest {
        method: req.method,
        path: req.path,
        headers: req.headers,
        body: req.body,
    }
}

fn to_micro(resp: HttpResponse) -> microserver::HttpResponse {
    microserver::HttpResponse {
        status: resp.status,
        headers: resp.headers,
        body: resp.body,
    }
}

fn handle_connection(stream: &mut (impl Read + Write), state: &ServerState, require_auth: bool) {
    let req = match microserver::read_request(stream) {
        Some(Ok(req)) => from_micro(req),
        Some(Err(msg)) => {
            let resp = to_micro(with_security_headers(json_error(400, &msg)));
            microserver::write_response(stream, &resp);
            return;
        }
        None => return,
    };

    let resp = if req.body.len() > MAX_BODY_SIZE {
        with_security_headers(json_error(413, "Request too large"))
    } else {
        route_request(&req, state, require_auth)
    };

    microserver::write_response(stream, &to_micro(resp));
}

/// Options for starting the daemon
pub struct DaemonOptions {
    pub host: Option<String>,
    pub port: u16,
}

impl Default for DaemonOptions {
    fn default() -> Self {
        Self {
            host: None,
            port: 50051,
        }
    }
}

/// Run the mother daemon server
pub fn run_server(options: DaemonOptions) -> Result<()> {
    // TCP opt-in path (--host flag) — requires bearer token
    if let Some(ref host) = options.host {
        let token = std::env::var("PATINA_SERVE_TOKEN").unwrap_or_else(|_| generate_token());
        let state = Arc::new(ServerState::new(token));
        let addr = format!("{}:{}", host, options.port);

        if host != "127.0.0.1" && host != "localhost" {
            eprintln!(
                "WARNING: Binding to {} exposes the server to the network.",
                host
            );
            eprintln!(
                "  The server has no encryption (HTTP only). Use a reverse proxy for production."
            );
        }

        let token_path = patina::paths::serve::token_path();
        std::fs::write(&token_path, state.token.as_bytes())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&token_path, std::fs::Permissions::from_mode(0o600))?;
        }
        eprintln!("Auth token written to {}", token_path.display());

        let listener = TcpListener::bind(&addr)?;
        println!("🚀 Mother daemon starting...");
        println!("   Listening on http://{}", addr);
        println!("   Press Ctrl+C to stop\n");

        accept_loop_tcp(listener, state);
    }

    // Default: UDS path (no TCP, no token needed — file permissions are auth)
    let state = Arc::new(ServerState::new(String::new()));
    let listener = super::setup_unix_listener()?;
    let socket_path = patina::paths::serve::socket_path();

    // Write PID file
    write_pid_file()?;

    // Register signal handlers for cleanup
    register_signal_handlers();

    println!("🚀 Mother daemon starting...");
    println!("   PID: {}", std::process::id());
    println!("   Listening on {}", socket_path.display());
    println!(
        "   Test: curl -s --unix-socket {} http://localhost/health",
        socket_path.display()
    );
    println!("   No TCP listener (use --host/--port for network access)");
    println!("   Press Ctrl+C to stop\n");

    accept_loop_uds(listener, state);
}

fn accept_loop_tcp(listener: TcpListener, state: Arc<ServerState>) -> ! {
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let state = Arc::clone(&state);
                std::thread::spawn(move || {
                    handle_connection(&mut stream, &state, true);
                    let _ = stream.shutdown(Shutdown::Write);
                });
            }
            Err(e) => eprintln!("TCP accept error: {}", e),
        }
    }
    std::process::exit(0);
}

fn accept_loop_uds(listener: std::os::unix::net::UnixListener, state: Arc<ServerState>) -> ! {
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let state = Arc::clone(&state);
                std::thread::spawn(move || {
                    handle_connection(&mut stream, &state, false);
                    let _ = stream.shutdown(Shutdown::Write);
                });
            }
            Err(e) => eprintln!("UDS accept error: {}", e),
        }
    }
    std::process::exit(0);
}

/// Write PID file for daemon lifecycle management
fn write_pid_file() -> Result<()> {
    use anyhow::Context;
    use std::os::unix::fs::PermissionsExt;

    let pid_path = patina::paths::serve::pid_path();
    let pid = std::process::id();

    std::fs::write(&pid_path, pid.to_string())
        .with_context(|| format!("writing PID file {}", pid_path.display()))?;

    std::fs::set_permissions(&pid_path, std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("setting permissions on {}", pid_path.display()))?;

    Ok(())
}

/// Clean up PID file on shutdown
fn cleanup_pid_file() {
    let pid_path = patina::paths::serve::pid_path();
    let _ = std::fs::remove_file(&pid_path);
}

/// Register signal handlers for graceful shutdown
fn register_signal_handlers() {
    unsafe {
        libc::signal(
            libc::SIGINT,
            sigint_handler as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            sigint_handler as *const () as libc::sighandler_t,
        );
    }
}

extern "C" fn sigint_handler(_: libc::c_int) {
    cleanup_pid_file();
    super::cleanup_socket();
    std::process::exit(0);
}
