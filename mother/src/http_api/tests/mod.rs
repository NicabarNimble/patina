use super::*;

struct StubRuntime;

fn stub_observability_gap() -> crate::view_buffer::ObservabilityGap {
    crate::view_buffer::ObservabilityGap {
        gap_id: "gap_1".to_string(),
        shape_id: Some(crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string()),
        missing_fact_path: "mother.status.version".to_string(),
        missing_source_id: Some("mother.status".to_string()),
        reason: "test gap".to_string(),
        status: crate::view_buffer::ObservabilityGapStatus::Open,
        linked_work_item_id: None,
        created_at: chrono::Utc::now(),
        resolved_at: None,
    }
}

fn stub_derivation() -> crate::view_buffer::ViewDerivation {
    crate::view_buffer::ViewDerivation {
        derivation_id: "derivation_1".to_string(),
        shape_id: crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string(),
        label: "Memory Pressure Summary".to_string(),
        expression_ref: "allium://views/mother/status/memory-pressure".to_string(),
        input_fact_paths: vec!["mother.status.memory_pressure".to_string()],
        maturity: crate::view_buffer::ViewShapeMaturity::Candidate,
    }
}

fn stub_pattern() -> crate::view_buffer::DisplayPattern {
    crate::view_buffer::DisplayPattern {
        pattern_id: "pattern_1".to_string(),
        shape_id: crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string(),
        pattern_kind: crate::view_buffer::DisplayPatternKind::Grouping,
        maturity: crate::view_buffer::ViewShapeMaturity::Exploratory,
    }
}

fn stub_maturation_event() -> crate::view_buffer::ViewMaturationEvent {
    crate::view_buffer::ViewMaturationEvent {
        maturation_id: "maturation_1".to_string(),
        target_kind: crate::view_buffer::ViewMaturationTargetKind::Derivation,
        shape_id: Some(crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string()),
        derivation_id: Some("derivation_1".to_string()),
        pattern_id: None,
        origin: crate::view_buffer::ViewMaturationOrigin::UserRequested,
        from_maturity: crate::view_buffer::ViewShapeMaturity::Candidate,
        to_maturity: crate::view_buffer::ViewShapeMaturity::Stable,
        created_at: chrono::Utc::now(),
    }
}

fn stub_observability_improvement() -> crate::view_buffer::ObservabilityImprovementArtifact {
    crate::view_buffer::ObservabilityImprovementArtifact {
        artifact_id: "maturation_1::observability-improvement".to_string(),
        source_gap_id: None,
        source_maturation_id: Some("maturation_1".to_string()),
        desired_fact_path: "mother.status.memory_pressure.summary".to_string(),
        reason: "stable derivation should become observable".to_string(),
        created_at: chrono::Utc::now(),
        work_item_created: false,
    }
}

impl ApiRuntime for StubRuntime {
    fn version(&self) -> String {
        "0.0.0-test".to_string()
    }

    fn uptime_secs(&self) -> u64 {
        42
    }

    fn ready_status(&self) -> Result<bool> {
        Ok(true)
    }

    fn health_all(&self) -> Vec<(String, crate::ChildHealth)> {
        vec![("ducklake".to_string(), crate::ChildHealth::Healthy)]
    }

    fn health_details(&self) -> Result<HealthDetails> {
        Ok(HealthDetails {
            registered_projects: 2,
            active_project_uid: Some("2bdc808e".to_string()),
            active_project_databases: Some(ProjectDatabases {
                events_db_bytes: Some(1024),
                patina_db_bytes: Some(2048),
                runtime_db_bytes: Some(512),
            }),
            state_db_bytes: Some(256),
            federation_available: true,
            federation_reason: None,
            federation_ducklake_loaded: true,
            federation_projects_attached: 2,
            federation_projects_failed: 1,
            federation_projects_stale: 1,
            startup_profile: "full".to_string(),
            rivet_integration: "disabled".to_string(),
            child_warmup: ChildWarmupState {
                mode: "auto".to_string(),
                state: "complete".to_string(),
                last_error: None,
            },
            memory: MemoryStatus {
                rss_bytes: Some(8 * 1024 * 1024),
                max_rss_bytes: Some(12 * 1024 * 1024),
                soft_limit_bytes: Some(64 * 1024 * 1024),
                pressure: "ok".to_string(),
            },
            control_plane_ready: true,
            children_ready_count: 1,
            children_total: 2,
            children_degraded: vec![DegradedChild {
                name: "catalog".to_string(),
                reason: "on_load failed".to_string(),
            }],
        })
    }

    fn child_health(&self, _child_name: &str) -> Result<crate::ChildHealth> {
        Ok(crate::ChildHealth::Healthy)
    }

    fn child_handle(
        &self,
        _child_name: &str,
        _action: String,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        Ok(payload)
    }

    fn child_call(
        &self,
        _child_name: &str,
        operation_id: String,
        args: serde_json::Value,
        correlation: Option<crate::CallCorrelation>,
    ) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "operation_id": operation_id,
            "args": args,
            "correlation": correlation,
            "typed": true,
        }))
    }

    fn bridge_translate(
        &self,
        request: crate::bridge::BridgeRequest,
    ) -> Result<crate::bridge::BridgeResponse> {
        Ok(crate::bridge::evaluate_bridge_request(&request))
    }

    fn scry_query(
        &self,
        _query: &str,
        _limit: usize,
        _repo: Option<String>,
        _all_repos: bool,
    ) -> Result<Vec<ScryHit>> {
        Ok(vec![])
    }

    fn federation_status(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({"federation": "available"}))
    }

    fn federation_refresh(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({"federation": "available"}))
    }

    fn federation_query(
        &self,
        _payload: crate::protocol::FederationQueryPayload,
    ) -> Result<serde_json::Value> {
        Ok(
            serde_json::json!({"columns":[], "rows":[], "row_count":0, "truncated":false, "elapsed_ms":1}),
        )
    }

    fn secrets_get(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({}))
    }

    fn secrets_cache(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
        Ok(payload)
    }

    fn secrets_lock(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({"locked": true}))
    }

    fn pando_registry_init(
        &self,
        request: patina_protocol::PandoRegistryInit,
    ) -> Result<patina_protocol::PandoRegistryState> {
        Ok(patina_protocol::PandoRegistryState {
            protocol_version: request.protocol_version,
            pandos: vec![],
        })
    }

    fn pando_list(&self) -> Result<patina_protocol::PandoRegistryState> {
        Ok(patina_protocol::PandoRegistryState {
            protocol_version: patina_protocol::PANDO_REGISTRY_PROTOCOL_VERSION,
            pandos: vec![],
        })
    }

    fn lifecycle_load_pando(&self, name: &str) -> Result<crate::PandoLoadResult> {
        Ok(crate::PandoLoadResult {
            pando: name.to_string(),
            status: "loaded".to_string(),
            children_activated: 1,
        })
    }

    fn lifecycle_refresh(&self) -> Result<crate::PandoRefreshResult> {
        Ok(crate::PandoRefreshResult {
            pandos_loaded: 1,
            pandos_failed: 0,
            children_activated: 1,
            children_failed: 0,
            degraded: vec![],
        })
    }

    fn lifecycle_reload_child(&self, name: &str) -> Result<crate::ChildReloadResult> {
        Ok(crate::ChildReloadResult {
            child: name.to_string(),
            status: "reloaded".to_string(),
            previous_instance: "drained".to_string(),
            reason: None,
        })
    }

    fn lifecycle_warmup_children(&self) -> Result<crate::ChildWarmupResult> {
        Ok(crate::ChildWarmupResult {
            status: "warmed".to_string(),
            discovered: 2,
            activated: 2,
            failed: 0,
            degraded: vec![],
        })
    }

    fn interface_control_call(
        &self,
        request: InterfaceControlCallRequest,
    ) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "adapter": "native",
            "operation_id": request.operation_id,
            "args": request.args,
            "correlation": request.correlation,
            "status": "scaffold"
        }))
    }

    fn rivet_dispatch(&self, request: RivetDispatchRequest) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "child": request.child,
            "operation_id": request.operation_id,
            "args": request.args,
            "correlation": request.correlation,
            "delivery": request.delivery_policy(),
            "dead_letter": request.dead_letter,
            "adapter": "rivet"
        }))
    }

    fn typed_call_history(&self, limit: usize) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "count": limit.min(1),
            "calls": [{
                "child": "fixture-typed-child",
                "operation_id": "patina:fixture/control.status",
                "outcome": "success",
                "correlation": {
                    "rivet_run_id": "run-123",
                    "rivet_actor_id": "actor-a",
                    "rivet_workflow_id": "workflow-z",
                    "rivet_job_id": "job-9"
                }
            }]
        }))
    }

    fn builtin_spec_dispatch(
        &self,
        _request: patina_protocol::SpecDispatchRequest,
    ) -> Result<serde_json::Value> {
        Ok(serde_json::json!({}))
    }

    fn builtin_lake_dispatch(
        &self,
        _request: patina_protocol::LakeDispatchRequest,
    ) -> Result<serde_json::Value> {
        Ok(serde_json::json!({}))
    }

    fn builtin_doctor_run(&self) -> Result<patina_protocol::DoctorRunResult> {
        Ok(patina_protocol::DoctorRunResult {
            data: serde_json::json!({}),
            exit_code: 0,
        })
    }

    fn view_shapes_list(&self) -> Result<Vec<crate::view_buffer::ViewShape>> {
        Ok(vec![crate::view_buffer::mother_status_shape()])
    }

    fn view_shape_get(&self, shape_id: &str) -> Result<Option<crate::view_buffer::ViewShape>> {
        let shape = crate::view_buffer::mother_status_shape();
        Ok((shape.shape_id == shape_id).then_some(shape))
    }

    fn view_shape_upsert(
        &self,
        shape: crate::view_buffer::ViewShape,
    ) -> Result<crate::view_buffer::ViewShape> {
        Ok(shape)
    }

    fn view_shape_deactivate(&self, shape_id: &str) -> Result<bool> {
        Ok(shape_id == crate::view_buffer::MOTHER_STATUS_SHAPE_ID)
    }

    fn view_shape_revisions_list(&self) -> Result<Vec<crate::view_buffer::ViewShapeRevision>> {
        Ok(vec![])
    }

    fn view_shape_revision_get(
        &self,
        _revision_id: &str,
    ) -> Result<Option<crate::view_buffer::ViewShapeRevision>> {
        Ok(None)
    }

    fn view_shape_revise(
        &self,
        request: crate::view_buffer::ReviseViewShapeRequest,
    ) -> Result<crate::view_buffer::RevisedViewShapeOutcome> {
        let catalog =
            crate::view_buffer::DataCatalog::mother_status(crate::view_buffer::MotherStatusFacts {
                version: ApiRuntime::version(self),
                uptime_secs: ApiRuntime::uptime_secs(self),
                control_plane_ready: true,
                registered_projects: 2,
                children_ready_count: 1,
                children_total: 2,
                startup_profile: "full".to_string(),
                memory_pressure: "ok".to_string(),
                observed_at: chrono::Utc::now(),
            });
        let mut service = crate::view_buffer::ViewBufferService::with_catalog(catalog);
        service.revise_view_shape(request)
    }

    fn view_derivations_list(&self) -> Result<Vec<crate::view_buffer::ViewDerivation>> {
        Ok(vec![stub_derivation()])
    }

    fn view_derivation_get(
        &self,
        derivation_id: &str,
    ) -> Result<Option<crate::view_buffer::ViewDerivation>> {
        let derivation = stub_derivation();
        Ok((derivation.derivation_id == derivation_id).then_some(derivation))
    }

    fn view_derivation_upsert(
        &self,
        derivation: crate::view_buffer::ViewDerivation,
    ) -> Result<crate::view_buffer::ViewDerivation> {
        Ok(derivation)
    }

    fn view_patterns_list(&self) -> Result<Vec<crate::view_buffer::DisplayPattern>> {
        Ok(vec![stub_pattern()])
    }

    fn view_pattern_get(
        &self,
        pattern_id: &str,
    ) -> Result<Option<crate::view_buffer::DisplayPattern>> {
        let pattern = stub_pattern();
        Ok((pattern.pattern_id == pattern_id).then_some(pattern))
    }

    fn view_pattern_upsert(
        &self,
        pattern: crate::view_buffer::DisplayPattern,
    ) -> Result<crate::view_buffer::DisplayPattern> {
        Ok(pattern)
    }

    fn view_maturation_events_list(&self) -> Result<Vec<crate::view_buffer::ViewMaturationEvent>> {
        Ok(vec![stub_maturation_event()])
    }

    fn view_maturation_event_get(
        &self,
        maturation_id: &str,
    ) -> Result<Option<crate::view_buffer::ViewMaturationEvent>> {
        let event = stub_maturation_event();
        Ok((event.maturation_id == maturation_id).then_some(event))
    }

    fn view_maturation_record(
        &self,
        request: crate::view_buffer::MatureViewArtifactRequest,
    ) -> Result<crate::view_buffer::MaturedViewArtifactOutcome> {
        let mut service = crate::view_buffer::ViewBufferService::with_catalog_artifacts(
            crate::view_buffer::DataCatalog::default(),
            vec![crate::view_buffer::mother_status_shape()],
            Vec::new(),
            Vec::new(),
            vec![stub_derivation()],
            vec![stub_pattern()],
        );
        service.mature_view_artifact(request)
    }

    fn view_observability_improvements_list(
        &self,
    ) -> Result<Vec<crate::view_buffer::ObservabilityImprovementArtifact>> {
        Ok(vec![stub_observability_improvement()])
    }

    fn view_observability_improvement_get(
        &self,
        artifact_id: &str,
    ) -> Result<Option<crate::view_buffer::ObservabilityImprovementArtifact>> {
        let artifact = stub_observability_improvement();
        Ok((artifact.artifact_id == artifact_id).then_some(artifact))
    }

    fn view_requests_list(&self) -> Result<Vec<crate::view_buffer::DisplayRequest>> {
        Ok(vec![crate::view_buffer::DisplayRequest::pending(
            "req_1".to_string(),
            "local-user".to_string(),
            "pi".to_string(),
            "show mother status".to_string(),
            chrono::Utc::now(),
        )])
    }

    fn view_request_get(
        &self,
        request_id: &str,
    ) -> Result<Option<crate::view_buffer::DisplayRequest>> {
        Ok((request_id == "req_1").then(|| {
            crate::view_buffer::DisplayRequest::pending(
                "req_1".to_string(),
                "local-user".to_string(),
                "pi".to_string(),
                "show mother status".to_string(),
                chrono::Utc::now(),
            )
        }))
    }

    fn view_request_details_list(&self) -> Result<Vec<crate::view_buffer::ViewRequestDetail>> {
        Ok(vec![
            ApiRuntime::view_request_detail_get(self, "req_1")?.unwrap()
        ])
    }

    fn view_request_detail_get(
        &self,
        request_id: &str,
    ) -> Result<Option<crate::view_buffer::ViewRequestDetail>> {
        let Some(request) = ApiRuntime::view_request_get(self, request_id)? else {
            return Ok(None);
        };
        let shape = crate::view_buffer::mother_status_shape();
        let shape_match = crate::view_buffer::ShapeMatch {
            request_id: request.request_id.clone(),
            shape_id: Some(shape.shape_id.clone()),
            match_kind: crate::view_buffer::ShapeMatchKind::ExplicitUserChoice,
            confidence: 1.0,
        };
        Ok(Some(crate::view_buffer::ViewRequestDetail::from_parts(
            request,
            Some(shape_match),
            None,
            None,
            None,
            None,
            Some(shape),
        )))
    }

    fn view_request_compose(
        &self,
        request: crate::view_buffer::ComposeViewRequest,
    ) -> Result<crate::view_buffer::ComposedViewRequest> {
        let catalog =
            crate::view_buffer::DataCatalog::mother_status(crate::view_buffer::MotherStatusFacts {
                version: ApiRuntime::version(self),
                uptime_secs: ApiRuntime::uptime_secs(self),
                control_plane_ready: true,
                registered_projects: 2,
                children_ready_count: 1,
                children_total: 2,
                startup_profile: "full".to_string(),
                memory_pressure: "ok".to_string(),
                observed_at: chrono::Utc::now(),
            });
        let mut service = crate::view_buffer::ViewBufferService::with_catalog(catalog);
        service.compose_request(request)
    }

    fn view_request_open_shape(
        &self,
        request: crate::view_buffer::OpenRequestShapeRequest,
    ) -> Result<Option<crate::view_buffer::OpenRequestShapeOutcome>> {
        let Some(detail) = ApiRuntime::view_request_detail_get(self, &request.request_id)? else {
            return Ok(None);
        };
        let catalog =
            crate::view_buffer::DataCatalog::mother_status(crate::view_buffer::MotherStatusFacts {
                version: ApiRuntime::version(self),
                uptime_secs: ApiRuntime::uptime_secs(self),
                control_plane_ready: true,
                registered_projects: 2,
                children_ready_count: 1,
                children_total: 2,
                startup_profile: "full".to_string(),
                memory_pressure: "ok".to_string(),
                observed_at: chrono::Utc::now(),
            });
        let mut service = crate::view_buffer::ViewBufferService::with_catalog(catalog);
        Ok(Some(service.open_request_shape(&detail, request)?))
    }

    fn view_buffers_list(&self) -> Result<Vec<crate::view_buffer::Buffer>> {
        Ok(vec![])
    }

    fn view_buffer_payload_get(&self, buffer_id: &str) -> Result<crate::view_buffer::OpenedBuffer> {
        let catalog =
            crate::view_buffer::DataCatalog::mother_status(crate::view_buffer::MotherStatusFacts {
                version: ApiRuntime::version(self),
                uptime_secs: ApiRuntime::uptime_secs(self),
                control_plane_ready: true,
                registered_projects: 2,
                children_ready_count: 1,
                children_total: 2,
                startup_profile: "full".to_string(),
                memory_pressure: "ok".to_string(),
                observed_at: chrono::Utc::now(),
            });
        let shape = crate::view_buffer::mother_status_shape();
        let buffer = crate::view_buffer::Buffer::live_from_shape(
            buffer_id.to_string(),
            &shape,
            chrono::Utc::now(),
        );
        let service = crate::view_buffer::ViewBufferService::with_catalog_shapes_and_buffers(
            catalog,
            vec![shape],
            vec![buffer],
        );
        service.opened_buffer_payload(buffer_id)
    }

    fn view_buffer_open(
        &self,
        request: crate::view_buffer::OpenBufferRequest,
    ) -> Result<crate::view_buffer::OpenBufferOutcome> {
        let catalog =
            crate::view_buffer::DataCatalog::mother_status(crate::view_buffer::MotherStatusFacts {
                version: ApiRuntime::version(self),
                uptime_secs: ApiRuntime::uptime_secs(self),
                control_plane_ready: true,
                registered_projects: 2,
                children_ready_count: 1,
                children_total: 2,
                startup_profile: "full".to_string(),
                memory_pressure: "ok".to_string(),
                observed_at: chrono::Utc::now(),
            });
        let mut service = crate::view_buffer::ViewBufferService::with_catalog(catalog);
        service.open_buffer(request)
    }

    fn view_buffer_connect_window(
        &self,
        request: crate::view_buffer::ConnectWindowRequest,
    ) -> Result<crate::view_buffer::Window> {
        Ok(crate::view_buffer::Window {
            window_id: request.window_id,
            frame_id: request.frame_id,
            buffer_id: Some(request.buffer_id),
            connection_state: crate::view_buffer::WindowConnectionState::Connected,
            connected_at: Some(chrono::Utc::now()),
            disconnected_at: None,
        })
    }

    fn view_buffer_disconnect_window(
        &self,
        request: crate::view_buffer::DisconnectWindowRequest,
    ) -> Result<crate::view_buffer::Window> {
        Ok(crate::view_buffer::Window {
            window_id: request.window_id,
            frame_id: "frame_tui".to_string(),
            buffer_id: None,
            connection_state: crate::view_buffer::WindowConnectionState::Disconnected,
            connected_at: None,
            disconnected_at: Some(chrono::Utc::now()),
        })
    }

    fn view_buffer_kill(
        &self,
        request: crate::view_buffer::KillBufferRequest,
    ) -> Result<crate::view_buffer::Buffer> {
        let shape = crate::view_buffer::mother_status_shape();
        let mut buffer = crate::view_buffer::Buffer::live_from_shape(
            request.buffer_id,
            &shape,
            chrono::Utc::now(),
        );
        buffer.state = crate::view_buffer::BufferState::Killed;
        buffer.killed_at = Some(chrono::Utc::now());
        Ok(buffer)
    }

    fn view_buffer_windows_list(&self) -> Result<Vec<crate::view_buffer::Window>> {
        Ok(vec![])
    }

    fn view_buffer_gaps_list(&self) -> Result<Vec<crate::view_buffer::ObservabilityGap>> {
        Ok(vec![stub_observability_gap()])
    }

    fn view_buffer_gap_get(
        &self,
        gap_id: &str,
    ) -> Result<Option<crate::view_buffer::ObservabilityGap>> {
        Ok((gap_id == "gap_1").then(stub_observability_gap))
    }

    fn view_buffer_gap_link_work_item(
        &self,
        request: crate::view_buffer::LinkObservabilityGapRequest,
    ) -> Result<crate::view_buffer::ObservabilityGap> {
        let mut service =
            crate::view_buffer::ViewBufferService::with_catalog_shapes_buffers_and_gaps(
                crate::view_buffer::DataCatalog::default(),
                Vec::new(),
                Vec::new(),
                vec![stub_observability_gap()],
            );
        service.link_observability_gap(request)
    }

    fn view_buffer_gap_resolve(
        &self,
        request: crate::view_buffer::ResolveObservabilityGapRequest,
    ) -> Result<crate::view_buffer::ObservabilityGap> {
        let catalog =
            crate::view_buffer::DataCatalog::mother_status(crate::view_buffer::MotherStatusFacts {
                version: ApiRuntime::version(self),
                uptime_secs: ApiRuntime::uptime_secs(self),
                control_plane_ready: true,
                registered_projects: 2,
                children_ready_count: 1,
                children_total: 2,
                startup_profile: "full".to_string(),
                memory_pressure: "ok".to_string(),
                observed_at: chrono::Utc::now(),
            });
        let mut service =
            crate::view_buffer::ViewBufferService::with_catalog_shapes_buffers_and_gaps(
                catalog,
                Vec::new(),
                Vec::new(),
                vec![stub_observability_gap()],
            );
        service.resolve_observability_gap(request)
    }

    fn builtin_secrets_dispatch(&self, _payload: serde_json::Value) -> HttpResponse {
        HttpResponse::json(200, &serde_json::json!({}))
    }
}

#[test]
fn health_response_includes_additive_deep_fields() {
    let response = handle_health(&StubRuntime);
    assert_eq!(response.status, 200);
    let json: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        json.get("version").and_then(|v| v.as_str()),
        Some("0.0.0-test")
    );
    assert_eq!(json.get("uptime_secs").and_then(|v| v.as_u64()), Some(42));
    assert_eq!(json.get("child_count").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(
        json.get("registered_projects").and_then(|v| v.as_u64()),
        Some(2)
    );
    assert_eq!(
        json.get("active_project_uid").and_then(|v| v.as_str()),
        Some("2bdc808e")
    );
    assert_eq!(
        json.get("state_db_bytes").and_then(|v| v.as_u64()),
        Some(256)
    );
    assert_eq!(
        json.get("active_project_databases")
            .and_then(|v| v.get("events_db_bytes"))
            .and_then(|v| v.as_u64()),
        Some(1024)
    );
    assert_eq!(
        json.get("federation_available").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        json.get("federation_projects_failed")
            .and_then(|v| v.as_u64()),
        Some(1)
    );
    assert_eq!(
        json.get("control_plane_ready").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        json.get("children_ready_count").and_then(|v| v.as_u64()),
        Some(1)
    );
    assert_eq!(json.get("children_total").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(
        json.get("startup_profile").and_then(|v| v.as_str()),
        Some("full")
    );
    assert_eq!(
        json.get("rivet_integration").and_then(|v| v.as_str()),
        Some("disabled")
    );
    assert_eq!(
        json.get("child_warmup")
            .and_then(|v| v.get("state"))
            .and_then(|v| v.as_str()),
        Some("complete")
    );
    assert_eq!(
        json.get("memory")
            .and_then(|v| v.get("pressure"))
            .and_then(|v| v.as_str()),
        Some("ok")
    );
    assert_eq!(
        json.get("children_degraded")
            .and_then(|v| v.as_array())
            .and_then(|items| items.first())
            .and_then(|entry| entry.get("name"))
            .and_then(|v| v.as_str()),
        Some("catalog")
    );
}

#[test]
fn ready_route_returns_204_when_runtime_is_ready() {
    let response = handle_ready(&StubRuntime);
    assert_eq!(response.status, 204);
    assert!(response.body.is_empty());
}

#[test]
fn view_request_compose_handler_returns_request_outcome() {
    // obligation: spec.mother-view-request-composer.mvrc3-compose-api
    // obligation: spec.mother-view-request-composer.mvrc4-explicit-exact-open
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-requests/compose".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&crate::view_buffer::ComposeViewRequest {
            user_id: "local-user".to_string(),
            agent_id: "pi".to_string(),
            raw_request: "show mother status".to_string(),
            proposed_match: Some(crate::view_buffer::ProposedShapeMatch {
                shape_id: Some(crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string()),
                match_kind: crate::view_buffer::ShapeMatchKind::ExplicitUserChoice,
                confidence: 1.0,
            }),
            proposed_initial_shape: None,
        })
        .unwrap(),
    };

    let response = view_buffer::handle_compose_view_request(&request, &StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload
            .get("request")
            .and_then(|request| request.get("outcome"))
            .and_then(|value| value.as_str()),
        Some("buffer_opened")
    );
}

#[test]
fn view_request_compose_handler_returns_shape_adaptation() {
    // obligation: spec.mother-view-shape-adaptation.mvsa4-compose-integration
    // obligation: rule-success.AdaptSimilarShapeWhenNoExactShapeExists
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-requests/compose".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&crate::view_buffer::ComposeViewRequest {
            user_id: "local-user".to_string(),
            agent_id: "pi".to_string(),
            raw_request: "show something like mother status".to_string(),
            proposed_match: Some(crate::view_buffer::ProposedShapeMatch {
                shape_id: Some(crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string()),
                match_kind: crate::view_buffer::ShapeMatchKind::Similar,
                confidence: crate::view_buffer::SHAPE_MATCH_CONFIDENCE_THRESHOLD,
            }),
            proposed_initial_shape: None,
        })
        .unwrap(),
    };

    let response = view_buffer::handle_compose_view_request(&request, &StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload
            .get("request")
            .and_then(|request| request.get("outcome"))
            .and_then(|value| value.as_str()),
        Some("unable")
    );
    assert_eq!(
        payload
            .get("shape_adaptation")
            .and_then(|adaptation| adaptation.get("precedent_shape_id"))
            .and_then(|value| value.as_str()),
        Some(crate::view_buffer::MOTHER_STATUS_SHAPE_ID)
    );
    assert!(payload
        .get("shape_adaptation")
        .and_then(|adaptation| adaptation.get("opens_buffer"))
        .and_then(|value| value.as_bool())
        .is_some_and(|opens| !opens));
    assert!(payload
        .get("adapted_shape")
        .and_then(|shape| shape.get("shape_id"))
        .and_then(|value| value.as_str())
        .is_some_and(|shape_id| shape_id.starts_with("mother.status.default::adapted::")));
    assert!(payload
        .get("open_outcome")
        .is_some_and(|value| value.is_null()));
}

#[test]
fn view_request_compose_handler_returns_initial_shape_creation() {
    // obligation: spec.mother-view-initial-shape-creation.mvisc5-compose-integration
    // obligation: rule-success.CreateInitialShapeWhenNoShapeMatches
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-requests/compose".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&crate::view_buffer::ComposeViewRequest {
            user_id: "local-user".to_string(),
            agent_id: "pi".to_string(),
            raw_request: "show runtime summary".to_string(),
            proposed_match: Some(crate::view_buffer::ProposedShapeMatch {
                shape_id: None,
                match_kind: crate::view_buffer::ShapeMatchKind::None,
                confidence: 0.0,
            }),
            proposed_initial_shape: Some(crate::view_buffer::ProposedInitialShape {
                title: "Mother Runtime Summary".to_string(),
                major_mode: crate::view_buffer::MajorMode::Table,
                minor_modes: vec![crate::view_buffer::MinorMode::Pinned],
                requirements: vec![crate::view_buffer::ViewRequirement {
                    fact_path: "mother.status.version".to_string(),
                    required: true,
                    purpose: "display Mother version".to_string(),
                }],
                vision_id: None,
                project_uid: None,
            }),
        })
        .unwrap(),
    };

    let response = view_buffer::handle_compose_view_request(&request, &StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload
            .get("request")
            .and_then(|request| request.get("outcome"))
            .and_then(|value| value.as_str()),
        Some("unable")
    );
    assert!(payload
        .get("shape_creation")
        .and_then(|creation| creation.get("created_shape_id"))
        .and_then(|value| value.as_str())
        .is_some_and(|shape_id| shape_id.starts_with("initial::req_")));
    assert!(payload
        .get("shape_creation")
        .and_then(|creation| creation.get("opens_buffer"))
        .and_then(|value| value.as_bool())
        .is_some_and(|opens| !opens));
    assert_eq!(
        payload
            .get("created_shape")
            .and_then(|shape| shape.get("title"))
            .and_then(|value| value.as_str()),
        Some("Mother Runtime Summary")
    );
    assert!(payload
        .get("open_outcome")
        .is_some_and(|value| value.is_null()));
}

#[test]
fn view_request_ux_detail_handlers_return_actions() {
    // obligation: spec.mother-view-request-ux.mvru3-detail-api
    let list_response = view_buffer::handle_list_view_request_details(&StubRuntime);
    assert_eq!(list_response.status, 200);
    let list_payload: serde_json::Value = serde_json::from_slice(&list_response.body).unwrap();
    assert_eq!(
        list_payload
            .get("details")
            .and_then(|details| details.as_array())
            .map(Vec::len),
        Some(1)
    );

    let detail_request = HttpRequest {
        method: "GET".to_string(),
        path: "/api/view-requests/req_1/detail".to_string(),
        headers: vec![],
        body: vec![],
    };
    let detail_response =
        view_buffer::handle_get_view_request_detail(&detail_request, &StubRuntime);
    assert_eq!(detail_response.status, 200);
    let detail_payload: serde_json::Value = serde_json::from_slice(&detail_response.body).unwrap();
    assert_eq!(
        detail_payload
            .get("detail")
            .and_then(|detail| detail.get("available_actions"))
            .and_then(|actions| actions.as_array())
            .and_then(|actions| actions.first())
            .and_then(|action| action.get("kind"))
            .and_then(|kind| kind.as_str()),
        Some("open_matched_shape")
    );
}

#[test]
fn view_request_ux_open_shape_handler_opens_linked_shape() {
    // obligation: spec.mother-view-request-ux.mvru4-open-linked-shape-action
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-requests/open-shape".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&crate::view_buffer::OpenRequestShapeRequest {
            request_id: "req_1".to_string(),
            shape_id: Some(crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string()),
        })
        .unwrap(),
    };

    let response = view_buffer::handle_open_view_request_shape(&request, &StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload.get("request_id").and_then(|value| value.as_str()),
        Some("req_1")
    );
    assert_eq!(
        payload
            .get("open_outcome")
            .and_then(|outcome| outcome.get("outcome"))
            .and_then(|value| value.as_str()),
        Some("opened")
    );
}

#[test]
fn view_request_ux_open_shape_handler_rejects_unlinked_shape() {
    // obligation: spec.mother-view-request-ux.mvru4-open-linked-shape-action
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-requests/open-shape".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "request_id": "req_1",
            "shape_id": "unlinked.shape"
        }))
        .unwrap(),
    };

    let response = view_buffer::handle_open_view_request_shape(&request, &StubRuntime);
    assert_eq!(response.status, 400);
}

#[test]
fn view_request_compose_handler_rejects_blank_requests() {
    // obligation: spec.mother-view-request-composer.mvrc5-fail-closed-outcomes
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-requests/compose".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "user_id": "local-user",
            "agent_id": "pi",
            "raw_request": "  ",
            "proposed_match": null,
        }))
        .unwrap(),
    };

    let response = view_buffer::handle_compose_view_request(&request, &StubRuntime);
    assert_eq!(response.status, 400);
}

#[test]
fn view_buffer_revision_handler_returns_revised_shape() {
    // obligation: spec.mother-view-buffer-revision.mvbr6-api
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-shapes/revise".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&crate::view_buffer::ReviseViewShapeRequest {
            user_id: "local-user".to_string(),
            agent_id: "pi".to_string(),
            shape_id: crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string(),
            previous_buffer_id: None,
            revision_scope: crate::view_buffer::ViewShapeScope::MotherUser,
            reason: "show readiness first".to_string(),
            title: Some("Mother Readiness".to_string()),
            major_mode: None,
            minor_modes: Some(vec![crate::view_buffer::MinorMode::Pinned]),
            requirements: None,
        })
        .unwrap(),
    };

    let response = view_buffer::handle_revise_view_shape(&request, &StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload
            .get("revised_shape")
            .and_then(|shape| shape.get("title"))
            .and_then(|value| value.as_str()),
        Some("Mother Readiness")
    );
    assert_eq!(
        payload
            .get("previous_shape")
            .and_then(|shape| shape.get("active"))
            .and_then(|value| value.as_bool()),
        Some(false)
    );
}

#[test]
fn view_buffer_revision_handler_rejects_invalid_revision() {
    // obligation: spec.mother-view-buffer-revision.mvbr7-fail-closed-guardrails
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-shapes/revise".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "user_id": "local-user",
            "agent_id": "pi",
            "shape_id": crate::view_buffer::MOTHER_STATUS_SHAPE_ID,
            "revision_scope": "mother-user",
            "reason": " "
        }))
        .unwrap(),
    };

    let response = view_buffer::handle_revise_view_shape(&request, &StubRuntime);
    assert_eq!(response.status, 400);
}

#[test]
fn view_shape_revision_list_handler_returns_revisions() {
    // obligation: spec.mother-view-buffer-revision.mvbr6-api
    let response = view_buffer::handle_list_view_shape_revisions(&StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert!(payload
        .get("revisions")
        .and_then(|revisions| revisions.as_array())
        .is_some());
}

#[test]
fn view_maturation_handlers_expose_artifacts_and_record_events() {
    // obligation: spec.mother-view-maturation.mvmat6-api
    let derivations = view_buffer::handle_list_view_derivations(&StubRuntime);
    assert_eq!(derivations.status, 200);
    let derivations_payload: serde_json::Value = serde_json::from_slice(&derivations.body).unwrap();
    assert_eq!(
        derivations_payload
            .get("derivations")
            .and_then(|items| items.as_array())
            .map(Vec::len),
        Some(1)
    );

    let derivation_request = HttpRequest {
        method: "GET".to_string(),
        path: "/api/view-derivations/derivation_1".to_string(),
        headers: vec![],
        body: vec![],
    };
    assert_eq!(
        view_buffer::handle_get_view_derivation(&derivation_request, &StubRuntime).status,
        200
    );

    let pattern_upsert = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-patterns/upsert".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&stub_pattern()).unwrap(),
    };
    assert_eq!(
        view_buffer::handle_upsert_view_pattern(&pattern_upsert, &StubRuntime).status,
        200
    );

    let record_request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-maturation-events/record".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&crate::view_buffer::MatureViewArtifactRequest {
            target_kind: crate::view_buffer::ViewMaturationTargetKind::Derivation,
            shape_id: None,
            derivation_id: Some("derivation_1".to_string()),
            pattern_id: None,
            origin: crate::view_buffer::ViewMaturationOrigin::UserRequested,
            to_maturity: crate::view_buffer::ViewShapeMaturity::Stable,
            observability_improvement: Some(crate::view_buffer::ProposedObservabilityImprovement {
                desired_fact_path: "mother.status.memory_pressure.summary".to_string(),
                reason: "stable derivation should become observable".to_string(),
            }),
        })
        .unwrap(),
    };
    let record_response = view_buffer::handle_record_view_maturation(&record_request, &StubRuntime);
    assert_eq!(record_response.status, 200);
    let record_payload: serde_json::Value = serde_json::from_slice(&record_response.body).unwrap();
    assert_eq!(
        record_payload
            .get("event")
            .and_then(|event| event.get("target_kind"))
            .and_then(|value| value.as_str()),
        Some("derivation")
    );
    assert_eq!(
        record_payload
            .get("observability_improvement")
            .and_then(|artifact| artifact.get("work_item_created"))
            .and_then(|value| value.as_bool()),
        Some(false)
    );

    let artifacts = view_buffer::handle_list_view_observability_improvements(&StubRuntime);
    assert_eq!(artifacts.status, 200);
    let artifact_request = HttpRequest {
        method: "GET".to_string(),
        path: "/api/view-observability-improvements/maturation_1::observability-improvement"
            .to_string(),
        headers: vec![],
        body: vec![],
    };
    assert_eq!(
        view_buffer::handle_get_view_observability_improvement(&artifact_request, &StubRuntime)
            .status,
        200
    );
}

#[test]
fn view_maturation_handler_rejects_invalid_requests() {
    // obligation: spec.mother-view-maturation.mvmat7-tests-and-trace
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-maturation-events/record".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&crate::view_buffer::MatureViewArtifactRequest {
            target_kind: crate::view_buffer::ViewMaturationTargetKind::Pattern,
            shape_id: None,
            derivation_id: None,
            pattern_id: Some("pattern_1".to_string()),
            origin: crate::view_buffer::ViewMaturationOrigin::UserRequested,
            to_maturity: crate::view_buffer::ViewShapeMaturity::Stable,
            observability_improvement: Some(crate::view_buffer::ProposedObservabilityImprovement {
                desired_fact_path: "derived.fact".to_string(),
                reason: "not allowed".to_string(),
            }),
        })
        .unwrap(),
    };

    let response = view_buffer::handle_record_view_maturation(&request, &StubRuntime);
    assert_eq!(response.status, 400);
}

#[test]
fn view_shape_upsert_handler_returns_structured_shape() {
    // obligation: spec.mother-view-shape-library.mvsl3-shape-api
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-shapes/upsert".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&crate::view_buffer::mother_status_shape()).unwrap(),
    };

    let response = view_buffer::handle_upsert_view_shape(&request, &StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload
            .get("shape")
            .and_then(|shape| shape.get("shape_id"))
            .and_then(|value| value.as_str()),
        Some(crate::view_buffer::MOTHER_STATUS_SHAPE_ID)
    );
}

#[test]
fn view_shape_upsert_handler_rejects_executable_payload_fields() {
    // obligation: spec.mother-view-shape-library.mvsl3-shape-api
    let mut shape = serde_json::to_value(crate::view_buffer::mother_status_shape()).unwrap();
    shape["typescript"] = serde_json::json!("alert('not a shape')");
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-shapes/upsert".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&shape).unwrap(),
    };

    let response = view_buffer::handle_upsert_view_shape(&request, &StubRuntime);
    assert_eq!(response.status, 400);
}

#[test]
fn view_shape_get_and_deactivate_handlers_are_deterministic() {
    // obligation: spec.mother-view-shape-library.mvsl3-shape-api
    let get = HttpRequest {
        method: "GET".to_string(),
        path: format!(
            "/api/view-shapes/{}",
            crate::view_buffer::MOTHER_STATUS_SHAPE_ID
        ),
        headers: vec![],
        body: vec![],
    };
    assert_eq!(
        view_buffer::handle_get_view_shape(&get, &StubRuntime).status,
        200
    );

    let deactivate = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-shapes/deactivate".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "shape_id": crate::view_buffer::MOTHER_STATUS_SHAPE_ID,
        }))
        .unwrap(),
    };
    assert_eq!(
        view_buffer::handle_deactivate_view_shape(&deactivate, &StubRuntime).status,
        200
    );
}

#[test]
fn view_observability_workflow_handlers_link_and_resolve_gap() {
    // obligation: spec.mother-view-observability-workflow.mvow5-api
    let get_request = HttpRequest {
        method: "GET".to_string(),
        path: "/api/view-buffers/gaps/gap_1".to_string(),
        headers: vec![],
        body: vec![],
    };
    let get_response = view_buffer::handle_get_view_buffer_gap(&get_request, &StubRuntime);
    assert_eq!(get_response.status, 200);

    let link_request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-buffers/gaps/link-work-item".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&crate::view_buffer::LinkObservabilityGapRequest {
            gap_id: "gap_1".to_string(),
            work_item_id: "work/MOTHER-123".to_string(),
        })
        .unwrap(),
    };
    let link_response =
        view_buffer::handle_link_view_buffer_gap_work_item(&link_request, &StubRuntime);
    assert_eq!(link_response.status, 200);
    let link_payload: serde_json::Value = serde_json::from_slice(&link_response.body).unwrap();
    assert_eq!(
        link_payload
            .get("gap")
            .and_then(|gap| gap.get("status"))
            .and_then(|status| status.as_str()),
        Some("linked-to-work-item")
    );

    let resolve_request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-buffers/gaps/resolve".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&crate::view_buffer::ResolveObservabilityGapRequest {
            gap_id: "gap_1".to_string(),
        })
        .unwrap(),
    };
    let resolve_response =
        view_buffer::handle_resolve_view_buffer_gap(&resolve_request, &StubRuntime);
    assert_eq!(resolve_response.status, 200);
    let resolve_payload: serde_json::Value =
        serde_json::from_slice(&resolve_response.body).unwrap();
    assert_eq!(
        resolve_payload
            .get("gap")
            .and_then(|gap| gap.get("status"))
            .and_then(|status| status.as_str()),
        Some("resolved")
    );
}

#[test]
fn view_observability_workflow_handlers_reject_invalid_link() {
    // obligation: spec.mother-view-observability-workflow.mvow6-fail-closed-guardrails
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-buffers/gaps/link-work-item".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "gap_id": "gap_1",
            "work_item_id": " "
        }))
        .unwrap(),
    };

    let response = view_buffer::handle_link_view_buffer_gap_work_item(&request, &StubRuntime);
    assert_eq!(response.status, 400);
}

#[test]
fn view_buffer_open_handler_returns_framed_payload() {
    // obligation: rule-success.OpenLiveBufferWhenRequiredFactsAreObserved
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-buffers/open".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&crate::view_buffer::OpenBufferRequest {
            shape_id: crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string(),
        })
        .unwrap(),
    };

    let response = view_buffer::handle_open_view_buffer(&request, &StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload
            .get("payload")
            .and_then(|payload| payload.get("frame"))
            .and_then(|frame| frame.get("protocol"))
            .and_then(|value| value.as_str()),
        Some("patina:view-buffer")
    );
}

#[test]
fn view_buffer_payload_handler_returns_existing_framed_payload() {
    // obligation: spec.mother-sveltekit-frame.mskf4-render-framed-json
    let request = HttpRequest {
        method: "GET".to_string(),
        path: "/api/view-buffers/buf_1/payload".to_string(),
        headers: vec![],
        body: vec![],
    };

    let response = view_buffer::handle_get_view_buffer_payload(&request, &StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload
            .get("opened")
            .and_then(|opened| opened.get("payload"))
            .and_then(|payload| payload.get("frame"))
            .and_then(|frame| frame.get("buffer_id"))
            .and_then(|value| value.as_str()),
        Some("buf_1")
    );
}

#[test]
fn view_buffer_connect_handler_returns_window() {
    // obligation: rule-success.ConnectWindowToExistingBuffer
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/view-buffers/connect".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&crate::view_buffer::ConnectWindowRequest {
            frame_id: "frame_tui".to_string(),
            frame_kind: crate::view_buffer::FrameKind::Tui,
            window_id: "win_1".to_string(),
            buffer_id: "buf_1".to_string(),
        })
        .unwrap(),
    };

    let response = view_buffer::handle_connect_view_buffer_window(&request, &StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload
            .get("connection_state")
            .and_then(|value| value.as_str()),
        Some("connected")
    );
}

#[test]
fn ready_route_returns_503_when_runtime_not_ready() {
    struct NotReady;

    impl HealthApi for NotReady {
        fn version(&self) -> String {
            "0.0.0-test".to_string()
        }

        fn uptime_secs(&self) -> u64 {
            0
        }

        fn ready_status(&self) -> Result<bool> {
            Ok(false)
        }

        fn health_all(&self) -> Vec<(String, crate::ChildHealth)> {
            vec![]
        }

        fn health_details(&self) -> Result<HealthDetails> {
            Err(anyhow::anyhow!("unused"))
        }
    }

    let response = handle_ready(&NotReady);
    assert_eq!(response.status, 503);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload.get("error").and_then(|v| v.as_str()),
        Some("not_ready")
    );
}

#[test]
fn ready_route_returns_503_when_runtime_errors() {
    struct ReadyUnavailable;

    impl HealthApi for ReadyUnavailable {
        fn version(&self) -> String {
            "0.0.0-test".to_string()
        }

        fn uptime_secs(&self) -> u64 {
            0
        }

        fn ready_status(&self) -> Result<bool> {
            Err(anyhow::anyhow!("probe failed"))
        }

        fn health_all(&self) -> Vec<(String, crate::ChildHealth)> {
            vec![]
        }

        fn health_details(&self) -> Result<HealthDetails> {
            Err(anyhow::anyhow!("unused"))
        }
    }

    let response = handle_ready(&ReadyUnavailable);
    assert_eq!(response.status, 503);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload.get("error").and_then(|v| v.as_str()),
        Some("not_ready")
    );
}

#[test]
fn interface_control_route_translates_to_native_typed_call_shape() {
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/interface/call".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "operation_id": "patina:interface/handshake.v1",
            "args": {"project_uid": "2bdc808e"},
            "correlation": {
                "project_uid": "2bdc808e",
                "interface": "pi",
                "launch_id": "launch-1"
            }
        }))
        .unwrap(),
    };

    let response = handle_interface_control_call(&request, &StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload.get("adapter").and_then(|v| v.as_str()),
        Some("native")
    );
    assert_eq!(
        payload.get("operation_id").and_then(|v| v.as_str()),
        Some("patina:interface/handshake.v1")
    );
}

#[test]
fn interface_control_route_rejects_missing_operation_id() {
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/interface/call".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({"args": []})).unwrap(),
    };

    let response = handle_interface_control_call(&request, &StubRuntime);
    assert_eq!(response.status, 400);
}

#[test]
fn child_call_route_dispatches_typed_operation() {
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/child/fixture-typed-child/call".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "operation_id": "patina:fixture/control.status",
            "args": [],
            "correlation": {
                "rivet_run_id": "run-123",
                "rivet_actor_id": "actor-a"
            }
        }))
        .unwrap(),
    };

    let response = handle_child_request(&request, &StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload.get("operation_id").and_then(|v| v.as_str()),
        Some("patina:fixture/control.status")
    );
    assert_eq!(
        payload
            .get("correlation")
            .and_then(|v| v.get("rivet_run_id"))
            .and_then(|v| v.as_str()),
        Some("run-123")
    );
    assert_eq!(payload.get("typed"), Some(&serde_json::json!(true)));
}

#[test]
fn rivet_dispatch_route_translates_to_typed_call_shape() {
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/rivet/dispatch".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "child": "fixture-typed-child",
            "operation_id": "patina:fixture/control.status",
            "args": [],
            "correlation": {
                "rivet_run_id": "run-123"
            }
        }))
        .unwrap(),
    };

    let response = handle_rivet_dispatch(&request, &StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload.get("adapter").and_then(|v| v.as_str()),
        Some("rivet")
    );
    assert_eq!(
        payload
            .get("correlation")
            .and_then(|v| v.get("rivet_run_id"))
            .and_then(|v| v.as_str()),
        Some("run-123")
    );
    assert_eq!(
        payload.get("delivery").and_then(|v| v.as_str()),
        Some("required")
    );
}

#[test]
fn rivet_dispatch_route_rejects_dead_letter_without_target() {
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/rivet/dispatch".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "child": "fixture-typed-child",
            "operation_id": "patina:fixture/control.status",
            "args": [],
            "delivery": "dead-letter"
        }))
        .unwrap(),
    };

    let response = handle_rivet_dispatch(&request, &StubRuntime);
    assert_eq!(response.status, 400);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        payload.get("error").and_then(|v| v.as_str()),
        Some("invalid_request")
    );
}

#[test]
fn child_call_route_rejects_missing_operation_id() {
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/child/fixture-typed-child/call".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({"args": []})).unwrap(),
    };

    let response = handle_child_request(&request, &StubRuntime);
    assert_eq!(response.status, 400);
}

#[test]
fn inspector_typed_calls_route_returns_history() {
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/inspector/typed-calls".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({"limit": 10})).unwrap(),
    };

    let response = handle_inspector_typed_calls(&request, &StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(payload.get("count").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(
        payload
            .get("calls")
            .and_then(|v| v.as_array())
            .and_then(|items| items.first())
            .and_then(|v| v.get("operation_id"))
            .and_then(|v| v.as_str()),
        Some("patina:fixture/control.status")
    );
}

#[test]
fn inspector_typed_calls_filters_by_rivet_run_id() {
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/inspector/typed-calls".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "limit": 10,
            "rivet_run_id": "run-123"
        }))
        .unwrap(),
    };

    let response = handle_inspector_typed_calls(&request, &StubRuntime);
    assert_eq!(response.status, 200);
    let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(payload.get("count").and_then(|v| v.as_u64()), Some(1));

    let miss_request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/inspector/typed-calls".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "limit": 10,
            "rivet_run_id": "run-not-found"
        }))
        .unwrap(),
    };

    let miss_response = handle_inspector_typed_calls(&miss_request, &StubRuntime);
    assert_eq!(miss_response.status, 200);
    let miss_payload: serde_json::Value = serde_json::from_slice(&miss_response.body).unwrap();
    assert_eq!(miss_payload.get("count").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(
        miss_payload
            .get("calls")
            .and_then(|v| v.as_array())
            .map(|items| items.len()),
        Some(0)
    );
}

#[test]
fn bridge_translate_route_returns_allow_and_deny_payloads() {
    let allow_request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/bridge/translate".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&crate::bridge::BridgeRequest {
            action: "dispatch".to_string(),
            legacy_toys: vec!["log".to_string(), "state".to_string()],
            payload: serde_json::Value::Null,
        })
        .unwrap(),
    };
    let allow_response = bridge::handle_bridge_translate(&allow_request, &StubRuntime);
    assert_eq!(allow_response.status, 200);
    let allow_json: serde_json::Value = serde_json::from_slice(&allow_response.body).unwrap();
    assert_eq!(
        allow_json.get("verdict").and_then(|v| v.as_str()),
        Some("allow")
    );

    let deny_request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/bridge/translate".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&crate::bridge::BridgeRequest {
            action: "dispatch".to_string(),
            legacy_toys: vec!["log".to_string(), "nope".to_string()],
            payload: serde_json::Value::Null,
        })
        .unwrap(),
    };
    let deny_response = bridge::handle_bridge_translate(&deny_request, &StubRuntime);
    assert_eq!(deny_response.status, 200);
    let deny_json: serde_json::Value = serde_json::from_slice(&deny_response.body).unwrap();
    assert_eq!(
        deny_json.get("verdict").and_then(|v| v.as_str()),
        Some("deny")
    );
}

#[test]
fn bridge_translate_route_rejects_invalid_json() {
    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/bridge/translate".to_string(),
        headers: vec![],
        body: b"not-json".to_vec(),
    };

    let response = bridge::handle_bridge_translate(&request, &StubRuntime);
    assert_eq!(response.status, 400);
}

#[test]
fn federation_routes_return_json_payloads() {
    let status_request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/federation/status".to_string(),
        headers: vec![],
        body: vec![],
    };
    let status_response = federation::handle_federation_status(&status_request, &StubRuntime);
    assert_eq!(status_response.status, 200);

    let query_request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/federation/query".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "sql": "SELECT 1",
            "params": [],
            "limit": 1,
            "timeout_ms": 100
        }))
        .unwrap(),
    };
    let query_response = federation::handle_federation_query(&query_request, &StubRuntime);
    assert_eq!(query_response.status, 200);
}

#[test]
fn route_table_wiring_preserves_handler_surface() {
    let routes = build_route_table(Arc::new(StubRuntime));

    let get = HttpRequest {
        method: "GET".to_string(),
        path: "/health".to_string(),
        headers: vec![],
        body: vec![],
    };
    assert_eq!((routes.get_health)(&get).status, 200);
    assert_eq!((routes.get_ready)(&get).status, 204);
    assert_eq!((routes.get_version)(&get).status, 200);

    let lifecycle_request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/lifecycle/load-pando".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({"name": "demo"})).unwrap(),
    };
    assert_eq!(
        (routes.post_lifecycle_load_pando)(&lifecycle_request).status,
        200
    );

    let interface_request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/interface/call".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "operation_id": "patina:interface/handshake.v1",
            "args": []
        }))
        .unwrap(),
    };
    assert_eq!((routes.post_interface_call)(&interface_request).status, 200);

    let rivet_request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/rivet/dispatch".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "child": "fixture-typed-child",
            "operation_id": "patina:fixture/control.status",
            "args": []
        }))
        .unwrap(),
    };
    assert_eq!((routes.post_rivet_dispatch)(&rivet_request).status, 200);

    let inspector_request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/inspector/typed-calls".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({"limit": 10})).unwrap(),
    };
    assert_eq!(
        (routes.post_inspector_typed_calls)(&inspector_request).status,
        200
    );

    let child_request = HttpRequest {
        method: "POST".to_string(),
        path: "/child/fixture-typed-child/call".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "operation_id": "patina:fixture/control.status",
            "args": []
        }))
        .unwrap(),
    };
    assert_eq!((routes.child_request)(&child_request).status, 200);
}

#[test]
fn lifecycle_warmup_maps_resource_exhausted_to_429_envelope() {
    struct BusyRuntime;

    impl LifecycleApi for BusyRuntime {
        fn lifecycle_load_pando(&self, _name: &str) -> Result<crate::PandoLoadResult> {
            Err(anyhow::Error::new(LifecycleError::operation_in_progress(
                "load already running",
            )))
        }

        fn lifecycle_refresh(&self) -> Result<crate::PandoRefreshResult> {
            Err(anyhow::Error::new(LifecycleError::operation_in_progress(
                "refresh already running",
            )))
        }

        fn lifecycle_reload_child(&self, _name: &str) -> Result<crate::ChildReloadResult> {
            Err(anyhow::Error::new(LifecycleError::operation_in_progress(
                "reload already running",
            )))
        }

        fn lifecycle_warmup_children(&self) -> Result<crate::ChildWarmupResult> {
            Err(anyhow::Error::new(LifecycleError::resource_exhausted(
                "memory pressure high; warmup denied",
            )))
        }
    }

    impl RivetApi for BusyRuntime {
        fn rivet_dispatch(&self, _request: RivetDispatchRequest) -> Result<serde_json::Value> {
            Err(anyhow::Error::new(LifecycleError::invalid_request(
                "rivet integration is disabled",
            )))
        }
    }

    let request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/lifecycle/reload-child".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({"name": "x"})).unwrap(),
    };
    let response = handle_lifecycle_reload_child(&request, &BusyRuntime);
    assert_eq!(response.status, 409);
    let json: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(
        json.get("error").and_then(|v| v.as_str()),
        Some("operation_in_progress")
    );

    let warmup_request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/lifecycle/warmup-children".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({})).unwrap(),
    };
    let warmup_response = handle_lifecycle_warmup_children(&warmup_request, &BusyRuntime);
    assert_eq!(warmup_response.status, 429);
    let warmup_json: serde_json::Value = serde_json::from_slice(&warmup_response.body).unwrap();
    assert_eq!(
        warmup_json.get("error").and_then(|v| v.as_str()),
        Some("resource_exhausted")
    );

    let rivet_request = HttpRequest {
        method: "POST".to_string(),
        path: "/api/rivet/dispatch".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "child": "fixture-typed-child",
            "operation_id": "patina:fixture/control.status",
            "args": []
        }))
        .unwrap(),
    };
    let rivet_response = handle_rivet_dispatch(&rivet_request, &BusyRuntime);
    assert_eq!(rivet_response.status, 400);
    let rivet_json: serde_json::Value = serde_json::from_slice(&rivet_response.body).unwrap();
    assert_eq!(
        rivet_json.get("error").and_then(|v| v.as_str()),
        Some("invalid_request")
    );
}
