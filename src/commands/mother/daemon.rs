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
use std::collections::HashSet;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use patina::mother::ChildRequest;

use super::adapters::{RetrievalScryBackend, ScryBackend};
use super::registry::ChildRegistry;
use mother_crate::http_api::ApiRuntime;
use mother_crate::http_daemon::{json_error, HttpRequest, HttpResponse, DEFAULT_MAX_BODY_SIZE};
use mother_crate::http_routes::Router;

// === Server state ===

/// Server state shared across request handlers
pub struct ServerState {
    start_time: Instant,
    version: String,
    token: String,
    pub(super) registry: ChildRegistry,
    scry_backend: Arc<dyn ScryBackend>,
}

impl ServerState {
    fn new(token: String, registry: ChildRegistry) -> Self {
        Self {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            token,
            registry,
            scry_backend: Arc::new(RetrievalScryBackend),
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

/// Spawn the heartbeat thread.
///
/// Default Mother runtime only advances knowledge children. Legacy shell-toy
/// children remain behind the explicit migration mode so the normal daemon
/// path does not teach two runtime stories at once.
fn spawn_heartbeat(state: Arc<ServerState>, legacy_migration: bool) {
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
            if legacy_migration {
                let toys = state.registry.tick_legacy_all();
                for toy in toys {
                    let mut flight = in_flight.lock().unwrap_or_else(|e| e.into_inner());
                    if flight.contains(&toy.name) {
                        eprintln!("[mother:toy] skipping '{}' (already in flight)", toy.name);
                        continue;
                    }
                    flight.insert(toy.name.clone());
                    drop(flight);

                    spawn_toy_tracked(toy, Arc::clone(&in_flight));
                }
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

// === Helpers ===

/// Generate a random 32-byte hex token
fn generate_token() -> String {
    (0..32)
        .map(|_| format!("{:02x}", fastrand::u8(..)))
        .collect()
}

// === Transport-free handlers ===

impl ApiRuntime for ServerState {
    fn version(&self) -> String {
        self.version.clone()
    }

    fn uptime_secs(&self) -> u64 {
        self.uptime_secs()
    }

    fn health_all(&self) -> Vec<(String, patina::mother::ChildHealth)> {
        self.registry.health_all()
    }

    fn child_health(&self, child_name: &str) -> anyhow::Result<patina::mother::ChildHealth> {
        self.registry.health(child_name)
    }

    fn child_handle(
        &self,
        child_name: &str,
        action: String,
        payload: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let request = ChildRequest { action, payload };
        Ok(self.registry.handle(child_name, &request)?.payload)
    }

    fn scry_query(
        &self,
        query: &str,
        limit: usize,
        repo: Option<String>,
        all_repos: bool,
    ) -> anyhow::Result<Vec<mother_crate::http_api::ScryHit>> {
        Ok(self
            .scry_backend
            .query(query, limit, repo, all_repos)?
            .into_iter()
            .map(|hit| mother_crate::http_api::ScryHit {
                content: hit.content,
                score: hit.score,
                event_type: hit.event_type,
                source_id: hit.source_id,
                timestamp: hit.timestamp,
            })
            .collect())
    }
}

fn handle_builtin_child_request(
    child_name: &str,
    action: &str,
    body: &[u8],
) -> Option<HttpResponse> {
    match (child_name, action) {
        ("spec-manager", "health") => Some(HttpResponse::json(
            200,
            &serde_json::json!({"status": "healthy"}),
        )),
        ("spec-manager", "dispatch") => {
            let payload = if body.is_empty() {
                serde_json::Value::Null
            } else {
                match serde_json::from_slice::<serde_json::Value>(body) {
                    Ok(v) => v,
                    Err(e) => return Some(json_error(400, &format!("Invalid JSON: {}", e))),
                }
            };
            let command_value = payload
                .get("command")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let command: crate::commands::spec::SpecCommands =
                match serde_json::from_value(command_value) {
                    Ok(command) => command,
                    Err(e) => {
                        return Some(json_error(
                            400,
                            &format!("Invalid spec-manager command payload: {}", e),
                        ));
                    }
                };

            match crate::commands::spec::execute_value(command) {
                Ok(value) => Some(HttpResponse::json(200, &value)),
                Err(e) => Some(json_error(400, &e.to_string())),
            }
        }
        ("lake-manager", "health") => Some(HttpResponse::json(
            200,
            &serde_json::json!({"status": "healthy"}),
        )),
        ("lake-manager", "dispatch") => {
            let payload = if body.is_empty() {
                serde_json::Value::Null
            } else {
                match serde_json::from_slice::<serde_json::Value>(body) {
                    Ok(v) => v,
                    Err(e) => return Some(json_error(400, &format!("Invalid JSON: {}", e))),
                }
            };
            let command_value = payload
                .get("command")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let command: crate::commands::lake::LakeCommands =
                match serde_json::from_value(command_value) {
                    Ok(command) => command,
                    Err(e) => {
                        return Some(json_error(
                            400,
                            &format!("Invalid lake-manager command payload: {}", e),
                        ));
                    }
                };

            match crate::commands::lake::execute_value(command) {
                Ok(value) => Some(HttpResponse::json(200, &value)),
                Err(e) => Some(json_error(400, &e.to_string())),
            }
        }
        ("doctor", "health") => Some(HttpResponse::json(
            200,
            &serde_json::json!({"status": "healthy"}),
        )),
        ("doctor", "run") => match crate::commands::doctor::execute_value() {
            Ok(value) => {
                let exit_code = value.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0);
                Some(HttpResponse::json(
                    200,
                    &serde_json::json!({
                        "child": "doctor",
                        "text": "",
                        "data": value,
                        "exit_code": exit_code,
                    }),
                ))
            }
            Err(e) => Some(json_error(400, &e.to_string())),
        },
        _ => None,
    }
}

fn build_router(state: Arc<ServerState>, require_auth: bool) -> Router {
    let token = state.token.clone();
    let route_table =
        mother_crate::http_api::build_route_table(state, Arc::new(handle_builtin_child_request));
    Router::new(require_auth, token, route_table)
}

/// Options for starting the daemon
pub struct DaemonOptions {
    pub host: Option<String>,
    pub port: u16,
    pub legacy_migration: bool,
}

impl Default for DaemonOptions {
    fn default() -> Self {
        Self {
            host: None,
            port: 50051,
            legacy_migration: false,
        }
    }
}

/// Run the mother daemon server
pub fn run_server(options: DaemonOptions) -> Result<()> {
    let extracted_mode = std::env::var("PATINA_MOTHER_EXTRACTED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if extracted_mode {
        if options.host.is_some() {
            anyhow::bail!(
                "PATINA_MOTHER_EXTRACTED only supports Unix socket mode (omit --host/--port)"
            );
        }

        let socket_path = patina::paths::serve::socket_path();
        write_pid_file()?;
        register_signal_handlers();

        println!("🚀 Mother daemon starting (extracted mode)...");
        println!("   PID: {}", std::process::id());
        println!("   Listening on {}", socket_path.display());
        println!("   Protocol: JSON-lines over Unix socket");
        println!("   Press Ctrl+C to stop\n");

        let state = mother_crate::daemon::DaemonState::default();
        return mother_crate::daemon::listen_with_state(&socket_path, &state);
    }

    // Build and load child registry
    let mut registry = ChildRegistry::new();
    let runtime = patina::mother::KnowledgeRuntimeStore::default();

    // Compiled-in children (always available)
    registry
        .register(Box::new(super::secrets::SecretsCacheChild::new()))
        .expect("failed to register secrets child");
    registry
        .register(Box::new(
            mother_crate::session_writer::SessionWriterChild::new(),
        ))
        .expect("failed to register session-writer child");
    registry
        .register(Box::new(mother_crate::static_child::StaticChild::new(
            "spec-manager",
        )))
        .expect("failed to register spec-manager child marker");
    registry
        .register(Box::new(mother_crate::static_child::StaticChild::new(
            "doctor",
        )))
        .expect("failed to register doctor child marker");
    registry
        .register(Box::new(mother_crate::static_child::StaticChild::new(
            "lake-manager",
        )))
        .expect("failed to register lake-manager child marker");

    // WASM children (discovered from ~/.patina/children/)
    let children_dir = patina::paths::child::children_dir();
    if children_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&children_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("wasm") {
                    let manifest_path = path.with_extension("toml");
                    match load_wasm_child(&path, &manifest_path) {
                        Ok(loaded) => match register_loaded_child(
                            &mut registry,
                            &runtime,
                            loaded,
                            options.legacy_migration,
                        ) {
                            Ok(Some(message)) => eprintln!("[mother] {}", message),
                            Ok(None) => {}
                            Err(error) => {
                                eprintln!("[mother] skipping {}: {}", path.display(), error)
                            }
                        },
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
        println!(
            "   Knowledge children: {} loaded",
            state.registry.knowledge_len()
        );
        if options.legacy_migration {
            println!(
                "   Legacy migration children: {} loaded",
                state.registry.legacy_len()
            );
        }
        println!("   Listening on http://{}", addr);
        println!("   Press Ctrl+C to stop\n");

        spawn_heartbeat(Arc::clone(&state), options.legacy_migration);
        let router = Arc::new(build_router(Arc::clone(&state), true));
        let handler = Arc::new(move |request: HttpRequest| router.route(&request));
        mother_crate::http_daemon::accept_loop_tcp(listener, DEFAULT_MAX_BODY_SIZE, handler);
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
    println!(
        "   Knowledge children: {} loaded",
        state.registry.knowledge_len()
    );
    if options.legacy_migration {
        println!(
            "   Legacy migration children: {} loaded",
            state.registry.legacy_len()
        );
    }
    println!("   Listening on {}", socket_path.display());
    println!(
        "   Test: curl -s --unix-socket {} http://localhost/health",
        socket_path.display()
    );
    println!("   No TCP listener (use --host/--port for network access)");
    println!("   Press Ctrl+C to stop\n");

    spawn_heartbeat(Arc::clone(&state), options.legacy_migration);
    let router = Arc::new(build_router(Arc::clone(&state), false));
    let handler = Arc::new(move |request: HttpRequest| router.route(&request));
    mother_crate::http_daemon::accept_loop_uds(listener, DEFAULT_MAX_BODY_SIZE, handler);
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
        relationship_listens: Vec<String>,
    },
}

fn register_loaded_child(
    registry: &mut ChildRegistry,
    runtime: &patina::mother::KnowledgeRuntimeStore,
    loaded: LoadedWasmChild,
    legacy_migration: bool,
) -> Result<Option<String>> {
    match loaded {
        LoadedWasmChild::Legacy { child, name } => {
            if legacy_migration {
                registry.register_legacy(child)?;
                Ok(Some(format!("loaded legacy migration child: {}", name)))
            } else {
                Ok(Some(format!(
                    "skipping legacy child {} (use --legacy-migration to load mother-child plugins)",
                    name
                )))
            }
        }
        LoadedWasmChild::Knowledge {
            child,
            name,
            subscribed_streams,
            relationship_listens,
        } => {
            let mut routes: std::collections::HashSet<String> =
                subscribed_streams.into_iter().collect();
            routes.extend(relationship_listens);
            let routing_table = routes.into_iter().collect::<Vec<_>>();
            runtime.ensure_subscriptions(&name, &routing_table)?;
            registry.register_knowledge(child)?;
            Ok(Some(format!("loaded knowledge WASM child: {}", name)))
        }
    }
}

fn parse_relationship_listens(manifest_path: &std::path::Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(manifest_path)?;
    let table: toml::Table = content.parse()?;

    let listens = table
        .get("relationships")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("listens"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(listens)
}

/// Load a WASM child from a .wasm file + child manifest.
fn load_wasm_child(
    wasm_path: &std::path::Path,
    manifest_path: &std::path::Path,
) -> Result<LoadedWasmChild> {
    let manifest = patina::child::engine::ChildManifest::from_path(manifest_path)?;
    let relationship_listens = parse_relationship_listens(manifest_path)?;
    let wasm_bytes = std::fs::read(wasm_path)?;
    match manifest.world {
        patina::child::engine::ChildKind::KnowledgeChild => {
            let engine = patina::child::engine::KnowledgeChildEngine::new()?;
            let component = engine.load_component(&wasm_bytes)?;
            let child = engine.instantiate_child(&component, &manifest, None)?;
            let name = child.name().to_string();
            Ok(LoadedWasmChild::Knowledge {
                child,
                name,
                subscribed_streams: manifest.subscribed_streams.clone(),
                relationship_listens,
            })
        }
        patina::child::engine::ChildKind::MotherChild => {
            let engine = patina::child::engine::MotherChildEngine::new()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use patina::mother::{
        ChildHealth, ChildRequest, ChildResponse, KnowledgeChild, MotherChild, MotherHost,
    };

    struct StubLegacy;

    impl MotherChild for StubLegacy {
        fn name(&self) -> &str {
            "legacy"
        }

        fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
            Ok(())
        }

        fn health(&self) -> ChildHealth {
            ChildHealth::Healthy
        }

        fn handle(&self, _request: &ChildRequest) -> Result<ChildResponse> {
            Ok(ChildResponse {
                payload: serde_json::Value::Null,
            })
        }
    }

    struct StubKnowledge;

    impl KnowledgeChild for StubKnowledge {
        fn name(&self) -> &str {
            "knowledge"
        }

        fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
            Ok(())
        }

        fn health(&self) -> ChildHealth {
            ChildHealth::Healthy
        }

        fn handle(&self, _request: &ChildRequest) -> Result<ChildResponse> {
            Ok(ChildResponse {
                payload: serde_json::Value::Null,
            })
        }
    }

    #[test]
    fn daemon_options_default_keeps_legacy_quarantined() {
        let options = DaemonOptions::default();
        assert!(!options.legacy_migration);
    }

    #[test]
    fn register_loaded_child_skips_legacy_without_migration_mode() {
        let mut registry = ChildRegistry::new();
        let runtime = patina::mother::KnowledgeRuntimeStore::default();

        let message = register_loaded_child(
            &mut registry,
            &runtime,
            LoadedWasmChild::Legacy {
                child: Box::new(StubLegacy),
                name: "legacy".into(),
            },
            false,
        )
        .unwrap()
        .unwrap();

        assert!(message.contains("skipping legacy child legacy"));
        assert_eq!(registry.legacy_len(), 0);
        assert_eq!(registry.knowledge_len(), 0);
    }

    #[test]
    fn register_loaded_child_loads_knowledge_by_default() {
        let mut registry = ChildRegistry::new();
        let runtime = patina::mother::KnowledgeRuntimeStore::default();

        register_loaded_child(
            &mut registry,
            &runtime,
            LoadedWasmChild::Knowledge {
                child: Box::new(StubKnowledge),
                name: "knowledge".into(),
                subscribed_streams: vec!["belief.changed".into()],
                relationship_listens: vec![],
            },
            false,
        )
        .unwrap();

        assert_eq!(registry.knowledge_len(), 1);
        assert_eq!(registry.legacy_len(), 0);
    }

    #[test]
    fn parse_relationship_listens_from_manifest() {
        let temp = tempfile::TempDir::new().unwrap();
        let manifest = temp.path().join("child.toml");
        std::fs::write(
            &manifest,
            r#"
[child]
name = "child"
kind = "knowledge-child"

[relationships]
emits = ["x"]
listens = ["data-ingested", "belief.changed"]
"#,
        )
        .unwrap();

        let listens = parse_relationship_listens(&manifest).unwrap();
        assert_eq!(
            listens,
            vec!["data-ingested".to_string(), "belief.changed".to_string()]
        );
    }

    #[test]
    fn parse_relationship_listens_defaults_empty() {
        let temp = tempfile::TempDir::new().unwrap();
        let manifest = temp.path().join("child.toml");
        std::fs::write(
            &manifest,
            r#"
[child]
name = "child"
kind = "knowledge-child"
"#,
        )
        .unwrap();

        let listens = parse_relationship_listens(&manifest).unwrap();
        assert!(listens.is_empty());
    }
}
