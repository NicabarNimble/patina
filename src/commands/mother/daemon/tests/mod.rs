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
struct SlateDispatchChild;
struct ScaffoldSlateDispatchChild;

fn spec_dispatch_request(
    command: patina::spec::SpecCommands,
    backend_mode: Option<&str>,
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
            project: None,
            origin_project: None,
            backend_mode: backend_mode.map(|value| value.to_string()),
        })
        .expect("serialize envelope"),
    }
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
        if request.operation_id != "patina:slate/control.dispatch" {
            return Err(anyhow::anyhow!(
                "unexpected operation_id: {}",
                request.operation_id
            ));
        }

        let command_json = request
            .args
            .as_array()
            .and_then(|values| values.first())
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow::anyhow!("expected slate args[0] command JSON string"))?;

        let data = serde_json::json!({
            "status": "from-slate",
            "command_bytes": command_json.len(),
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
        if request.operation_id != "patina:slate/control.dispatch" {
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
allow = ["patina:slate/control.dispatch"]
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
fn builtin_spec_dispatch_execute_fails_closed_when_slate_is_scaffold_only() {
    with_temp_project(|project_root| {
        std::fs::create_dir_all(project_root.join(".patina")).expect("create .patina");
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
allow = ["patina:slate/control.dispatch"]
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
        std::fs::create_dir_all(project_root.join(".patina")).expect("create .patina");
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
allow = ["patina:slate/control.dispatch"]
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
