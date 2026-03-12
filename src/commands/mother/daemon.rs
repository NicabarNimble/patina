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
use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use patina::mother::ChildRequest;

use super::microserver;
use super::registry::ChildRegistry;
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

/// Server state shared across request handlers
pub struct ServerState {
    start_time: Instant,
    version: String,
    token: String,
    pub(super) registry: ChildRegistry,
}

impl ServerState {
    fn new(token: String, registry: ChildRegistry) -> Self {
        Self {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            token,
            registry,
        }
    }

    fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

// === Host capabilities ===

/// MotherHost implementation for the daemon process.
struct DaemonHost;

impl patina::mother::MotherHost for DaemonHost {
    fn log(&self, child: &str, message: &str) {
        eprintln!("[mother:{}] {}", child, message);
    }
}

// === Heartbeat ===

/// Heartbeat interval in seconds
const HEARTBEAT_INTERVAL_SECS: u64 = 60;

/// Spawn the heartbeat thread — ticks all children periodically.
/// Toys returned by children are spawned as child processes.
/// Tracks in-flight toys to prevent duplicate spawning.
fn spawn_heartbeat(state: Arc<ServerState>) {
    let in_flight: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let runtime = patina::mother::KnowledgeRuntimeStore::default();

    std::thread::Builder::new()
        .name("mother-heartbeat".to_string())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
            if let Err(error) = state
                .registry
                .run_knowledge_cycles(&runtime, "mother-heartbeat")
            {
                eprintln!("[mother] knowledge-child heartbeat failed: {}", error);
            }
            let toys = state.registry.tick_legacy_all();
            for toy in toys {
                let mut flight = in_flight.lock().unwrap_or_else(|e| e.into_inner());
                if flight.contains(&toy.name) {
                    eprintln!("[mother:toy] skipping '{}' (already in flight)", toy.name);
                    continue;
                }
                flight.insert(toy.name.clone());
                drop(flight); // release lock before spawning thread

                spawn_toy_tracked(toy, Arc::clone(&in_flight));
            }
        })
        .expect("failed to spawn heartbeat thread");
}

/// Spawn a toy as a child process in a background thread with in-flight tracking.
///
/// The child decides *what* to run. Mother handles *how*.
/// Each toy runs in its own thread so the heartbeat loop isn't blocked.
/// On completion (success or failure), the toy name is removed from the
/// in-flight set so it's eligible for retry on the next heartbeat.
fn spawn_toy_tracked(toy: patina::mother::Toy, in_flight: Arc<Mutex<HashSet<String>>>) {
    let toy_name = toy.name.clone();
    let in_flight_thread = Arc::clone(&in_flight);

    match std::thread::Builder::new()
        .name(format!("toy-{}", toy.name))
        .spawn(move || {
            eprintln!(
                "[mother:toy] spawning '{}': {} {:?}",
                toy.name, toy.command, toy.args
            );
            match std::process::Command::new(&toy.command)
                .args(&toy.args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .status()
            {
                Ok(status) if status.success() => {
                    eprintln!("[mother:toy] '{}' completed successfully", toy.name);
                }
                Ok(status) => {
                    eprintln!("[mother:toy] '{}' failed with {}", toy.name, status);
                }
                Err(e) => {
                    eprintln!("[mother:toy] '{}' failed to spawn: {}", toy.name, e);
                }
            }
            // Remove from in-flight set when done (success or failure)
            let mut flight = in_flight_thread.lock().unwrap_or_else(|e| e.into_inner());
            flight.remove(&toy.name);
        }) {
        Ok(_) => {} // thread owns cleanup via in_flight
        Err(e) => {
            // Thread failed to spawn — remove from in-flight so it's
            // eligible for retry on next heartbeat. Don't leave stuck.
            eprintln!("[mother:toy] thread spawn failed for '{}': {}", toy_name, e);
            let mut flight = in_flight.lock().unwrap_or_else(|e| e.into_inner());
            flight.remove(&toy_name);
        }
    }
}

// === API types ===

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    uptime_secs: u64,
    children: Vec<ChildHealthJson>,
}

/// Child health in JSON form (uses Display impl of ChildHealth)
#[derive(Serialize)]
struct ChildHealthJson {
    name: String,
    status: String,
}

/// Scry API request
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
        // Generic child routing: /child/{name}/{action}
        _ if request.path.starts_with("/child/") => {
            handle_child_request(request, state, require_auth)
        }
        _ => json_error(404, "Not found"),
    };
    with_security_headers(response)
}

/// Handle GET /health
fn handle_health(state: &ServerState) -> HttpResponse {
    let children: Vec<ChildHealthJson> = state
        .registry
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
            version: state.version.clone(),
            uptime_secs: state.uptime_secs(),
            children,
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
        ..Default::default()
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

// === Secrets cache handlers (delegate to secrets child) ===

fn handle_secrets_get(
    request: &HttpRequest,
    state: &ServerState,
    require_auth: bool,
) -> HttpResponse {
    if require_auth && !check_auth(request, &state.token) {
        return json_error(401, "Unauthorized");
    }

    let child_req = ChildRequest {
        action: "get".into(),
        payload: serde_json::Value::Null,
    };

    match state.registry.handle("secrets", &child_req) {
        Ok(resp) => HttpResponse::json(200, &resp.payload),
        Err(_) => json_error(404, "No cached secrets"),
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

    let payload: serde_json::Value = match serde_json::from_slice(&request.body) {
        Ok(v) => v,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
    };

    let child_req = ChildRequest {
        action: "cache".into(),
        payload,
    };

    match state.registry.handle("secrets", &child_req) {
        Ok(resp) => HttpResponse::json(200, &resp.payload),
        Err(e) => json_error(500, &format!("Cache failed: {}", e)),
    }
}

fn handle_secrets_lock(
    request: &HttpRequest,
    state: &ServerState,
    require_auth: bool,
) -> HttpResponse {
    if require_auth && !check_auth(request, &state.token) {
        return json_error(401, "Unauthorized");
    }

    let child_req = ChildRequest {
        action: "lock".into(),
        payload: serde_json::Value::Null,
    };

    match state.registry.handle("secrets", &child_req) {
        Ok(resp) => HttpResponse::json(200, &resp.payload),
        Err(e) => json_error(500, &format!("Lock failed: {}", e)),
    }
}

// === Generic child routing ===

/// Handle /child/{name}/{action} — generic routing to any child.
///
/// GET requests pass null payload; POST requests parse body as JSON.
/// Existing /secrets/* routes are legacy aliases for this mechanism.
fn handle_child_request(
    request: &HttpRequest,
    state: &ServerState,
    require_auth: bool,
) -> HttpResponse {
    if require_auth && !check_auth(request, &state.token) {
        return json_error(401, "Unauthorized");
    }

    // Parse /child/{name}/{action}
    let parts: Vec<&str> = request.path[1..].split('/').collect();
    if parts.len() != 3 {
        return json_error(400, "Expected /child/{name}/{action}");
    }
    let child_name = parts[1];
    let action = parts[2];

    let payload = if request.body.is_empty() {
        serde_json::Value::Null
    } else {
        match serde_json::from_slice(&request.body) {
            Ok(v) => v,
            Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
        }
    };

    let child_req = ChildRequest {
        action: action.to_string(),
        payload,
    };

    match state.registry.handle(child_name, &child_req) {
        Ok(resp) => HttpResponse::json(200, &resp.payload),
        Err(e) => json_error(404, &format!("{}", e)),
    }
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
    // Build and load child registry
    let mut registry = ChildRegistry::new();
    let runtime = patina::mother::KnowledgeRuntimeStore::default();

    // Compiled-in children (always available)
    registry
        .register(Box::new(super::secrets::SecretsCacheChild::new()))
        .expect("failed to register secrets child");

    // WASM children (discovered from ~/.patina/children/)
    let children_dir = patina::paths::plugin::children_dir();
    if children_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&children_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("wasm") {
                    let manifest_path = path.with_extension("toml");
                    match load_wasm_child(&path, &manifest_path) {
                        Ok(LoadedWasmChild::Legacy { child, name }) => {
                            match registry.register_legacy(child) {
                                Ok(()) => eprintln!("[mother] loaded legacy WASM child: {}", name),
                                Err(e) => eprintln!("[mother] skipping {}: {}", path.display(), e),
                            }
                        }
                        Ok(LoadedWasmChild::Knowledge {
                            child,
                            name,
                            subscribed_streams,
                        }) => {
                            if let Err(error) =
                                runtime.ensure_subscriptions(&name, &subscribed_streams)
                            {
                                eprintln!(
                                    "[mother] failed to persist subscriptions for {}: {}",
                                    name, error
                                );
                                continue;
                            }
                            match registry.register_knowledge(child) {
                                Ok(()) => {
                                    eprintln!("[mother] loaded knowledge WASM child: {}", name)
                                }
                                Err(e) => eprintln!("[mother] skipping {}: {}", path.display(), e),
                            }
                        }
                        Err(e) => {
                            eprintln!("[mother] failed to load {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
        if let Ok(entries) = std::fs::read_dir(&children_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("toml")
                    && !path.with_extension("wasm").exists()
                {
                    eprintln!("[mother] orphaned manifest (no .wasm): {}", path.display());
                }
            }
        }
    }

    let daemon_host = DaemonHost;
    registry.load_all(&daemon_host)?;
    let child_count = registry.len();

    // TCP opt-in path (--host flag) — requires bearer token
    if let Some(ref host) = options.host {
        let token = std::env::var("PATINA_SERVE_TOKEN").unwrap_or_else(|_| generate_token());
        let state = Arc::new(ServerState::new(token, registry));
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
        println!("   Children: {} loaded", child_count);
        println!("   Listening on http://{}", addr);
        println!("   Press Ctrl+C to stop\n");

        spawn_heartbeat(Arc::clone(&state));
        accept_loop_tcp(listener, state);
    }

    // Default: UDS path (no TCP, no token needed — file permissions are auth)
    let state = Arc::new(ServerState::new(String::new(), registry));
    let listener = super::setup_unix_listener()?;
    let socket_path = patina::paths::serve::socket_path();

    // Write PID file
    write_pid_file()?;

    // Register signal handlers for cleanup
    register_signal_handlers();

    println!("🚀 Mother daemon starting...");
    println!("   PID: {}", std::process::id());
    println!("   Children: {} loaded", child_count);
    println!("   Listening on {}", socket_path.display());
    println!(
        "   Test: curl -s --unix-socket {} http://localhost/health",
        socket_path.display()
    );
    println!("   No TCP listener (use --host/--port for network access)");
    println!("   Press Ctrl+C to stop\n");

    spawn_heartbeat(Arc::clone(&state));
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

enum LoadedWasmChild {
    Legacy {
        child: Box<dyn patina::mother::MotherChild>,
        name: String,
    },
    Knowledge {
        child: Box<dyn patina::mother::KnowledgeChild>,
        name: String,
        subscribed_streams: Vec<String>,
    },
}

/// Load a WASM child from a .wasm file + plugin.toml manifest.
fn load_wasm_child(
    wasm_path: &std::path::Path,
    manifest_path: &std::path::Path,
) -> Result<LoadedWasmChild> {
    let manifest = patina::plugin::PluginManifest::from_path(manifest_path)?;
    let wasm_bytes = std::fs::read(wasm_path)?;
    match manifest.world {
        patina::plugin::PluginWorld::KnowledgeChild => {
            let engine = patina::plugin::KnowledgeChildEngine::new()?;
            let component = engine.load_component(&wasm_bytes)?;
            let child = engine.instantiate_child(&component, &manifest, None)?;
            let name = child.name().to_string();
            Ok(LoadedWasmChild::Knowledge {
                child,
                name,
                subscribed_streams: manifest.subscribed_streams.clone(),
            })
        }
        patina::plugin::PluginWorld::MotherChild => {
            let engine = patina::plugin::PluginEngine::new()?;
            let component = engine.load_component(&wasm_bytes)?;
            let child = engine.instantiate_child(&component, &manifest, None)?;
            let name = child.name().to_string();
            Ok(LoadedWasmChild::Legacy { child, name })
        }
        other => anyhow::bail!(
            "child manifest world '{}' is not loadable by the daemon child loader",
            other
        ),
    }
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
