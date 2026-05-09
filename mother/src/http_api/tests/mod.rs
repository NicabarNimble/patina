use super::*;

struct StubRuntime;

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
                "child": "folder-watch-actor",
                "operation_id": "patina:watch/control.status",
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
        path: "/child/folder-watch-actor/call".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "operation_id": "patina:watch/control.status",
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
        Some("patina:watch/control.status")
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
            "child": "folder-watch-actor",
            "operation_id": "patina:watch/control.status",
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
            "child": "folder-watch-actor",
            "operation_id": "patina:watch/control.status",
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
        path: "/child/folder-watch-actor/call".to_string(),
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
        Some("patina:watch/control.status")
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
            "child": "folder-watch-actor",
            "operation_id": "patina:watch/control.status",
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
        path: "/child/folder-watch-actor/call".to_string(),
        headers: vec![],
        body: serde_json::to_vec(&serde_json::json!({
            "operation_id": "patina:watch/control.status",
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
            "child": "folder-watch-actor",
            "operation_id": "patina:watch/control.status",
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
