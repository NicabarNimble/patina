use super::*;
use crate::commands::mother::{integrity, loader};
use anyhow::Context;

impl ServerState {
    fn ensure_builtin_view_shapes(&self) -> anyhow::Result<()> {
        self.runtime_store
            .seed_view_shape(&mother_crate::view_buffer::mother_status_shape())?;
        Ok(())
    }

    fn build_view_request_detail(
        &self,
        request: mother_crate::view_buffer::DisplayRequest,
    ) -> anyhow::Result<mother_crate::view_buffer::ViewRequestDetail> {
        // obligation: spec.mother-view-request-ux.mvru1-detail-model
        // obligation: spec.mother-view-request-ux.mvru6-no-fake-data-guardrails
        let shape_match = self
            .runtime_store
            .get_view_shape_match(&request.request_id)?;
        let shape_adaptation = self
            .runtime_store
            .get_view_shape_adaptation(&request.request_id)?;
        let adapted_shape = shape_adaptation
            .as_ref()
            .map(|adaptation| {
                self.runtime_store
                    .get_view_shape(&adaptation.adapted_shape_id)
            })
            .transpose()?
            .flatten();
        let shape_creation = self
            .runtime_store
            .get_view_shape_creation(&request.request_id)?;
        let created_shape = shape_creation
            .as_ref()
            .map(|creation| {
                self.runtime_store
                    .get_view_shape(&creation.created_shape_id)
            })
            .transpose()?
            .flatten();
        let matched_shape = shape_match
            .as_ref()
            .and_then(|shape_match| shape_match.shape_id.as_deref())
            .map(|shape_id| self.runtime_store.get_view_shape(shape_id))
            .transpose()?
            .flatten();

        Ok(mother_crate::view_buffer::ViewRequestDetail::from_parts(
            request,
            shape_match,
            shape_adaptation,
            adapted_shape,
            shape_creation,
            created_shape,
            matched_shape,
        ))
    }
}

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

        let dispatch_target =
            |child: &str,
             operation_id: &str,
             args: &serde_json::Value,
             correlation: Option<patina::mother::CallCorrelation>| {
                if child == "interface-control" {
                    return super::interface_control::dispatch_interface_control_call(
                        mother_crate::http_api::InterfaceControlCallRequest {
                            operation_id: operation_id.to_string(),
                            args: args.clone(),
                            correlation: None,
                        },
                    );
                }

                let call = patina::mother::ChildCallRequest {
                    operation_id: operation_id.to_string(),
                    args: args.clone(),
                    correlation,
                };
                self.registry
                    .call(child, &call)
                    .map(|response| response.payload)
            };

        let delivery = request.delivery_policy();
        let operation_id = request.operation_id.clone();
        let args = request.args.clone();
        let correlation = request.correlation.clone();

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

        match dispatch_target(&request.child, &operation_id, &args, correlation.clone()) {
            Ok(payload) => Ok(serde_json::json!({
                "adapter": "rivet",
                "child": request.child,
                "operation_id": operation_id,
                "delivery": delivery,
                "status": "delivered",
                "payload": payload,
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

                    match dispatch_target(
                        &dead_letter.child,
                        &dead_letter_operation,
                        &args,
                        correlation,
                    ) {
                        Ok(dead_payload) => Ok(serde_json::json!({
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
                            "payload": dead_payload,
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
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum SpecBackendMode {
            Off,
            Observe,
            Execute,
        }

        impl SpecBackendMode {
            fn parse(raw: Option<&str>) -> Self {
                match raw
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_ascii_lowercase())
                    .as_deref()
                {
                    Some("off") | Some("legacy") => Self::Off,
                    Some("observe") | Some("slate-observe") => Self::Observe,
                    Some("execute") | Some("slate-execute") => Self::Execute,
                    _ => Self::Off,
                }
            }

            fn as_str(self) -> &'static str {
                match self {
                    Self::Off => "off",
                    Self::Observe => "observe",
                    Self::Execute => "execute",
                }
            }
        }

        #[derive(serde::Deserialize, serde::Serialize)]
        struct SpecDispatchEnvelope {
            command: patina::spec::SpecCommands,
            #[serde(default)]
            project: Option<String>,
            #[serde(default)]
            origin_project: Option<String>,
            #[serde(default)]
            backend_mode: Option<String>,
        }

        if let Ok(envelope) =
            serde_json::from_value::<SpecDispatchEnvelope>(request.command.clone())
        {
            let backend_mode = SpecBackendMode::parse(envelope.backend_mode.as_deref());

            let dispatch_to_slate =
                |command: &patina::spec::SpecCommands,
                 project: Option<&str>|
                 -> anyhow::Result<(serde_json::Value, serde_json::Value, String)> {
                    let project_value = project.map(|value| value.to_string());

                    let typed_operation = match command {
                        patina::spec::SpecCommands::List { status, target, .. } => Some((
                            "patina:slate/control@0.1.0.list-specs".to_string(),
                            serde_json::json!([{
                                "project": project_value,
                                "status": status,
                                "target": target,
                            }]),
                        )),
                        patina::spec::SpecCommands::Next { .. } => Some((
                            "patina:slate/control@0.1.0.next-specs".to_string(),
                            serde_json::json!([{
                                "project": project_value,
                            }]),
                        )),
                        patina::spec::SpecCommands::Check { id, .. } => Some((
                            "patina:slate/control@0.1.0.check-spec".to_string(),
                            serde_json::json!([{
                                "project": project_value,
                                "id": id,
                            }]),
                        )),
                        patina::spec::SpecCommands::Show { id, .. } => Some((
                            "patina:slate/control@0.1.0.show-spec".to_string(),
                            serde_json::json!([{
                                "project": project_value,
                                "id": id,
                            }]),
                        )),
                        patina::spec::SpecCommands::Prompt { id, .. } => Some((
                            "patina:slate/control@0.1.0.prompt-spec".to_string(),
                            serde_json::json!([{
                                "project": project_value,
                                "id": id,
                            }]),
                        )),
                        patina::spec::SpecCommands::Handoff { id, .. } => Some((
                            "patina:slate/control@0.1.0.handoff-spec".to_string(),
                            serde_json::json!([{
                                "project": project_value,
                                "id": id,
                            }]),
                        )),
                        patina::spec::SpecCommands::Packet { id, .. } => Some((
                            "patina:slate/control@0.1.0.packet-spec".to_string(),
                            serde_json::json!([{
                                "project": project_value,
                                "id": id,
                            }]),
                        )),
                        patina::spec::SpecCommands::Complete {
                            id, major, force, ..
                        } => Some((
                            "patina:slate/control@0.1.0.complete-spec".to_string(),
                            serde_json::json!([{
                                "project": project_value,
                                "id": id,
                                "major": major,
                                "force": force,
                            }]),
                        )),
                        patina::spec::SpecCommands::Archive { id, dry_run, stale } => Some((
                            "patina:slate/control@0.1.0.archive-spec".to_string(),
                            serde_json::json!([{
                                "project": project_value,
                                "id": id,
                                "stale": stale,
                                "dry-run": dry_run,
                            }]),
                        )),
                        _ => None,
                    };

                    let (operation_id, args) = if let Some((operation_id, args)) = typed_operation {
                        (operation_id, args)
                    } else {
                        let command_payload = serde_json::to_value(SpecDispatchEnvelope {
                            command: command.clone(),
                            project: project_value,
                            origin_project: envelope.origin_project.clone(),
                            backend_mode: envelope.backend_mode.clone(),
                        })
                        .context("Failed to serialize fallback spec dispatch envelope")?;
                        let command_json = serde_json::to_string(&command_payload).context(
                            "Failed to serialize fallback spec dispatch envelope as string",
                        )?;
                        (
                            "patina:slate/control@0.1.0.dispatch".to_string(),
                            serde_json::json!([command_json]),
                        )
                    };

                    let response = self
                        .registry
                        .call(
                            "slate-manager",
                            &patina::mother::ChildCallRequest {
                                operation_id: operation_id.clone(),
                                args,
                                correlation: None,
                            },
                        )
                        .map_err(|error| anyhow::anyhow!("slate-manager unavailable: {}", error))?;

                    let first_result = response
                        .payload
                        .get("results")
                        .and_then(|value| value.as_array())
                        .and_then(|values| values.first())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "invalid slate response payload: expected results[0], got {}",
                                response.payload
                            )
                        })?;

                    if let Some(error_value) = first_result.get("err") {
                        let error_text = error_value
                            .as_str()
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| error_value.to_string());
                        anyhow::bail!("slate dispatch returned error: {}", error_text);
                    }

                    let ok_value = first_result.get("ok").ok_or_else(|| {
                        anyhow::anyhow!(
                            "invalid slate response payload: missing ok/err variant in {}",
                            first_result
                        )
                    })?;

                    let data = if let Some(text) = ok_value.as_str() {
                        serde_json::from_str::<serde_json::Value>(text)
                            .unwrap_or_else(|_| serde_json::Value::String(text.to_string()))
                    } else {
                        ok_value.clone()
                    };

                    Ok((data, response.payload, operation_id))
                };

            if backend_mode == SpecBackendMode::Execute {
                let json_mode = envelope.command.wants_json();
                let (slate_data, slate_payload, slate_operation_id) =
                    dispatch_to_slate(&envelope.command, envelope.project.as_deref()).map_err(
                        |error| anyhow::anyhow!("slate execute dispatch failed: {}", error),
                    )?;

                let scaffold_only = slate_data
                    .get("status")
                    .and_then(|value| value.as_str())
                    .map(|value| value.eq_ignore_ascii_case("scaffold"))
                    .unwrap_or(false);

                if scaffold_only {
                    anyhow::bail!(
                        "slate execute dispatch failed: slate-manager returned scaffold response for execute mode"
                    );
                }

                return Ok(serde_json::json!({
                    "child": "spec-manager",
                    "json": json_mode,
                    "text": serde_json::Value::Null,
                    "data": slate_data,
                    "backend": {
                        "mode": backend_mode.as_str(),
                        "engine": "slate-manager",
                        "operation_id": slate_operation_id,
                        "slate_payload": slate_payload,
                    }
                }));
            }

            let mut payload = patina::spec::execute_command_value_with_route_backend(
                envelope.command.clone(),
                envelope.project.clone(),
                envelope.origin_project.clone(),
                envelope.backend_mode.clone(),
            )?;

            if backend_mode == SpecBackendMode::Observe {
                let probe = match dispatch_to_slate(&envelope.command, envelope.project.as_deref())
                {
                    Ok((data, raw_payload, operation_id)) => serde_json::json!({
                        "status": "called",
                        "child": "slate-manager",
                        "operation_id": operation_id,
                        "data": data,
                        "payload": raw_payload,
                    }),
                    Err(error) => serde_json::json!({
                        "status": "unavailable",
                        "child": "slate-manager",
                        "operation_id": "patina:slate/control@0.1.0.dispatch",
                        "error": error.to_string(),
                    }),
                };

                if let Some(root) = payload.as_object_mut() {
                    let backend_entry = root
                        .entry("backend".to_string())
                        .or_insert_with(|| serde_json::json!({}));
                    if !backend_entry.is_object() {
                        *backend_entry = serde_json::json!({});
                    }
                    if let Some(backend) = backend_entry.as_object_mut() {
                        backend
                            .insert("mode".to_string(), serde_json::json!(backend_mode.as_str()));
                        backend.insert(
                            "engine".to_string(),
                            serde_json::json!("builtin-spec-manager"),
                        );
                        backend.insert("slate_probe".to_string(), probe);
                    }
                }
            }

            return Ok(payload);
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

    fn view_shapes_list(&self) -> anyhow::Result<Vec<mother_crate::view_buffer::ViewShape>> {
        self.ensure_builtin_view_shapes()?;
        self.runtime_store.list_view_shapes()
    }

    fn view_shape_get(
        &self,
        shape_id: &str,
    ) -> anyhow::Result<Option<mother_crate::view_buffer::ViewShape>> {
        self.ensure_builtin_view_shapes()?;
        self.runtime_store.get_view_shape(shape_id)
    }

    fn view_shape_upsert(
        &self,
        shape: mother_crate::view_buffer::ViewShape,
    ) -> anyhow::Result<mother_crate::view_buffer::ViewShape> {
        self.runtime_store.upsert_view_shape(&shape)?;
        Ok(shape)
    }

    fn view_shape_deactivate(&self, shape_id: &str) -> anyhow::Result<bool> {
        self.runtime_store.deactivate_view_shape(shape_id)
    }

    fn view_shape_revisions_list(
        &self,
    ) -> anyhow::Result<Vec<mother_crate::view_buffer::ViewShapeRevision>> {
        self.runtime_store.list_view_shape_revisions()
    }

    fn view_shape_revision_get(
        &self,
        revision_id: &str,
    ) -> anyhow::Result<Option<mother_crate::view_buffer::ViewShapeRevision>> {
        self.runtime_store.get_view_shape_revision(revision_id)
    }

    fn view_shape_revise(
        &self,
        request: mother_crate::view_buffer::ReviseViewShapeRequest,
    ) -> anyhow::Result<mother_crate::view_buffer::RevisedViewShapeOutcome> {
        // obligation: spec.mother-view-buffer-revision.mvbr5-persistence
        // obligation: spec.mother-view-buffer-revision.mvbr6-api
        self.ensure_builtin_view_shapes()?;
        let details = self.health_details()?;
        let catalog = mother_crate::view_buffer::DataCatalog::mother_status(
            mother_crate::view_buffer::MotherStatusFacts {
                version: self.version(),
                uptime_secs: self.uptime_secs(),
                control_plane_ready: details.control_plane_ready,
                registered_projects: details.registered_projects,
                children_ready_count: details.children_ready_count,
                children_total: details.children_total,
                startup_profile: details.startup_profile,
                memory_pressure: details.memory.pressure,
                observed_at: Utc::now(),
            },
        );
        let shapes = self.runtime_store.list_view_shapes()?;
        let buffers = self.runtime_store.list_view_buffers()?;
        let mut service =
            mother_crate::view_buffer::ViewBufferService::with_catalog_shapes_and_buffers(
                catalog, shapes, buffers,
            );
        let outcome = service.revise_view_shape(request)?;

        self.runtime_store
            .upsert_view_shape(&outcome.previous_shape)?;
        self.runtime_store
            .upsert_view_shape(&outcome.revised_shape)?;
        self.runtime_store
            .save_view_shape_revision(&outcome.revision)?;
        if let Some(replaced_buffer) = &outcome.replaced_buffer {
            self.runtime_store.save_view_buffer(replaced_buffer)?;
        }
        if let Some(open_outcome) = &outcome.replacement_open_outcome {
            match open_outcome {
                mother_crate::view_buffer::OpenBufferOutcome::Opened(opened) => {
                    self.runtime_store.save_view_buffer(&opened.buffer)?;
                }
                mother_crate::view_buffer::OpenBufferOutcome::ObservabilityGap(gap) => {
                    self.runtime_store.save_view_observability_gap(gap)?;
                }
            }
        }
        Ok(outcome)
    }

    fn view_requests_list(&self) -> anyhow::Result<Vec<mother_crate::view_buffer::DisplayRequest>> {
        self.runtime_store.list_view_display_requests()
    }

    fn view_request_get(
        &self,
        request_id: &str,
    ) -> anyhow::Result<Option<mother_crate::view_buffer::DisplayRequest>> {
        self.runtime_store.get_view_display_request(request_id)
    }

    fn view_request_details_list(
        &self,
    ) -> anyhow::Result<Vec<mother_crate::view_buffer::ViewRequestDetail>> {
        // obligation: spec.mother-view-request-ux.mvru3-detail-api
        self.ensure_builtin_view_shapes()?;
        self.runtime_store
            .list_view_display_requests()?
            .into_iter()
            .map(|request| self.build_view_request_detail(request))
            .collect()
    }

    fn view_request_detail_get(
        &self,
        request_id: &str,
    ) -> anyhow::Result<Option<mother_crate::view_buffer::ViewRequestDetail>> {
        // obligation: spec.mother-view-request-ux.mvru3-detail-api
        self.ensure_builtin_view_shapes()?;
        self.runtime_store
            .get_view_display_request(request_id)?
            .map(|request| self.build_view_request_detail(request))
            .transpose()
    }

    fn view_request_compose(
        &self,
        request: mother_crate::view_buffer::ComposeViewRequest,
    ) -> anyhow::Result<mother_crate::view_buffer::ComposedViewRequest> {
        let details = self.health_details()?;
        let catalog = mother_crate::view_buffer::DataCatalog::mother_status(
            mother_crate::view_buffer::MotherStatusFacts {
                version: self.version(),
                uptime_secs: self.uptime_secs(),
                control_plane_ready: details.control_plane_ready,
                registered_projects: details.registered_projects,
                children_ready_count: details.children_ready_count,
                children_total: details.children_total,
                startup_profile: details.startup_profile,
                memory_pressure: details.memory.pressure,
                observed_at: Utc::now(),
            },
        );
        self.ensure_builtin_view_shapes()?;
        let shapes = self.runtime_store.list_view_shapes()?;
        let mut service =
            mother_crate::view_buffer::ViewBufferService::with_catalog_and_shapes(catalog, shapes);
        let composed = service.compose_request(request)?;

        self.runtime_store
            .save_view_display_request(&composed.request)?;
        if let Some(shape_match) = &composed.shape_match {
            self.runtime_store.save_view_shape_match(shape_match)?;
        }
        if let Some(shape_adaptation) = &composed.shape_adaptation {
            self.runtime_store
                .save_view_shape_adaptation(shape_adaptation)?;
        }
        if let Some(adapted_shape) = &composed.adapted_shape {
            self.runtime_store.upsert_view_shape(adapted_shape)?;
        }
        if let Some(shape_creation) = &composed.shape_creation {
            self.runtime_store
                .save_view_shape_creation(shape_creation)?;
        }
        if let Some(created_shape) = &composed.created_shape {
            self.runtime_store.upsert_view_shape(created_shape)?;
        }
        if let Some(open_outcome) = &composed.open_outcome {
            match open_outcome {
                mother_crate::view_buffer::OpenBufferOutcome::Opened(opened) => {
                    self.runtime_store.save_view_buffer(&opened.buffer)?;
                }
                mother_crate::view_buffer::OpenBufferOutcome::ObservabilityGap(gap) => {
                    self.runtime_store.save_view_observability_gap(gap)?;
                }
            }
        }
        Ok(composed)
    }

    fn view_request_open_shape(
        &self,
        request: mother_crate::view_buffer::OpenRequestShapeRequest,
    ) -> anyhow::Result<Option<mother_crate::view_buffer::OpenRequestShapeOutcome>> {
        // obligation: spec.mother-view-request-ux.mvru4-open-linked-shape-action
        // obligation: spec.mother-view-request-ux.mvru5-non-mutating-history
        let Some(detail) = self.view_request_detail_get(&request.request_id)? else {
            return Ok(None);
        };
        let details = self.health_details()?;
        let catalog = mother_crate::view_buffer::DataCatalog::mother_status(
            mother_crate::view_buffer::MotherStatusFacts {
                version: self.version(),
                uptime_secs: self.uptime_secs(),
                control_plane_ready: details.control_plane_ready,
                registered_projects: details.registered_projects,
                children_ready_count: details.children_ready_count,
                children_total: details.children_total,
                startup_profile: details.startup_profile,
                memory_pressure: details.memory.pressure,
                observed_at: Utc::now(),
            },
        );
        let shapes = self.runtime_store.list_view_shapes()?;
        let mut service =
            mother_crate::view_buffer::ViewBufferService::with_catalog_and_shapes(catalog, shapes);
        let outcome = service.open_request_shape(&detail, request)?;
        match &outcome.open_outcome {
            mother_crate::view_buffer::OpenBufferOutcome::Opened(opened) => {
                self.runtime_store.save_view_buffer(&opened.buffer)?;
            }
            mother_crate::view_buffer::OpenBufferOutcome::ObservabilityGap(gap) => {
                self.runtime_store.save_view_observability_gap(gap)?;
            }
        }
        Ok(Some(outcome))
    }

    fn view_buffers_list(&self) -> anyhow::Result<Vec<mother_crate::view_buffer::Buffer>> {
        self.runtime_store.list_view_buffers()
    }

    fn view_buffer_open(
        &self,
        request: mother_crate::view_buffer::OpenBufferRequest,
    ) -> anyhow::Result<mother_crate::view_buffer::OpenBufferOutcome> {
        let details = self.health_details()?;
        let catalog = mother_crate::view_buffer::DataCatalog::mother_status(
            mother_crate::view_buffer::MotherStatusFacts {
                version: self.version(),
                uptime_secs: self.uptime_secs(),
                control_plane_ready: details.control_plane_ready,
                registered_projects: details.registered_projects,
                children_ready_count: details.children_ready_count,
                children_total: details.children_total,
                startup_profile: details.startup_profile,
                memory_pressure: details.memory.pressure,
                observed_at: Utc::now(),
            },
        );
        self.ensure_builtin_view_shapes()?;
        let shapes = self.runtime_store.list_view_shapes()?;
        let mut service =
            mother_crate::view_buffer::ViewBufferService::with_catalog_and_shapes(catalog, shapes);
        let outcome = service.open_buffer(request)?;
        match &outcome {
            mother_crate::view_buffer::OpenBufferOutcome::Opened(opened) => {
                self.runtime_store.save_view_buffer(&opened.buffer)?;
            }
            mother_crate::view_buffer::OpenBufferOutcome::ObservabilityGap(gap) => {
                self.runtime_store.save_view_observability_gap(gap)?;
            }
        }
        Ok(outcome)
    }

    fn view_buffer_connect_window(
        &self,
        request: mother_crate::view_buffer::ConnectWindowRequest,
    ) -> anyhow::Result<mother_crate::view_buffer::Window> {
        let buffer = self
            .runtime_store
            .list_view_buffers()?
            .into_iter()
            .find(|buffer| buffer.buffer_id == request.buffer_id)
            .ok_or_else(|| anyhow::anyhow!("unknown view buffer '{}'", request.buffer_id))?;
        if !buffer.state.is_connectable() {
            anyhow::bail!(
                "view buffer '{}' is not connectable in state {:?}",
                buffer.buffer_id,
                buffer.state
            );
        }

        let now = Utc::now();
        let frame = mother_crate::view_buffer::Frame {
            frame_id: request.frame_id,
            frame_kind: request.frame_kind,
            connected_at: now,
        };
        let window = mother_crate::view_buffer::Window {
            window_id: request.window_id,
            frame_id: frame.frame_id.clone(),
            buffer_id: Some(buffer.buffer_id),
            connection_state: mother_crate::view_buffer::WindowConnectionState::Connected,
            connected_at: Some(now),
            disconnected_at: None,
        };
        self.runtime_store.save_view_frame(&frame)?;
        self.runtime_store.save_view_window(&window)?;
        Ok(window)
    }

    fn view_buffer_disconnect_window(
        &self,
        request: mother_crate::view_buffer::DisconnectWindowRequest,
    ) -> anyhow::Result<mother_crate::view_buffer::Window> {
        let mut window = self
            .runtime_store
            .list_view_windows()?
            .into_iter()
            .find(|window| window.window_id == request.window_id)
            .ok_or_else(|| anyhow::anyhow!("unknown view window '{}'", request.window_id))?;
        if window.connection_state != mother_crate::view_buffer::WindowConnectionState::Connected {
            anyhow::bail!("view window '{}' is not connected", request.window_id);
        }
        window.connection_state = mother_crate::view_buffer::WindowConnectionState::Disconnected;
        window.disconnected_at = Some(Utc::now());
        self.runtime_store.save_view_window(&window)?;
        Ok(window)
    }

    fn view_buffer_kill(
        &self,
        request: mother_crate::view_buffer::KillBufferRequest,
    ) -> anyhow::Result<mother_crate::view_buffer::Buffer> {
        let mut buffer = self
            .runtime_store
            .list_view_buffers()?
            .into_iter()
            .find(|buffer| buffer.buffer_id == request.buffer_id)
            .ok_or_else(|| anyhow::anyhow!("unknown view buffer '{}'", request.buffer_id))?;
        if !buffer.state.is_connectable() {
            anyhow::bail!(
                "view buffer '{}' cannot be killed from state {:?}",
                request.buffer_id,
                buffer.state
            );
        }
        buffer.state = mother_crate::view_buffer::BufferState::Killed;
        buffer.killed_at = Some(Utc::now());
        self.runtime_store.save_view_buffer(&buffer)?;
        Ok(buffer)
    }

    fn view_buffer_windows_list(&self) -> anyhow::Result<Vec<mother_crate::view_buffer::Window>> {
        self.runtime_store.list_view_windows()
    }

    fn view_buffer_gaps_list(
        &self,
    ) -> anyhow::Result<Vec<mother_crate::view_buffer::ObservabilityGap>> {
        self.runtime_store.list_view_observability_gaps()
    }

    fn view_buffer_gap_get(
        &self,
        gap_id: &str,
    ) -> anyhow::Result<Option<mother_crate::view_buffer::ObservabilityGap>> {
        self.runtime_store.get_view_observability_gap(gap_id)
    }

    fn view_buffer_gap_link_work_item(
        &self,
        request: mother_crate::view_buffer::LinkObservabilityGapRequest,
    ) -> anyhow::Result<mother_crate::view_buffer::ObservabilityGap> {
        // obligation: spec.mother-view-observability-workflow.mvow4-persistence
        let gaps = self.runtime_store.list_view_observability_gaps()?;
        let mut service =
            mother_crate::view_buffer::ViewBufferService::with_catalog_shapes_buffers_and_gaps(
                mother_crate::view_buffer::DataCatalog::default(),
                Vec::new(),
                Vec::new(),
                gaps,
            );
        let gap = service.link_observability_gap(request)?;
        self.runtime_store.save_view_observability_gap(&gap)?;
        Ok(gap)
    }

    fn view_buffer_gap_resolve(
        &self,
        request: mother_crate::view_buffer::ResolveObservabilityGapRequest,
    ) -> anyhow::Result<mother_crate::view_buffer::ObservabilityGap> {
        // obligation: spec.mother-view-observability-workflow.mvow3-resolve-from-catalog
        // obligation: spec.mother-view-observability-workflow.mvow4-persistence
        let details = self.health_details()?;
        let catalog = mother_crate::view_buffer::DataCatalog::mother_status(
            mother_crate::view_buffer::MotherStatusFacts {
                version: self.version(),
                uptime_secs: self.uptime_secs(),
                control_plane_ready: details.control_plane_ready,
                registered_projects: details.registered_projects,
                children_ready_count: details.children_ready_count,
                children_total: details.children_total,
                startup_profile: details.startup_profile,
                memory_pressure: details.memory.pressure,
                observed_at: Utc::now(),
            },
        );
        let gaps = self.runtime_store.list_view_observability_gaps()?;
        let mut service =
            mother_crate::view_buffer::ViewBufferService::with_catalog_shapes_buffers_and_gaps(
                catalog,
                Vec::new(),
                Vec::new(),
                gaps,
            );
        let gap = service.resolve_observability_gap(request)?;
        self.runtime_store.save_view_observability_gap(&gap)?;
        Ok(gap)
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
