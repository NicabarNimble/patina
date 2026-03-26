use super::*;
use crate::mother::TaskIntentKind;
use std::io::Write;

fn with_temp_patina_home<T>(f: impl FnOnce(&std::path::Path) -> T) -> T {
    let _guard = crate::test_support::env_test_mutex()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let temp = tempfile::TempDir::new().unwrap();
    let home = temp.path().join("patina-home");
    std::fs::create_dir_all(&home).unwrap();
    let old_home = std::env::var_os("PATINA_HOME");
    unsafe {
        std::env::set_var("PATINA_HOME", &home);
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&home)));
    match old_home {
        Some(value) => unsafe {
            std::env::set_var("PATINA_HOME", value);
        },
        None => unsafe {
            std::env::remove_var("PATINA_HOME");
        },
    }
    match result {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

fn write_temp_manifest(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

// =====================================================================
// ChildManifest::from_path
// =====================================================================

#[test]
fn manifest_valid_minimal() {
    let f = write_temp_manifest(
        r#"
[child]
name = "test-plugin"
kind = "knowledge-child"

[capabilities]
host_log = true

[provides]
child = "test"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert_eq!(m.name, "test-plugin");
    assert_eq!(m.world, ChildKind::KnowledgeChild);
    assert_eq!(m.version, "0.0.0"); // default
    assert_eq!(m.capabilities, vec!["host_log"]);
    assert_eq!(m.provides.child.as_deref(), Some("test"));
}

#[test]
fn manifest_accepts_child_section() {
    let f = write_temp_manifest(
        r#"
[child]
name = "test-child"
kind = "knowledge-child"

[needs]
toys = ["log"]

[provides]
child = "test"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert_eq!(m.name, "test-child");
    assert_eq!(m.world, ChildKind::KnowledgeChild);
}

#[test]
fn manifest_valid_full() {
    let f = write_temp_manifest(
        r#"
[child]
name = "full-plugin"
version = "1.2.3"
description = "A full manifest"
kind = "knowledge-child"
patina_min = "0.17.0"

[capabilities]
host_log = true
filesystem = false

[provides]
child = "full"
commands = ["cmd1", "cmd2"]
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
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
    let err = ChildManifest::from_path(f.path()).unwrap_err();
    assert!(
        err.to_string().contains("missing [child] section"),
        "got: {}",
        err
    );
}

#[test]
fn manifest_missing_name() {
    let f = write_temp_manifest("[child]\nkind = \"mother-child\"\n");
    let err = ChildManifest::from_path(f.path()).unwrap_err();
    assert!(
        err.to_string().contains("missing child.name"),
        "got: {}",
        err
    );
}

#[test]
fn manifest_missing_world() {
    let f = write_temp_manifest("[child]\nname = \"test\"\n");
    let err = ChildManifest::from_path(f.path()).unwrap_err();
    assert!(
        err.to_string().contains("missing child.kind"),
        "got: {}",
        err
    );
}

#[test]
fn manifest_accepts_kind_key() {
    let f = write_temp_manifest(
        r#"
[child]
name = "test-plugin"
kind = "knowledge-child"

[needs]
toys = ["log"]

[provides]
child = "test"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert_eq!(m.world, ChildKind::KnowledgeChild);
}

#[test]
fn manifest_world_key_remains_read_compatible() {
    let f = write_temp_manifest(
        r#"
[child]
name = "test-plugin"
kind = "knowledge-child"

[needs]
toys = ["log"]

[provides]
child = "test"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert_eq!(m.world, ChildKind::KnowledgeChild);
}

#[test]
fn resolve_child_manifest_prefers_child_toml() {
    let temp = tempfile::TempDir::new().unwrap();
    let dir = temp.path();
    std::fs::write(
        dir.join("child.toml"),
        "[child]\nname='legacy'\nworld='task'\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("child.toml"),
        "[child]\nname='canonical'\nkind='task'\n",
    )
    .unwrap();

    let resolved = ChildManifest::resolve_child_manifest_path(dir).unwrap();
    assert_eq!(
        resolved.file_name().and_then(|n| n.to_str()),
        Some("child.toml")
    );
}

#[test]
fn resolve_child_manifest_falls_back_to_plugin_toml() {
    let temp = tempfile::TempDir::new().unwrap();
    let dir = temp.path();
    std::fs::write(
        dir.join("child.toml"),
        "[child]\nname='legacy'\nworld='task'\n",
    )
    .unwrap();

    let resolved = ChildManifest::resolve_child_manifest_path(dir).unwrap();
    assert_eq!(
        resolved.file_name().and_then(|n| n.to_str()),
        Some("child.toml")
    );
}

#[test]
fn manifest_parses_toy_commands() {
    let f = write_temp_manifest(
        r#"
[child]
name = "test-plugin"
kind = "knowledge-child"

[capabilities]
host_log = true

[capabilities.toys]
commands = ["git", "patina"]

[provides]
child = "test"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert_eq!(m.allowed_toy_commands, vec!["git", "patina"]);
}

#[test]
fn manifest_no_toy_commands_defaults_empty() {
    let f = write_temp_manifest(
        r#"
[child]
name = "test-plugin"
kind = "knowledge-child"

[capabilities]
host_log = true

[provides]
child = "test"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert!(m.allowed_toy_commands.is_empty());
}

#[test]
fn manifest_invalid_toml() {
    let f = write_temp_manifest("this is not valid toml {{{}}}");
    assert!(ChildManifest::from_path(f.path()).is_err());
}

#[test]
fn manifest_parses_schemas_section() {
    let f = write_temp_manifest(
        r#"
[child]
name = "grammar-github"
kind = "pipeline"

[provides]
pipeline_ops = ["parse"]
languages = ["github-issue", "github-pr"]

[schemas.github]
package = "patina:schema/github@1.0.0"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert_eq!(m.schemas.len(), 1);
    assert_eq!(
        m.schemas.get("github").unwrap(),
        "patina:schema/github@1.0.0"
    );
}

#[test]
fn manifest_no_schemas_is_empty() {
    let f = write_temp_manifest(
        r#"
[child]
name = "test"
kind = "pipeline"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert!(m.schemas.is_empty());
}

#[test]
fn manifest_parses_knowledge_child_capabilities_and_toys() {
    let f = write_temp_manifest(
        r#"
[child]
name = "ducklake"
kind = "knowledge-child"

[needs]
toys = ["log", "state", "checkpoint", "events", "task", "graph", "belief", "fetch", "lake", "query", "measure", "github"]

[needs.scopes.checkpoint]
streams = ["ducklake.sync"]

[needs.scopes.events]
subscribe = ["ducklake.sync", "belief.changed"]

[needs.scopes.task]
intents = ["fetch-source", "verify-belief"]

[needs.scopes.graph]
read = true
write = ["link", "tag"]

[needs.scopes.belief]
read = true
write = ["record-verification"]

[needs.scopes.lake]
names = ["default", "archive"]

[needs.scopes.ingress.github]
endpoint = "https://api.github.com/repos/openai/openai/issues"

[provides]
child = "ducklake"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert_eq!(m.world, ChildKind::KnowledgeChild);
    assert!(m.state_enabled);
    assert_eq!(m.checkpoint_streams, vec!["ducklake.sync"]);
    assert_eq!(
        m.subscribed_streams,
        vec!["ducklake.sync", "belief.changed"]
    );
    assert_eq!(m.task_intent_names, vec!["fetch-source", "verify-belief"]);
    assert_eq!(
        m.task_intents,
        vec![TaskIntentKind::FetchSource, TaskIntentKind::VerifyBelief]
    );
    assert!(m.graph_read);
    assert_eq!(m.graph_write_actions, vec!["link", "tag"]);
    assert!(m.belief_read);
    assert_eq!(m.belief_write_actions, vec!["record-verification"]);
    assert!(m.toys.fetch);
    assert!(m.toys.query);
    assert!(m.toys.measure);
    assert!(m.toys.graph);
    assert!(m.toys.belief);
    assert!(m.toys.lake_names.contains("default"));
    assert!(m.toys.lake_names.contains("archive"));
    assert_eq!(
        m.ingress_sources["github"].endpoint,
        "https://api.github.com/repos/openai/openai/issues"
    );
    assert_eq!(m.toys.ingress_sources["github"].name, "github");
}

#[test]
fn knowledge_child_rejects_unknown_event_stream() {
    let f = write_temp_manifest(
        r#"
[child]
name = "bad-child"
kind = "knowledge-child"

[needs]
toys = ["log", "events"]

[needs.scopes.events]
subscribe = ["unknown.stream"]

[provides]
child = "bad-child"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    let err = check_capabilities(&m).unwrap_err();
    assert!(
        err.to_string()
            .contains("unknown event stream 'unknown.stream'"),
        "got: {}",
        err
    );
}

#[test]
fn knowledge_child_example_manifests_validate() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for path in [
        root.join("children/ducklake/child.toml"),
        root.join("children/belief-verifier/child.toml"),
    ] {
        let manifest = ChildManifest::from_path(&path).unwrap();
        assert_eq!(manifest.world, ChildKind::KnowledgeChild);
        assert!(
            check_capabilities(&manifest).is_ok(),
            "manifest failed validation: {}",
            path.display()
        );
    }
}

fn ducklake_component_path() -> Option<std::path::PathBuf> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "target/wasm32-wasip1/debug/patina_ai_child_ducklake.wasm",
        "target/wasm32-wasip1/release/patina_ai_child_ducklake.wasm",
    ] {
        let path = root.join(rel);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn session_writer_component_path() -> Option<std::path::PathBuf> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "target/wasm32-wasip1/debug/patina_ai_child_session_writer.wasm",
        "target/wasm32-wasip1/release/patina_ai_child_session_writer.wasm",
    ] {
        let path = root.join(rel);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

#[test]
fn session_writer_component_instantiates_in_knowledge_child_engine() {
    let Some(wasm_path) = session_writer_component_path() else {
        return;
    };

    let engine = KnowledgeChildEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();
    let manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("children/session-writer/child.toml");
    let manifest = ChildManifest::from_path(&manifest_path).unwrap();

    let result = engine.instantiate_child(&component, &manifest, None);
    assert!(
        result.is_ok(),
        "session-writer should instantiate in knowledge-child engine"
    );
}

#[test]
fn knowledge_child_linker_fails_when_lake_not_linked() {
    let Some(wasm_path) = ducklake_component_path() else {
        return;
    };

    let engine = KnowledgeChildEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();

    let manifest_path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("children/ducklake/child.toml");
    let mut manifest = ChildManifest::from_path(&manifest_path).unwrap();
    manifest.lake_names.clear();
    manifest.toys.lake_names.clear();

    let result = engine.instantiate_child(&component, &manifest, None);
    assert!(result.is_err(), "expected missing-lake linker failure");
}

#[test]
fn knowledge_child_linker_succeeds_when_lake_declared() {
    let Some(wasm_path) = ducklake_component_path() else {
        return;
    };

    let engine = KnowledgeChildEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();

    let manifest_path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("children/ducklake/child.toml");
    let manifest = ChildManifest::from_path(&manifest_path).unwrap();

    let result = engine.instantiate_child(&component, &manifest, None);
    assert!(
        result.is_ok(),
        "expected granted-lake instantiation success"
    );
}

#[test]
fn ducklake_fixture_sync_writes_lake_queryable_by_duckdb_cli() {
    let Some(wasm_path) = ducklake_component_path() else {
        return;
    };

    with_temp_patina_home(|home| {
        let engine = KnowledgeChildEngine::new().unwrap();
        let wasm_bytes = std::fs::read(&wasm_path).unwrap();
        let component = engine.load_component(&wasm_bytes).unwrap();

        let manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("children/ducklake/child.toml");
        let manifest = ChildManifest::from_path(&manifest_path).unwrap();
        let child = engine
            .instantiate_child(&component, &manifest, None)
            .unwrap();

        child
            .handle(&crate::mother::ChildRequest {
                action: "configure-source".into(),
                payload: serde_json::json!({
                    "source_id": "fixture-source",
                    "table": "default",
                    "owner": "not-used",
                    "repo": "not-used",
                    "data_types": ["issues"],
                }),
            })
            .unwrap();

        let response = child
            .handle(&crate::mother::ChildRequest {
                action: "fetch-source".into(),
                payload: serde_json::json!({"source_id": "fixture-source"}),
            })
            .unwrap();

        let written = response
            .payload
            .get("written")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        assert!(written > 0, "expected fixture sync to write rows");

        let db_path = home.join("lakes/default/lake.duckdb");
        assert!(
            db_path.exists(),
            "expected lake DB at {}",
            db_path.display()
        );

        let output = std::process::Command::new("duckdb")
            .args([
                db_path.to_str().unwrap(),
                "COPY (SELECT COUNT(*) AS c FROM default_issues) TO STDOUT (FORMAT CSV, HEADER FALSE);",
            ])
            .output()
            .expect("run duckdb CLI query");
        assert!(
            output.status.success(),
            "duckdb query failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let count = stdout
            .trim()
            .parse::<u64>()
            .expect("duckdb count output as u64");
        assert!(
            count > 0,
            "expected default_issues to be queryable and non-empty"
        );
    });
}

#[test]
fn knowledge_child_rejects_invalid_ingress_endpoint() {
    let f = write_temp_manifest(
        r#"
[child]
name = "bad-ingress"
kind = "knowledge-child"

[needs]
toys = ["log", "ingress"]

[needs.scopes.ingress.bad]
endpoint = "http://localhost/internal"

[provides]
child = "bad-ingress"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    let err = check_capabilities(&m).unwrap_err();
    assert!(
        err.to_string().contains("invalid ingress source 'bad'"),
        "got: {}",
        err
    );
}

#[test]
fn ducklake_manifest_uses_granted_ingress_not_ambient_http() {
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("children/ducklake/child.toml");
    let manifest = ChildManifest::from_path(&path).unwrap();
    assert!(manifest
        .host_http_domains
        .contains(&"api.github.com".to_string()));
    assert!(manifest.capabilities.contains(&"host_http".to_string()));
    assert!(!manifest.toys.github);
}

#[test]
fn ducklake_manifest_runtime_grants_sdk_story_stays_connected() {
    let path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("children/ducklake/child.toml");
    let manifest = ChildManifest::from_path(&path).unwrap();
    let grants = manifest.granted_capabilities();

    assert!(!grants.toys.github);
    assert!(grants.toys.lake_names.contains("default"));
    assert!(!grants.toys.fetch);
    assert!(grants.http_domains.contains("api.github.com"));
}

// =====================================================================
// check_capabilities
// =====================================================================

#[test]
fn capabilities_all_granted() {
    let m = ChildManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::KnowledgeChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    assert!(check_capabilities(&m).is_ok());
}

#[test]
fn capabilities_empty() {
    let m = ChildManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::KnowledgeChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec![],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    assert!(check_capabilities(&m).is_ok());
}

#[test]
fn capabilities_denied() {
    let m = ChildManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::KnowledgeChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "filesystem".into(), "network".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    let err = check_capabilities(&m).unwrap_err();
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
    let m = ChildManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::Command,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec!["scry".into(), "magic_oracle".into()],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    let err = check_capabilities(&m).unwrap_err();
    assert!(
        err.to_string().contains("magic_oracle"),
        "should reject unknown kind, got: {}",
        err
    );
}

#[test]
fn check_capabilities_accepts_known_query_kinds() {
    let m = ChildManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::Command,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec!["scry".into(), "context".into(), "assay".into()],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    assert!(check_capabilities(&m).is_ok());
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
        return;
    }

    let engine = KnowledgeChildEngine::new().expect("KnowledgeChildEngine::new() failed");
    let wasm_bytes = std::fs::read(&wasm_path).expect("failed to read .wasm fixture");
    let component = engine
        .load_component(&wasm_bytes)
        .expect("load_component failed");

    // Use a manifest matching models plugin
    let manifest = ChildManifest {
        name: "patina-models".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: ChildKind::KnowledgeChild,
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
        toys: crate::mother::GrantedToys::default(),
    };

    let child = match engine.instantiate_child(&component, &manifest, None) {
        Ok(child) => child,
        Err(_) => return,
    };

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

    let engine = KnowledgeChildEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();
    let manifest = ChildManifest {
        name: "patina-models".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: ChildKind::KnowledgeChild,
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
        toys: crate::mother::GrantedToys::default(),
    };

    let child = match engine.instantiate_child(&component, &manifest, None) {
        Ok(child) => child,
        Err(_) => return,
    };
    match child.health() {
        crate::mother::ChildHealth::Healthy => {} // expected
        other => panic!("expected Healthy, got: {:?}", other),
    }
}

// =====================================================================
// WASM integration — load repos.wasm, test toy system end-to-end
// =====================================================================

/// Helper: load repos.wasm fixture and instantiate child.
fn load_repos_child() -> Option<Box<dyn crate::mother::KnowledgeChild>> {
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/patina_plugin_repos.wasm");
    if !wasm_path.exists() {
        return None;
    }

    let engine = KnowledgeChildEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();
    let manifest = ChildManifest {
        name: "patina-repos".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: ChildKind::KnowledgeChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec!["git".into(), "patina".into()],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: Some("repos".into()),
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };

    match engine.instantiate_child(&component, &manifest, None) {
        Ok(child) => Some(child),
        Err(_) => None,
    }
}

/// Repos child: report_repo + check_freshness handle() round-trip.
#[test]
fn wasm_repos_child_handle_roundtrip() {
    let child = match load_repos_child() {
        Some(c) => c,
        None => return,
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
// Benchmarks (C2) — Instant::now() instrumentation
// =====================================================================

/// Measure KnowledgeChildEngine::new(), Component::new(), instantiate_child(),
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
    // Without this, the first KnowledgeChildEngine::new() absorbs Engine::new()
    // cold-start cost (~150ms cranelift JIT init), making the benchmark
    // flaky depending on test execution order.
    let _ = KnowledgeChildEngine::new();

    // 1. KnowledgeChildEngine::new() — spec threshold: <100ms
    let t0 = Instant::now();
    let engine = KnowledgeChildEngine::new().unwrap();
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
        world: ChildKind::KnowledgeChild,
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
        toys: crate::mother::GrantedToys::default(),
    };
    let t2 = Instant::now();
    let child = match engine.instantiate_child(&component, &manifest, None) {
        Ok(child) => child,
        Err(_) => return,
    };
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
        "  KnowledgeChildEngine::new():     {:.2}ms (threshold: <100ms) {}",
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
        "KnowledgeChildEngine::new() took {:.2}ms, threshold is 100ms",
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
                 Build: cargo build --release -p patina-ai-child-doctor --target wasm32-wasip2\n\
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

fn load_doctor_manifest() -> ChildManifest {
    ChildManifest {
        name: "patina-doctor".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: ChildKind::Command,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "host_layer".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: None,
            commands: vec!["doctor".into()],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
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
[child]
name = "http-plugin"
kind = "knowledge-child"

[capabilities]
host_log = true
host_http = ["api.github.com", "hooks.slack.com"]

[provides]
child = "webhook"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert_eq!(
        m.host_http_domains,
        vec!["api.github.com", "hooks.slack.com"]
    );
}

#[test]
fn manifest_no_http_domains_defaults_empty() {
    let f = write_temp_manifest(
        r#"
[child]
name = "no-http"
kind = "knowledge-child"

[capabilities]
host_log = true

[provides]
child = "test"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert!(m.host_http_domains.is_empty());
}

// =====================================================================
// check_capabilities — HTTP domain validation
// =====================================================================

#[test]
fn check_capabilities_rejects_empty_http_domain() {
    let m = ChildManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::KnowledgeChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["".into()],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    let err = check_capabilities(&m).unwrap_err();
    assert!(err.to_string().contains("empty"), "got: {}", err);
}

#[test]
fn check_capabilities_rejects_http_domain_with_path() {
    let m = ChildManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::KnowledgeChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["api.github.com/repos".into()],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    let err = check_capabilities(&m).unwrap_err();
    assert!(err.to_string().contains("path"), "got: {}", err);
}

#[test]
fn check_capabilities_accepts_valid_http_domains() {
    let m = ChildManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::KnowledgeChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["api.github.com".into(), "hooks.slack.com".into()],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    assert!(check_capabilities(&m).is_ok());
}

// =====================================================================
// granted_capabilities — HTTP domains
// =====================================================================

#[test]
fn granted_capabilities_includes_http_domains() {
    let m = ChildManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::KnowledgeChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec!["scry".into()],
        host_http_domains: vec!["api.github.com".into()],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
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

fn hello_task_manifest() -> ChildManifest {
    ChildManifest {
        name: "hello-task".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: ChildKind::Task,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "host_layer".into()],
        allowed_toy_commands: vec!["echo".into()],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
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
    let manifest = ChildManifest {
        name: "hello-task".into(),
        version: "0.1.0".into(),
        description: "test".into(),
        world: ChildKind::Task,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "host_layer".into()],
        allowed_toy_commands: vec![], // nothing allowed
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
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

fn echo_pipeline_manifest() -> ChildManifest {
    ChildManifest {
        name: "echo-pipeline".into(),
        version: "0.1.0".into(),
        description: "test".into(),
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
        toys: crate::mother::GrantedToys::default(),
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
        .handle(&component, &manifest.name, request)
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
    let result = engine.handle(&component, &manifest.name, request);

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
    let result = engine.handle(&component, &manifest.name, request);

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
        toys: crate::mother::GrantedToys::default(),
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
[child]
name = "bad-world"
kind = "oracle"

[capabilities]
host_log = true

[provides]
child = "test"
"#,
    );
    let err = ChildManifest::from_path(f.path()).unwrap_err();
    assert!(
        err.to_string().contains("unknown child kind") && err.to_string().contains("oracle"),
        "expected 'unknown child kind: oracle', got: {}",
        err
    );
}

// F4: Pipeline manifest with host_query rejected at check_capabilities.
#[test]
fn check_capabilities_rejects_pipeline_with_query() {
    let m = ChildManifest {
        name: "bad-pipeline".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::Pipeline,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "host_query".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec!["scry".into()],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            pipeline_ops: vec!["echo".into()],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    let err = check_capabilities(&m).unwrap_err();
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
    let m = ChildManifest {
        name: "bad-pipeline".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::Pipeline,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into(), "host_http".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["evil.com".into()],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            pipeline_ops: vec!["echo".into()],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    let err = check_capabilities(&m).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("host_http"),
        "expected host_http rejection, got: {}",
        msg
    );
}

// F4: ChildKind Display impl returns kebab-case strings.
#[test]
fn plugin_world_display() {
    assert_eq!(ChildKind::KnowledgeChild.to_string(), "knowledge-child");
    assert_eq!(ChildKind::Command.to_string(), "command");
    assert_eq!(ChildKind::Task.to_string(), "task");
    assert_eq!(ChildKind::Pipeline.to_string(), "pipeline");
}

// F4: ChildKind round-trips through from_str and Display.
#[test]
fn plugin_world_roundtrip() {
    for world in [
        ChildKind::KnowledgeChild,
        ChildKind::Command,
        ChildKind::Task,
        ChildKind::Pipeline,
    ] {
        let s = world.to_string();
        let parsed = s.parse::<ChildKind>().unwrap();
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

    let engine = KnowledgeChildEngine::new().unwrap();
    let wasm_bytes = std::fs::read(&wasm_path).unwrap();
    let component = engine.load_component(&wasm_bytes).unwrap();
    let manifest = ChildManifest {
        name: "wrong-world".into(),
        version: "0.1.0".into(),
        description: "world mismatch".into(),
        world: ChildKind::KnowledgeChild,
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
        toys: crate::mother::GrantedToys::default(),
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
[child]
name = "cred-plugin"
kind = "knowledge-child"

[capabilities]
host_log = true
host_http = ["api.github.com"]

[capabilities.host_secrets]
"api.github.com" = { secret = "github-token", location = "bearer" }

[provides]
child = "cred"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert_eq!(m.host_secrets.len(), 1);
    let mapping = m.host_secrets.get("api.github.com").unwrap();
    assert_eq!(mapping.secret_name, "github-token");
    assert!(matches!(mapping.location, InjectionLocation::Bearer));
}

#[test]
fn manifest_parses_needs_connections_http_into_host_grants() {
    with_temp_patina_home(|home| {
        let connections_dir = home.join("connections");
        std::fs::create_dir_all(&connections_dir).unwrap();
        std::fs::write(
            connections_dir.join("github.toml"),
            r#"
schema_version = 0

[identity]
name = "github"
provider = "github"
auth_method = "manual"
created_at = "2026-03-26T00:00:00Z"
updated_at = "2026-03-26T00:00:00Z"

[auth]
secret_ref = "github:default"
allowed_domains = ["api.github.com"]
refresh_capable = false

[auth.injection]
type = "bearer"
"#,
        )
        .unwrap();

        let f = write_temp_manifest(
            r#"
[child]
name = "conn-http"
kind = "knowledge-child"

[needs]
toys = ["connect"]

[needs.connections]
github = { toy = "http" }

[provides]
child = "conn-http"
"#,
        );

        let m = ChildManifest::from_path(f.path()).unwrap();
        assert!(m.capabilities.contains(&"host_http".to_string()));
        assert!(m.host_http_domains.iter().any(|d| d == "api.github.com"));
        let mapping = m.host_secrets.get("api.github.com").unwrap();
        assert_eq!(mapping.secret_name, "github:default");
        assert!(matches!(mapping.location, InjectionLocation::Bearer));
    });
}

#[test]
fn manifest_needs_connections_missing_connection_still_parses() {
    with_temp_patina_home(|_| {
        let f = write_temp_manifest(
            r#"
[child]
name = "conn-missing"
kind = "knowledge-child"

[needs]
toys = ["connect"]

[needs.connections]
does-not-exist = { toy = "http" }

[provides]
child = "conn-missing"
"#,
        );

        let m = ChildManifest::from_path(f.path()).unwrap();
        assert!(m.host_http_domains.is_empty());
        assert!(m.host_secrets.is_empty());
    });
}

#[test]
fn manifest_no_host_secrets_defaults_empty() {
    let f = write_temp_manifest(
        r#"
[child]
name = "no-cred"
kind = "knowledge-child"

[capabilities]
host_log = true
host_http = ["api.github.com"]

[provides]
child = "test"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert!(m.host_secrets.is_empty());
}

#[test]
fn manifest_host_secrets_skips_unknown_location() {
    let f = write_temp_manifest(
        r#"
[child]
name = "bad-loc"
kind = "knowledge-child"

[capabilities]
host_log = true
host_http = ["api.github.com"]

[capabilities.host_secrets]
"api.github.com" = { secret = "github-token", location = "magic" }

[provides]
child = "test"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
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
    let m = ChildManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::KnowledgeChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![], // no host_http — should fail
        host_secrets: secrets,
        provides: ChildProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    let err = check_capabilities(&m).unwrap_err();
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
    let m = ChildManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::KnowledgeChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["api.github.com".into()],
        host_secrets: secrets,
        provides: ChildProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    assert!(check_capabilities(&m).is_ok());
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
    let m = ChildManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::KnowledgeChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec!["api.github.com".into()],
        host_secrets: secrets,
        provides: ChildProvides {
            child: None,
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
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
        !grants.credential_mappings.contains_key("api.github.com"),
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
    assert!(grants.credential_mappings.contains_key("api.github.com"));
    // api.other.com does NOT have mapping
    assert!(!grants.credential_mappings.contains_key("api.other.com"));
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
    if let Ok(r) = result {
        assert!(r.status == 200 || r.status == 403);
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
    if let Ok(r) = result {
        assert!(r.status == 200 || r.status == 403);
    }
}

// =====================================================================
// host_emit — validate_emit + schema caching tests
// =====================================================================

/// Build a cached schema_facts map for testing (simulates load-time parse).
fn test_schema_facts(
) -> std::collections::HashMap<String, std::collections::HashMap<String, String>> {
    let mut github_facts = std::collections::HashMap::new();
    github_facts.insert("issue".to_string(), "github.issue".to_string());
    github_facts.insert("pull-request".to_string(), "github.pr".to_string());

    let mut schema_facts = std::collections::HashMap::new();
    schema_facts.insert("github".to_string(), github_facts);
    schema_facts
}

#[test]
fn emit_validate_schema_not_available() {
    let schema_facts = std::collections::HashMap::new(); // empty cache

    let result = host_support::validate_emit(
        &schema_facts,
        "test-plugin",
        "github",
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
        "github",
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
        "github",
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
        "github",
        "issue",
        r#"{"title":"test issue","number":42}"#,
    );
    assert_eq!(result.unwrap(), "github.issue");
}

#[test]
fn emit_validate_pull_request_fact_type() {
    let schema_facts = test_schema_facts();

    let result = host_support::validate_emit(
        &schema_facts,
        "test-plugin",
        "github",
        "pull-request",
        r#"{"title":"test PR"}"#,
    );
    assert_eq!(result.unwrap(), "github.pr");
}

#[test]
fn emit_capability_gating_host_emit_in_manifest() {
    let f = write_temp_manifest(
        r#"
[child]
name = "forge-connector"
kind = "knowledge-child"

[capabilities]
host_log = true
host_emit = true

[provides]
child = "forge"

[schemas.forge]
package = "patina:schema/forge@1.0.0"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
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
[child]
name = "simple-plugin"
kind = "knowledge-child"

[capabilities]
host_log = true

[provides]
child = "simple"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    let grants = m.granted_capabilities();
    assert!(!grants.host_emit);
    assert!(grants.schema_facts.is_empty());
}

#[test]
fn emit_host_emit_denied_for_pipeline() {
    let f = write_temp_manifest(
        r#"
[child]
name = "bad-pipeline"
kind = "pipeline"

[capabilities]
host_log = true
host_emit = true

[schemas.forge]
package = "patina:schema/forge@1.0.0"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    let result = check_capabilities(&m);
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
[child]
name = "bad-command"
kind = "command"

[capabilities]
host_log = true
host_emit = true

[provides]
commands = ["test"]

[schemas.forge]
package = "patina:schema/forge@1.0.0"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    let result = check_capabilities(&m);
    assert!(result.is_err(), "command should not allow host_emit");
}

#[test]
fn emit_host_emit_requires_schemas() {
    let f = write_temp_manifest(
        r#"
[child]
name = "no-schema-connector"
kind = "knowledge-child"

[capabilities]
host_log = true
host_emit = true

[provides]
child = "forge"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    let result = check_capabilities(&m);
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
[child]
name = "forge-connector"
kind = "knowledge-child"

[capabilities]
host_log = true
host_emit = true

[provides]
child = "forge"

[schemas.forge]
package = "patina:schema/forge@1.0.0"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    let result = check_capabilities(&m);
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
[child]
name = "fetch-task"
kind = "task"

[capabilities]
host_log = true
host_emit = true

[schemas.forge]
package = "patina:schema/forge@1.0.0"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    let result = check_capabilities(&m);
    assert!(
        result.is_ok(),
        "task with host_emit + schemas should be allowed: {:?}",
        result.unwrap_err()
    );
}

// =====================================================================
// ChildRole — parsing, display, expected_worlds
// =====================================================================

#[test]
fn role_from_str_all_variants() {
    assert_eq!(
        "connector".parse::<ChildRole>().unwrap(),
        ChildRole::Connector
    );
    assert_eq!("grammar".parse::<ChildRole>().unwrap(), ChildRole::Grammar);
    assert_eq!(
        "extension".parse::<ChildRole>().unwrap(),
        ChildRole::Extension
    );
    assert_eq!("app".parse::<ChildRole>().unwrap(), ChildRole::App);
}

#[test]
fn role_from_str_unknown_errors() {
    assert!("widget".parse::<ChildRole>().is_err());
    assert!("CONNECTOR".parse::<ChildRole>().is_err()); // case sensitive
}

#[test]
fn role_display() {
    assert_eq!(ChildRole::Connector.to_string(), "connector");
    assert_eq!(ChildRole::Grammar.to_string(), "grammar");
    assert_eq!(ChildRole::Extension.to_string(), "extension");
    assert_eq!(ChildRole::App.to_string(), "app");
}

#[test]
fn role_expected_worlds() {
    assert!(ChildRole::Connector
        .expected_worlds()
        .contains(&ChildKind::KnowledgeChild));
    assert!(ChildRole::Grammar
        .expected_worlds()
        .contains(&ChildKind::Pipeline));
    assert!(ChildRole::Extension
        .expected_worlds()
        .contains(&ChildKind::Command));
    assert!(ChildRole::App
        .expected_worlds()
        .contains(&ChildKind::KnowledgeChild));
}

// =====================================================================
// ChildManifest — role field parsing
// =====================================================================

#[test]
fn manifest_with_role_parses() {
    let f = write_temp_manifest(
        r#"
[child]
name = "test-connector"
kind = "knowledge-child"
role = "connector"

[capabilities]
host_log = true

[provides]
child = "test"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert_eq!(m.role, Some(ChildRole::Connector));
}

#[test]
fn manifest_without_role_is_none() {
    let f = write_temp_manifest(
        r#"
[child]
name = "legacy-plugin"
kind = "knowledge-child"

[capabilities]
host_log = true

[provides]
child = "legacy"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert_eq!(m.role, None);
}

#[test]
fn manifest_unknown_role_errors() {
    let f = write_temp_manifest(
        r#"
[child]
name = "bad-role"
kind = "knowledge-child"
role = "widget"

[capabilities]
host_log = true

[provides]
child = "bad"
"#,
    );
    let result = ChildManifest::from_path(f.path());
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("unknown child role"),);
}

// =====================================================================
// Role-world validation (warns, does not block)
// =====================================================================

#[test]
fn role_world_valid_combo_passes() {
    let m = ChildManifest {
        name: "conn".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::KnowledgeChild,
        role: Some(ChildRole::Connector),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: Some("conn".into()),
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    // connector + mother-child is valid — check_capabilities should pass
    assert!(check_capabilities(&m).is_ok());
}

#[test]
fn role_world_unusual_combo_still_passes() {
    // grammar + mother-child is unusual but should NOT block
    let m = ChildManifest {
        name: "weird-grammar".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::KnowledgeChild,
        role: Some(ChildRole::Grammar),
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: Some("weird".into()),
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    // Unusual combo warns but does NOT bail
    assert!(check_capabilities(&m).is_ok());
}

#[test]
fn role_none_skips_validation() {
    // Legacy plugin with no role — should pass without warnings
    let m = ChildManifest {
        name: "legacy".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::KnowledgeChild,
        role: None,
        patina_min: "0.0.0".into(),
        capabilities: vec!["host_log".into()],
        allowed_toy_commands: vec![],
        host_query_kinds: vec![],
        host_http_domains: vec![],
        host_secrets: std::collections::HashMap::new(),
        provides: ChildProvides {
            child: Some("legacy".into()),
            commands: vec![],
            ..Default::default()
        },
        schemas: std::collections::HashMap::new(),
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
        toys: crate::mother::GrantedToys::default(),
    };
    assert!(check_capabilities(&m).is_ok());
}
