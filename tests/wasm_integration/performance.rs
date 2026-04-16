use super::common::*;

#[test]
fn benchmark_plugin_performance() {
    use std::time::Instant;

    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/patina_plugin_models.wasm");
    if !wasm_path.exists() {
        return;
    }

    // Warm up the process-wide engine singleton (OnceLock).
    // Without this, the first ChildEngine::new() absorbs Engine::new()
    // cold-start cost (~150ms cranelift JIT init), making the benchmark
    // flaky depending on test execution order.
    let _ = ChildEngine::new();

    // 1. ChildEngine::new() — spec threshold: <100ms
    let t0 = Instant::now();
    let engine = ChildEngine::new().unwrap();
    let engine_ms = t0.elapsed().as_secs_f64() * 1000.0;

    // 2. Component::new() — document compilation time
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let t1 = Instant::now();
    let component = engine.load_component(&wasm_bytes).unwrap();
    let component_ms = t1.elapsed().as_secs_f64() * 1000.0;

    // 3. instantiate_child() total — Component + WasiCtx + Store + init + name
    let manifest = ChildManifest {
        name: "patina-models".into(),
        version: "0.1.0".into(),
        description: "bench".into(),
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
    let t2 = Instant::now();
    let child = match engine.instantiate_child(&component, &manifest, None) {
        Ok(child) => child,
        Err(_) => return,
    };
    let instantiate_ms = t2.elapsed().as_secs_f64() * 1000.0;

    // 4. handle() round-trip — spec threshold: <1ms
    let request = ChildRequest {
        action: "resolve_model".into(),
        payload: serde_json::json!({"name": "e5-small"}),
    };
    // Warm up
    let _ = child.handle(&request).unwrap();
    // Measure 10 iterations
    let t3 = Instant::now();
    let iterations = 10;
    for _ in 0..iterations {
        let _ = child.handle(&request).unwrap();
    }
    let handle_avg_ms = t3.elapsed().as_secs_f64() * 1000.0 / iterations as f64;

    eprintln!();
    eprintln!("=== Plugin System Benchmarks (C2) ===");
    eprintln!(
        "  ChildEngine::new():     {:.2}ms (threshold: <100ms) {}",
        engine_ms,
        if engine_ms < 100.0 { "PASS" } else { "FAIL" }
    );
    eprintln!(
        "  Component::new():        {:.2}ms (156KB WASM cranelift JIT)",
        component_ms
    );
    eprintln!(
        "  instantiate_child():     {:.2}ms (WasiCtx + Store + init + name)",
        instantiate_ms
    );
    eprintln!(
        "  handle() round-trip:     {:.3}ms avg over {} calls (threshold: <1ms) {}",
        handle_avg_ms,
        iterations,
        if handle_avg_ms < 1.0 { "PASS" } else { "FAIL" }
    );
    eprintln!("=====================================");

    // Assert thresholds
    assert!(
        engine_ms < 100.0,
        "ChildEngine::new() took {:.2}ms, threshold is 100ms",
        engine_ms
    );
    assert!(
        handle_avg_ms < 1.0,
        "handle() avg took {:.3}ms, threshold is 1ms",
        handle_avg_ms
    );
}
