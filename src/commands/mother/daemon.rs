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
mod interface_control;
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
mod tests;
