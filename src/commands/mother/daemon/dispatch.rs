use super::*;
use crate::commands::mother::{integrity, loader};

impl ApiRuntime for ServerState {
    fn version(&self) -> String {
        self.version.clone()
    }

    fn uptime_secs(&self) -> u64 {
        self.uptime_secs()
    }

    fn ready_status(&self) -> anyhow::Result<bool> {
        Ok(self.query_readiness().control_plane_ready)
    }

    fn health_all(&self) -> Vec<(String, patina::mother::ChildHealth)> {
        self.services.health.child_health_all(&self.registry)
    }

    fn health_details(&self) -> anyhow::Result<mother_crate::http_api::HealthDetails> {
        let registered_projects = self.runtime_store.list_registered_projects()?;
        let state_db_bytes = health::file_size_if_exists(self.runtime_store.path());
        let active_project_uid = health::read_current_project_uid();

        let active_project_databases = active_project_uid.as_ref().and_then(|uid| {
            let state_parent = self.runtime_store.path().parent()?;
            let project_dir = state_parent.join("projects").join(uid);
            Some(mother_crate::http_api::ProjectDatabases {
                events_db_bytes: health::file_size_if_exists(&project_dir.join("events.db")),
                patina_db_bytes: health::file_size_if_exists(&project_dir.join("patina.db")),
                runtime_db_bytes: health::file_size_if_exists(&project_dir.join("runtime.db")),
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
        let child_warmup = self
            .child_warmup_state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let memory = self.current_memory_status();

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
            startup_profile: self.startup_profile.as_str().to_string(),
            rivet_integration: self.rivet_integration.as_str().to_string(),
            child_warmup,
            memory,
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

    fn child_call(
        &self,
        child_name: &str,
        operation_id: String,
        args: serde_json::Value,
        correlation: Option<patina::mother::CallCorrelation>,
    ) -> anyhow::Result<serde_json::Value> {
        let request = patina::mother::ChildCallRequest {
            operation_id,
            args,
            correlation,
        };
        Ok(self.registry.call(child_name, &request)?.payload)
    }

    fn atlas_dashboard_html(&self) -> anyhow::Result<String> {
        let root = patina::session::SessionManager::find_project_root()
            .or_else(|_| std::env::current_dir())
            .map_err(|error| anyhow::anyhow!("resolve atlas project root: {}", error))?;
        crate::commands::atlas::dashboard_html_for_root(&root, Some(3))
    }

    fn atlas_snapshot(&self) -> anyhow::Result<serde_json::Value> {
        let root = patina::session::SessionManager::find_project_root()
            .or_else(|_| std::env::current_dir())
            .map_err(|error| anyhow::anyhow!("resolve atlas project root: {}", error))?;
        crate::commands::atlas::snapshot_json_for_root(&root)
    }

    fn bridge_translate(
        &self,
        request: mother_crate::bridge::BridgeRequest,
    ) -> anyhow::Result<mother_crate::bridge::BridgeResponse> {
        Ok(mother_crate::bridge::evaluate_bridge_request(&request))
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

    fn lifecycle_warmup_children(&self) -> anyhow::Result<mother_crate::ChildWarmupResult> {
        self.warmup_children_now()
    }

    fn interface_control_call(
        &self,
        request: mother_crate::http_api::InterfaceControlCallRequest,
    ) -> anyhow::Result<serde_json::Value> {
        super::interface_control::dispatch_interface_control_call(request)
    }

    fn rivet_dispatch(
        &self,
        request: mother_crate::http_api::RivetDispatchRequest,
    ) -> anyhow::Result<serde_json::Value> {
        if self.rivet_integration != RivetIntegrationProfile::Enabled {
            return Err(anyhow::Error::new(
                mother_crate::http_api::LifecycleError::invalid_request(
                    "rivet integration is disabled",
                ),
            ));
        }

        let delivery = request.delivery_policy();
        let operation_id = request.operation_id.clone();
        let args = request.args.clone();
        let correlation = request.correlation.clone();
        let call = patina::mother::ChildCallRequest {
            operation_id: operation_id.clone(),
            args: args.clone(),
            correlation: correlation.clone(),
        };

        let map_primary_error = |error: anyhow::Error| {
            let detail = error.to_string();
            if let Some(child) = detail.strip_prefix("unknown child: ") {
                anyhow::Error::new(mother_crate::http_api::LifecycleError::child_not_found(
                    child.to_string(),
                ))
            } else {
                error
            }
        };

        match self.registry.call(&request.child, &call) {
            Ok(response) => Ok(serde_json::json!({
                "adapter": "rivet",
                "child": request.child,
                "operation_id": operation_id,
                "delivery": delivery,
                "status": "delivered",
                "payload": response.payload,
            })),
            Err(primary_error) => match delivery {
                mother_crate::pando::PandoDeliveryPolicy::Required => {
                    Err(map_primary_error(primary_error))
                }
                mother_crate::pando::PandoDeliveryPolicy::BestEffort => Ok(serde_json::json!({
                    "adapter": "rivet",
                    "child": request.child,
                    "operation_id": operation_id,
                    "delivery": delivery,
                    "status": "best-effort-skipped",
                    "error": primary_error.to_string(),
                })),
                mother_crate::pando::PandoDeliveryPolicy::DeadLetter => {
                    let dead_letter = request.dead_letter.ok_or_else(|| {
                        anyhow::Error::new(mother_crate::http_api::LifecycleError::invalid_request(
                            "dead-letter policy requires dead-letter target",
                        ))
                    })?;
                    let dead_letter_operation = dead_letter
                        .operation_id
                        .clone()
                        .unwrap_or_else(|| operation_id.clone());
                    let dead_letter_call = patina::mother::ChildCallRequest {
                        operation_id: dead_letter_operation.clone(),
                        args,
                        correlation,
                    };
                    match self.registry.call(&dead_letter.child, &dead_letter_call) {
                        Ok(dead_response) => Ok(serde_json::json!({
                            "adapter": "rivet",
                            "child": request.child,
                            "operation_id": operation_id,
                            "delivery": delivery,
                            "status": "dead-letter-delivered",
                            "primary_error": primary_error.to_string(),
                            "dead_letter": {
                                "child": dead_letter.child,
                                "operation_id": dead_letter_operation,
                            },
                            "payload": dead_response.payload,
                        })),
                        Err(dead_error) => Err(anyhow::anyhow!(
                            "dead_letter_failed: primary='{}'; dead_letter='{}'",
                            primary_error,
                            dead_error
                        )),
                    }
                }
            },
        }
    }

    fn typed_call_history(&self, limit: usize) -> anyhow::Result<serde_json::Value> {
        let calls = self.registry.typed_call_history(limit);
        Ok(serde_json::json!({
            "count": calls.len(),
            "calls": calls,
        }))
    }

    fn builtin_spec_dispatch(
        &self,
        request: patina_protocol::SpecDispatchRequest,
    ) -> anyhow::Result<serde_json::Value> {
        #[derive(serde::Deserialize)]
        struct SpecDispatchEnvelope {
            command: patina::spec::SpecCommands,
            #[serde(default)]
            project: Option<String>,
            #[serde(default)]
            origin_project: Option<String>,
        }

        if let Ok(envelope) =
            serde_json::from_value::<SpecDispatchEnvelope>(request.command.clone())
        {
            return patina::spec::execute_command_value_with_route(
                envelope.command,
                envelope.project,
                envelope.origin_project,
            );
        }

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
                return Err(anyhow::Error::new(
                    mother_crate::http_api::LifecycleError::pando_not_found(format!(
                        "no pando named '{}'",
                        name
                    )),
                ));
            }
            integrity::verify_pando_integrity(&self.pandos_root.join(name))?;
            let manifest =
                mother_crate::pando::parse_manifest_path(&manifest_path).map_err(|e| {
                    anyhow::Error::new(mother_crate::http_api::LifecycleError::invalid_request(
                        e.to_string(),
                    ))
                })?;
            self.validate_typed_composition(&manifest)?;
            let has_typed_wiring = manifest
                .composition
                .as_ref()
                .map(|composition| {
                    composition
                        .wiring
                        .iter()
                        .any(|rule| matches!(rule, mother_crate::pando::PandoWiring::Typed(_)))
                })
                .unwrap_or(false);
            let loaded_component = if has_typed_wiring {
                LoadedComponent::Composed
            } else {
                LoadedComponent::HandleBased
            };
            let mut children_activated = 0usize;
            for child in &manifest.children {
                if self.registry.child_paths(&child.name).is_none() {
                    continue;
                }
                let reload = self.reload_child(&child.name)?;
                if reload.status == "reloaded" {
                    children_activated += 1;
                }
            }
            if matches!(loaded_component, LoadedComponent::Composed) {
                self.execute_typed_composition(&manifest)?;
            }
            self.reload_pando_registry()?;

            Ok(mother_crate::runtime::PandoLoadResult {
                pando: name.to_string(),
                status: "loaded".to_string(),
                children_activated,
            })
        })();

        startup::emit_lifecycle_metric(
            "load_pando_latency_ms",
            "gauge",
            started.elapsed().as_millis() as f64,
            &[("action", "load_pando"), ("pando", name)],
        );
        if result.is_err() {
            startup::emit_lifecycle_metric(
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
                    return Err(anyhow::Error::new(
                        mother_crate::http_api::LifecycleError::operation_in_progress(
                            "refresh already running",
                        ),
                    ));
                }
                Err(TryLockError::Poisoned(_)) => {
                    return Err(anyhow::Error::new(
                        mother_crate::http_api::LifecycleError::internal_error(
                            "refresh lock poisoned",
                        ),
                    ));
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
                    if integrity::verify_pando_integrity(&dir.path()).is_err() {
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
                    let has_typed_wiring = manifest
                        .composition
                        .as_ref()
                        .map(|composition| {
                            composition.wiring.iter().any(|rule| {
                                matches!(rule, mother_crate::pando::PandoWiring::Typed(_))
                            })
                        })
                        .unwrap_or(false);
                    let loaded_component = if has_typed_wiring {
                        LoadedComponent::Composed
                    } else {
                        LoadedComponent::HandleBased
                    };
                    pandos_loaded += 1;
                    for child in &manifest.children {
                        if self.registry.child_paths(&child.name).is_none() {
                            continue;
                        }
                        match self.reload_child(&child.name) {
                            Ok(outcome) if outcome.status == "reloaded" => {
                                children_activated += 1;
                            }
                            Ok(outcome) => {
                                children_failed += 1;
                                degraded.push(mother_crate::runtime::DegradedChild {
                                    name: child.name.clone(),
                                    reason: outcome
                                        .reason
                                        .unwrap_or_else(|| "reload failed".to_string()),
                                });
                            }
                            Err(error) => {
                                children_failed += 1;
                                degraded.push(mother_crate::runtime::DegradedChild {
                                    name: child.name.clone(),
                                    reason: error.to_string(),
                                });
                            }
                        }
                    }
                    if matches!(loaded_component, LoadedComponent::Composed) {
                        if let Err(error) = self.execute_typed_composition(&manifest) {
                            pandos_failed += 1;
                            degraded.push(mother_crate::runtime::DegradedChild {
                                name: manifest.pando.name.clone(),
                                reason: format!("typed composition execution failed: {}", error),
                            });
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

        startup::emit_lifecycle_metric(
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
            let reload_lock = self.registry.child_reload_lock(name).ok_or_else(|| {
                anyhow::Error::new(mother_crate::http_api::LifecycleError::child_not_found(
                    format!("no child named '{}'", name),
                ))
            })?;
            let _guard = match reload_lock.try_lock() {
                Ok(guard) => guard,
                Err(TryLockError::WouldBlock) => {
                    return Err(anyhow::Error::new(
                        mother_crate::http_api::LifecycleError::operation_in_progress(format!(
                            "reload already running for '{}'",
                            name
                        )),
                    ));
                }
                Err(TryLockError::Poisoned(_)) => {
                    return Err(anyhow::Error::new(
                        mother_crate::http_api::LifecycleError::internal_error(format!(
                            "reload lock poisoned for '{}'",
                            name
                        )),
                    ));
                }
            };

            let (wasm_path, manifest_path) = self.registry.child_paths(name).ok_or_else(|| {
                anyhow::Error::new(mother_crate::http_api::LifecycleError::child_not_found(
                    format!("no child named '{}'", name),
                ))
            })?;

            let loaded = loader::load_wasm_child(&wasm_path, &manifest_path)?;
            let mut replacement = match loaded {
                mother_crate::daemon_bootstrap::LoadedChild::Knowledge {
                    child,
                    name: loaded_name,
                    ..
                } => {
                    if loaded_name != name {
                        return Err(anyhow::Error::new(
                            mother_crate::http_api::LifecycleError::internal_error(format!(
                                "manifest child '{}' does not match reload target '{}'",
                                loaded_name, name
                            )),
                        ));
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

        startup::emit_lifecycle_metric(
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
            startup::emit_lifecycle_metric(
                "reload_child_failure",
                "counter",
                1.0,
                &[("action", "reload_child"), ("child", name)],
            );
        }

        result
    }

    fn warmup_children(&self) -> Result<mother_crate::runtime::ChildWarmupResult> {
        self.warmup_children_now()
    }

    fn query_readiness(&self) -> mother_crate::runtime::ReadinessState {
        self.readiness
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}
