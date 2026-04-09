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
use chrono::Utc;
use rusqlite::params;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::sync::TryLockError;
use std::time::{Duration, Instant};
use wac_graph::{types::Package, CompositionGraph, EncodeOptions};

use patina::mother::ChildRequest;

use super::adapters::{RetrievalScryBackend, ScryBackend};
use super::federation::{FederationAvailability, FederationQueryResult, FederationRuntime};
use super::registry::ChildRegistry;
use mother_crate::http_api::ApiRuntime;
use mother_crate::http_routes::Router;
use mother_crate::runtime::MotherRuntime;

// === Server state ===

/// Server state shared across request handlers
pub struct ServerState {
    start_time: Instant,
    version: String,
    token: String,
    pub(super) registry: Arc<ChildRegistry>,
    runtime_store: patina::mother::MotherRuntimeStore,
    services: mother_crate::services::MotherServices,
    scry_backend: Arc<dyn ScryBackend>,
    federation_runtime: Mutex<FederationRuntime>,
    pandos_root: PathBuf,
    pando_registry: Mutex<mother_crate::pando::PandoRegistry>,
    native_commands: Mutex<HashSet<String>>,
    refresh_lock: Mutex<()>,
    aliases: HashMap<String, String>,
    readiness: Arc<RwLock<mother_crate::runtime::ReadinessState>>,
}

impl ServerState {
    fn new(
        token: String,
        registry: ChildRegistry,
        runtime_store: patina::mother::MotherRuntimeStore,
        federation_runtime: FederationRuntime,
        readiness: Arc<RwLock<mother_crate::runtime::ReadinessState>>,
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
            federation_runtime: Mutex::new(federation_runtime),
            pandos_root: patina::paths::pando::pandos_dir(),
            pando_registry: Mutex::new(mother_crate::pando::PandoRegistry::default()),
            native_commands: Mutex::new(HashSet::new()),
            refresh_lock: Mutex::new(()),
            aliases: HashMap::new(),
            readiness,
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

    fn typed_interface_candidates(interface_name: &str) -> Vec<String> {
        if interface_name.contains('@') {
            vec![interface_name.to_string()]
        } else {
            vec![
                interface_name.to_string(),
                format!("{}@0.1.0", interface_name),
            ]
        }
    }

    fn resolve_child_instance<'a>(
        manifest: &'a mother_crate::pando::PandoManifest,
        instance: &str,
    ) -> Option<&'a mother_crate::pando::PandoChild> {
        manifest
            .children
            .iter()
            .find(|child| child.id.as_deref().unwrap_or(&child.name) == instance)
    }

    fn validate_typed_composition(
        &self,
        manifest: &mother_crate::pando::PandoManifest,
    ) -> Result<()> {
        let Some(composition) = &manifest.composition else {
            return Ok(());
        };

        let typed_rules = composition
            .wiring
            .iter()
            .filter_map(|rule| match rule {
                mother_crate::pando::PandoWiring::Typed(typed) => Some(typed),
                mother_crate::pando::PandoWiring::Legacy(_) => None,
            })
            .collect::<Vec<_>>();
        if typed_rules.is_empty() {
            return Ok(());
        }

        let mut graph = CompositionGraph::new();
        let mut instance_ids = HashMap::new();

        for child in &manifest.children {
            let instance_name = child.id.as_deref().unwrap_or(&child.name);
            let (wasm_path, _) = self.registry.child_paths(&child.name).ok_or_else(|| {
                anyhow::anyhow!(
                    "typed composition child '{}' is not installed/registered",
                    child.name
                )
            })?;

            let package_name = format!(
                "patina:{}:{}",
                manifest.pando.name,
                child.id.as_deref().unwrap_or(&child.name)
            );
            let package = Package::from_file(&package_name, None, &wasm_path, graph.types_mut())
                .map_err(|e| anyhow::anyhow!("loading typed child '{}': {}", child.name, e))?;
            let package_id = graph
                .register_package(package)
                .map_err(|e| anyhow::anyhow!("registering typed child '{}': {}", child.name, e))?;
            let instance_id = graph.instantiate(package_id);
            instance_ids.insert(instance_name.to_string(), instance_id);
        }

        for rule in typed_rules {
            let Some(from_id) = instance_ids.get(&rule.from) else {
                anyhow::bail!(
                    "typed composition wiring references unknown from instance '{}'",
                    rule.from
                );
            };
            let Some(to_id) = instance_ids.get(&rule.to) else {
                anyhow::bail!(
                    "typed composition wiring references unknown to instance '{}'",
                    rule.to
                );
            };

            let candidates = Self::typed_interface_candidates(&rule.toy);
            let mut wired = false;
            for candidate in &candidates {
                let export_id = match graph.alias_instance_export(*from_id, candidate) {
                    Ok(id) => id,
                    Err(_) => continue,
                };
                if graph
                    .set_instantiation_argument(*to_id, candidate, export_id)
                    .is_ok()
                {
                    wired = true;
                    break;
                }
            }
            if !wired {
                let from_child = Self::resolve_child_instance(manifest, &rule.from)
                    .map(|child| child.name.clone())
                    .unwrap_or_else(|| rule.from.clone());
                let to_child = Self::resolve_child_instance(manifest, &rule.to)
                    .map(|child| child.name.clone())
                    .unwrap_or_else(|| rule.to.clone());
                anyhow::bail!(
                    "typed wiring failed for '{}' -> '{}' on '{}' (from child '{}', to child '{}')",
                    rule.from,
                    rule.to,
                    rule.toy,
                    from_child,
                    to_child
                );
            }
        }

        if let Some(entry) = &composition.entry {
            let Some(entry_id) = instance_ids.get(&entry.child) else {
                anyhow::bail!(
                    "typed composition entry references unknown child instance '{}'",
                    entry.child
                );
            };
            let candidates = Self::typed_interface_candidates(&entry.toy);
            let mut exported = false;
            for candidate in &candidates {
                let export_id = match graph.alias_instance_export(*entry_id, candidate) {
                    Ok(id) => id,
                    Err(_) => continue,
                };
                if graph.export(export_id, candidate).is_ok() {
                    exported = true;
                    break;
                }
            }
            if !exported {
                anyhow::bail!(
                    "typed composition entry export failed for instance '{}' toy '{}'",
                    entry.child,
                    entry.toy
                );
            }
        }

        graph.encode(EncodeOptions::default()).map_err(|e| {
            anyhow::anyhow!(
                "encoding typed composition '{}': {}",
                manifest.pando.name,
                e
            )
        })?;

        Ok(())
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

        let federation_status = self
            .federation_runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status()
            .clone();
        let readiness = self
            .readiness
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        Ok(mother_crate::http_api::HealthDetails {
            registered_projects: registered_projects.len(),
            active_project_uid,
            active_project_databases,
            state_db_bytes,
            federation_available: matches!(
                federation_status.availability,
                FederationAvailability::Available
            ),
            federation_reason: match &federation_status.availability {
                FederationAvailability::Available => None,
                FederationAvailability::Unavailable { reason } => Some(reason.clone()),
            },
            federation_ducklake_loaded: federation_status.ducklake_loaded,
            federation_projects_attached: federation_status.attached_count(),
            federation_projects_failed: federation_status.failed_count(),
            federation_projects_stale: federation_status.stale_count(),
            control_plane_ready: readiness.control_plane_ready,
            children_ready_count: readiness.children_ready_count,
            children_total: readiness.children_total,
            children_degraded: readiness
                .children_degraded
                .iter()
                .map(|entry| mother_crate::http_api::DegradedChild {
                    name: entry.name.clone(),
                    reason: entry.reason.clone(),
                })
                .collect(),
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

    fn lifecycle_load_pando(&self, name: &str) -> anyhow::Result<mother_crate::PandoLoadResult> {
        <Self as MotherRuntime>::load_pando(self, name)
    }

    fn lifecycle_refresh(&self) -> anyhow::Result<mother_crate::PandoRefreshResult> {
        <Self as MotherRuntime>::refresh_pandos(self)
    }

    fn lifecycle_reload_child(
        &self,
        name: &str,
    ) -> anyhow::Result<mother_crate::ChildReloadResult> {
        <Self as MotherRuntime>::reload_child(self, name)
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

    fn federation_status(&self) -> anyhow::Result<serde_json::Value> {
        let runtime = self
            .federation_runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        Ok(runtime.status_json())
    }

    fn federation_refresh(&self) -> anyhow::Result<serde_json::Value> {
        let mut runtime = self
            .federation_runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        runtime.refresh(&self.runtime_store);
        Ok(runtime.status_json())
    }

    fn federation_query(
        &self,
        payload: mother_crate::protocol::FederationQueryPayload,
    ) -> anyhow::Result<serde_json::Value> {
        let runtime = self
            .federation_runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let result = runtime.execute_query(
            &payload.sql,
            &payload.params,
            payload.limit.unwrap_or(1000),
            payload.timeout_ms.unwrap_or(30_000),
        );
        Ok(match result {
            FederationQueryResult::Success(_) => result.into_json(),
            FederationQueryResult::Error(_) => result.into_json(),
        })
    }
}

impl MotherRuntime for ServerState {
    fn load_pando(&self, name: &str) -> Result<mother_crate::runtime::PandoLoadResult> {
        let started = Instant::now();
        let result = (|| {
            let manifest_path = self.pandos_root.join(name).join("pando.toml");
            if !manifest_path.exists() {
                anyhow::bail!("pando_not_found: no pando named '{}'", name);
            }
            super::integrity::verify_pando_integrity(&self.pandos_root.join(name))?;
            let manifest = mother_crate::pando::parse_manifest_path(&manifest_path)
                .map_err(|e| anyhow::anyhow!("invalid_request: {}", e))?;
            self.validate_typed_composition(&manifest)?;
            let mut children_activated = 0usize;
            for child in manifest.children {
                let reload = self.reload_child(&child.name)?;
                if reload.status == "reloaded" {
                    children_activated += 1;
                }
            }
            self.reload_pando_registry()?;

            Ok(mother_crate::runtime::PandoLoadResult {
                pando: name.to_string(),
                status: "loaded".to_string(),
                children_activated,
            })
        })();

        emit_lifecycle_metric(
            "load_pando_latency_ms",
            "gauge",
            started.elapsed().as_millis() as f64,
            &[("action", "load_pando"), ("pando", name)],
        );
        if result.is_err() {
            emit_lifecycle_metric(
                "load_pando_failure",
                "counter",
                1.0,
                &[("action", "load_pando"), ("pando", name)],
            );
        }

        result
    }

    fn refresh_pandos(&self) -> Result<mother_crate::runtime::PandoRefreshResult> {
        let started = Instant::now();
        let result = (|| {
            let _refresh_guard = match self.refresh_lock.try_lock() {
                Ok(guard) => guard,
                Err(TryLockError::WouldBlock) => {
                    anyhow::bail!("operation_in_progress: refresh already running")
                }
                Err(TryLockError::Poisoned(_)) => {
                    anyhow::bail!("internal_error: refresh lock poisoned")
                }
            };

            let mut pandos_loaded = 0usize;
            let mut pandos_failed = 0usize;
            let mut children_activated = 0usize;
            let mut children_failed = 0usize;
            let mut degraded = Vec::new();

            if self.pandos_root.exists() {
                let mut dirs = std::fs::read_dir(&self.pandos_root)?
                    .flatten()
                    .filter(|entry| entry.path().is_dir())
                    .collect::<Vec<_>>();
                dirs.sort_by_key(|entry| entry.file_name());

                for dir in dirs {
                    let manifest_path = dir.path().join("pando.toml");
                    if !manifest_path.exists() {
                        continue;
                    }
                    if super::integrity::verify_pando_integrity(&dir.path()).is_err() {
                        pandos_failed += 1;
                        continue;
                    }
                    let manifest = match mother_crate::pando::parse_manifest_path(&manifest_path) {
                        Ok(m) => m,
                        Err(_) => {
                            pandos_failed += 1;
                            continue;
                        }
                    };
                    if self.validate_typed_composition(&manifest).is_err() {
                        pandos_failed += 1;
                        continue;
                    }
                    pandos_loaded += 1;
                    for child in manifest.children {
                        match self.reload_child(&child.name) {
                            Ok(outcome) if outcome.status == "reloaded" => {
                                children_activated += 1;
                            }
                            Ok(outcome) => {
                                children_failed += 1;
                                degraded.push(mother_crate::runtime::DegradedChild {
                                    name: child.name,
                                    reason: outcome
                                        .reason
                                        .unwrap_or_else(|| "reload failed".to_string()),
                                });
                            }
                            Err(error) => {
                                children_failed += 1;
                                degraded.push(mother_crate::runtime::DegradedChild {
                                    name: child.name,
                                    reason: error.to_string(),
                                });
                            }
                        }
                    }
                }
            }

            self.reload_pando_registry()?;

            Ok(mother_crate::runtime::PandoRefreshResult {
                pandos_loaded,
                pandos_failed,
                children_activated,
                children_failed,
                degraded,
            })
        })();

        emit_lifecycle_metric(
            "refresh_latency_ms",
            "gauge",
            started.elapsed().as_millis() as f64,
            &[("action", "refresh")],
        );

        result
    }

    fn reload_child(&self, name: &str) -> Result<mother_crate::runtime::ChildReloadResult> {
        let started = Instant::now();
        let result = (|| {
            let reload_lock = self
                .registry
                .child_reload_lock(name)
                .ok_or_else(|| anyhow::anyhow!("child_not_found: no child named '{}'", name))?;
            let _guard = match reload_lock.try_lock() {
                Ok(guard) => guard,
                Err(TryLockError::WouldBlock) => {
                    anyhow::bail!(
                        "operation_in_progress: reload already running for '{}'",
                        name
                    )
                }
                Err(TryLockError::Poisoned(_)) => {
                    anyhow::bail!("internal_error: reload lock poisoned for '{}'", name)
                }
            };

            let (wasm_path, manifest_path) = self
                .registry
                .child_paths(name)
                .ok_or_else(|| anyhow::anyhow!("child_not_found: no child named '{}'", name))?;

            let loaded = super::loader::load_wasm_child(&wasm_path, &manifest_path)?;
            let mut replacement = match loaded {
                mother_crate::daemon_bootstrap::LoadedChild::Knowledge {
                    child,
                    name: loaded_name,
                    ..
                } => {
                    if loaded_name != name {
                        anyhow::bail!(
                            "internal_error: manifest child '{}' does not match reload target '{}'",
                            loaded_name,
                            name
                        );
                    }
                    child
                }
            };

            let daemon_host = DaemonHost;
            if let Err(error) = replacement.on_load(&daemon_host) {
                return Ok(mother_crate::runtime::ChildReloadResult {
                    child: name.to_string(),
                    status: "reload_failed".to_string(),
                    previous_instance: "active".to_string(),
                    reason: Some(format!("on_load failed: {}", error)),
                });
            }

            let mut previous = self.registry.swap_knowledge_child(name, replacement)?;
            let _ = previous.drain(64);
            previous.on_unload();

            {
                let mut readiness = self.readiness.write().unwrap_or_else(|e| e.into_inner());
                let degraded_before = readiness.children_degraded.len();
                readiness
                    .children_degraded
                    .retain(|entry| entry.name != name);
                if readiness.children_degraded.len() < degraded_before
                    && readiness.children_ready_count < readiness.children_total
                {
                    readiness.children_ready_count += 1;
                }
            }

            Ok(mother_crate::runtime::ChildReloadResult {
                child: name.to_string(),
                status: "reloaded".to_string(),
                previous_instance: "drained".to_string(),
                reason: None,
            })
        })();

        emit_lifecycle_metric(
            "reload_child_latency_ms",
            "gauge",
            started.elapsed().as_millis() as f64,
            &[("action", "reload_child"), ("child", name)],
        );

        let emit_failure = match &result {
            Ok(payload) => payload.status == "reload_failed",
            Err(_) => true,
        };
        if emit_failure {
            emit_lifecycle_metric(
                "reload_child_failure",
                "counter",
                1.0,
                &[("action", "reload_child"), ("child", name)],
            );
        }

        result
    }

    fn query_readiness(&self) -> mother_crate::runtime::ReadinessState {
        self.readiness
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
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
    startup_store: &patina::mother::MotherRuntimeStore,
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
            let duration_ms = started.elapsed().as_millis() as u64;
            tracing::info!(
                stage,
                event = "startup.stage.success",
                duration_ms,
                "mother startup stage success"
            );
            emit_startup_metric("stage_latency_ms", "gauge", duration_ms as f64, stage);
            Ok(value)
        }
        Err(error) => {
            let _ = startup_store.record_startup_attempt(stage, "failed", Some(&error.to_string()));
            let duration_ms = started.elapsed().as_millis() as u64;
            tracing::warn!(
                stage,
                event = "startup.stage.failure",
                duration_ms,
                error = %error,
                "mother startup stage failure"
            );
            emit_startup_metric("stage_failure", "counter", 1.0, stage);
            let log_path = patina::paths::patina_home().join("mother/logs/mother.jsonl");
            eprintln!("Mother startup failed at stage '{}' ({})", stage, error);
            eprintln!("See logs: {}", log_path.display());
            Err(error)
        }
    }
}

fn emit_startup_metric(name: &str, kind: &str, value: f64, action: &str) {
    emit_startup_metric_with_labels(name, kind, value, &[("action", action)]);
}

fn emit_lifecycle_metric(name: &str, kind: &str, value: f64, labels: &[(&str, &str)]) {
    let mut metric_labels = vec![vec!["scope".to_string(), "lifecycle".to_string()]];
    for (key, value) in labels {
        metric_labels.push(vec![(*key).to_string(), (*value).to_string()]);
    }

    let events_path = match patina::eventlog::events_db_path() {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!(metric = name, %error, "failed to resolve events path for lifecycle metric");
            return;
        }
    };

    let conn = match rusqlite::Connection::open(&events_path) {
        Ok(conn) => conn,
        Err(error) => {
            tracing::warn!(metric = name, path = %events_path.display(), %error, "failed to open events db for lifecycle metric");
            return;
        }
    };

    if let Err(error) = mother_crate::eventlog_schema::prepare_events_db(&conn) {
        tracing::warn!(metric = name, %error, "failed to initialize events schema for lifecycle metric");
        return;
    }

    let payload = serde_json::json!({
        "name": format!("mother:lifecycle:{}", name),
        "kind": kind,
        "value": value,
        "labels": metric_labels,
        "source": "mother",
        "scope": "lifecycle",
    });

    if let Err(error) = conn.execute(
        "INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data, provenance)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "measure.metric",
            Utc::now().to_rfc3339(),
            format!("mother:lifecycle:{}", name),
            Option::<String>::None,
            payload.to_string(),
            "local"
        ],
    ) {
        tracing::warn!(metric = name, %error, "failed to emit lifecycle metric");
    }
}

fn emit_startup_metric_with_labels(name: &str, kind: &str, value: f64, labels: &[(&str, &str)]) {
    let events_path = match patina::eventlog::events_db_path() {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!(metric = name, %error, "failed to resolve events path for startup metric");
            return;
        }
    };

    let conn = match rusqlite::Connection::open(&events_path) {
        Ok(conn) => conn,
        Err(error) => {
            tracing::warn!(metric = name, path = %events_path.display(), %error, "failed to open events db for startup metric");
            return;
        }
    };

    if let Err(error) = mother_crate::eventlog_schema::prepare_events_db(&conn) {
        tracing::warn!(metric = name, %error, "failed to initialize events schema for startup metric");
        return;
    }

    let mut metric_labels = vec![vec!["scope".to_string(), "startup".to_string()]];
    for (key, value) in labels {
        metric_labels.push(vec![(*key).to_string(), (*value).to_string()]);
    }

    let payload = serde_json::json!({
        "name": format!("mother:startup:{}", name),
        "kind": kind,
        "value": value,
        "labels": metric_labels,
        "source": "mother",
        "scope": "startup",
    });

    if let Err(error) = conn.execute(
        "INSERT INTO eventlog (event_type, timestamp, source_id, source_file, data, provenance)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "measure.metric",
            Utc::now().to_rfc3339(),
            format!("mother:startup:{}", name),
            Option::<String>::None,
            payload.to_string(),
            "local"
        ],
    ) {
        tracing::warn!(metric = name, %error, "failed to emit startup metric");
    }
}

fn emit_child_activation_metric(name: &str, kind: &str, value: f64, child: &str) {
    emit_startup_metric_with_labels(
        name,
        kind,
        value,
        &[("action", "child_activate"), ("child", child)],
    );
}

fn set_control_plane_ready(readiness: &Arc<RwLock<mother_crate::runtime::ReadinessState>>) {
    let mut guard = readiness.write().unwrap_or_else(|e| e.into_inner());
    guard.control_plane_ready = true;
}

fn set_children_total(
    readiness: &Arc<RwLock<mother_crate::runtime::ReadinessState>>,
    total: usize,
) {
    let mut guard = readiness.write().unwrap_or_else(|e| e.into_inner());
    guard.children_total = total;
}

fn record_child_activation(
    readiness: &Arc<RwLock<mother_crate::runtime::ReadinessState>>,
    child: &str,
    error: Option<&str>,
) {
    let mut guard = readiness.write().unwrap_or_else(|e| e.into_inner());
    match error {
        Some(reason) => {
            if let Some(existing) = guard
                .children_degraded
                .iter_mut()
                .find(|entry| entry.name == child)
            {
                existing.reason = reason.to_string();
            } else {
                guard
                    .children_degraded
                    .push(mother_crate::runtime::DegradedChild {
                        name: child.to_string(),
                        reason: reason.to_string(),
                    });
            }
        }
        None => {
            guard.children_ready_count += 1;
            guard.children_degraded.retain(|entry| entry.name != child);
        }
    }
}

enum WarmupProbe {
    Uds {
        socket_path: PathBuf,
    },
    Tcp {
        host: String,
        port: u16,
        token: String,
    },
}

fn parse_http_status(response: &[u8]) -> Option<u16> {
    let status_end = response.iter().position(|&b| b == b'\r')?;
    let first_line = std::str::from_utf8(&response[..status_end]).ok()?;
    first_line.split_whitespace().nth(1)?.parse().ok()
}

fn probe_health_uds(socket_path: &Path) -> bool {
    let Ok(mut stream) = UnixStream::connect(socket_path) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    if stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut response = Vec::new();
    if stream.read_to_end(&mut response).is_err() {
        return false;
    }
    parse_http_status(&response) == Some(200)
}

fn probe_health_tcp(host: &str, port: u16, token: &str) -> bool {
    let Ok(mut stream) = TcpStream::connect(format!("{}:{}", host, port)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let auth_header = if token.is_empty() {
        String::new()
    } else {
        format!("Authorization: Bearer {}\r\n", token)
    };
    let request =
        format!("GET /health HTTP/1.1\r\nHost: {host}\r\n{auth_header}Connection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut response = Vec::new();
    if stream.read_to_end(&mut response).is_err() {
        return false;
    }
    parse_http_status(&response) == Some(200)
}

fn wait_for_health_200(probe: &WarmupProbe, timeout: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < timeout {
        let is_ready = match probe {
            WarmupProbe::Uds { socket_path } => probe_health_uds(socket_path),
            WarmupProbe::Tcp { host, port, token } => probe_health_tcp(host, *port, token),
        };
        if is_ready {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

fn spawn_child_warmup(
    startup_store: patina::mother::MotherRuntimeStore,
    runtime: patina::mother::MotherRuntimeStore,
    registry: Arc<ChildRegistry>,
    readiness: Arc<RwLock<mother_crate::runtime::ReadinessState>>,
    probe: WarmupProbe,
) {
    let _ = std::thread::Builder::new()
        .name("mother-child-warmup".to_string())
        .spawn(move || {
            if !wait_for_health_200(&probe, Duration::from_secs(30)) {
                tracing::warn!("health endpoint was not reachable before child warmup");
                return;
            }

            let children_dir = patina::paths::child::children_dir();
            let discovered = match run_startup_stage("child_discovery", &startup_store, || {
                Ok(mother_crate::daemon_bootstrap::load_children_from_dir(
                    &children_dir,
                    registry.as_ref(),
                    &runtime,
                    super::loader::load_wasm_child,
                ))
            }) {
                Ok(count) => count,
                Err(error) => {
                    tracing::warn!(%error, "child discovery failed during background warmup");
                    return;
                }
            };
            set_children_total(&readiness, discovered);

            let daemon_host = DaemonHost;
            let _ = run_startup_stage("registry_load_all", &startup_store, || {
                let activations = registry.activate_all(&daemon_host);
                for activation in activations {
                    emit_child_activation_metric(
                        "child_activation_ms",
                        "gauge",
                        activation.duration_ms as f64,
                        &activation.name,
                    );
                    if let Some(error) = activation.error.as_deref() {
                        emit_child_activation_metric(
                            "child_activation_failure",
                            "counter",
                            1.0,
                            &activation.name,
                        );
                        record_child_activation(&readiness, &activation.name, Some(error));
                    } else {
                        record_child_activation(&readiness, &activation.name, None);
                    }
                }
                Ok(())
            });
        });
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

    // Build control-plane state first; child warmup runs in background.
    let registry = ChildRegistry::new();
    let runtime = patina::mother::MotherRuntimeStore::default();
    let startup_store = patina::mother::MotherRuntimeStore::default();
    let readiness = Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default()));
    run_startup_stage("state_db_open", &startup_store, || {
        startup_store.list_registered_projects().map(|_| ())
    })?;
    let federation_runtime = super::federation::startup(&startup_store);

    // TCP opt-in path (--host flag) — requires bearer token
    if let Some(ref host) = options.host {
        let (state, router) = run_startup_stage("router_build", &startup_store, || {
            let token = std::env::var("PATINA_SERVE_TOKEN").unwrap_or_else(|_| generate_token());
            let state = Arc::new(ServerState::new(
                token,
                registry,
                runtime.clone(),
                federation_runtime,
                Arc::clone(&readiness),
            ));
            let router = Arc::new(build_router(Arc::clone(&state), true));
            Ok((state, router))
        })?;
        set_control_plane_ready(&readiness);
        spawn_child_warmup(
            startup_store.clone(),
            runtime.clone(),
            Arc::clone(&state.registry),
            Arc::clone(&readiness),
            WarmupProbe::Tcp {
                host: host.clone(),
                port: options.port,
                token: state.token.clone(),
            },
        );
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
        let state = Arc::new(ServerState::new(
            String::new(),
            registry,
            runtime.clone(),
            federation_runtime,
            Arc::clone(&readiness),
        ));
        let router = Arc::new(build_router(Arc::clone(&state), false));
        Ok((state, router))
    })?;
    set_control_plane_ready(&readiness);
    let socket_path = patina::paths::serve::socket_path();
    spawn_child_warmup(
        startup_store.clone(),
        runtime,
        Arc::clone(&state.registry),
        Arc::clone(&readiness),
        WarmupProbe::Uds {
            socket_path: socket_path.clone(),
        },
    );
    let config = mother_crate::daemon_bootstrap_config::DaemonBootstrapConfig {
        transport: mother_crate::daemon_bootstrap_config::TransportMode::UdsHttp {
            run_dir: patina::paths::serve::run_dir(),
            socket_path,
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
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;

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

    struct NotifyingChild {
        tx: mpsc::Sender<()>,
    }

    impl Child for NotifyingChild {
        fn name(&self) -> &str {
            "notifying"
        }

        fn on_load(&mut self, _host: &dyn MotherHost) -> Result<()> {
            let _ = self.tx.send(());
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
        let runtime = patina::mother::MotherRuntimeStore::new_with_project(
            runtime_root.path().join("mother/state.db"),
            mother_crate::state::ProjectUid::new("2bdc808e").unwrap(),
        );

        mother_crate::daemon_bootstrap::register_loaded_child(
            &mut registry,
            &runtime,
            mother_crate::daemon_bootstrap::LoadedChild::Knowledge {
                child: Box::new(StubKnowledge),
                name: "knowledge".into(),
                wasm_path: std::path::PathBuf::from("knowledge.wasm"),
                manifest_path: std::path::PathBuf::from("knowledge.toml"),
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
        let startup_store = patina::mother::MotherRuntimeStore::new(temp.path().join("state.db"));

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
        let startup_store = patina::mother::MotherRuntimeStore::new(temp.path().join("state.db"));

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

    #[test]
    fn child_warmup_waits_for_health_before_on_load() {
        let gate = Arc::new(AtomicBool::new(false));
        let gate_for_probe = Arc::clone(&gate);
        let (tx, rx) = mpsc::channel();
        let registry = Arc::new(ChildRegistry::new());
        registry
            .register_knowledge(Box::new(NotifyingChild { tx }))
            .unwrap();

        let probe_thread = std::thread::spawn(move || {
            while !gate_for_probe.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(10));
            }
            let host = DaemonHost;
            let _ = registry.activate_all(&host);
        });

        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "on_load fired before health gate opened"
        );
        gate.store(true, Ordering::SeqCst);
        assert!(
            rx.recv_timeout(Duration::from_secs(1)).is_ok(),
            "on_load did not fire after health gate opened"
        );
        probe_thread.join().unwrap();
    }
}
