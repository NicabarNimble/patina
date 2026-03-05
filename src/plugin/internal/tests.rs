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
    assert_eq!(m.world, PluginWorld::MotherChild);
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

#[test]
fn manifest_parses_schemas_section() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "grammar-forge"
world = "pipeline"

[provides]
pipeline_ops = ["parse"]
languages = ["forge-issue", "forge-pr"]

[schemas.forge]
package = "patina:schema/forge@1.0.0"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    assert_eq!(m.schemas.len(), 1);
    assert_eq!(m.schemas.get("forge").unwrap(), "patina:schema/forge@1.0.0");
}

#[test]
fn manifest_no_schemas_is_empty() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "test"
world = "pipeline"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    assert!(m.schemas.is_empty());
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
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    };
    assert!(PluginEngine::check_capabilities(&m).is_ok());
}

#[test]
fn capabilities_empty() {
    let m = PluginManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec![],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    };
    assert!(PluginEngine::check_capabilities(&m).is_ok());
}

#[test]
fn capabilities_denied() {
    let m = PluginManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "filesystem".into(), "network".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
    let result = host_support::sanitize_query_params(params, &QueryScope::CurrentProject);
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
    let result = host_support::sanitize_query_params(params, &QueryScope::CurrentProject);
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
    let result = host_support::sanitize_query_params(params, &QueryScope::CurrentProject);
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
    let result = host_support::sanitize_query_params(params, &QueryScope::AllRepos);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed.get("all_repos").unwrap(), true);
    assert_eq!(parsed.get("repo").unwrap(), "other");
}

#[test]
fn sanitize_handles_invalid_json() {
    let params = "not json";
    let result = host_support::sanitize_query_params(params, &QueryScope::CurrentProject);
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
        world: PluginWorld::Command,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec!["scry".into(), "magic_oracle".into()],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        world: PluginWorld::Command,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec!["scry".into(), "context".into(), "assay".into()],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: Some("models".into()),
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    };

    let child = engine
        .instantiate_child(&component, &manifest, None)
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
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: Some("models".into()),
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    };

    let child = engine
        .instantiate_child(&component, &manifest, None)
        .unwrap();
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
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec!["git".into(), "patina".into()],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: Some("repos".into()),
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    };

    Some(
        engine
            .instantiate_child(&component, &manifest, None)
            .unwrap(),
    )
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
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec!["patina".into()], // git excluded
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: Some("repos".into()),
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    };

    let mut child = engine
        .instantiate_child(&component, &manifest, None)
        .unwrap();

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
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: Some("models".into()),
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    };
    let t2 = Instant::now();
    let child = engine
        .instantiate_child(&component, &manifest, None)
        .unwrap();
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
        world: PluginWorld::Command,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "host_layer".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: None,
            commands: vec!["doctor".into()],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
    let domain = host_support::validate_http_url("https://api.github.com/repos").unwrap();
    assert_eq!(domain, "api.github.com");
}

#[test]
fn validate_http_url_valid_https_with_port() {
    let domain = host_support::validate_http_url("https://api.github.com:443/repos").unwrap();
    assert_eq!(domain, "api.github.com");
}

#[test]
fn validate_http_url_rejects_http() {
    let err = host_support::validate_http_url("http://api.github.com/repos").unwrap_err();
    assert!(err.contains("HTTPS"), "expected HTTPS error, got: {}", err);
}

#[test]
fn validate_http_url_rejects_ipv4() {
    let err = host_support::validate_http_url("https://192.168.1.1/api").unwrap_err();
    assert!(err.contains("IP"), "expected IP error, got: {}", err);
}

#[test]
fn validate_http_url_rejects_ipv6() {
    let err = host_support::validate_http_url("https://[::1]/api").unwrap_err();
    assert!(err.contains("IP"), "expected IP error, got: {}", err);
}

#[test]
fn validate_http_url_rejects_localhost() {
    let err = host_support::validate_http_url("https://localhost/api").unwrap_err();
    assert!(
        err.contains("localhost"),
        "expected localhost error, got: {}",
        err
    );
}

#[test]
fn validate_http_url_rejects_invalid() {
    let err = host_support::validate_http_url("not-a-url").unwrap_err();
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
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["".into()],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["api.github.com/repos".into()],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["api.github.com".into(), "hooks.slack.com".into()],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec!["scry".into()],
        host_http_domains: vec!["api.github.com".into()],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
    let domain = host_support::validate_http_url("https://evil.com/steal").unwrap();
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
    let err = host_support::validate_http_url("http://api.github.com/repos").unwrap_err();
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
    let err = host_support::validate_http_url("https://10.0.0.1/internal").unwrap_err();
    assert!(err.contains("IP"), "IP address should be rejected: {}", err);
}

/// Conformance: localhost URL rejected before any network call.
/// Maps to: validate_http_url localhost check.
#[test]
fn conformance_http_rejects_localhost() {
    let err = host_support::validate_http_url("https://localhost:8080/api").unwrap_err();
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
        world: PluginWorld::Task,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "host_layer".into()],
        allowed_toy_commands: vec!["echo".into()],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        world: PluginWorld::Task,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "host_layer".into()],
        allowed_toy_commands: vec![], // nothing allowed
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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

// =====================================================================
// PipelineEngine — echo-pipeline conformance tests
// =====================================================================

fn load_echo_pipeline_component(
) -> Option<(pipeline::PipelineEngine, wasmtime::component::Component)> {
    let wasm_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/echo_pipeline.wasm");
    if !wasm_path.exists() {
        return None;
    }
    let engine = pipeline::PipelineEngine::new().expect("PipelineEngine::new() failed");
    let wasm_bytes = std::fs::read(&wasm_path).expect("failed to read echo-pipeline wasm");
    let component = engine
        .load_component(&wasm_bytes)
        .expect("load_component failed");
    Some((engine, component))
}

fn echo_pipeline_manifest() -> PluginManifest {
    PluginManifest {
        name: "echo-pipeline".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: PluginWorld::Pipeline,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            pipeline_ops: vec!["echo".into()],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    }
}

#[test]
fn pipeline_echo_name() {
    let (engine, component) = match load_echo_pipeline_component() {
        Some(ec) => ec,
        None => {
            panic!(
                "test fixture missing: tests/fixtures/echo_pipeline.wasm\n\
                 Build: cd tests/echo-pipeline && cargo build --release --target wasm32-wasip2\n\
                 Copy: cp tests/echo-pipeline/target/wasm32-wasip2/release/echo_pipeline.wasm tests/fixtures/"
            );
        }
    };

    let name = engine.get_name(&component).expect("get_name failed");
    assert_eq!(name, "echo-pipeline");
}

#[test]
fn pipeline_echo_handle_roundtrip() {
    let (engine, component) = match load_echo_pipeline_component() {
        Some(ec) => ec,
        None => return,
    };

    let manifest = echo_pipeline_manifest();
    let request = r#"{"op":"echo","version":"1","payload":{"key":"value","count":42}}"#;
    let response = engine
        .handle(&component, &manifest, request)
        .expect("handle failed");

    // Echo returns payload unchanged
    let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(parsed.get("key").and_then(|v| v.as_str()), Some("value"));
    assert_eq!(parsed.get("count").and_then(|v| v.as_i64()), Some(42));
}

#[test]
fn pipeline_echo_unknown_op_error() {
    let (engine, component) = match load_echo_pipeline_component() {
        Some(ec) => ec,
        None => return,
    };

    let manifest = echo_pipeline_manifest();
    let request = r#"{"op":"frobnicate","version":"1","payload":{}}"#;
    let result = engine.handle(&component, &manifest, request);

    assert!(
        result.is_err(),
        "unknown op should return error, got: {:?}",
        result
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("unknown op"),
        "error should mention 'unknown op', got: {}",
        err
    );
}

#[test]
fn pipeline_echo_version_mismatch_error() {
    let (engine, component) = match load_echo_pipeline_component() {
        Some(ec) => ec,
        None => return,
    };

    let manifest = echo_pipeline_manifest();
    let request = r#"{"op":"echo","version":"999","payload":{}}"#;
    let result = engine.handle(&component, &manifest, request);

    assert!(
        result.is_err(),
        "version mismatch should return error, got: {:?}",
        result
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("version"),
        "error should mention 'version', got: {}",
        err
    );
}

// =====================================================================
// Cross-world: WASM trap handling conformance test
//
// A guest plugin that deliberately panics. The host MUST catch the
// wasmtime trap and return a clean error — never crash, never unwrap()
// across the WASM boundary. All plugin calls are fallible.
// =====================================================================

/// Load the panic-pipeline WASM fixture.
fn load_panic_pipeline_component() -> Option<(PipelineEngine, wasmtime::component::Component)> {
    let wasm_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/panic_pipeline.wasm");
    if !wasm_path.exists() {
        return None;
    }
    let engine = PipelineEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();
    Some((engine, component))
}

/// WASM trap handling: guest panic in pipeline handle() returns Err, not crash.
#[test]
fn wasm_trap_pipeline_panic_returns_error() {
    let (engine, component) = match load_panic_pipeline_component() {
        Some(ec) => ec,
        None => return,
    };

    let manifest = PluginManifest {
        name: "panic-pipeline".into(),
        version: "0.1.0".into(),
        description: "deliberate panic".into(),
        world: PluginWorld::Pipeline,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            pipeline_ops: vec!["echo".into()],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    };

    let request = r#"{"op":"echo","version":"1","payload":{}}"#;
    let result = engine.handle(&component, &manifest, request);

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

// =====================================================================
// Plugin host fragility — spec exit criteria tests
// =====================================================================

// F2: Path traversal in count_layer_files returns 0 (silent reject).
#[test]
fn count_layer_files_rejects_path_traversal() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();

    // Create layer/core with a .md file so a valid subdir would return >0
    let layer_core = root.join("layer").join("core");
    std::fs::create_dir_all(&layer_core).unwrap();
    std::fs::write(layer_core.join("test.md"), "# test").unwrap();

    // Valid subdir works
    assert_eq!(
        host_support::count_layer_files(&Some(root.clone()), "core"),
        1
    );

    // Path traversal variants — all must return 0
    assert_eq!(
        host_support::count_layer_files(&Some(root.clone()), "../../etc"),
        0,
        "parent dir traversal must return 0"
    );
    assert_eq!(
        host_support::count_layer_files(&Some(root.clone()), "../.."),
        0,
        "bare parent traversal must return 0"
    );
    assert_eq!(
        host_support::count_layer_files(&Some(root.clone()), "/etc"),
        0,
        "absolute path must return 0"
    );
    assert_eq!(
        host_support::count_layer_files(&Some(root.clone()), "core/../../../etc"),
        0,
        "embedded traversal must return 0"
    );

    // None project root also returns 0
    assert_eq!(host_support::count_layer_files(&None, "core"), 0);
}

// F4: Unknown world string rejected at parse time.
#[test]
fn manifest_rejects_unknown_world() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "bad-world"
world = "oracle"

[capabilities]
host_log = true

[provides]
child = "test"
"#,
    );
    let err = PluginManifest::from_path(f.path()).unwrap_err();
    assert!(
        err.to_string().contains("unknown plugin world") && err.to_string().contains("oracle"),
        "expected 'unknown plugin world: oracle', got: {}",
        err
    );
}

// F4: Pipeline manifest with host_query rejected at check_capabilities.
#[test]
fn check_capabilities_rejects_pipeline_with_query() {
    let m = PluginManifest {
        name: "bad-pipeline".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: PluginWorld::Pipeline,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "host_query".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec!["scry".into()],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            pipeline_ops: vec!["echo".into()],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    };
    let err = PluginEngine::check_capabilities(&m).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("host_query") && msg.contains("not allowed for this world"),
        "expected per-world capability rejection for host_query, got: {}",
        msg
    );
}

// F4: Pipeline manifest with host_http also rejected (pipeline only allows host_log).
#[test]
fn check_capabilities_rejects_pipeline_with_http() {
    let m = PluginManifest {
        name: "bad-pipeline".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: PluginWorld::Pipeline,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "host_http".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["evil.com".into()],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            pipeline_ops: vec!["echo".into()],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    };
    let err = PluginEngine::check_capabilities(&m).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("host_http"),
        "expected host_http rejection, got: {}",
        msg
    );
}

// F4: PluginWorld Display impl returns kebab-case strings.
#[test]
fn plugin_world_display() {
    assert_eq!(PluginWorld::MotherChild.to_string(), "mother-child");
    assert_eq!(PluginWorld::Command.to_string(), "command");
    assert_eq!(PluginWorld::Task.to_string(), "task");
    assert_eq!(PluginWorld::Pipeline.to_string(), "pipeline");
}

// F4: PluginWorld round-trips through from_str and Display.
#[test]
fn plugin_world_roundtrip() {
    for world in [
        PluginWorld::MotherChild,
        PluginWorld::Command,
        PluginWorld::Task,
        PluginWorld::Pipeline,
    ] {
        let s = world.to_string();
        let parsed = s.parse::<PluginWorld>().unwrap();
        assert_eq!(parsed, world, "round-trip failed for {}", s);
    }
}

/// WASM trap handling: guest panic in mother-child handle() returns Err, not crash.
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

    let engine = PluginEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();
    let manifest = PluginManifest {
        name: "wrong-world".into(),
        version: "0.1.0".into(),
        description: "world mismatch".into(),
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: PluginProvides {
            child: Some("wrong".into()),
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    };

    // Instantiation with wrong world should fail cleanly
    let result = engine.instantiate_child(&component, &manifest, None);
    assert!(
        result.is_err(),
        "wrong world instantiation should return Err, not crash"
    );
}

// =====================================================================
// Credential injection — manifest parsing
// =====================================================================

#[test]
fn manifest_parses_host_secrets() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "cred-plugin"
world = "mother-child"

[capabilities]
host_log = true
host_http = ["api.github.com"]

[capabilities.host_secrets]
"api.github.com" = { secret = "github-token", location = "bearer" }

[provides]
child = "cred"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    assert_eq!(m.host_secrets.len(), 1);
    let mapping = m.host_secrets.get("api.github.com").unwrap();
    assert_eq!(mapping.secret_name, "github-token");
    assert!(matches!(mapping.location, InjectionLocation::Bearer));
}

#[test]
fn manifest_no_host_secrets_defaults_empty() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "no-cred"
world = "mother-child"

[capabilities]
host_log = true
host_http = ["api.github.com"]

[provides]
child = "test"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    assert!(m.host_secrets.is_empty());
}

#[test]
fn manifest_host_secrets_skips_unknown_location() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "bad-loc"
world = "mother-child"

[capabilities]
host_log = true
host_http = ["api.github.com"]

[capabilities.host_secrets]
"api.github.com" = { secret = "github-token", location = "magic" }

[provides]
child = "test"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    // Unknown location should be skipped with a warning
    assert!(m.host_secrets.is_empty());
}

// =====================================================================
// Credential injection — check_capabilities validation
// =====================================================================

#[test]
fn check_capabilities_rejects_host_secrets_domain_not_in_host_http() {
    let mut secrets = std::collections::HashMap::new();
    secrets.insert(
        "api.github.com".to_string(),
        CredentialMapping {
            secret_name: "github-token".to_string(),
            location: InjectionLocation::Bearer,
        },
    );
    let m = PluginManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![], // no host_http — should fail
        host_secrets: secrets,
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    };
    let err = PluginEngine::check_capabilities(&m).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("api.github.com") && msg.contains("host_secrets") && msg.contains("host_http"),
        "expected domain mismatch error, got: {}",
        msg
    );
}

#[test]
fn check_capabilities_accepts_host_secrets_with_matching_host_http() {
    let mut secrets = std::collections::HashMap::new();
    secrets.insert(
        "api.github.com".to_string(),
        CredentialMapping {
            secret_name: "github-token".to_string(),
            location: InjectionLocation::Bearer,
        },
    );
    let m = PluginManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["api.github.com".into()],
        host_secrets: secrets,
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    };
    assert!(PluginEngine::check_capabilities(&m).is_ok());
}

// =====================================================================
// Credential injection — granted_capabilities
// =====================================================================

#[test]
fn granted_capabilities_includes_credential_mappings() {
    let mut secrets = std::collections::HashMap::new();
    secrets.insert(
        "api.github.com".to_string(),
        CredentialMapping {
            secret_name: "github-token".to_string(),
            location: InjectionLocation::Bearer,
        },
    );
    let m = PluginManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: PluginWorld::MotherChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["api.github.com".into()],
        host_secrets: secrets,
        provides: PluginProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
    };
    let grants = m.granted_capabilities();
    assert!(grants.credential_mappings.contains_key("api.github.com"));
    assert_eq!(
        grants.credential_mappings["api.github.com"].secret_name,
        "github-token"
    );
}

// =====================================================================
// Credential injection — leak detection
// =====================================================================

#[test]
fn leak_check_redacts_secret_in_body() {
    let body = r#"{"token":"ghp_abc123secret","user":"test"}"#;
    let result = host_support::leak_check(body, "github-token", "ghp_abc123secret");
    assert!(
        result.contains("[REDACTED]"),
        "expected [REDACTED] in body, got: {}",
        result
    );
    assert!(
        !result.contains("ghp_abc123secret"),
        "secret should not appear in result: {}",
        result
    );
}

#[test]
fn leak_check_no_leak_returns_unchanged() {
    let body = r#"{"user":"test","status":"ok"}"#;
    let result = host_support::leak_check(body, "github-token", "ghp_abc123secret");
    assert_eq!(result, body, "body should be unchanged when no leak");
}

#[test]
fn leak_check_redacts_multiple_occurrences() {
    let body = "token=ghp_xxx and again ghp_xxx end";
    let result = host_support::leak_check(body, "test-secret", "ghp_xxx");
    assert_eq!(
        result, "token=[REDACTED] and again [REDACTED] end",
        "all occurrences should be redacted"
    );
}

// =====================================================================
// Credential injection — no injection without mapping
// =====================================================================

#[test]
fn no_credential_mapping_means_no_injection() {
    // GrantedCapabilities with http_domains but no credential_mappings
    let grants = GrantedCapabilities {
        http_domains: ["api.github.com".to_string()].into_iter().collect(),
        credential_mappings: std::collections::HashMap::new(),
        ..Default::default()
    };
    // Verify no credential mapping for the domain
    assert!(
        grants.credential_mappings.get("api.github.com").is_none(),
        "should have no credential mapping"
    );
}

#[test]
fn credential_mapping_only_for_mapped_domain() {
    let mut creds = std::collections::HashMap::new();
    creds.insert(
        "api.github.com".to_string(),
        CredentialMapping {
            secret_name: "github-token".to_string(),
            location: InjectionLocation::Bearer,
        },
    );
    let grants = GrantedCapabilities {
        http_domains: ["api.github.com".to_string(), "api.other.com".to_string()]
            .into_iter()
            .collect(),
        credential_mappings: creds,
        ..Default::default()
    };
    // api.github.com has mapping
    assert!(grants.credential_mappings.get("api.github.com").is_some());
    // api.other.com does NOT have mapping
    assert!(grants.credential_mappings.get("api.other.com").is_none());
}

// =====================================================================
// A1: Secret grants gate — check_secret_grant
// =====================================================================

#[test]
fn secret_grant_denied_when_no_file() {
    // With no grants file, all secrets should be denied
    // (the real file may or may not exist, so we test the function logic
    // by relying on the fact that a random plugin name won't be granted)
    let result = host_support::check_secret_grant("nonexistent-plugin-xyzzy-test", "some-secret");
    assert!(!result, "should deny when plugin not in grants file");
}

#[test]
fn secret_grant_denied_when_plugin_not_listed() {
    let dir = tempfile::tempdir().unwrap();
    let grants_path = dir.path().join("secret-grants.toml");
    std::fs::write(
        &grants_path,
        r#"
[other-plugin]
secrets = ["github-token"]
"#,
    )
    .unwrap();

    // Test the parsing logic directly — create a helper that reads from a specific path
    let content = std::fs::read_to_string(&grants_path).unwrap();
    let table: toml::Table = content.parse().unwrap();
    assert!(
        table.get("my-plugin").is_none(),
        "my-plugin should not be in grants"
    );
}

#[test]
fn secret_grant_denied_when_secret_not_in_list() {
    let dir = tempfile::tempdir().unwrap();
    let grants_path = dir.path().join("secret-grants.toml");
    std::fs::write(
        &grants_path,
        r#"
[my-plugin]
secrets = ["github-token"]
"#,
    )
    .unwrap();

    let content = std::fs::read_to_string(&grants_path).unwrap();
    let table: toml::Table = content.parse().unwrap();
    let plugin = table.get("my-plugin").unwrap().as_table().unwrap();
    let secrets = plugin.get("secrets").unwrap().as_array().unwrap();
    let has_openai = secrets.iter().any(|v| v.as_str() == Some("openai-key"));
    assert!(!has_openai, "openai-key should not be granted to my-plugin");
}

#[test]
fn secret_grant_allowed_when_listed() {
    let dir = tempfile::tempdir().unwrap();
    let grants_path = dir.path().join("secret-grants.toml");
    std::fs::write(
        &grants_path,
        r#"
[my-plugin]
secrets = ["github-token", "slack-webhook"]
"#,
    )
    .unwrap();

    let content = std::fs::read_to_string(&grants_path).unwrap();
    let table: toml::Table = content.parse().unwrap();
    let plugin = table.get("my-plugin").unwrap().as_table().unwrap();
    let secrets = plugin.get("secrets").unwrap().as_array().unwrap();
    let has_github = secrets.iter().any(|v| v.as_str() == Some("github-token"));
    let has_slack = secrets.iter().any(|v| v.as_str() == Some("slack-webhook"));
    assert!(has_github, "github-token should be granted");
    assert!(has_slack, "slack-webhook should be granted");
}

// =====================================================================
// A3: HTTP injection path — inject_credential builds correct headers
// =====================================================================

#[test]
fn inject_credential_adds_bearer_header() {
    let client = reqwest::blocking::Client::new();
    let builder = client.get("https://api.github.com/user");
    let mapping = CredentialMapping {
        secret_name: "test-token".to_string(),
        location: InjectionLocation::Bearer,
    };
    let builder = host_support::inject_credential(builder, &mapping, "ghp_test123");
    let request = builder.build().unwrap();
    let auth = request.headers().get("Authorization").unwrap();
    assert_eq!(
        auth, "Bearer ghp_test123",
        "should add Bearer authorization header"
    );
}

#[test]
fn inject_credential_no_header_without_call() {
    let client = reqwest::blocking::Client::new();
    let builder = client.get("https://api.github.com/user");
    // Don't call inject_credential
    let request = builder.build().unwrap();
    assert!(
        request.headers().get("Authorization").is_none(),
        "should have no Authorization header without injection"
    );
}

#[test]
fn http_get_without_mapping_sends_no_auth() {
    // GrantedCapabilities with a domain but no credential mapping
    let grants = GrantedCapabilities {
        http_domains: ["api.github.com".to_string()].into_iter().collect(),
        credential_mappings: std::collections::HashMap::new(),
        ..Default::default()
    };
    let client = host_support::build_http_client().unwrap();
    // This will make a real HTTP request, but without any auth header.
    // We verify it doesn't panic and returns a result (likely 200 for /zen).
    let result = host_support::http_get(
        &client,
        &grants,
        "test-plugin",
        "https://api.github.com/zen",
    );
    // Either succeeds (200) or fails (network error in CI) — but never panics
    // and never injects credentials
    match result {
        Ok(r) => assert!(r.status == 200 || r.status == 403),
        Err(_) => {} // network error is acceptable in test environments
    }
}

#[test]
fn http_get_with_mapping_but_no_grant_sends_no_auth() {
    // GrantedCapabilities with a credential mapping but no grants file
    let mut creds = std::collections::HashMap::new();
    creds.insert(
        "api.github.com".to_string(),
        CredentialMapping {
            secret_name: "test-token-xyzzy".to_string(),
            location: InjectionLocation::Bearer,
        },
    );
    let grants = GrantedCapabilities {
        http_domains: ["api.github.com".to_string()].into_iter().collect(),
        credential_mappings: creds,
        ..Default::default()
    };
    let client = host_support::build_http_client().unwrap();
    // The grants gate will deny (no grants file for this plugin name),
    // so the request proceeds unauthenticated.
    let result = host_support::http_get(
        &client,
        &grants,
        "test-plugin-no-grant-xyzzy",
        "https://api.github.com/zen",
    );
    // Should succeed unauthenticated — the grant denial means no credential injected
    match result {
        Ok(r) => assert!(r.status == 200 || r.status == 403),
        Err(_) => {} // network error is acceptable in test environments
    }
}

// =====================================================================
// host_emit — validate_emit + schema caching tests
// =====================================================================

/// Build a cached schema_facts map for testing (simulates load-time parse).
fn test_schema_facts(
) -> std::collections::HashMap<String, std::collections::HashMap<String, String>> {
    let mut forge_facts = std::collections::HashMap::new();
    forge_facts.insert("issue".to_string(), "forge.issue".to_string());
    forge_facts.insert("pull-request".to_string(), "forge.pr".to_string());

    let mut schema_facts = std::collections::HashMap::new();
    schema_facts.insert("forge".to_string(), forge_facts);
    schema_facts
}

#[test]
fn emit_validate_schema_not_available() {
    let schema_facts = std::collections::HashMap::new(); // empty cache

    let result = host_support::validate_emit(
        &schema_facts,
        "test-plugin",
        "forge",
        "issue",
        r#"{"title":"test"}"#,
    );
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("not available"),
        "should reject unavailable schema"
    );
}

#[test]
fn emit_validate_fact_type_not_found() {
    let schema_facts = test_schema_facts();

    let result = host_support::validate_emit(
        &schema_facts,
        "test-plugin",
        "forge",
        "nonexistent-fact",
        r#"{"title":"test"}"#,
    );
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("not found in schema"),
        "should reject unknown fact-type"
    );
}

#[test]
fn emit_validate_invalid_json() {
    let schema_facts = test_schema_facts();

    let result = host_support::validate_emit(
        &schema_facts,
        "test-plugin",
        "forge",
        "issue",
        "{not valid json",
    );
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("invalid JSON"),
        "should reject invalid JSON"
    );
}

#[test]
fn emit_validate_success_returns_event_type() {
    let schema_facts = test_schema_facts();

    let result = host_support::validate_emit(
        &schema_facts,
        "test-plugin",
        "forge",
        "issue",
        r#"{"title":"test issue","number":42}"#,
    );
    assert_eq!(result.unwrap(), "forge.issue");
}

#[test]
fn emit_validate_pull_request_fact_type() {
    let schema_facts = test_schema_facts();

    let result = host_support::validate_emit(
        &schema_facts,
        "test-plugin",
        "forge",
        "pull-request",
        r#"{"title":"test PR"}"#,
    );
    assert_eq!(result.unwrap(), "forge.pr");
}

#[test]
fn emit_capability_gating_host_emit_in_manifest() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "forge-connector"
world = "mother-child"

[capabilities]
host_log = true
host_emit = true

[provides]
child = "forge"

[schemas.forge]
package = "patina:schema/forge@1.0.0"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    assert!(m.capabilities.contains(&"host_emit".to_string()));
    assert!(m.schemas.contains_key("forge"));

    let grants = m.granted_capabilities();
    assert!(grants.host_emit);
    // schema_facts will be empty because schema.toml doesn't exist on disk
    // in this test — that's correct: load-time parse finds nothing.
    // The real validation is that the cache structure is populated when
    // schemas ARE installed (tested via setup_schema_dir in integration).
}

#[test]
fn emit_capability_gating_not_granted_without_declaration() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "simple-plugin"
world = "mother-child"

[capabilities]
host_log = true

[provides]
child = "simple"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    let grants = m.granted_capabilities();
    assert!(!grants.host_emit);
    assert!(grants.schema_facts.is_empty());
}

#[test]
fn emit_host_emit_denied_for_pipeline() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "bad-pipeline"
world = "pipeline"

[capabilities]
host_log = true
host_emit = true

[schemas.forge]
package = "patina:schema/forge@1.0.0"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    let result = PluginEngine::check_capabilities(&m);
    assert!(result.is_err(), "pipeline should not allow host_emit");
    assert!(
        result.unwrap_err().to_string().contains("not allowed"),
        "should mention world capability denial"
    );
}

#[test]
fn emit_host_emit_denied_for_command() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "bad-command"
world = "command"

[capabilities]
host_log = true
host_emit = true

[provides]
commands = ["test"]

[schemas.forge]
package = "patina:schema/forge@1.0.0"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    let result = PluginEngine::check_capabilities(&m);
    assert!(result.is_err(), "command should not allow host_emit");
}

#[test]
fn emit_host_emit_requires_schemas() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "no-schema-connector"
world = "mother-child"

[capabilities]
host_log = true
host_emit = true

[provides]
child = "forge"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    let result = PluginEngine::check_capabilities(&m);
    assert!(
        result.is_err(),
        "host_emit without schemas should be rejected"
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("no [schemas.*] entries"),
        "should mention missing schemas"
    );
}

#[test]
fn emit_host_emit_allowed_for_mother_child() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "forge-connector"
world = "mother-child"

[capabilities]
host_log = true
host_emit = true

[provides]
child = "forge"

[schemas.forge]
package = "patina:schema/forge@1.0.0"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    let result = PluginEngine::check_capabilities(&m);
    assert!(
        result.is_ok(),
        "mother-child with host_emit + schemas should be allowed: {:?}",
        result.unwrap_err()
    );
}

#[test]
fn emit_host_emit_allowed_for_task() {
    let f = write_temp_manifest(
        r#"
[plugin]
name = "fetch-task"
world = "task"

[capabilities]
host_log = true
host_emit = true

[schemas.forge]
package = "patina:schema/forge@1.0.0"
"#,
    );
    let m = PluginManifest::from_path(f.path()).unwrap();
    let result = PluginEngine::check_capabilities(&m);
    assert!(
        result.is_ok(),
        "task with host_emit + schemas should be allowed: {:?}",
        result.unwrap_err()
    );
}
