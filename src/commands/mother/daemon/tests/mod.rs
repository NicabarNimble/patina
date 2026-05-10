use super::*;
use crate::commands::mother::federation;
use patina::mother::{Child, ChildHealth, ChildRequest, ChildResponse, MotherHost};
use patina_ai_child_slate_manager as slate_manager_child;
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
struct SlateDispatchChild;
struct ParitySlateDispatchChild;
struct ScaffoldSlateDispatchChild;

fn spec_dispatch_request_with_route(
    command: patina::spec::SpecCommands,
    backend_mode: Option<&str>,
    project: Option<String>,
    origin_project: Option<String>,
) -> patina_protocol::SpecDispatchRequest {
    #[derive(serde::Serialize)]
    struct SpecDispatchEnvelope {
        command: patina::spec::SpecCommands,
        project: Option<String>,
        origin_project: Option<String>,
        backend_mode: Option<String>,
    }

    patina_protocol::SpecDispatchRequest {
        command: serde_json::to_value(SpecDispatchEnvelope {
            command,
            project,
            origin_project,
            backend_mode: backend_mode.map(|value| value.to_string()),
        })
        .expect("serialize envelope"),
    }
}

fn spec_dispatch_request(
    command: patina::spec::SpecCommands,
    backend_mode: Option<&str>,
) -> patina_protocol::SpecDispatchRequest {
    spec_dispatch_request_with_route(command, backend_mode, None, None)
}

fn slate_call_project_value(request: &patina::mother::ChildCallRequest) -> serde_json::Value {
    request
        .args
        .as_array()
        .and_then(|values| values.first())
        .and_then(|value| value.as_object())
        .and_then(|row| row.get("project"))
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

fn slate_typed_call_to_dispatch_envelope(
    request: &patina::mother::ChildCallRequest,
) -> Result<String> {
    let args = request
        .args
        .as_array()
        .and_then(|values| values.first())
        .and_then(|value| value.as_object())
        .ok_or_else(|| anyhow::anyhow!("expected typed slate args[0] object"))?;

    let command = match request.operation_id.as_str() {
        "patina:slate/control@0.1.0.list-specs" => serde_json::json!({
            "list": {
                "status": args.get("status").cloned().unwrap_or(serde_json::Value::Null),
                "target": args.get("target").cloned().unwrap_or(serde_json::Value::Null),
                "json": true,
            }
        }),
        "patina:slate/control@0.1.0.next-specs" => serde_json::json!({
            "next": {
                "json": true,
            }
        }),
        "patina:slate/control@0.1.0.check-spec" => serde_json::json!({
            "check": {
                "id": args.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "json": true,
            }
        }),
        "patina:slate/control@0.1.0.show-spec" => serde_json::json!({
            "show": {
                "id": args.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "handoff": false,
                "json": true,
            }
        }),
        "patina:slate/control@0.1.0.prompt-spec" => serde_json::json!({
            "prompt": {
                "id": args.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "json": true,
            }
        }),
        "patina:slate/control@0.1.0.handoff-spec" => serde_json::json!({
            "handoff": {
                "id": args.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "json": true,
            }
        }),
        "patina:slate/control@0.1.0.packet-spec" => serde_json::json!({
            "packet": {
                "id": args.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "json": true,
            }
        }),
        "patina:slate/control@0.1.0.complete-spec" => serde_json::json!({
            "complete": {
                "id": args.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "major": args.get("major").cloned().unwrap_or(serde_json::Value::Bool(false)),
                "force": args.get("force").cloned().unwrap_or(serde_json::Value::Bool(false)),
                "json": true,
            }
        }),
        "patina:slate/control@0.1.0.archive-spec" => serde_json::json!({
            "archive": {
                "id": args.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "stale": args.get("stale").cloned().unwrap_or(serde_json::Value::Bool(false)),
                "dry_run": args.get("dry-run").cloned().unwrap_or(serde_json::Value::Bool(false)),
            }
        }),
        other => {
            return Err(anyhow::anyhow!(
                "unexpected typed slate operation id: {}",
                other
            ));
        }
    };

    let envelope = serde_json::json!({
        "command": command,
        "project": args.get("project").cloned().unwrap_or(serde_json::Value::Null),
        "backend_mode": "execute",
    });

    Ok(envelope.to_string())
}

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

impl Child for SlateDispatchChild {
    fn name(&self) -> &str {
        "slate-manager"
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
        if !request
            .operation_id
            .starts_with("patina:slate/control@0.1.0.")
        {
            return Err(anyhow::anyhow!(
                "unexpected operation_id: {}",
                request.operation_id
            ));
        }

        let (command_bytes, project) =
            if request.operation_id == "patina:slate/control@0.1.0.dispatch" {
                let command_json = request
                    .args
                    .as_array()
                    .and_then(|values| values.first())
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| anyhow::anyhow!("expected slate args[0] command JSON string"))?;
                let envelope: serde_json::Value = serde_json::from_str(command_json)
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                (
                    command_json.len(),
                    envelope
                        .get("project")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                )
            } else {
                (0, slate_call_project_value(request))
            };

        let data = serde_json::json!({
            "status": "from-slate",
            "command_bytes": command_bytes,
            "project": project,
            "operation_id": request.operation_id,
        });

        Ok(ChildResponse {
            payload: serde_json::json!({
                "results": [
                    {
                        "ok": data.to_string(),
                    }
                ]
            }),
        })
    }
}

impl Child for ParitySlateDispatchChild {
    fn name(&self) -> &str {
        "slate-manager"
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
        if !request
            .operation_id
            .starts_with("patina:slate/control@0.1.0.")
        {
            return Err(anyhow::anyhow!(
                "unexpected operation_id: {}",
                request.operation_id
            ));
        }

        let command_json = if request.operation_id == "patina:slate/control@0.1.0.dispatch" {
            request
                .args
                .as_array()
                .and_then(|values| values.first())
                .and_then(|value| value.as_str())
                .ok_or_else(|| anyhow::anyhow!("expected slate args[0] command JSON string"))?
                .to_string()
        } else {
            slate_typed_call_to_dispatch_envelope(request)?
        };

        let payload = match slate_manager_child::dispatch_for_test(&command_json) {
            Ok(data) => serde_json::json!({
                "results": [
                    {
                        "ok": data,
                    }
                ]
            }),
            Err(error) => serde_json::json!({
                "results": [
                    {
                        "err": error,
                    }
                ]
            }),
        };

        Ok(ChildResponse { payload })
    }
}

impl Child for ScaffoldSlateDispatchChild {
    fn name(&self) -> &str {
        "slate-manager"
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
        if !request
            .operation_id
            .starts_with("patina:slate/control@0.1.0.")
        {
            return Err(anyhow::anyhow!(
                "unexpected operation_id: {}",
                request.operation_id
            ));
        }

        let scaffold = serde_json::json!({
            "status": "scaffold",
            "message": "not implemented",
        });

        Ok(ChildResponse {
            payload: serde_json::json!({
                "results": [
                    {
                        "ok": scaffold.to_string(),
                    }
                ]
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
fn daemon_view_request_compose_persists_adapted_shape() {
    // obligation: spec.mother-view-shape-adaptation.mvsa3-adapted-shape-persistence
    // obligation: spec.mother-view-shape-adaptation.mvsa4-compose-integration
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

        let composed = <ServerState as mother_crate::http_api::ApiRuntime>::view_request_compose(
            &state,
            mother_crate::view_buffer::ComposeViewRequest {
                user_id: "local-user".to_string(),
                agent_id: "pi".to_string(),
                raw_request: "show something like mother status".to_string(),
                proposed_match: Some(mother_crate::view_buffer::ProposedShapeMatch {
                    shape_id: Some(mother_crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string()),
                    match_kind: mother_crate::view_buffer::ShapeMatchKind::Similar,
                    confidence: mother_crate::view_buffer::SHAPE_MATCH_CONFIDENCE_THRESHOLD,
                }),
                proposed_initial_shape: None,
            },
        )
        .expect("compose should adapt similar shape");

        let adapted_shape = composed
            .adapted_shape
            .expect("daemon compose should return adapted shape");
        assert_eq!(
            composed.request.outcome,
            mother_crate::view_buffer::DisplayRequestOutcome::Unable
        );
        assert!(composed.open_outcome.is_none());
        assert_eq!(runtime_store.list_view_buffers().unwrap().len(), 0);
        assert_eq!(
            runtime_store
                .get_view_shape_match(&composed.request.request_id)
                .unwrap()
                .expect("shape match should persist")
                .match_kind,
            mother_crate::view_buffer::ShapeMatchKind::Similar
        );
        let persisted_shape = runtime_store
            .get_view_shape(&adapted_shape.shape_id)
            .unwrap()
            .expect("adapted shape should persist through daemon");
        assert_eq!(persisted_shape.shape_id, adapted_shape.shape_id);
        assert_eq!(
            persisted_shape.maturity,
            mother_crate::view_buffer::ViewShapeMaturity::Exploratory
        );
    });
}

#[test]
fn daemon_view_request_compose_persists_created_initial_shape() {
    // obligation: spec.mother-view-initial-shape-creation.mvisc4-persistence
    // obligation: spec.mother-view-initial-shape-creation.mvisc5-compose-integration
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

        let composed = <ServerState as mother_crate::http_api::ApiRuntime>::view_request_compose(
            &state,
            mother_crate::view_buffer::ComposeViewRequest {
                user_id: "local-user".to_string(),
                agent_id: "pi".to_string(),
                raw_request: "show runtime summary".to_string(),
                proposed_match: Some(mother_crate::view_buffer::ProposedShapeMatch {
                    shape_id: None,
                    match_kind: mother_crate::view_buffer::ShapeMatchKind::None,
                    confidence: 0.0,
                }),
                proposed_initial_shape: Some(mother_crate::view_buffer::ProposedInitialShape {
                    title: "Mother Runtime Summary".to_string(),
                    major_mode: mother_crate::view_buffer::MajorMode::Table,
                    minor_modes: vec![mother_crate::view_buffer::MinorMode::Pinned],
                    requirements: vec![mother_crate::view_buffer::ViewRequirement {
                        fact_path: "mother.status.version".to_string(),
                        required: true,
                        purpose: "display Mother version".to_string(),
                    }],
                    vision_id: None,
                    project_uid: None,
                }),
            },
        )
        .expect("compose should create initial shape");

        let created_shape = composed
            .created_shape
            .expect("daemon compose should return created shape");
        assert_eq!(
            composed.request.outcome,
            mother_crate::view_buffer::DisplayRequestOutcome::Unable
        );
        assert!(composed.open_outcome.is_none());
        assert_eq!(runtime_store.list_view_buffers().unwrap().len(), 0);
        assert_eq!(
            runtime_store
                .get_view_shape_match(&composed.request.request_id)
                .unwrap()
                .expect("shape match should persist")
                .match_kind,
            mother_crate::view_buffer::ShapeMatchKind::None
        );
        let persisted_shape = runtime_store
            .get_view_shape(&created_shape.shape_id)
            .unwrap()
            .expect("created shape should persist through daemon");
        assert_eq!(persisted_shape.shape_id, created_shape.shape_id);
        assert_eq!(persisted_shape.title, "Mother Runtime Summary");
        assert_eq!(
            persisted_shape.maturity,
            mother_crate::view_buffer::ViewShapeMaturity::Exploratory
        );
    });
}

#[test]
fn daemon_view_maturation_persists_event_and_improvement_artifact() {
    // obligation: spec.mother-view-maturation.mvmat6-api
    // obligation: spec.mother-view-maturation.mvmat7-tests-and-trace
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

        let derivation = mother_crate::view_buffer::ViewDerivation {
            derivation_id: "derivation_1".to_string(),
            shape_id: mother_crate::view_buffer::MOTHER_STATUS_SHAPE_ID.to_string(),
            label: "Memory Pressure Summary".to_string(),
            expression_ref: "allium://views/mother/status/memory-pressure".to_string(),
            input_fact_paths: vec!["mother.status.memory_pressure".to_string()],
            maturity: mother_crate::view_buffer::ViewShapeMaturity::Candidate,
        };
        <ServerState as mother_crate::http_api::ApiRuntime>::view_derivation_upsert(
            &state,
            derivation.clone(),
        )
        .expect("derivation should persist");

        let outcome = <ServerState as mother_crate::http_api::ApiRuntime>::view_maturation_record(
            &state,
            mother_crate::view_buffer::MatureViewArtifactRequest {
                target_kind: mother_crate::view_buffer::ViewMaturationTargetKind::Derivation,
                shape_id: None,
                derivation_id: Some(derivation.derivation_id.clone()),
                pattern_id: None,
                origin: mother_crate::view_buffer::ViewMaturationOrigin::UserRequested,
                to_maturity: mother_crate::view_buffer::ViewShapeMaturity::Stable,
                observability_improvement: Some(
                    mother_crate::view_buffer::ProposedObservabilityImprovement {
                        desired_fact_path: "mother.status.memory_pressure.summary".to_string(),
                        reason: "stable derivation should become observable".to_string(),
                    },
                ),
            },
        )
        .expect("maturation should persist");

        assert_eq!(
            runtime_store
                .get_view_derivation(&derivation.derivation_id)
                .unwrap()
                .expect("derivation persists")
                .maturity,
            mother_crate::view_buffer::ViewShapeMaturity::Stable
        );
        assert!(runtime_store
            .get_view_maturation_event(&outcome.event.maturation_id)
            .unwrap()
            .is_some());
        let artifact = outcome
            .observability_improvement
            .expect("observability improvement returned");
        assert_eq!(artifact.work_item_created, false);
        assert_eq!(
            runtime_store
                .get_view_observability_improvement(&artifact.artifact_id)
                .unwrap()
                .and_then(|artifact| artifact.source_maturation_id),
            Some(outcome.event.maturation_id)
        );
    });
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
fn rivet_dispatch_interface_control_child_uses_native_interface_handler() {
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

        let project_uid = patina::project::create_uid_if_missing(project_root).unwrap();

        let response = state
            .rivet_dispatch(mother_crate::http_api::RivetDispatchRequest {
                child: "interface-control".to_string(),
                operation_id: "patina:interface/handshake.v1".to_string(),
                args: serde_json::json!({
                    "protocol_version": "0.1",
                    "cli_version": env!("CARGO_PKG_VERSION"),
                    "project_uid": project_uid,
                    "project_root": project_root.display().to_string(),
                    "interface_name": "pi",
                    "interface_kind": "hitl",
                    "launch_intent": "attach-or-create",
                    "tty": false
                }),
                correlation: None,
                delivery: None,
                dead_letter: None,
            })
            .expect("rivet interface-control dispatch should use native interface handler");

        assert_eq!(
            response.get("adapter").and_then(|v| v.as_str()),
            Some("rivet")
        );
        assert_eq!(
            response
                .get("payload")
                .and_then(|v| v.get("adapter"))
                .and_then(|v| v.as_str()),
            Some("native")
        );
        assert_eq!(
            response
                .get("payload")
                .and_then(|v| v.get("operation_id"))
                .and_then(|v| v.as_str()),
            Some("patina:interface/handshake.v1")
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
fn builtin_spec_dispatch_execute_routes_through_slate_manager() {
    with_temp_project(|project_root| {
        let runtime_store = patina::mother::MotherRuntimeStore::new(
            project_root.join(".patina/local/data/mother-state.db"),
        );

        let manifest_path = project_root.join("slate-manager.toml");
        std::fs::write(
            &manifest_path,
            r#"[child]
name = "slate-manager"
version = "0.1.0"
world = "child"

[child.ingress]
mode = "hybrid"

[child.contract]
allow = [
  "patina:slate/control@0.1.0.dispatch",
  "patina:slate/control@0.1.0.list-specs",
  "patina:slate/control@0.1.0.next-specs",
  "patina:slate/control@0.1.0.check-spec",
  "patina:slate/control@0.1.0.show-spec",
  "patina:slate/control@0.1.0.prompt-spec",
  "patina:slate/control@0.1.0.handoff-spec",
  "patina:slate/control@0.1.0.packet-spec",
  "patina:slate/control@0.1.0.complete-spec",
  "patina:slate/control@0.1.0.archive-spec",
]
"#,
        )
        .expect("write manifest");

        let registry = ChildRegistry::new();
        registry
            .register_knowledge_with_paths(
                Box::new(SlateDispatchChild),
                std::path::PathBuf::new(),
                manifest_path,
            )
            .expect("register slate child");

        let state = ServerState::new(ServerStateInit {
            token: "test-token".to_string(),
            startup_profile: DaemonStartupProfile::Core,
            rivet_integration: RivetIntegrationProfile::Disabled,
            registry,
            runtime_store: runtime_store.clone(),
            startup_store: runtime_store.clone(),
            federation_runtime: federation::startup(&runtime_store),
            readiness: Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default())),
        });

        let request = spec_dispatch_request(
            patina::spec::SpecCommands::Next { json: true },
            Some("execute"),
        );

        let response = <ServerState as mother_crate::http_api::ApiRuntime>::builtin_spec_dispatch(
            &state, request,
        )
        .expect("execute mode should route through slate");

        assert_eq!(
            response.get("backend").and_then(|v| v.get("mode")),
            Some(&serde_json::json!("execute"))
        );
        assert_eq!(
            response.get("backend").and_then(|v| v.get("engine")),
            Some(&serde_json::json!("slate-manager"))
        );
        assert_eq!(
            response.get("data").and_then(|v| v.get("status")),
            Some(&serde_json::json!("from-slate"))
        );
        assert_eq!(response.get("json"), Some(&serde_json::json!(true)));
    });
}

#[test]
fn builtin_spec_dispatch_execute_forwards_project_route_to_slate_manager() {
    with_temp_project(|project_root| {
        let runtime_store = patina::mother::MotherRuntimeStore::new(
            project_root.join(".patina/local/data/mother-state.db"),
        );

        let manifest_path = project_root.join("slate-manager.toml");
        std::fs::write(
            &manifest_path,
            r#"[child]
name = "slate-manager"
version = "0.1.0"
world = "child"

[child.ingress]
mode = "hybrid"

[child.contract]
allow = [
  "patina:slate/control@0.1.0.dispatch",
  "patina:slate/control@0.1.0.list-specs",
  "patina:slate/control@0.1.0.next-specs",
  "patina:slate/control@0.1.0.check-spec",
  "patina:slate/control@0.1.0.show-spec",
  "patina:slate/control@0.1.0.prompt-spec",
  "patina:slate/control@0.1.0.handoff-spec",
  "patina:slate/control@0.1.0.packet-spec",
  "patina:slate/control@0.1.0.complete-spec",
  "patina:slate/control@0.1.0.archive-spec",
]
"#,
        )
        .expect("write manifest");

        let registry = ChildRegistry::new();
        registry
            .register_knowledge_with_paths(
                Box::new(SlateDispatchChild),
                std::path::PathBuf::new(),
                manifest_path,
            )
            .expect("register slate child");

        let state = ServerState::new(ServerStateInit {
            token: "test-token".to_string(),
            startup_profile: DaemonStartupProfile::Core,
            rivet_integration: RivetIntegrationProfile::Disabled,
            registry,
            runtime_store: runtime_store.clone(),
            startup_store: runtime_store.clone(),
            federation_runtime: federation::startup(&runtime_store),
            readiness: Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default())),
        });

        let expected_project = project_root.display().to_string();
        let request = spec_dispatch_request_with_route(
            patina::spec::SpecCommands::Next { json: true },
            Some("execute"),
            Some(expected_project.clone()),
            None,
        );

        let response = <ServerState as mother_crate::http_api::ApiRuntime>::builtin_spec_dispatch(
            &state, request,
        )
        .expect("execute mode should route through slate");

        assert_eq!(
            response
                .get("data")
                .and_then(|v| v.get("project"))
                .and_then(|v| v.as_str()),
            Some(expected_project.as_str())
        );
    });
}

#[test]
fn builtin_spec_dispatch_execute_fails_closed_when_slate_is_scaffold_only() {
    with_temp_project(|project_root| {
        std::fs::create_dir_all(patina::paths::project::patina_dir(project_root))
            .expect("create .patina");
        std::fs::create_dir_all(project_root.join("layer")).expect("create layer");

        let runtime_store = patina::mother::MotherRuntimeStore::new(
            project_root.join(".patina/local/data/mother-state.db"),
        );

        let manifest_path = project_root.join("slate-manager.toml");
        std::fs::write(
            &manifest_path,
            r#"[child]
name = "slate-manager"
version = "0.1.0"
world = "child"

[child.ingress]
mode = "hybrid"

[child.contract]
allow = [
  "patina:slate/control@0.1.0.dispatch",
  "patina:slate/control@0.1.0.list-specs",
  "patina:slate/control@0.1.0.next-specs",
  "patina:slate/control@0.1.0.check-spec",
  "patina:slate/control@0.1.0.show-spec",
  "patina:slate/control@0.1.0.prompt-spec",
  "patina:slate/control@0.1.0.handoff-spec",
  "patina:slate/control@0.1.0.packet-spec",
  "patina:slate/control@0.1.0.complete-spec",
  "patina:slate/control@0.1.0.archive-spec",
]
"#,
        )
        .expect("write manifest");

        let registry = ChildRegistry::new();
        registry
            .register_knowledge_with_paths(
                Box::new(ScaffoldSlateDispatchChild),
                std::path::PathBuf::new(),
                manifest_path,
            )
            .expect("register scaffold slate child");

        let state = ServerState::new(ServerStateInit {
            token: "test-token".to_string(),
            startup_profile: DaemonStartupProfile::Core,
            rivet_integration: RivetIntegrationProfile::Disabled,
            registry,
            runtime_store: runtime_store.clone(),
            startup_store: runtime_store.clone(),
            federation_runtime: federation::startup(&runtime_store),
            readiness: Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default())),
        });

        let request = spec_dispatch_request(
            patina::spec::SpecCommands::List {
                status: None,
                target: None,
                json: true,
            },
            Some("execute"),
        );

        let error = <ServerState as mother_crate::http_api::ApiRuntime>::builtin_spec_dispatch(
            &state, request,
        )
        .expect_err("execute mode should fail closed when slate is scaffold-only");

        assert!(error
            .to_string()
            .contains("slate-manager returned scaffold response"));
    });
}

#[test]
fn builtin_spec_dispatch_execute_fails_closed_without_slate_manager() {
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

        let request = spec_dispatch_request(
            patina::spec::SpecCommands::Next { json: true },
            Some("execute"),
        );

        let error = <ServerState as mother_crate::http_api::ApiRuntime>::builtin_spec_dispatch(
            &state, request,
        )
        .expect_err("execute mode should fail closed when slate child is missing");

        assert!(error.to_string().contains("slate execute dispatch failed"));
        assert!(error.to_string().contains("slate-manager unavailable"));
    });
}

#[test]
fn builtin_spec_dispatch_observe_includes_slate_probe_when_available() {
    with_temp_project(|project_root| {
        std::fs::create_dir_all(patina::paths::project::patina_dir(project_root))
            .expect("create .patina");
        std::fs::create_dir_all(project_root.join("layer")).expect("create layer");

        let runtime_store = patina::mother::MotherRuntimeStore::new(
            project_root.join(".patina/local/data/mother-state.db"),
        );

        let manifest_path = project_root.join("slate-manager.toml");
        std::fs::write(
            &manifest_path,
            r#"[child]
name = "slate-manager"
version = "0.1.0"
world = "child"

[child.ingress]
mode = "hybrid"

[child.contract]
allow = [
  "patina:slate/control@0.1.0.dispatch",
  "patina:slate/control@0.1.0.list-specs",
  "patina:slate/control@0.1.0.next-specs",
  "patina:slate/control@0.1.0.check-spec",
  "patina:slate/control@0.1.0.show-spec",
  "patina:slate/control@0.1.0.prompt-spec",
  "patina:slate/control@0.1.0.handoff-spec",
  "patina:slate/control@0.1.0.packet-spec",
  "patina:slate/control@0.1.0.complete-spec",
  "patina:slate/control@0.1.0.archive-spec",
]
"#,
        )
        .expect("write manifest");

        let registry = ChildRegistry::new();
        registry
            .register_knowledge_with_paths(
                Box::new(SlateDispatchChild),
                std::path::PathBuf::new(),
                manifest_path,
            )
            .expect("register slate child");

        let state = ServerState::new(ServerStateInit {
            token: "test-token".to_string(),
            startup_profile: DaemonStartupProfile::Core,
            rivet_integration: RivetIntegrationProfile::Disabled,
            registry,
            runtime_store: runtime_store.clone(),
            startup_store: runtime_store.clone(),
            federation_runtime: federation::startup(&runtime_store),
            readiness: Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default())),
        });

        let request = spec_dispatch_request(
            patina::spec::SpecCommands::List {
                status: None,
                target: None,
                json: true,
            },
            Some("observe"),
        );

        let response = <ServerState as mother_crate::http_api::ApiRuntime>::builtin_spec_dispatch(
            &state, request,
        )
        .expect("observe mode should preserve builtin response and include probe");

        assert_eq!(
            response.get("backend").and_then(|v| v.get("mode")),
            Some(&serde_json::json!("observe"))
        );
        assert_eq!(
            response.get("backend").and_then(|v| v.get("engine")),
            Some(&serde_json::json!("builtin-spec-manager"))
        );
        assert_eq!(
            response
                .get("backend")
                .and_then(|v| v.get("slate_probe"))
                .and_then(|v| v.get("status")),
            Some(&serde_json::json!("called"))
        );
        assert_eq!(
            response
                .get("backend")
                .and_then(|v| v.get("slate_probe"))
                .and_then(|v| v.get("data"))
                .and_then(|v| v.get("status")),
            Some(&serde_json::json!("from-slate"))
        );
    });
}

#[test]
fn builtin_spec_dispatch_observe_fixture_diff_harness_reports_builtin_and_probe_payloads() {
    with_temp_project(|project_root| {
        std::fs::create_dir_all(patina::paths::project::patina_dir(project_root))
            .expect("create .patina");
        std::fs::create_dir_all(project_root.join("layer")).expect("create layer");
        patina::project::save(project_root, &patina::project::ProjectConfig::default())
            .expect("write project config");
        let spec_dir = project_root.join("layer/surface/build/feat/slate-observe-fixture");
        std::fs::create_dir_all(&spec_dir).expect("create spec dir");

        std::fs::write(
            spec_dir.join("SPEC.md"),
            r#"---
type: feat
id: slate-observe-fixture
status: active
target: "1"
exit_criteria:
  - id: fixture-pass
    text: "Fixture criterion"
    checked: true
---
# Slate observe fixture

## Goal
Validate observe-mode diff harness plumbing.

## Key Files
```
src/spec.rs
children/slate-manager/src/lib.rs
```

## Implementation Order
- Step one

## Resolved Decisions
- Decision one

## Verification
- Run command parity checks
"#,
        )
        .expect("write SPEC.md");

        std::fs::write(
            spec_dir.join("DESIGN.md"),
            r#"# Design

## Direct Code Targets
- src/spec.rs
- children/slate-manager/src/lib.rs

## Open Questions
- None
"#,
        )
        .expect("write DESIGN.md");

        let runtime_store = patina::mother::MotherRuntimeStore::new(
            project_root.join(".patina/local/data/mother-state.db"),
        );

        let manifest_path = project_root.join("slate-manager.toml");
        std::fs::write(
            &manifest_path,
            r#"[child]
name = "slate-manager"
version = "0.1.0"
world = "child"

[child.ingress]
mode = "hybrid"

[child.contract]
allow = [
  "patina:slate/control@0.1.0.dispatch",
  "patina:slate/control@0.1.0.list-specs",
  "patina:slate/control@0.1.0.next-specs",
  "patina:slate/control@0.1.0.check-spec",
  "patina:slate/control@0.1.0.show-spec",
  "patina:slate/control@0.1.0.prompt-spec",
  "patina:slate/control@0.1.0.handoff-spec",
  "patina:slate/control@0.1.0.packet-spec",
  "patina:slate/control@0.1.0.complete-spec",
  "patina:slate/control@0.1.0.archive-spec",
]
"#,
        )
        .expect("write manifest");

        let registry = ChildRegistry::new();
        registry
            .register_knowledge_with_paths(
                Box::new(ParitySlateDispatchChild),
                std::path::PathBuf::new(),
                manifest_path,
            )
            .expect("register slate child");

        let state = ServerState::new(ServerStateInit {
            token: "test-token".to_string(),
            startup_profile: DaemonStartupProfile::Core,
            rivet_integration: RivetIntegrationProfile::Disabled,
            registry,
            runtime_store: runtime_store.clone(),
            startup_store: runtime_store.clone(),
            federation_runtime: federation::startup(&runtime_store),
            readiness: Arc::new(RwLock::new(mother_crate::runtime::ReadinessState::default())),
        });

        let commands = vec![
            (
                "list",
                patina::spec::SpecCommands::List {
                    status: None,
                    target: None,
                    json: true,
                },
            ),
            ("next", patina::spec::SpecCommands::Next { json: true }),
            (
                "show",
                patina::spec::SpecCommands::Show {
                    id: "slate-observe-fixture".to_string(),
                    handoff: false,
                    json: true,
                },
            ),
            (
                "check",
                patina::spec::SpecCommands::Check {
                    id: "slate-observe-fixture".to_string(),
                    json: true,
                },
            ),
            (
                "prompt",
                patina::spec::SpecCommands::Prompt {
                    id: "slate-observe-fixture".to_string(),
                    json: true,
                },
            ),
            (
                "handoff",
                patina::spec::SpecCommands::Handoff {
                    id: "slate-observe-fixture".to_string(),
                    json: true,
                },
            ),
            (
                "packet",
                patina::spec::SpecCommands::Packet {
                    id: "slate-observe-fixture".to_string(),
                    json: true,
                },
            ),
        ];

        let mut report = Vec::new();

        for (name, command) in commands {
            let request = spec_dispatch_request(command, Some("observe"));

            let response =
                <ServerState as mother_crate::http_api::ApiRuntime>::builtin_spec_dispatch(
                    &state, request,
                )
                .expect("observe mode response");

            let probe = response
                .get("backend")
                .and_then(|v| v.get("slate_probe"))
                .cloned()
                .expect("slate probe payload");

            assert_eq!(probe.get("status").and_then(|v| v.as_str()), Some("called"));

            let builtin_data = response.get("data").cloned().expect("builtin data payload");
            let probe_data = probe.get("data").cloned().expect("probe data payload");

            assert_eq!(
                builtin_data, probe_data,
                "observe parity mismatch for command '{}'",
                name
            );

            report.push(serde_json::json!({
                "command": name,
                "parity": "equal",
            }));
        }

        assert_eq!(report.len(), 7);
        for row in report {
            assert!(row.get("command").is_some());
            assert_eq!(row.get("parity").and_then(|v| v.as_str()), Some("equal"));
        }
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
fn daemon_sources_do_not_emit_legacy_lifecycle_error_prefix_strings() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let files = [
        "src/commands/mother/daemon/dispatch.rs",
        "src/commands/mother/daemon/startup.rs",
        "src/commands/mother/daemon/health.rs",
    ];
    let forbidden = [
        "invalid_request: ",
        "child_not_found: ",
        "pando_not_found: ",
        "operation_in_progress: ",
        "resource_exhausted: ",
        "internal_error: ",
    ];

    for rel_path in files {
        let content = std::fs::read_to_string(root.join(rel_path))
            .unwrap_or_else(|e| panic!("read {rel_path}: {e}"));
        for marker in forbidden {
            assert!(
                !content.contains(marker),
                "{rel_path} contains legacy lifecycle error marker: {marker}"
            );
        }
    }
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

#[test]
fn interface_control_http_route_rejects_unknown_operation_end_to_end() {
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

        let request = mother_crate::http_daemon::HttpRequest {
            method: "POST".to_string(),
            path: "/api/interface/call".to_string(),
            headers: vec![],
            body: serde_json::to_vec(&serde_json::json!({
                "operation_id": "patina:interface/unknown.v1",
                "args": []
            }))
            .unwrap(),
        };

        let response = mother_crate::http_api::handle_interface_control_call(&request, &state);
        assert_eq!(response.status, 400);
        let payload: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(
            payload.get("error").and_then(|v| v.as_str()),
            Some("invalid_request")
        );
    });
}
