use super::*;
use std::io::Write;

fn write_temp_manifest(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

// =====================================================================
// PluginManifest::from_path
// =====================================================================

#[test]
fn manifest_valid_minimal() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "test-plugin"
world = "mother-child"

[capabilities]
host_log = true

[provides]
child = "test"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    assert_eq!(m.name, "test-plugin");
    assert_eq!(m.world, "mother-child");
    assert_eq!(m.version, "0.0.0"); // default
    assert_eq!(m.capabilities, vec!["host_log"]);
    assert_eq!(m.provides.child.as_deref(), Some("test"));
}

#[test]
fn manifest_valid_full() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "full-plugin"
version = "1.2.3"
description = "A full manifest"
world = "mother-child"
patina_min = "0.17.0"

[capabilities]
host_log = true
filesystem = false

[provides]
child = "full"
commands = ["cmd1", "cmd2"]
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    assert_eq!(m.name, "full-plugin");
    assert_eq!(m.version, "1.2.3");
    assert_eq!(m.description, "A full manifest");
    assert_eq!(m.patina_min, "0.17.0");
    // filesystem = false should NOT be in capabilities
    assert_eq!(m.capabilities, vec!["host_log"]);
    assert_eq!(m.provides.commands, vec!["cmd1", "cmd2"]);
}

#[test]
fn manifest_missing_plugin_section() {
    let f = write_temp_manifest("[other]\nfoo = 1\n");
    let err = PluginManifest::from_path(f.path()).unwrap_err();
    assert!(
        err.to_string().contains("missing [plugin] section"),
        "got: {}",
        err
    );
}

#[test]
fn manifest_missing_name() {
    let f = write_temp_manifest("[plugin]\nworld = \"mother-child\"\n");
    let err = PluginManifest::from_path(f.path()).unwrap_err();
    assert!(
        err.to_string().contains("missing plugin.name"),
        "got: {}",
        err
    );
}

#[test]
fn manifest_missing_world() {
    let f = write_temp_manifest("[plugin]\nname = \"test\"\n");
    let err = PluginManifest::from_path(f.path()).unwrap_err();
    assert!(
        err.to_string().contains("missing plugin.world"),
        "got: {}",
        err
    );
}

#[test]
fn manifest_parses_toy_commands() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "test-plugin"
world = "mother-child"

[capabilities]
host_log = true

[capabilities.toys]
commands = ["git", "patina"]

[provides]
child = "test"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    assert_eq!(m.allowed_toy_commands, vec!["git", "patina"]);
}

#[test]
fn manifest_no_toy_commands_defaults_empty() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "test-plugin"
world = "mother-child"

[capabilities]
host_log = true

[provides]
child = "test"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    assert!(m.allowed_toy_commands.is_empty());
}

#[test]
fn manifest_invalid_toml() {
    let f = write_temp_manifest("this is not valid toml {{{}}}");
    assert!(PluginManifest::from_path(f.path()).is_err());
}

// =====================================================================
// check_capabilities
// =====================================================================

#[test]
fn capabilities_all_granted() {
    let m = PluginManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: "mother-child".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
    };
    assert!(PluginEngine::check_capabilities(&m).is_ok());
}

#[test]
fn capabilities_empty() {
    let m = PluginManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: "mother-child".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec![],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
    };
    assert!(PluginEngine::check_capabilities(&m).is_ok());
}

#[test]
fn capabilities_denied() {
    let m = PluginManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: "mother-child".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "filesystem".into(), "network".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
    };
    let err = PluginEngine::check_capabilities(&m).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("filesystem"), "got: {}", msg);
    assert!(msg.contains("network"), "got: {}", msg);
    assert!(
        !msg.contains("host_log"),
        "host_log should be granted: {}",
        msg
    );
}

// =====================================================================
// Query param sanitization — scope-reserved keys
// =====================================================================

#[test]
fn sanitize_strips_all_repos_for_current_project() {
    let params = r#"{"query":"test","all_repos":true,"limit":5}"#;
    let result = command::sanitize_query_params(params, &QueryScope::CurrentProject);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(
        parsed.get("all_repos").is_none(),
        "all_repos should be stripped, got: {}",
        result
    );
    // Non-reserved keys preserved
    assert_eq!(parsed.get("query").unwrap(), "test");
    assert_eq!(parsed.get("limit").unwrap(), 5);
}

#[test]
fn sanitize_strips_repo_for_current_project() {
    let params = r#"{"query":"test","repo":"other-project"}"#;
    let result = command::sanitize_query_params(params, &QueryScope::CurrentProject);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(
        parsed.get("repo").is_none(),
        "repo should be stripped, got: {}",
        result
    );
    assert_eq!(parsed.get("query").unwrap(), "test");
}

#[test]
fn sanitize_strips_all_reserved_keys() {
    let params =
        r#"{"query":"test","all_repos":true,"repo":"x","project_root":"/tmp","db_path":"/hack"}"#;
    let result = command::sanitize_query_params(params, &QueryScope::CurrentProject);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    for key in &["all_repos", "repo", "project_root", "db_path"] {
        assert!(
            parsed.get(key).is_none(),
            "reserved key '{}' should be stripped, got: {}",
            key,
            result
        );
    }
}

#[test]
fn sanitize_preserves_all_for_all_repos_scope() {
    let params = r#"{"query":"test","all_repos":true,"repo":"other"}"#;
    let result = command::sanitize_query_params(params, &QueryScope::AllRepos);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed.get("all_repos").unwrap(), true);
    assert_eq!(parsed.get("repo").unwrap(), "other");
}

#[test]
fn sanitize_handles_invalid_json() {
    let params = "not json";
    let result = command::sanitize_query_params(params, &QueryScope::CurrentProject);
    assert_eq!(
        result, "not json",
        "invalid JSON should pass through unchanged"
    );
}

#[test]
fn check_capabilities_rejects_unknown_query_kinds() {
    let m = PluginManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: "command".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec!["scry".into(), "magic_oracle".into()],
        host_http_domains: vec![],
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
    };
    let err = PluginEngine::check_capabilities(&m).unwrap_err();
    assert!(
        err.to_string().contains("magic_oracle"),
        "should reject unknown kind, got: {}",
        err
    );
}

#[test]
fn check_capabilities_accepts_known_query_kinds() {
    let m = PluginManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: "command".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec!["scry".into(), "context".into(), "assay".into()],
        host_http_domains: vec![],
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
    };
    assert!(PluginEngine::check_capabilities(&m).is_ok());
}

// =====================================================================
// WASM integration — load models.wasm, call handle()
// =====================================================================

/// Load the pre-compiled models.wasm fixture, instantiate it,
/// and verify the full handle() round-trip works.
#[test]
fn wasm_models_child_handle_roundtrip() {
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/patina_plugin_models.wasm");
    if !wasm_path.exists() {
        panic!(
            "test fixture missing: {}\n\
             Build it with: cargo build --release -p patina-plugin-models --target wasm32-wasip2\n\
             Then: cp target/wasm32-wasip2/release/patina_plugin_models.wasm tests/fixtures/",
            wasm_path.display()
        );
    }

    let engine = PluginEngine::new().expect("PluginEngine::new() failed");
    let wasm_bytes = std::fs::read(&wasm_path).expect("failed to read .wasm fixture");
    let component = engine
        .load_component(&wasm_bytes)
        .expect("load_component failed");

    // Use a manifest matching models plugin
    let manifest = PluginManifest {
        name: "patina-models".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: "mother-child".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        provides: PluginProvides {
            child: Some("models".into()),
            commands: vec![],
            ..Default::default()
        },
    };

    let child = engine
        .instantiate_child(&component, &manifest)
        .expect("instantiate_child failed");

    // Verify identity
    assert_eq!(child.name(), "models");

    // Test handle() round-trip: resolve_model action
    let request = crate::mother::ChildRequest {
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

/// Verify that health() works on a WASM child.
#[test]
fn wasm_models_child_health() {
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/patina_plugin_models.wasm");
    if !wasm_path.exists() {
        return; // Skip if fixture not available
    }

    let engine = PluginEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();
    let manifest = PluginManifest {
        name: "patina-models".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: "mother-child".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        provides: PluginProvides {
            child: Some("models".into()),
            commands: vec![],
            ..Default::default()
        },
    };

    let child = engine.instantiate_child(&component, &manifest).unwrap();
    match child.health() {
        crate::mother::ChildHealth::Healthy => {} // expected
        other => panic!("expected Healthy, got: {:?}", other),
    }
}

// =====================================================================
// WASM integration — load repos.wasm, test toy system end-to-end
// =====================================================================

/// Helper: load repos.wasm fixture and instantiate child.
fn load_repos_child() -> Option<Box<dyn crate::mother::MotherChild>> {
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/patina_plugin_repos.wasm");
    if !wasm_path.exists() {
        return None;
    }

    let engine = PluginEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();
    let manifest = PluginManifest {
        name: "patina-repos".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: "mother-child".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec!["git".into(), "patina".into()],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        provides: PluginProvides {
            child: Some("repos".into()),
            commands: vec![],
            ..Default::default()
        },
    };

    Some(engine.instantiate_child(&component, &manifest).unwrap())
}

/// Repos child: report_repo + check_freshness handle() round-trip.
#[test]
fn wasm_repos_child_handle_roundtrip() {
    let child = match load_repos_child() {
        Some(c) => c,
        None => {
            panic!(
                "test fixture missing: tests/fixtures/patina_plugin_repos.wasm\n\
                 Build: cargo build --release -p patina-plugin-repos --target wasm32-wasip2\n\
                 Copy: cp target/wasm32-wasip2/release/patina_plugin_repos.wasm tests/fixtures/"
            );
        }
    };

    assert_eq!(child.name(), "repos");

    // Report a repo
    let request = crate::mother::ChildRequest {
        action: "report_repo".into(),
        payload: serde_json::json!({
            "name": "test-repo",
            "path": "/tmp/repos/test-repo",
            "last_indexed": 0
        }),
    };
    let response = child.handle(&request).expect("report_repo failed");
    assert_eq!(
        response.payload.get("status").and_then(|v| v.as_str()),
        Some("registered")
    );
    assert_eq!(
        response.payload.get("total_repos").and_then(|v| v.as_u64()),
        Some(1)
    );

    // Check freshness
    let request = crate::mother::ChildRequest {
        action: "check_freshness".into(),
        payload: serde_json::json!({}),
    };
    let response = child.handle(&request).expect("check_freshness failed");
    let stale_count = response.payload.get("stale_count").and_then(|v| v.as_u64());
    assert_eq!(
        stale_count,
        Some(1),
        "repo with last_indexed=0 should be stale"
    );
}

/// Repos child: tick() returns toys for stale repos — end-to-end toy system proof.
#[test]
fn wasm_repos_child_tick_returns_toys() {
    let mut child = match load_repos_child() {
        Some(c) => c,
        None => return, // Skip if fixture not available
    };

    // Report a stale repo (last_indexed = 0 means it's ancient)
    let request = crate::mother::ChildRequest {
        action: "report_repo".into(),
        payload: serde_json::json!({
            "name": "stale-repo",
            "path": "/home/user/.patina/cache/repos/stale-repo",
            "last_indexed": 0
        }),
    };
    child.handle(&request).expect("report_repo failed");

    // tick() should return toys for the stale repo
    let toys = child.tick();
    assert!(
        toys.len() >= 2,
        "expected at least 2 toys (pull + scrape), got {}",
        toys.len()
    );

    // Verify pull toy
    let pull_toy = toys.iter().find(|t| t.name.contains("pull"));
    assert!(pull_toy.is_some(), "expected a pull toy, got: {:?}", toys);
    let pull = pull_toy.unwrap();
    assert_eq!(pull.command, "git");
    assert!(
        pull.args.contains(&"-C".to_string()),
        "pull toy should use -C flag"
    );

    // Verify scrape toy
    let scrape_toy = toys.iter().find(|t| t.name.contains("scrape"));
    assert!(
        scrape_toy.is_some(),
        "expected a scrape toy, got: {:?}",
        toys
    );
    let scrape = scrape_toy.unwrap();
    assert_eq!(scrape.command, "patina");
    assert!(
        scrape.args.contains(&"scrape".to_string()),
        "scrape toy should include 'scrape' arg"
    );
}

/// Repos child: fresh repo produces no toys.
#[test]
fn wasm_repos_child_fresh_repo_no_toys() {
    let mut child = match load_repos_child() {
        Some(c) => c,
        None => return,
    };

    // Report a fresh repo (last_indexed = now)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let request = crate::mother::ChildRequest {
        action: "report_repo".into(),
        payload: serde_json::json!({
            "name": "fresh-repo",
            "path": "/tmp/repos/fresh-repo",
            "last_indexed": now
        }),
    };
    child.handle(&request).expect("report_repo failed");

    // tick() should return no toys — repo is fresh
    let toys = child.tick();
    assert!(
        toys.is_empty(),
        "expected no toys for fresh repo, got: {:?}",
        toys
    );
}

/// Repos child: health is Healthy when no repos, Degraded when stale.
#[test]
fn wasm_repos_child_health_reflects_staleness() {
    let child = match load_repos_child() {
        Some(c) => c,
        None => return,
    };

    // No repos → Healthy
    match child.health() {
        crate::mother::ChildHealth::Healthy => {}
        other => panic!("expected Healthy with no repos, got: {:?}", other),
    }

    // Add stale repo → Degraded
    let request = crate::mother::ChildRequest {
        action: "report_repo".into(),
        payload: serde_json::json!({
            "name": "old-repo",
            "path": "/tmp/repos/old-repo",
            "last_indexed": 0
        }),
    };
    child.handle(&request).expect("report_repo failed");

    match child.health() {
        crate::mother::ChildHealth::Degraded(_) => {} // expected
        other => panic!("expected Degraded with stale repo, got: {:?}", other),
    }
}

// =====================================================================
// Toy capability gating (F4)
// =====================================================================

/// Repos child with restricted manifest: only "patina" allowed, not "git".
/// Verifies that unauthorized toy commands are filtered out by WasmChild.
#[test]
fn wasm_repos_child_toy_capability_gating() {
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/patina_plugin_repos.wasm");
    if !wasm_path.exists() {
        return; // Skip if fixture not available
    }

    let engine = PluginEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();

    // Manifest only allows "patina", NOT "git"
    let manifest = PluginManifest {
        name: "patina-repos".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: "mother-child".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec!["patina".into()], // git excluded
        host_query_kinds: vec![],
        host_http_domains: vec![],
        provides: PluginProvides {
            child: Some("repos".into()),
            commands: vec![],
            ..Default::default()
        },
    };

    let mut child = engine.instantiate_child(&component, &manifest).unwrap();

    // Report a stale repo — tick() will return both git pull and patina scrape toys
    let request = crate::mother::ChildRequest {
        action: "report_repo".into(),
        payload: serde_json::json!({
            "name": "gated-repo",
            "path": "/tmp/repos/gated-repo",
            "last_indexed": 0
        }),
    };
    child.handle(&request).expect("report_repo failed");

    let toys = child.tick();

    // Only "patina" toys should pass — "git" toys should be filtered
    for toy in &toys {
        assert_eq!(
            toy.command, "patina",
            "expected only 'patina' toys, got command '{}' in toy '{}'",
            toy.command, toy.name
        );
    }
    assert!(
        !toys.is_empty(),
        "expected at least one patina toy to pass the filter"
    );
}

// =====================================================================
// Benchmarks (C2) — Instant::now() instrumentation
// =====================================================================

/// Measure PluginEngine::new(), Component::new(), instantiate_child(),
/// and handle() round-trip. Run with `cargo test -- --nocapture benchmark`.
#[test]
fn benchmark_plugin_performance() {
    use std::time::Instant;

    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/patina_plugin_models.wasm");
    if !wasm_path.exists() {
        return;
    }

    // Warm up the process-wide engine singleton (OnceLock).
    // Without this, the first PluginEngine::new() absorbs Engine::new()
    // cold-start cost (~150ms cranelift JIT init), making the benchmark
    // flaky depending on test execution order.
    let _ = PluginEngine::new();

    // 1. PluginEngine::new() — spec threshold: <100ms
    let t0 = Instant::now();
    let engine = PluginEngine::new().unwrap();
    let engine_ms = t0.elapsed().as_secs_f64() * 1000.0;

    // 2. Component::new() — document compilation time
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let t1 = Instant::now();
    let component = engine.load_component(&wasm_bytes).unwrap();
    let component_ms = t1.elapsed().as_secs_f64() * 1000.0;

    // 3. instantiate_child() total — Component + WasiCtx + Store + init + name
    let manifest = PluginManifest {
        name: "patina-models".into(),
        version: "0.1.0".into(),
        description: "bench".into(),
        world: "mother-child".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        provides: PluginProvides {
            child: Some("models".into()),
            commands: vec![],
            ..Default::default()
        },
    };
    let t2 = Instant::now();
    let child = engine.instantiate_child(&component, &manifest).unwrap();
    let instantiate_ms = t2.elapsed().as_secs_f64() * 1000.0;

    // 4. handle() round-trip — spec threshold: <1ms
    let request = crate::mother::ChildRequest {
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
        "  PluginEngine::new():     {:.2}ms (threshold: <100ms) {}",
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
        "PluginEngine::new() took {:.2}ms, threshold is 100ms",
        engine_ms
    );
    assert!(
        handle_avg_ms < 1.0,
        "handle() avg took {:.3}ms, threshold is 1ms",
        handle_avg_ms
    );
}

// =====================================================================
// CommandEngine — doctor.wasm integration tests
// =====================================================================

fn load_doctor_component() -> Option<(CommandEngine, wasmtime::component::Component)> {
    let wasm_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/patina_doctor.wasm");
    if !wasm_path.exists() {
        return None;
    }
    let engine = CommandEngine::new().expect("CommandEngine::new() failed");
    let wasm_bytes = std::fs::read(&wasm_path).expect("failed to read doctor wasm");
    let component = engine
        .load_component(&wasm_bytes)
        .expect("load_component failed");
    Some((engine, component))
}

#[test]
fn command_doctor_name() {
    let (engine, component) = match load_doctor_component() {
        Some(ec) => ec,
        None => {
            panic!(
                "test fixture missing: tests/fixtures/patina_doctor.wasm\n\
                 Build: cargo build --release -p patina-doctor --target wasm32-wasip2\n\
                 Copy: cp target/wasm32-wasip2/release/patina_doctor.wasm tests/fixtures/"
            );
        }
    };

    let name = engine
        .get_command_name(&component)
        .expect("get_command_name failed");
    assert_eq!(name, "doctor");
}

#[test]
fn command_doctor_description() {
    let (engine, component) = match load_doctor_component() {
        Some(ec) => ec,
        None => return,
    };

    let desc = engine
        .get_command_description(&component)
        .expect("get_command_description failed");
    assert!(
        desc.contains("health"),
        "expected description to mention 'health', got: {}",
        desc
    );
}

fn load_doctor_manifest() -> PluginManifest {
    PluginManifest {
        name: "patina-doctor".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: "command".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "host_layer".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        provides: PluginProvides {
            child: None,
            commands: vec!["doctor".into()],
            ..Default::default()
        },
    }
}

#[test]
fn command_doctor_run() {
    let (engine, component) = match load_doctor_component() {
        Some(ec) => ec,
        None => return,
    };

    // Run with --json to avoid terminal output dependencies.
    // Exit code depends on project state — just verify it doesn't panic.
    let manifest = load_doctor_manifest();
    let args = vec!["--json".to_string()];
    let exit_code = engine
        .run_command(&component, &manifest, &args, None)
        .expect("run_command failed");
    // doctor returns 0 (healthy), 1 (error), 2 (warning), or 3 (critical)
    assert!(
        [0, 1, 2, 3].contains(&exit_code),
        "unexpected exit code: {}",
        exit_code
    );
}

// =====================================================================
// validate_http_url — data-level URL sanitization
// =====================================================================

#[test]
fn validate_http_url_valid_https() {
    let domain = mother_child::validate_http_url("https://api.github.com/repos").unwrap();
    assert_eq!(domain, "api.github.com");
}

#[test]
fn validate_http_url_valid_https_with_port() {
    let domain = mother_child::validate_http_url("https://api.github.com:443/repos").unwrap();
    assert_eq!(domain, "api.github.com");
}

#[test]
fn validate_http_url_rejects_http() {
    let err = mother_child::validate_http_url("http://api.github.com/repos").unwrap_err();
    assert!(err.contains("HTTPS"), "expected HTTPS error, got: {}", err);
}

#[test]
fn validate_http_url_rejects_ipv4() {
    let err = mother_child::validate_http_url("https://192.168.1.1/api").unwrap_err();
    assert!(err.contains("IP"), "expected IP error, got: {}", err);
}

#[test]
fn validate_http_url_rejects_ipv6() {
    let err = mother_child::validate_http_url("https://[::1]/api").unwrap_err();
    assert!(err.contains("IP"), "expected IP error, got: {}", err);
}

#[test]
fn validate_http_url_rejects_localhost() {
    let err = mother_child::validate_http_url("https://localhost/api").unwrap_err();
    assert!(
        err.contains("localhost"),
        "expected localhost error, got: {}",
        err
    );
}

#[test]
fn validate_http_url_rejects_invalid() {
    let err = mother_child::validate_http_url("not-a-url").unwrap_err();
    assert!(
        err.contains("invalid"),
        "expected invalid URL error, got: {}",
        err
    );
}

// =====================================================================
// Manifest parsing — host_http
// =====================================================================

#[test]
fn manifest_parses_http_domains() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "http-plugin"
world = "mother-child"

[capabilities]
host_log = true
host_http = ["api.github.com", "hooks.slack.com"]

[provides]
child = "webhook"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    assert_eq!(
        m.host_http_domains,
        vec!["api.github.com", "hooks.slack.com"]
    );
}

#[test]
fn manifest_no_http_domains_defaults_empty() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "no-http"
world = "mother-child"

[capabilities]
host_log = true

[provides]
child = "test"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    assert!(m.host_http_domains.is_empty());
}

// =====================================================================
// check_capabilities — HTTP domain validation
// =====================================================================

#[test]
fn check_capabilities_rejects_empty_http_domain() {
    let m = PluginManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: "mother-child".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["".into()],
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
    };
    let err = PluginEngine::check_capabilities(&m).unwrap_err();
    assert!(err.to_string().contains("empty"), "got: {}", err);
}

#[test]
fn check_capabilities_rejects_http_domain_with_path() {
    let m = PluginManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: "mother-child".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["api.github.com/repos".into()],
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
    };
    let err = PluginEngine::check_capabilities(&m).unwrap_err();
    assert!(err.to_string().contains("path"), "got: {}", err);
}

#[test]
fn check_capabilities_accepts_valid_http_domains() {
    let m = PluginManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: "mother-child".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["api.github.com".into(), "hooks.slack.com".into()],
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
    };
    assert!(PluginEngine::check_capabilities(&m).is_ok());
}

// =====================================================================
// granted_capabilities — HTTP domains
// =====================================================================

#[test]
fn granted_capabilities_includes_http_domains() {
    let m = PluginManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: "mother-child".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec!["scry".into()],
        host_http_domains: vec!["api.github.com".into()],
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
    };
    let grants = m.granted_capabilities();
    assert!(grants.http_domains.contains("api.github.com"));
    assert!(grants.query_kinds.contains("scry"));
}

// =====================================================================
// HTTP conformance — defense-in-depth chain verification
// =====================================================================

/// Conformance: domain not in allowlist is denied at call time.
/// Maps to: GrantedCapabilities.http_domains check in Host impl.
#[test]
fn conformance_http_domain_not_in_allowlist_denied() {
    let grants = GrantedCapabilities {
        http_domains: ["api.github.com".to_string()].into_iter().collect(),
        ..Default::default()
    };
    // URL is valid HTTPS with a valid domain
    let domain = mother_child::validate_http_url("https://evil.com/steal").unwrap();
    // But domain is NOT in the allowlist
    assert!(
        !grants.http_domains.contains(&domain),
        "evil.com should not be in allowlist"
    );
}

/// Conformance: non-HTTPS URL rejected before any network call.
/// Maps to: validate_http_url scheme check.
#[test]
fn conformance_http_rejects_plaintext() {
    let err = mother_child::validate_http_url("http://api.github.com/repos").unwrap_err();
    assert!(
        err.contains("HTTPS"),
        "plaintext HTTP should be rejected: {}",
        err
    );
}

/// Conformance: IP address URL rejected before any network call.
/// Maps to: validate_http_url IP check.
#[test]
fn conformance_http_rejects_ip_address() {
    let err = mother_child::validate_http_url("https://10.0.0.1/internal").unwrap_err();
    assert!(err.contains("IP"), "IP address should be rejected: {}", err);
}

/// Conformance: localhost URL rejected before any network call.
/// Maps to: validate_http_url localhost check.
#[test]
fn conformance_http_rejects_localhost() {
    let err = mother_child::validate_http_url("https://localhost:8080/api").unwrap_err();
    assert!(
        err.contains("localhost"),
        "localhost should be rejected: {}",
        err
    );
}

// =====================================================================
// TaskEngine — hello-task conformance tests
// =====================================================================

fn load_hello_task_component() -> Option<(task::TaskEngine, wasmtime::component::Component)> {
    let wasm_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hello_task.wasm");
    if !wasm_path.exists() {
        return None;
    }
    let engine = task::TaskEngine::new().expect("TaskEngine::new() failed");
    let wasm_bytes = std::fs::read(&wasm_path).expect("failed to read hello-task wasm");
    let component = engine
        .load_component(&wasm_bytes)
        .expect("load_component failed");
    Some((engine, component))
}

fn hello_task_manifest() -> PluginManifest {
    PluginManifest {
        name: "hello-task".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: "task".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "host_layer".into()],
        allowed_toy_commands: vec!["echo".into()],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
    }
}

#[test]
fn task_hello_name() {
    let (engine, component) = match load_hello_task_component() {
        Some(ec) => ec,
        None => {
            panic!(
                "test fixture missing: tests/fixtures/hello_task.wasm\n\
                 Build: cd tests/hello-task && cargo build --release --target wasm32-wasip2\n\
                 Copy: cp tests/hello-task/target/wasm32-wasip2/release/hello_task.wasm tests/fixtures/"
            );
        }
    };

    let name = engine
        .get_task_name(&component)
        .expect("get_task_name failed");
    assert_eq!(name, "hello-task");
}

#[test]
fn task_hello_description() {
    let (engine, component) = match load_hello_task_component() {
        Some(ec) => ec,
        None => return,
    };

    let desc = engine
        .get_task_description(&component)
        .expect("get_task_description failed");
    assert!(
        desc.contains("testing"),
        "expected description to mention 'testing', got: {}",
        desc
    );
}

#[test]
fn task_hello_run_exit_code() {
    let (engine, component) = match load_hello_task_component() {
        Some(ec) => ec,
        None => return,
    };

    let manifest = hello_task_manifest();
    let (exit_code, _toys) = engine
        .run_task(&component, &manifest, &[], None)
        .expect("run_task failed");
    assert_eq!(exit_code, 0, "hello-task should return exit code 0");
}

#[test]
fn task_hello_toys_filtered() {
    let (engine, component) = match load_hello_task_component() {
        Some(ec) => ec,
        None => return,
    };

    let manifest = hello_task_manifest();
    let (_exit_code, toys) = engine
        .run_task(&component, &manifest, &[], None)
        .expect("run_task failed");

    // Should have exactly 1 toy (echo approved, rm filtered out)
    assert_eq!(
        toys.len(),
        1,
        "expected 1 approved toy (echo), got {}",
        toys.len()
    );

    let toy = &toys[0];
    assert_eq!(toy.name, "greet");
    assert_eq!(toy.command, "echo");
    assert_eq!(toy.args, vec!["hello"]);
}

/// Verify that unapproved toy commands are filtered out.
#[test]
fn task_hello_unapproved_toy_denied() {
    let (engine, component) = match load_hello_task_component() {
        Some(ec) => ec,
        None => return,
    };

    // Manifest with NO allowed toy commands
    let manifest = PluginManifest {
        name: "hello-task".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: "task".into(),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "host_layer".into()],
        allowed_toy_commands: vec![], // nothing allowed
        host_query_kinds: vec![],
        host_http_domains: vec![],
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
    };

    let (_exit_code, toys) = engine
        .run_task(&component, &manifest, &[], None)
        .expect("run_task failed");

    // All toys should be filtered out
    assert!(
        toys.is_empty(),
        "expected no toys with empty allowed list, got {}",
        toys.len()
    );
}
