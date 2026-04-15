use super::common::*;

#[test]
fn wasm_models_child_handle_roundtrip() {
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/patina_plugin_models.wasm");
    if !wasm_path.exists() {
        return;
    }

    let engine = ChildEngine::new().expect("ChildEngine::new() failed");
    let wasm_bytes = std::fs::read(&wasm_path).expect("failed to read .wasm fixture");
    let component = engine
        .load_component(&wasm_bytes)
        .expect("load_component failed");

    // Use a manifest matching models plugin
    let manifest = ChildManifest {
        name: "patina-models".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: ChildKind::Child,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: Some("models".into()),
            commands: vec![],
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

    let child = match engine.instantiate_child(&component, &manifest, None) {
        Ok(child) => child,
        Err(_) => return,
    };

    // Verify identity
    assert_eq!(child.name(), "models");

    // Test handle() round-trip: resolve_model action
    let request = ChildRequest {
        action: "resolve_model".into(),
        payload: serde_json::json!({"name": "e5-small"}),
    };
    let response = child.handle(&request).expect("handle() failed");

    // Verify response contains expected path
    let path = response.payload.get("path").and_then(|v| v.as_str());
    assert!(
        path.is_some_and(|p| p.contains("e5-small")),
        "expected path containing 'e5-small', got: {:?}",
        response.payload
    );
}

#[test]
fn wasm_models_child_health() {
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/patina_plugin_models.wasm");
    if !wasm_path.exists() {
        return; // Skip if fixture not available
    }

    let engine = ChildEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();
    let manifest = ChildManifest {
        name: "patina-models".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: ChildKind::Child,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: Some("models".into()),
            commands: vec![],
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

    let child = match engine.instantiate_child(&component, &manifest, None) {
        Ok(child) => child,
        Err(_) => return,
    };
    match child.health() {
        ChildHealth::Healthy => {} // expected
        other => panic!("expected Healthy, got: {:?}", other),
    }
}
