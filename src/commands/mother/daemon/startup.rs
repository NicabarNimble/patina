use super::transport::{build_router, spawn_child_warmup, WarmupProbe};
use super::*;
use crate::commands::mother::{federation, loader};

impl ServerState {
    pub(super) fn execute_child_warmup_once(
        &self,
    ) -> Result<mother_crate::runtime::ChildWarmupResult> {
        let discovered = run_startup_stage("child_discovery", &self.startup_store, || {
            Ok(mother_crate::daemon_bootstrap::load_children_from_dir(
                &patina::paths::child::children_dir(),
                self.registry.as_ref(),
                &self.runtime_store,
                loader::load_wasm_child,
            ))
        })?;

        set_children_total(&self.readiness, self.registry.knowledge_len());
        {
            let mut readiness = self.readiness.write().unwrap_or_else(|e| e.into_inner());
            readiness.children_ready_count = 0;
            readiness.children_degraded.clear();
        }

        let daemon_host = DaemonHost;
        let activations = self.registry.activate_all(&daemon_host);
        let mut activated = 0usize;
        let mut failed = 0usize;
        let mut degraded = Vec::new();
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
                failed += 1;
                degraded.push(mother_crate::runtime::DegradedChild {
                    name: activation.name.clone(),
                    reason: error.to_string(),
                });
                record_child_activation(&self.readiness, &activation.name, Some(error));
            } else {
                activated += 1;
                record_child_activation(&self.readiness, &activation.name, None);
            }
        }

        Ok(mother_crate::runtime::ChildWarmupResult {
            status: if failed == 0 {
                "warmed".to_string()
            } else {
                "warmed-with-failures".to_string()
            },
            discovered,
            activated,
            failed,
            degraded,
        })
    }

    pub(super) fn warmup_children_now(&self) -> Result<mother_crate::runtime::ChildWarmupResult> {
        if self
            .child_warmup_state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .state
            == "complete"
        {
            return Ok(mother_crate::runtime::ChildWarmupResult {
                status: "already-warmed".to_string(),
                discovered: 0,
                activated: 0,
                failed: 0,
                degraded: vec![],
            });
        }

        self.ensure_memory_allows_warmup()?;

        let _guard = match self.child_warmup_lock.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::WouldBlock) => {
                anyhow::bail!("operation_in_progress: warmup already running")
            }
            Err(TryLockError::Poisoned(_)) => {
                anyhow::bail!("internal_error: warmup lock poisoned")
            }
        };

        self.set_child_warmup_state("running", None);
        match self.execute_child_warmup_once() {
            Ok(result) => {
                self.set_child_warmup_state("complete", None);
                Ok(result)
            }
            Err(error) => {
                self.set_child_warmup_state("failed", Some(error.to_string()));
                Err(error)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum DaemonStartupProfile {
    Full,
    Core,
}

impl DaemonStartupProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Core => "core",
        }
    }

    pub fn auto_warmup(self) -> bool {
        matches!(self, Self::Full)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum RivetIntegrationProfile {
    Disabled,
    Enabled,
}

impl RivetIntegrationProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Enabled => "enabled",
        }
    }
}

/// Options for starting the daemon
pub struct DaemonOptions {
    pub host: Option<String>,
    pub port: u16,
    pub profile: DaemonStartupProfile,
    pub rivet: RivetIntegrationProfile,
}

pub(super) fn run_startup_stage<T, F>(
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

pub(super) fn emit_startup_metric(name: &str, kind: &str, value: f64, action: &str) {
    emit_startup_metric_with_labels(name, kind, value, &[("action", action)]);
}

pub(super) fn emit_lifecycle_metric(name: &str, kind: &str, value: f64, labels: &[(&str, &str)]) {
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

pub(super) fn emit_startup_metric_with_labels(
    name: &str,
    kind: &str,
    value: f64,
    labels: &[(&str, &str)],
) {
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

pub(super) fn emit_child_activation_metric(name: &str, kind: &str, value: f64, child: &str) {
    emit_startup_metric_with_labels(
        name,
        kind,
        value,
        &[("action", "child_activate"), ("child", child)],
    );
}

pub(super) fn set_control_plane_ready(
    readiness: &Arc<RwLock<mother_crate::runtime::ReadinessState>>,
) {
    let mut guard = readiness.write().unwrap_or_else(|e| e.into_inner());
    guard.control_plane_ready = true;
}

pub(super) fn set_children_total(
    readiness: &Arc<RwLock<mother_crate::runtime::ReadinessState>>,
    total: usize,
) {
    let mut guard = readiness.write().unwrap_or_else(|e| e.into_inner());
    guard.children_total = total;
}

pub(super) fn record_child_activation(
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

impl Default for DaemonOptions {
    fn default() -> Self {
        Self {
            host: None,
            port: 50051,
            profile: DaemonStartupProfile::Full,
            rivet: RivetIntegrationProfile::Disabled,
        }
    }
}

/// Run the mother daemon server
pub fn run_server(options: DaemonOptions) -> Result<()> {
    mother_crate::daemon_bootstrap_config::ensure_logging_initialized()?;

    // Build control-plane state first; child warmup runs in background in full profile.
    let registry = ChildRegistry::new();
    let runtime = patina::mother::MotherRuntimeStore::default();
    let startup_store = patina::mother::MotherRuntimeStore::default();
    let readiness = Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default()));
    run_startup_stage("state_db_open", &startup_store, || {
        startup_store.list_registered_projects().map(|_| ())
    })?;
    let federation_runtime = federation::startup(&startup_store);
    let profile = options.profile;
    let rivet = options.rivet;

    // TCP opt-in path (--host flag) — requires bearer token
    if let Some(ref host) = options.host {
        let (state, router) = run_startup_stage("router_build", &startup_store, || {
            let token = std::env::var("PATINA_SERVE_TOKEN").unwrap_or_else(|_| generate_token());
            let state = Arc::new(ServerState::new(ServerStateInit {
                token,
                startup_profile: profile,
                rivet_integration: rivet,
                registry,
                runtime_store: runtime.clone(),
                startup_store: startup_store.clone(),
                federation_runtime,
                readiness: Arc::clone(&readiness),
            }));
            let router = Arc::new(build_router(Arc::clone(&state), true));
            Ok((state, router))
        })?;
        set_control_plane_ready(&readiness);
        if profile.auto_warmup() {
            spawn_child_warmup(
                Arc::clone(&state),
                WarmupProbe::Tcp {
                    host: host.clone(),
                    port: options.port,
                    token: state.token.clone(),
                },
            );
        }
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
        let state = Arc::new(ServerState::new(ServerStateInit {
            token: String::new(),
            startup_profile: profile,
            rivet_integration: rivet,
            registry,
            runtime_store: runtime.clone(),
            startup_store: startup_store.clone(),
            federation_runtime,
            readiness: Arc::clone(&readiness),
        }));
        let router = Arc::new(build_router(Arc::clone(&state), false));
        Ok((state, router))
    })?;
    set_control_plane_ready(&readiness);
    let socket_path = patina::paths::serve::socket_path();
    if profile.auto_warmup() {
        spawn_child_warmup(
            Arc::clone(&state),
            WarmupProbe::Uds {
                socket_path: socket_path.clone(),
            },
        );
    }
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
