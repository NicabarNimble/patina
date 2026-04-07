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
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
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
    runtime_store: patina::mother::KnowledgeRuntimeStore,
    services: mother_crate::services::MotherServices,
    scry_backend: Arc<dyn ScryBackend>,
    pandos_root: PathBuf,
    pando_registry: Mutex<mother_crate::pando::PandoRegistry>,
    native_commands: Mutex<HashSet<String>>,
    aliases: HashMap<String, String>,
}

impl ServerState {
    fn new(
        token: String,
        registry: ChildRegistry,
        runtime_store: patina::mother::KnowledgeRuntimeStore,
    ) -> Self {
        let services_store = runtime_store.clone();
        let state = Self {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            token,
            registry: Arc::new(registry),
            runtime_store,
            services: mother_crate::services::MotherServices::new(services_store),
            scry_backend: Arc::new(RetrievalScryBackend),
            pandos_root: patina::paths::pando::pandos_dir(),
            pando_registry: Mutex::new(mother_crate::pando::PandoRegistry::default()),
            native_commands: Mutex::new(HashSet::new()),
            aliases: HashMap::new(),
        };

        let _ = state.reload_pando_registry();
        state
    }

    fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    fn installed_children(&self) -> HashSet<String> {
        let children_dir = patina::paths::child::children_dir();
        installed_child_names_from_dir(&children_dir)
    }

    fn live_children(&self) -> HashSet<String> {
        self.registry
            .health_all()
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    fn reload_pando_registry(&self) -> Result<()> {
        let native = self
            .native_commands
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let installed_children = self.installed_children();
        let live_children = self.live_children();
        let registry = mother_crate::pando::build_registry(
            &self.pandos_root,
            &native,
            &self.aliases,
            &installed_children,
            &live_children,
        )?;
        *self
            .pando_registry
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = registry;
        Ok(())
    }

    fn current_pando_state(&self) -> patina_protocol::PandoRegistryState {
        let registry = self
            .pando_registry
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        patina_protocol::PandoRegistryState {
            protocol_version: patina_protocol::PANDO_REGISTRY_PROTOCOL_VERSION,
            pandos: registry
                .pandos
                .into_iter()
                .map(|entry| patina_protocol::PandoStateEntry {
                    name: entry.name,
                    status: match entry.status {
                        mother_crate::pando::PandoLifecycleStatus::Registered => {
                            patina_protocol::PandoStatus::Registered
                        }
                        mother_crate::pando::PandoLifecycleStatus::Ready => {
                            patina_protocol::PandoStatus::Ready
                        }
                        mother_crate::pando::PandoLifecycleStatus::Live => {
                            patina_protocol::PandoStatus::Live
                        }
                        mother_crate::pando::PandoLifecycleStatus::Degraded => {
                            patina_protocol::PandoStatus::Degraded
                        }
                        mother_crate::pando::PandoLifecycleStatus::Error => {
                            patina_protocol::PandoStatus::Error
                        }
                    },
                    commands: entry.commands,
                    aliases: entry.aliases,
                    child_count: entry.child_count,
                })
                .collect(),
        }
    }
}

fn read_current_project_uid() -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let uid = std::fs::read_to_string(patina::paths::project::uid_path(&cwd)).ok()?;
    let uid = uid.trim();
    if uid.len() == 8
        && uid
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        Some(uid.to_string())
    } else {
        None
    }
}

fn file_size_if_exists(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.len())
}

fn installed_child_names_from_dir(children_dir: &Path) -> HashSet<String> {
    if !children_dir.exists() {
        return HashSet::new();
    }

    let mut installed = HashSet::new();
    if let Ok(entries) = std::fs::read_dir(children_dir) {
        for entry in entries.flatten() {
            let wasm_path = entry.path();
            if wasm_path.extension().and_then(|ext| ext.to_str()) != Some("wasm") {
                continue;
            }
            let manifest_path = wasm_path.with_extension("toml");
            if !manifest_path.exists() {
                continue;
            }

            match patina::child::engine::ChildManifest::from_path(&manifest_path) {
                Ok(manifest) => {
                    installed.insert(manifest.name);
                }
                Err(error) => {
                    tracing::warn!(
                        manifest_path = %manifest_path.display(),
                        %error,
                        "failed to parse child manifest for installed-child identity"
                    );
                }
            }
        }
    }

    installed
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

    fn health_details(&self) -> anyhow::Result<mother_crate::http_api::HealthDetails> {
        let registered_projects = self.runtime_store.list_registered_projects()?;
        let state_db_bytes = file_size_if_exists(self.runtime_store.path());
        let active_project_uid = read_current_project_uid();

        let active_project_databases = active_project_uid.as_ref().and_then(|uid| {
            let state_parent = self.runtime_store.path().parent()?;
            let project_dir = state_parent.join("projects").join(uid);
            Some(mother_crate::http_api::ProjectDatabases {
                events_db_bytes: file_size_if_exists(&project_dir.join("events.db")),
                patina_db_bytes: file_size_if_exists(&project_dir.join("patina.db")),
                runtime_db_bytes: file_size_if_exists(&project_dir.join("runtime.db")),
            })
        });

        Ok(mother_crate::http_api::HealthDetails {
            registered_projects: registered_projects.len(),
            active_project_uid,
            active_project_databases,
            state_db_bytes,
        })
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

    fn pando_registry_init(
        &self,
        request: patina_protocol::PandoRegistryInit,
    ) -> anyhow::Result<patina_protocol::PandoRegistryState> {
        if request.protocol_version != patina_protocol::PANDO_REGISTRY_PROTOCOL_VERSION {
            anyhow::bail!(
                "Mother protocol v{} incompatible with binary v{} — upgrade patina",
                patina_protocol::PANDO_REGISTRY_PROTOCOL_VERSION,
                request.binary_version
            );
        }

        *self
            .native_commands
            .lock()
            .unwrap_or_else(|e| e.into_inner()) =
            request.native_commands.into_iter().collect::<HashSet<_>>();

        self.reload_pando_registry()?;
        Ok(self.current_pando_state())
    }

    fn pando_list(&self) -> anyhow::Result<patina_protocol::PandoRegistryState> {
        self.reload_pando_registry()?;
        Ok(self.current_pando_state())
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
}

fn run_startup_stage<T, F>(
    stage: &'static str,
    startup_store: &patina::mother::KnowledgeRuntimeStore,
    operation: F,
) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let _ = startup_store.record_startup_attempt(stage, "running", None);
    tracing::info!(
        stage,
        event = "startup.stage.begin",
        "mother startup stage begin"
    );
    let started = Instant::now();
    match operation() {
        Ok(value) => {
            tracing::info!(
                stage,
                event = "startup.stage.success",
                duration_ms = started.elapsed().as_millis() as u64,
                "mother startup stage success"
            );
            Ok(value)
        }
        Err(error) => {
            let _ = startup_store.record_startup_attempt(stage, "failed", Some(&error.to_string()));
            tracing::warn!(
                stage,
                event = "startup.stage.failure",
                duration_ms = started.elapsed().as_millis() as u64,
                error = %error,
                "mother startup stage failure"
            );
            let log_path = patina::paths::patina_home().join("mother/logs/mother.jsonl");
            eprintln!("Mother startup failed at stage '{}' ({})", stage, error);
            eprintln!("See logs: {}", log_path.display());
            Err(error)
        }
    }
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
    mother_crate::daemon_bootstrap_config::ensure_logging_initialized()?;

    // Build and load child registry
    let mut registry = ChildRegistry::new();
    let runtime = patina::mother::KnowledgeRuntimeStore::default();
    let startup_store = patina::mother::KnowledgeRuntimeStore::default();

    // WASM children (discovered from ~/.patina/children/)
    let children_dir = patina::paths::child::children_dir();
    run_startup_stage("child_discovery", &startup_store, || {
        mother_crate::daemon_bootstrap::load_children_from_dir(
            &children_dir,
            &mut registry,
            &runtime,
            super::loader::load_wasm_child,
        );
        Ok(())
    })?;

    let daemon_host = DaemonHost;
    run_startup_stage("registry_load_all", &startup_store, || {
        registry.load_all(&daemon_host)
    })?;

    // TCP opt-in path (--host flag) — requires bearer token
    if let Some(ref host) = options.host {
        let (state, router) = run_startup_stage("router_build", &startup_store, || {
            let token = std::env::var("PATINA_SERVE_TOKEN").unwrap_or_else(|_| generate_token());
            let state = Arc::new(ServerState::new(token, registry, runtime.clone()));
            let router = Arc::new(build_router(Arc::clone(&state), true));
            Ok((state, router))
        })?;
        let config = mother_crate::daemon_bootstrap_config::DaemonBootstrapConfig {
            transport: mother_crate::daemon_bootstrap_config::TransportMode::TcpHttp {
                host: host.clone(),
                port: options.port,
                token_path: patina::paths::serve::token_path(),
                token: state.token.clone(),
            },
            max_connections: mother_crate::daemon_bootstrap_config::DEFAULT_MAX_CONNECTIONS,
            wal_checkpoint_interval_secs:
                mother_crate::daemon_bootstrap_config::DEFAULT_WAL_CHECKPOINT_INTERVAL_SECS,
        };
        return run_startup_stage("transport_bootstrap", &startup_store, || {
            mother_crate::daemon_bootstrap_config::start(
                config.clone(),
                mother_crate::daemon_bootstrap_config::DaemonBootstrapRuntime {
                    registry: Arc::clone(&state.registry),
                    router: Arc::clone(&router),
                },
            )
        });
    }

    // Default: UDS path (no TCP, no token needed — file permissions are auth)
    let (state, router) = run_startup_stage("router_build", &startup_store, || {
        let state = Arc::new(ServerState::new(String::new(), registry, runtime));
        let router = Arc::new(build_router(Arc::clone(&state), false));
        Ok((state, router))
    })?;
    let config = mother_crate::daemon_bootstrap_config::DaemonBootstrapConfig {
        transport: mother_crate::daemon_bootstrap_config::TransportMode::UdsHttp {
            run_dir: patina::paths::serve::run_dir(),
            socket_path: patina::paths::serve::socket_path(),
            pid_path: patina::paths::serve::pid_path(),
        },
        max_connections: mother_crate::daemon_bootstrap_config::DEFAULT_MAX_CONNECTIONS,
        wal_checkpoint_interval_secs:
            mother_crate::daemon_bootstrap_config::DEFAULT_WAL_CHECKPOINT_INTERVAL_SECS,
    };
    run_startup_stage("transport_bootstrap", &startup_store, || {
        mother_crate::daemon_bootstrap_config::start(
            config.clone(),
            mother_crate::daemon_bootstrap_config::DaemonBootstrapRuntime {
                registry: Arc::clone(&state.registry),
                router: Arc::clone(&router),
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use patina::mother::{Child, ChildHealth, ChildRequest, ChildResponse, MotherHost};

    struct StubKnowledge;

    impl Child for StubKnowledge {
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
    fn daemon_options_default() {
        let options = DaemonOptions::default();
        assert_eq!(options.port, 50051);
        assert!(options.host.is_none());
    }

    #[test]
    fn register_loaded_child_loads_knowledge_by_default() {
        let mut registry = ChildRegistry::new();
        let runtime_root = tempfile::tempdir().unwrap();
        let runtime = patina::mother::KnowledgeRuntimeStore::new_with_project(
            runtime_root.path().join("mother/state.db"),
            mother_crate::state::ProjectUid::new("2bdc808e").unwrap(),
        );

        mother_crate::daemon_bootstrap::register_loaded_child(
            &mut registry,
            &runtime,
            mother_crate::daemon_bootstrap::LoadedChild::Knowledge {
                child: Box::new(StubKnowledge),
                name: "knowledge".into(),
                subscribed_streams: vec!["belief.changed".into()],
                relationship_listens: vec![],
            },
        )
        .unwrap();

        assert_eq!(registry.knowledge_len(), 1);
    }

    #[test]
    fn startup_stage_failure_is_persisted_for_status_surface() {
        let temp = tempfile::tempdir().unwrap();
        let startup_store =
            patina::mother::KnowledgeRuntimeStore::new(temp.path().join("state.db"));

        let err = run_startup_stage::<(), _>("unit_test_stage", &startup_store, || {
            anyhow::bail!("intentional startup failure")
        })
        .unwrap_err();

        assert!(err.to_string().contains("intentional startup failure"));

        let failure = startup_store.last_startup_failure().unwrap().unwrap();
        assert_eq!(failure.stage, "unit_test_stage");
        assert_eq!(failure.status, "failed");
        assert!(failure
            .error_excerpt
            .as_deref()
            .unwrap_or_default()
            .contains("intentional startup failure"));
    }

    #[test]
    fn successful_stage_does_not_create_failure_record() {
        let temp = tempfile::tempdir().unwrap();
        let startup_store =
            patina::mother::KnowledgeRuntimeStore::new(temp.path().join("state.db"));

        run_startup_stage("unit_test_stage_success", &startup_store, || Ok(())).unwrap();

        assert!(startup_store.last_startup_failure().unwrap().is_none());
    }

    #[test]
    fn installed_children_use_manifest_name_not_wasm_stem() {
        let temp = tempfile::tempdir().unwrap();
        let children_dir = temp.path();
        std::fs::write(
            children_dir.join("patina_ai_child_record_writer.wasm"),
            b"wasm",
        )
        .unwrap();
        std::fs::write(
            children_dir.join("patina_ai_child_record_writer.toml"),
            r#"
[child]
name = "record-writer"
kind = "child"
"#,
        )
        .unwrap();

        let installed = installed_child_names_from_dir(children_dir);
        assert!(installed.contains("record-writer"));
        assert!(!installed.contains("patina_ai_child_record_writer"));
    }
}
