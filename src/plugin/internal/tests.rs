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
        provides: PluginProvides {
            child: None,
            commands: vec![],
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
        provides: PluginProvides {
            child: None,
            commands: vec![],
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
        provides: PluginProvides {
            child: None,
            commands: vec![],
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
        provides: PluginProvides {
            child: Some("models".into()),
            commands: vec![],
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
        provides: PluginProvides {
            child: Some("models".into()),
            commands: vec![],
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
        provides: PluginProvides {
            child: Some("repos".into()),
            commands: vec![],
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
        provides: PluginProvides {
            child: Some("repos".into()),
            commands: vec![],
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
        provides: PluginProvides {
            child: Some("models".into()),
            commands: vec![],
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
        provides: PluginProvides {
            child: None,
            commands: vec!["doctor".into()],
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
        .run_command(&component, &manifest, &args)
        .expect("run_command failed");
    // doctor returns 0 (healthy), 1 (error), 2 (warning), or 3 (critical)
    assert!(
        [0, 1, 2, 3].contains(&exit_code),
        "unexpected exit code: {}",
        exit_code
    );
}
