use super::common::*;

#[test]
fn wasm_trap_pipeline_panic_returns_error() {
    let (engine, component) = match load_panic_pipeline_component() {
        Some(ec) => ec,
        None => return,
    };

    let manifest = ChildManifest {
        name: "panic-pipeline".into(),
        version: "0.1.0".into(),
        description: "deliberate panic".into(),
        world: ChildKind::Pipeline,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            pipeline_ops: vec!["echo".into()],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
        declared_metrics: std::collections::HashMap::new(),
        filesystem_preopens: vec![],
        state_enabled: false,
        checkpoint_streams: vec![],
        lake_names: vec![],
        ingress_sources: std::collections::HashMap::new(),
        subscribed_streams: vec![],
        task_intent_names: vec![],
        task_intents: vec![],
        graph_read: false,
        graph_write_actions: vec![],
        belief_read: false,
        belief_write_actions: vec![],
        toys: GrantedToys::default(),
        inside_accepts: vec![],
        ingress_mode: ChildIngressMode::Handle,
        contract_default_operation: None,
        contract_allow_operations: vec![],
    };

    let request = r#"{"op":"echo","version":"1","payload":{}}"#;
    let result = engine.handle(&component, &manifest.name, request);

    // The guest panics — host MUST catch the trap and return Err
    assert!(
        result.is_err(),
        "guest panic should be caught as error, not crash the host"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("unreachable") || err.contains("panic") || err.contains("trap"),
        "error should indicate a WASM trap, got: {}",
        err
    );
}

#[test]
fn wasm_trap_mother_child_panic_returns_error() {
    // We reuse the panic-pipeline fixture but try to load it as mother-child.
    // This will fail at instantiation (wrong world) — which also proves
    // that world mismatch produces a clean error, not a crash.
    let wasm_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/panic_pipeline.wasm");
    if !wasm_path.exists() {
        return;
    }

    let engine = ChildEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();
    let manifest = ChildManifest {
        name: "wrong-world".into(),
        version: "0.1.0".into(),
        description: "world mismatch".into(),
        world: ChildKind::Child,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: Some("wrong".into()),
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
        declared_metrics: std::collections::HashMap::new(),
        filesystem_preopens: vec![],
        state_enabled: false,
        checkpoint_streams: vec![],
        lake_names: vec![],
        ingress_sources: std::collections::HashMap::new(),
        subscribed_streams: vec![],
        task_intent_names: vec![],
        task_intents: vec![],
        graph_read: false,
        graph_write_actions: vec![],
        belief_read: false,
        belief_write_actions: vec![],
        toys: GrantedToys::default(),
        inside_accepts: vec![],
        ingress_mode: ChildIngressMode::Handle,
        contract_default_operation: None,
        contract_allow_operations: vec![],
    };

    // Instantiation with wrong world should fail cleanly
    let result = engine.instantiate_child(&component, &manifest, None);
    assert!(
        result.is_err(),
        "wrong world instantiation should return Err, not crash"
    );
}
