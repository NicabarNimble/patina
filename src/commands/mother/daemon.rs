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
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::Store;

use patina::mother::ChildRequest;

use super::adapters::{RetrievalScryBackend, ScryBackend};
use super::audit;
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
    startup_profile: DaemonStartupProfile,
    rivet_integration: RivetIntegrationProfile,
    pub(super) registry: Arc<ChildRegistry>,
    runtime_store: patina::mother::MotherRuntimeStore,
    startup_store: patina::mother::MotherRuntimeStore,
    memory_soft_limit_bytes: Option<u64>,
    child_warmup_lock: Arc<Mutex<()>>,
    child_warmup_state: Arc<RwLock<mother_crate::http_api::ChildWarmupState>>,
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

struct ServerStateInit {
    token: String,
    startup_profile: DaemonStartupProfile,
    rivet_integration: RivetIntegrationProfile,
    registry: ChildRegistry,
    runtime_store: patina::mother::MotherRuntimeStore,
    startup_store: patina::mother::MotherRuntimeStore,
    federation_runtime: FederationRuntime,
    readiness: Arc<RwLock<mother_crate::runtime::ReadinessState>>,
}

enum LoadedComponent {
    HandleBased,
    Composed,
}

mod composed_bindings {
    pub struct HostState {
        pub wasi: wasmtime_wasi::WasiCtx,
        pub wasi_table: wasmtime::component::ResourceTable,
        pub runtime: mother_crate::MotherRuntimeStore,
        pub config: std::collections::HashMap<String, String>,
    }

    #[derive(Debug, Clone)]
    pub struct StateBucketHandle {
        pub identifier: String,
    }

    impl wasmtime_wasi::WasiView for HostState {
        fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
            wasmtime_wasi::WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.wasi_table,
            }
        }
    }

    wasmtime::component::bindgen!({
        path: "wit/pando",
        world: "catalog-runner",
    });

    impl wasi::logging::logging::Host for HostState {
        fn log(&mut self, level: wasi::logging::logging::Level, context: String, message: String) {
            let level = match level {
                wasi::logging::logging::Level::Trace => "TRACE",
                wasi::logging::logging::Level::Debug => "DEBUG",
                wasi::logging::logging::Level::Info => "INFO",
                wasi::logging::logging::Level::Warn => "WARN",
                wasi::logging::logging::Level::Error => "ERROR",
                wasi::logging::logging::Level::Critical => "CRITICAL",
            };
            eprintln!("[pando:{}:{}] {}", context, level, message);
        }
    }

    impl wasi::keyvalue::store::Host for HostState {
        fn open(
            &mut self,
            identifier: String,
        ) -> Result<
            wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            wasi::keyvalue::store::Error,
        > {
            let handle = StateBucketHandle { identifier };
            let rep = self
                .wasi_table
                .push(handle)
                .map_err(|error| wasi::keyvalue::store::Error::Other(error.to_string()))?;
            Ok(wasmtime::component::Resource::new_own(rep.rep()))
        }
    }

    impl wasi::keyvalue::store::HostBucket for HostState {
        fn get(
            &mut self,
            bucket: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            key: String,
        ) -> Result<Option<Vec<u8>>, wasi::keyvalue::store::Error> {
            let rep = wasmtime::component::Resource::<StateBucketHandle>::new_borrow(bucket.rep());
            let handle = self
                .wasi_table
                .get(&rep)
                .map_err(|error| wasi::keyvalue::store::Error::Other(error.to_string()))?;
            let scoped = format!("{}:{}", handle.identifier, key);
            let value = self
                .runtime
                .get_state("pando", &scoped)
                .map_err(|error| wasi::keyvalue::store::Error::Other(error.to_string()))?;
            let decoded = value.map(|raw| {
                serde_json::from_str::<Vec<u8>>(&raw).unwrap_or_else(|_| raw.as_bytes().to_vec())
            });
            Ok(decoded)
        }

        fn set(
            &mut self,
            bucket: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            key: String,
            value: Vec<u8>,
        ) -> Result<(), wasi::keyvalue::store::Error> {
            let rep = wasmtime::component::Resource::<StateBucketHandle>::new_borrow(bucket.rep());
            let handle = self
                .wasi_table
                .get(&rep)
                .map_err(|error| wasi::keyvalue::store::Error::Other(error.to_string()))?;
            let scoped = format!("{}:{}", handle.identifier, key);
            let encoded = serde_json::to_string(&value)
                .map_err(|error| wasi::keyvalue::store::Error::Other(error.to_string()))?;
            self.runtime
                .put_state("pando", &scoped, &encoded)
                .map_err(|error| wasi::keyvalue::store::Error::Other(error.to_string()))
        }

        fn exists(
            &mut self,
            bucket: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
            key: String,
        ) -> Result<bool, wasi::keyvalue::store::Error> {
            Ok(self.get(bucket, key)?.is_some())
        }

        fn drop(
            &mut self,
            bucket: wasmtime::component::Resource<wasi::keyvalue::store::Bucket>,
        ) -> anyhow::Result<()> {
            let rep = wasmtime::component::Resource::<StateBucketHandle>::new_own(bucket.rep());
            let _ = self.wasi_table.delete(rep)?;
            Ok(())
        }
    }

    impl patina::measure::measure::Host for HostState {
        fn emit(&mut self, metric: patina::measure::measure::Metric) -> Result<(), String> {
            eprintln!(
                "[pando:measure] {}={} {:?}",
                metric.name, metric.value, metric.labels
            );
            Ok(())
        }

        fn gauge(&mut self, name: String, value: f64) -> Result<(), String> {
            eprintln!("[pando:measure] {}={}", name, value);
            Ok(())
        }

        fn counter(&mut self, name: String, delta: f64) -> Result<(), String> {
            eprintln!("[pando:measure] {}+={}", name, delta);
            Ok(())
        }
    }

    impl patina::config::config::Host for HostState {
        fn get(&mut self, key: String) -> Result<String, String> {
            self.config
                .get(&key)
                .cloned()
                .ok_or_else(|| format!("missing config key '{}'", key))
        }
    }

    impl patina::records::types::Host for HostState {}
}

mod composition;
mod dispatch;
mod health;
mod startup;
mod transport;

pub use startup::{run_server, DaemonOptions, DaemonStartupProfile, RivetIntegrationProfile};

#[cfg(test)]
use health::installed_child_names_from_dir;
#[cfg(test)]
use startup::run_startup_stage;

impl ServerState {
    fn new(init: ServerStateInit) -> Self {
        let ServerStateInit {
            token,
            startup_profile,
            rivet_integration,
            registry,
            runtime_store,
            startup_store,
            federation_runtime,
            readiness,
        } = init;
        let services_store = runtime_store.clone();
        let child_warmup_mode = if startup_profile.auto_warmup() {
            "auto"
        } else {
            "manual"
        };
        let state = Self {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            token,
            startup_profile,
            rivet_integration,
            registry: Arc::new(registry),
            runtime_store,
            startup_store,
            memory_soft_limit_bytes: health::resolve_memory_soft_limit_bytes(),
            child_warmup_lock: Arc::new(Mutex::new(())),
            child_warmup_state: Arc::new(RwLock::new(mother_crate::http_api::ChildWarmupState {
                mode: child_warmup_mode.to_string(),
                state: "pending".to_string(),
                last_error: None,
            })),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::mother::federation;
    use patina::mother::{Child, ChildHealth, ChildRequest, ChildResponse, MotherHost};
    use serde_json::Value;
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;

    fn with_temp_project<T>(f: impl FnOnce(&Path) -> T) -> T {
        let _guard = patina::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp = tempfile::TempDir::new().unwrap();
        let project_root = temp.path().join("project");
        std::fs::create_dir_all(&project_root).unwrap();

        let old_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&project_root).unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&project_root)));
        std::env::set_current_dir(old_cwd).unwrap();

        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    fn latest_grant_payload() -> Value {
        let conn = patina::eventlog::open_events_db().expect("open events db");
        let data: String = conn
            .query_row(
                "SELECT data FROM eventlog WHERE event_type = 'mother.grant' ORDER BY seq DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("query mother.grant event");
        serde_json::from_str(&data).expect("parse mother.grant payload")
    }

    struct StubKnowledge;

    struct NamedStubKnowledge {
        name: String,
    }

    impl Child for NamedStubKnowledge {
        fn name(&self) -> &str {
            &self.name
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

    fn fixture_wasm_path(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/mother-typed")
            .join(name)
    }

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

    struct TypedDispatchChild;

    impl Child for TypedDispatchChild {
        fn name(&self) -> &str {
            "rivet-dispatch-child"
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

        fn call(&self, request: &patina::mother::ChildCallRequest) -> Result<ChildResponse> {
            Ok(ChildResponse {
                payload: serde_json::json!({
                    "typed": true,
                    "operation_id": request.operation_id,
                }),
            })
        }
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
        assert_eq!(options.profile, DaemonStartupProfile::Full);
        assert_eq!(options.rivet, RivetIntegrationProfile::Disabled);
    }

    #[test]
    fn rivet_dispatch_denied_when_profile_disabled() {
        with_temp_project(|project_root| {
            let runtime_store = patina::mother::MotherRuntimeStore::new(
                project_root.join(".patina/local/data/mother-state.db"),
            );
            let state = ServerState::new(ServerStateInit {
                token: "test-token".to_string(),
                startup_profile: DaemonStartupProfile::Core,
                rivet_integration: RivetIntegrationProfile::Disabled,
                registry: ChildRegistry::new(),
                runtime_store: runtime_store.clone(),
                startup_store: runtime_store.clone(),
                federation_runtime: federation::startup(&runtime_store),
                readiness: Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default())),
            });

            let err = state
                .rivet_dispatch(mother_crate::http_api::RivetDispatchRequest {
                    child: "rivet-dispatch-child".to_string(),
                    operation_id: "patina:watch/control.status".to_string(),
                    args: serde_json::json!([]),
                    correlation: None,
                    delivery: None,
                    dead_letter: None,
                })
                .unwrap_err();
            assert!(
                err.to_string().contains("rivet integration is disabled"),
                "got: {}",
                err
            );
        });
    }

    #[test]
    fn rivet_dispatch_enabled_routes_through_registry_typed_call() {
        with_temp_project(|project_root| {
            let runtime_store = patina::mother::MotherRuntimeStore::new(
                project_root.join(".patina/local/data/mother-state.db"),
            );

            let manifest_path = project_root.join("rivet-dispatch-child.toml");
            std::fs::write(
                &manifest_path,
                r#"[child]
name = "rivet-dispatch-child"
version = "0.1.0"
world = "child"

[child.ingress]
mode = "hybrid"

[child.contract]
allow = ["patina:watch/control.status"]
"#,
            )
            .expect("write manifest");

            let registry = ChildRegistry::new();
            registry
                .register_knowledge_with_paths(
                    Box::new(TypedDispatchChild),
                    std::path::PathBuf::new(),
                    manifest_path,
                )
                .expect("register typed dispatch child");

            let state = ServerState::new(ServerStateInit {
                token: "test-token".to_string(),
                startup_profile: DaemonStartupProfile::Core,
                rivet_integration: RivetIntegrationProfile::Enabled,
                registry,
                runtime_store: runtime_store.clone(),
                startup_store: runtime_store.clone(),
                federation_runtime: federation::startup(&runtime_store),
                readiness: Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default())),
            });

            let response = state
                .rivet_dispatch(mother_crate::http_api::RivetDispatchRequest {
                    child: "rivet-dispatch-child".to_string(),
                    operation_id: "patina:watch/control.status".to_string(),
                    args: serde_json::json!([]),
                    correlation: Some(patina::mother::CallCorrelation {
                        rivet_run_id: Some("run-rivet-1".to_string()),
                        rivet_actor_id: Some("actor-1".to_string()),
                        rivet_workflow_id: None,
                        rivet_job_id: None,
                    }),
                    delivery: None,
                    dead_letter: None,
                })
                .expect("rivet dispatch should use typed path");

            assert_eq!(
                response.get("adapter").and_then(|v| v.as_str()),
                Some("rivet")
            );
            assert_eq!(
                response
                    .get("payload")
                    .and_then(|v| v.get("typed"))
                    .and_then(|v| v.as_bool()),
                Some(true)
            );

            let history = state.registry.typed_call_history(10);
            let first = history.first().expect("typed call observation expected");
            assert_eq!(first.child, "rivet-dispatch-child");
            assert_eq!(first.operation_id, "patina:watch/control.status");
            assert_eq!(
                first
                    .correlation
                    .as_ref()
                    .and_then(|c| c.rivet_run_id.as_deref()),
                Some("run-rivet-1")
            );
        });
    }

    #[test]
    fn rivet_dispatch_required_maps_unknown_child_to_not_found() {
        with_temp_project(|project_root| {
            let runtime_store = patina::mother::MotherRuntimeStore::new(
                project_root.join(".patina/local/data/mother-state.db"),
            );
            let state = ServerState::new(ServerStateInit {
                token: "test-token".to_string(),
                startup_profile: DaemonStartupProfile::Core,
                rivet_integration: RivetIntegrationProfile::Enabled,
                registry: ChildRegistry::new(),
                runtime_store: runtime_store.clone(),
                startup_store: runtime_store.clone(),
                federation_runtime: federation::startup(&runtime_store),
                readiness: Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default())),
            });

            let err = state
                .rivet_dispatch(mother_crate::http_api::RivetDispatchRequest {
                    child: "missing-primary".to_string(),
                    operation_id: "patina:watch/control.status".to_string(),
                    args: serde_json::json!([]),
                    correlation: None,
                    delivery: Some(mother_crate::pando::PandoDeliveryPolicy::Required),
                    dead_letter: None,
                })
                .unwrap_err();
            assert!(err.to_string().contains("child_not_found: missing-primary"));
        });
    }

    #[test]
    fn rivet_dispatch_best_effort_skips_primary_error() {
        with_temp_project(|project_root| {
            let runtime_store = patina::mother::MotherRuntimeStore::new(
                project_root.join(".patina/local/data/mother-state.db"),
            );
            let state = ServerState::new(ServerStateInit {
                token: "test-token".to_string(),
                startup_profile: DaemonStartupProfile::Core,
                rivet_integration: RivetIntegrationProfile::Enabled,
                registry: ChildRegistry::new(),
                runtime_store: runtime_store.clone(),
                startup_store: runtime_store.clone(),
                federation_runtime: federation::startup(&runtime_store),
                readiness: Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default())),
            });

            let response = state
                .rivet_dispatch(mother_crate::http_api::RivetDispatchRequest {
                    child: "missing-primary".to_string(),
                    operation_id: "patina:watch/control.status".to_string(),
                    args: serde_json::json!([]),
                    correlation: None,
                    delivery: Some(mother_crate::pando::PandoDeliveryPolicy::BestEffort),
                    dead_letter: None,
                })
                .expect("best-effort should not fail request");

            assert_eq!(
                response.get("status").and_then(|v| v.as_str()),
                Some("best-effort-skipped")
            );
            assert_eq!(
                response.get("delivery").and_then(|v| v.as_str()),
                Some("best-effort")
            );
        });
    }

    #[test]
    fn rivet_dispatch_dead_letter_reroutes_primary_error() {
        with_temp_project(|project_root| {
            let runtime_store = patina::mother::MotherRuntimeStore::new(
                project_root.join(".patina/local/data/mother-state.db"),
            );

            let manifest_path = project_root.join("rivet-dispatch-child.toml");
            std::fs::write(
                &manifest_path,
                r#"[child]
name = "rivet-dispatch-child"
version = "0.1.0"
world = "child"

[child.ingress]
mode = "hybrid"

[child.contract]
allow = ["patina:watch/control.status"]
"#,
            )
            .expect("write manifest");

            let registry = ChildRegistry::new();
            registry
                .register_knowledge_with_paths(
                    Box::new(TypedDispatchChild),
                    std::path::PathBuf::new(),
                    manifest_path,
                )
                .expect("register typed dispatch child");

            let state = ServerState::new(ServerStateInit {
                token: "test-token".to_string(),
                startup_profile: DaemonStartupProfile::Core,
                rivet_integration: RivetIntegrationProfile::Enabled,
                registry,
                runtime_store: runtime_store.clone(),
                startup_store: runtime_store.clone(),
                federation_runtime: federation::startup(&runtime_store),
                readiness: Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default())),
            });

            let response = state
                .rivet_dispatch(mother_crate::http_api::RivetDispatchRequest {
                    child: "missing-primary".to_string(),
                    operation_id: "patina:watch/control.status".to_string(),
                    args: serde_json::json!([]),
                    correlation: Some(patina::mother::CallCorrelation {
                        rivet_run_id: Some("run-rivet-dead-letter".to_string()),
                        rivet_actor_id: None,
                        rivet_workflow_id: None,
                        rivet_job_id: None,
                    }),
                    delivery: Some(mother_crate::pando::PandoDeliveryPolicy::DeadLetter),
                    dead_letter: Some(mother_crate::http_api::RivetDispatchDeadLetter {
                        child: "rivet-dispatch-child".to_string(),
                        operation_id: None,
                    }),
                })
                .expect("dead-letter should reroute primary failure");

            assert_eq!(
                response.get("status").and_then(|v| v.as_str()),
                Some("dead-letter-delivered")
            );
            assert_eq!(
                response
                    .get("dead_letter")
                    .and_then(|v| v.get("child"))
                    .and_then(|v| v.as_str()),
                Some("rivet-dispatch-child")
            );

            let history = state.registry.typed_call_history(10);
            let first = history.first().expect("typed call observation expected");
            assert_eq!(first.child, "rivet-dispatch-child");
            assert_eq!(
                first
                    .correlation
                    .as_ref()
                    .and_then(|c| c.rivet_run_id.as_deref()),
                Some("run-rivet-dead-letter")
            );
        });
    }

    #[test]
    fn register_loaded_child_loads_knowledge_by_default() {
        let registry = ChildRegistry::new();
        let runtime_root = tempfile::tempdir().unwrap();
        let runtime = patina::mother::MotherRuntimeStore::new_with_project(
            runtime_root.path().join("mother/state.db"),
            mother_crate::state::ProjectUid::new("2bdc808e").unwrap(),
        );

        mother_crate::daemon_bootstrap::register_loaded_child(
            &registry,
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
            children_dir.join("patina_ai_child_parquet_writer.wasm"),
            b"wasm",
        )
        .unwrap();
        std::fs::write(
            children_dir.join("patina_ai_child_parquet_writer.toml"),
            r#"
[child]
name = "parquet-writer"
kind = "child"
"#,
        )
        .unwrap();

        let installed = installed_child_names_from_dir(children_dir);
        assert!(installed.contains("parquet-writer"));
        assert!(!installed.contains("patina_ai_child_parquet_writer"));
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

    #[test]
    fn warmup_children_now_fails_when_operation_in_progress() {
        with_temp_project(|project_root| {
            let runtime_store = patina::mother::MotherRuntimeStore::new(
                project_root.join(".patina/local/data/mother-state.db"),
            );
            let state = ServerState::new(ServerStateInit {
                token: "test-token".to_string(),
                startup_profile: DaemonStartupProfile::Core,
                rivet_integration: RivetIntegrationProfile::Disabled,
                registry: ChildRegistry::new(),
                runtime_store: runtime_store.clone(),
                startup_store: runtime_store.clone(),
                federation_runtime: federation::startup(&runtime_store),
                readiness: Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default())),
            });

            let _guard = state.child_warmup_lock.lock().expect("acquire warmup lock");
            let err = state
                .warmup_children_now()
                .expect_err("expected in-progress error");
            assert!(
                err.to_string()
                    .contains("operation_in_progress: warmup already running"),
                "got: {}",
                err
            );
        });
    }

    #[test]
    fn memory_pressure_guard_is_fail_closed_resource_exhausted() {
        let memory = mother_crate::http_api::MemoryStatus {
            rss_bytes: Some(128),
            max_rss_bytes: Some(128),
            soft_limit_bytes: Some(64),
            pressure: "high".to_string(),
        };

        let err = health::ensure_memory_status_allows_warmup(&memory)
            .expect_err("high pressure should fail closed");
        assert!(err.to_string().contains("resource_exhausted"));
    }

    #[test]
    fn typed_wiring_unknown_from_emits_deny_audit_event() {
        with_temp_project(|project_root| {
            let runtime_store = patina::mother::MotherRuntimeStore::new(
                project_root.join(".patina/local/data/mother-state.db"),
            );
            let state = ServerState::new(ServerStateInit {
                token: "test-token".to_string(),
                startup_profile: DaemonStartupProfile::Full,
                rivet_integration: RivetIntegrationProfile::Disabled,
                registry: ChildRegistry::new(),
                runtime_store: runtime_store.clone(),
                startup_store: runtime_store.clone(),
                federation_runtime: federation::startup(&runtime_store),
                readiness: Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default())),
            });

            let manifest = mother_crate::pando::PandoManifest {
                pando: mother_crate::pando::PandoSection {
                    name: "audit-deny".to_string(),
                    description: "test".to_string(),
                    version: "0.1.0".to_string(),
                },
                children: vec![],
                commands: BTreeMap::new(),
                composition: Some(mother_crate::pando::PandoComposition {
                    wiring: vec![mother_crate::pando::PandoWiring::Typed(
                        mother_crate::pando::PandoTypedWiring {
                            from: "missing-from".to_string(),
                            to: "missing-to".to_string(),
                            toy: "patina:records/transform@0.1.0".to_string(),
                            delivery: None,
                        },
                    )],
                    entry: None,
                    dead_letter: None,
                }),
            };

            let err = state
                .compose_typed_component(&manifest)
                .expect_err("compose should fail for unknown from instance");
            assert!(
                err.to_string().contains("unknown from instance"),
                "unexpected error: {}",
                err
            );

            let payload = latest_grant_payload();
            assert_eq!(payload["scope"], "inside-typed");
            assert_eq!(payload["outcome"], "DENY");
            assert_eq!(payload["from"], "missing-from");
            assert_eq!(payload["to"], "missing-to");
            assert_eq!(
                payload["toy_or_capability"],
                "patina:records/transform@0.1.0"
            );
            assert!(payload["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("unknown from instance"));
        });
    }

    #[test]
    fn typed_wiring_dead_letter_reroutes_when_primary_target_missing() {
        let schema_enforcer_wasm = fixture_wasm_path("schema-enforcer-child.wasm");
        let schema_transform_wasm = fixture_wasm_path("se-pando-adapter.wasm");
        assert!(
            schema_enforcer_wasm.exists(),
            "missing fixture {}",
            schema_enforcer_wasm.display()
        );
        assert!(
            schema_transform_wasm.exists(),
            "missing fixture {}",
            schema_transform_wasm.display()
        );

        with_temp_project(|project_root| {
            let runtime_store = patina::mother::MotherRuntimeStore::new(
                project_root.join(".patina/local/data/mother-state.db"),
            );

            let registry = ChildRegistry::new();
            registry
                .register_knowledge_with_paths(
                    Box::new(NamedStubKnowledge {
                        name: "schema-enforcer".to_string(),
                    }),
                    schema_enforcer_wasm,
                    std::path::PathBuf::from("schema-enforcer.toml"),
                )
                .unwrap();
            registry
                .register_knowledge_with_paths(
                    Box::new(NamedStubKnowledge {
                        name: "schema-transform".to_string(),
                    }),
                    schema_transform_wasm,
                    std::path::PathBuf::from("schema-transform.toml"),
                )
                .unwrap();

            let state = ServerState::new(ServerStateInit {
                token: "test-token".to_string(),
                startup_profile: DaemonStartupProfile::Full,
                rivet_integration: RivetIntegrationProfile::Disabled,
                registry,
                runtime_store: runtime_store.clone(),
                startup_store: runtime_store.clone(),
                federation_runtime: federation::startup(&runtime_store),
                readiness: Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default())),
            });

            let manifest = mother_crate::pando::PandoManifest {
                pando: mother_crate::pando::PandoSection {
                    name: "audit-dead-letter".to_string(),
                    description: "test".to_string(),
                    version: "0.1.0".to_string(),
                },
                children: vec![
                    mother_crate::pando::PandoChild {
                        name: "schema-enforcer".to_string(),
                        id: Some("se".to_string()),
                    },
                    mother_crate::pando::PandoChild {
                        name: "schema-transform".to_string(),
                        id: Some("dlq".to_string()),
                    },
                ],
                commands: BTreeMap::new(),
                composition: Some(mother_crate::pando::PandoComposition {
                    wiring: vec![mother_crate::pando::PandoWiring::Typed(
                        mother_crate::pando::PandoTypedWiring {
                            from: "se".to_string(),
                            to: "missing-target".to_string(),
                            toy: "patina:records/transform".to_string(),
                            delivery: Some(mother_crate::pando::PandoDeliveryPolicy::DeadLetter),
                        },
                    )],
                    entry: None,
                    dead_letter: Some(mother_crate::pando::PandoDeadLetter {
                        child: "dlq".to_string(),
                        toy: Some("patina:records/transform".to_string()),
                    }),
                }),
            };

            let _composed = state
                .compose_typed_component(&manifest)
                .expect("compose should succeed by rerouting to dead-letter child");

            let payload = latest_grant_payload();
            assert_eq!(payload["scope"], "inside-typed");
            assert_eq!(payload["outcome"], "GRANT");
            assert_eq!(payload["from"], "se");
            assert_eq!(payload["to"], "dlq");
            assert_eq!(payload["toy_or_capability"], "patina:records/transform");
            assert!(payload["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("dead-letter reroute succeeded"));
        });
    }

    #[test]
    fn typed_wiring_success_emits_grant_audit_event() {
        let schema_enforcer_wasm = fixture_wasm_path("schema-enforcer-child.wasm");
        let schema_transform_wasm = fixture_wasm_path("se-pando-adapter.wasm");
        assert!(
            schema_enforcer_wasm.exists(),
            "missing fixture {}",
            schema_enforcer_wasm.display()
        );
        assert!(
            schema_transform_wasm.exists(),
            "missing fixture {}",
            schema_transform_wasm.display()
        );

        with_temp_project(|project_root| {
            let runtime_store = patina::mother::MotherRuntimeStore::new(
                project_root.join(".patina/local/data/mother-state.db"),
            );

            let registry = ChildRegistry::new();
            registry
                .register_knowledge_with_paths(
                    Box::new(NamedStubKnowledge {
                        name: "schema-enforcer".to_string(),
                    }),
                    schema_enforcer_wasm,
                    std::path::PathBuf::from("schema-enforcer.toml"),
                )
                .unwrap();
            registry
                .register_knowledge_with_paths(
                    Box::new(NamedStubKnowledge {
                        name: "schema-transform".to_string(),
                    }),
                    schema_transform_wasm,
                    std::path::PathBuf::from("schema-transform.toml"),
                )
                .unwrap();

            let state = ServerState::new(ServerStateInit {
                token: "test-token".to_string(),
                startup_profile: DaemonStartupProfile::Full,
                rivet_integration: RivetIntegrationProfile::Disabled,
                registry,
                runtime_store: runtime_store.clone(),
                startup_store: runtime_store.clone(),
                federation_runtime: federation::startup(&runtime_store),
                readiness: Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default())),
            });

            let manifest = mother_crate::pando::PandoManifest {
                pando: mother_crate::pando::PandoSection {
                    name: "audit-grant".to_string(),
                    description: "test".to_string(),
                    version: "0.1.0".to_string(),
                },
                children: vec![
                    mother_crate::pando::PandoChild {
                        name: "schema-enforcer".to_string(),
                        id: Some("se".to_string()),
                    },
                    mother_crate::pando::PandoChild {
                        name: "schema-transform".to_string(),
                        id: None,
                    },
                ],
                commands: BTreeMap::new(),
                composition: Some(mother_crate::pando::PandoComposition {
                    wiring: vec![mother_crate::pando::PandoWiring::Typed(
                        mother_crate::pando::PandoTypedWiring {
                            from: "se".to_string(),
                            to: "schema-transform".to_string(),
                            toy: "patina:records/transform".to_string(),
                            delivery: None,
                        },
                    )],
                    entry: None,
                    dead_letter: None,
                }),
            };

            let _composed = state
                .compose_typed_component(&manifest)
                .expect("compose should succeed for known schema parity wiring");

            let payload = latest_grant_payload();
            assert_eq!(payload["scope"], "inside-typed");
            assert_eq!(payload["outcome"], "GRANT");
            assert_eq!(payload["from"], "se");
            assert_eq!(payload["to"], "schema-transform");
            assert_eq!(payload["toy_or_capability"], "patina:records/transform");
            assert!(payload["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("wired via interface"));
        });
    }
}
