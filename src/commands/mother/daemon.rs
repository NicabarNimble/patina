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
use std::sync::Arc;
use std::time::Instant;

use patina::mother::ChildRequest;

use super::adapters::{RetrievalScryBackend, ScryBackend};
use super::registry::ChildRegistry;
use mother_crate::http_api::ApiRuntime;
use mother_crate::http_routes::Router;

// === Server state ===

/// Server state shared across request handlers
pub struct ServerState {
    start_time: Instant,
    version: String,
    token: String,
    pub(super) registry: Arc<ChildRegistry>,
    scry_backend: Arc<dyn ScryBackend>,
}

impl ServerState {
    fn new(token: String, registry: ChildRegistry) -> Self {
        Self {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            token,
            registry: Arc::new(registry),
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

fn build_router(state: Arc<ServerState>, require_auth: bool) -> Router {
    let token = state.token.clone();
    let route_table = mother_crate::http_api::build_route_table(
        state,
        Arc::new(super::builtin_dispatch::handle_builtin_child_request),
    );
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
        let pid_path = patina::paths::serve::pid_path();
        mother_crate::daemon_lifecycle::write_pid_file(&pid_path)?;
        mother_crate::daemon_lifecycle::register_signal_handlers(pid_path, socket_path.clone());

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
    mother_crate::daemon_bootstrap::register_builtin_children(&mut registry)?;

    // WASM children (discovered from ~/.patina/children/)
    let children_dir = patina::paths::child::children_dir();
    mother_crate::daemon_bootstrap::load_children_from_dir(
        &children_dir,
        &mut registry,
        &runtime,
        options.legacy_migration,
        super::loader::load_wasm_child,
    );

    let daemon_host = DaemonHost;
    registry.load_all(&daemon_host)?;

    // TCP opt-in path (--host flag) — requires bearer token
    if let Some(ref host) = options.host {
        let token = std::env::var("PATINA_SERVE_TOKEN").unwrap_or_else(|_| generate_token());
        let state = Arc::new(ServerState::new(token, registry));
        let addr = format!("{}:{}", host, options.port);
        let listener = std::net::TcpListener::bind(&addr)?;
        let router = Arc::new(build_router(Arc::clone(&state), true));
        mother_crate::daemon_runner::run_tcp_server(mother_crate::daemon_runner::TcpServerLaunch {
            listener,
            host: host.clone(),
            addr,
            token_path: patina::paths::serve::token_path(),
            token: state.token.clone(),
            legacy_migration: options.legacy_migration,
            registry: Arc::clone(&state.registry),
            router,
        });
    }

    // Default: UDS path (no TCP, no token needed — file permissions are auth)
    let state = Arc::new(ServerState::new(String::new(), registry));
    let listener = super::setup_unix_listener()?;
    let socket_path = patina::paths::serve::socket_path();
    let pid_path = patina::paths::serve::pid_path();

    // Write PID file
    mother_crate::daemon_lifecycle::write_pid_file(&pid_path)?;

    // Register signal handlers for cleanup
    mother_crate::daemon_lifecycle::register_signal_handlers(pid_path, socket_path.clone());

    let router = Arc::new(build_router(Arc::clone(&state), false));
    mother_crate::daemon_runner::run_uds_server(mother_crate::daemon_runner::UdsServerLaunch {
        listener,
        socket_path,
        legacy_migration: options.legacy_migration,
        registry: Arc::clone(&state.registry),
        router,
    });
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

        let message = mother_crate::daemon_bootstrap::register_loaded_child(
            &mut registry,
            &runtime,
            mother_crate::daemon_bootstrap::LoadedChild::Legacy {
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

        mother_crate::daemon_bootstrap::register_loaded_child(
            &mut registry,
            &runtime,
            mother_crate::daemon_bootstrap::LoadedChild::Knowledge {
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
}
