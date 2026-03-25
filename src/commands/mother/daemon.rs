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
    services: mother_crate::services::MotherServices,
    scry_backend: Arc<dyn ScryBackend>,
}

impl ServerState {
    fn new(
        token: String,
        registry: ChildRegistry,
        runtime_store: patina::mother::KnowledgeRuntimeStore,
    ) -> Self {
        Self {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            token,
            registry: Arc::new(registry),
            services: mother_crate::services::MotherServices::new(runtime_store),
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
        self.services.health.child_health_all(&self.registry)
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

    fn secrets_get(&self) -> anyhow::Result<serde_json::Value> {
        self.services.secrets.get()
    }

    fn secrets_cache(&self, payload: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        self.services.secrets.cache(&payload)
    }

    fn secrets_lock(&self) -> anyhow::Result<serde_json::Value> {
        Ok(self.services.secrets.lock())
    }

    fn builtin_spec_dispatch(
        &self,
        request: patina_protocol::SpecDispatchRequest,
    ) -> anyhow::Result<serde_json::Value> {
        let command: patina::spec::SpecCommands = serde_json::from_value(request.command)
            .map_err(|e| anyhow::anyhow!("Invalid spec-manager command payload: {}", e))?;
        patina::spec::execute_command_value(command)
    }

    fn builtin_lake_dispatch(
        &self,
        request: patina_protocol::LakeDispatchRequest,
    ) -> anyhow::Result<serde_json::Value> {
        let command: patina::lake::LakeCommand = serde_json::from_value(request.command)
            .map_err(|e| anyhow::anyhow!("Invalid lake-manager command payload: {}", e))?;
        patina::lake::execute_value(command)
    }

    fn builtin_doctor_run(&self) -> anyhow::Result<patina_protocol::DoctorRunResult> {
        let value = patina::mother::doctor_runtime::execute_value()?;
        let exit_code = value.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0);
        Ok(patina_protocol::DoctorRunResult {
            data: value,
            exit_code,
        })
    }

    fn builtin_secrets_dispatch(
        &self,
        payload: serde_json::Value,
    ) -> mother_crate::http_daemon::HttpResponse {
        mother_crate::secrets_authority_api::dispatch(
            payload,
            &mother_crate::secrets_authority_backend::MotherSecretsAuthorityBackend,
        )
    }
}

fn build_router(state: Arc<ServerState>, require_auth: bool) -> Router {
    let token = state.token.clone();
    let route_table = mother_crate::http_api::build_route_table(state);
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
        let state = Arc::new(ServerState::new(token, registry, runtime.clone()));
        let router = Arc::new(build_router(Arc::clone(&state), true));
        let config = mother_crate::daemon_bootstrap_config::DaemonBootstrapConfig {
            transport: mother_crate::daemon_bootstrap_config::TransportMode::TcpHttp {
                host: host.clone(),
                port: options.port,
                token_path: patina::paths::serve::token_path(),
                token: state.token.clone(),
            },
            legacy_migration: options.legacy_migration,
        };
        return mother_crate::daemon_bootstrap_config::start(
            config,
            mother_crate::daemon_bootstrap_config::DaemonBootstrapRuntime {
                registry: Arc::clone(&state.registry),
                router,
            },
        );
    }

    // Default: UDS path (no TCP, no token needed — file permissions are auth)
    let state = Arc::new(ServerState::new(String::new(), registry, runtime));
    let router = Arc::new(build_router(Arc::clone(&state), false));
    let config = mother_crate::daemon_bootstrap_config::DaemonBootstrapConfig {
        transport: mother_crate::daemon_bootstrap_config::TransportMode::UdsHttp {
            run_dir: patina::paths::serve::run_dir(),
            socket_path: patina::paths::serve::socket_path(),
            pid_path: patina::paths::serve::pid_path(),
        },
        legacy_migration: options.legacy_migration,
    };
    mother_crate::daemon_bootstrap_config::start(
        config,
        mother_crate::daemon_bootstrap_config::DaemonBootstrapRuntime {
            registry: Arc::clone(&state.registry),
            router,
        },
    )
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
