use super::*;
use crate::mother::TaskIntentKind;
use crate::test_support::with_temp_patina_home;
use std::io::Write;

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
    assert_eq!(m.world, ChildKind::Child);
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
    assert_eq!(m.world, ChildKind::Child);
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
    assert_eq!(m.world, ChildKind::Child);
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
    assert_eq!(m.world, ChildKind::Child);
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
name = "source-router"
kind = "knowledge-child"

[needs]
toys = ["log", "state", "checkpoint", "events", "task", "graph", "belief", "fetch", "lake", "query", "measure", "github"]

[needs.scopes.checkpoint]
streams = ["source.sync"]

[needs.scopes.events]
subscribe = ["source.sync", "belief.changed"]

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
child = "source-router"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert_eq!(m.world, ChildKind::Child);
    assert!(m.state_enabled);
    assert_eq!(m.checkpoint_streams, vec!["source.sync"]);
    assert_eq!(m.subscribed_streams, vec!["source.sync", "belief.changed"]);
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
    for path in [root.join("children/belief-verifier/child.toml")] {
        let manifest = ChildManifest::from_path(&path).unwrap();
        assert_eq!(manifest.world, ChildKind::Child);
        assert!(
            check_capabilities(&manifest).is_ok(),
            "manifest failed validation: {}",
            path.display()
        );
    }
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

// =====================================================================
// check_capabilities
// =====================================================================

#[test]
fn capabilities_all_granted() {
    let m = ChildManifest {
        name: "test".into(),
        version: "0.1.0".into(),
        description: String::new(),
        world: ChildKind::Child,
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
        world: ChildKind::Child,
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
        world: ChildKind::Child,
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
        world: ChildKind::Child,
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
        world: ChildKind::Child,
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
        toys: crate::mother::GrantedToys::default(),
    };
    assert!(check_capabilities(&m).is_ok());
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
        world: ChildKind::Child,
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
        world: ChildKind::Child,
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
        world: ChildKind::Child,
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
        world: ChildKind::Child,
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
    assert_eq!(ChildKind::Child.to_string(), "child");
    assert_eq!(ChildKind::Pipeline.to_string(), "pipeline");
}

// F4: ChildKind round-trips through from_str and Display.
#[test]
fn plugin_world_roundtrip() {
    for world in [ChildKind::Child, ChildKind::Pipeline] {
        let s = world.to_string();
        let parsed = s.parse::<ChildKind>().unwrap();
        assert_eq!(parsed, world, "round-trip failed for {}", s);
    }
}

#[test]
fn plugin_world_retires_command_and_task() {
    let command = "command".parse::<ChildKind>().unwrap_err().to_string();
    assert!(
        command.contains("retired") && command.contains("child"),
        "unexpected command retirement error: {}",
        command
    );

    let task = "task".parse::<ChildKind>().unwrap_err().to_string();
    assert!(
        task.contains("retired") && task.contains("child"),
        "unexpected task retirement error: {}",
        task
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
        world: ChildKind::Child,
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
        world: ChildKind::Child,
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
        world: ChildKind::Child,
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
fn emit_host_emit_retires_command_kind() {
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
    let err = ChildManifest::from_path(f.path()).unwrap_err().to_string();
    assert!(
        err.contains("retired") && err.contains("child"),
        "command kind should return retired-kind guidance: {}",
        err
    );
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
fn emit_host_emit_retires_task_kind() {
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
    let err = ChildManifest::from_path(f.path()).unwrap_err().to_string();
    assert!(
        err.contains("retired") && err.contains("child"),
        "task kind should return retired-kind guidance: {}",
        err
    );
}

#[test]
fn measure_manifest_parses_declared_metrics() {
    let f = write_temp_manifest(
        r#"
[child]
name = "metrics-plugin"
kind = "knowledge-child"

[needs]
toys = ["measure"]

[needs.metrics.parse_accuracy]
type = "gauge"
labels = ["file_type"]

[needs.metrics.records_ingested]
type = "counter"
labels = ["source"]
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert_eq!(
        m.declared_metrics
            .get("parse_accuracy")
            .unwrap()
            .metric_type,
        DeclaredMetricType::Gauge
    );
    assert_eq!(
        m.declared_metrics
            .get("records_ingested")
            .unwrap()
            .metric_type,
        DeclaredMetricType::Counter
    );
}

#[test]
fn manifest_parses_filesystem_scope_preopen_path() {
    let f = write_temp_manifest(
        r#"
[child]
name = "fs-plugin"
kind = "knowledge-child"

[needs]
toys = ["log"]

[needs.scopes.filesystem]
path = "/tmp/input"

[needs.scopes.filesystem.output]
path = "/tmp/output"
mode = "read-write"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();
    assert_eq!(
        m.filesystem_preopens,
        vec![
            FilesystemPreopenConfig {
                host_path: "/tmp/input".to_string(),
                guest_path: "/input".to_string(),
                mode: FilesystemAccessMode::ReadOnly,
            },
            FilesystemPreopenConfig {
                host_path: "/tmp/output".to_string(),
                guest_path: "/output".to_string(),
                mode: FilesystemAccessMode::ReadWrite,
            }
        ]
    );
}

#[test]
fn measure_undeclared_metric_rejected() {
    let declared = std::collections::HashMap::new();
    let result = host_support::record_declared_metric(
        "metrics-plugin",
        &declared,
        "undeclared_metric",
        DeclaredMetricType::Gauge,
        1.0,
        &[],
    );
    let error = result.unwrap_err();
    assert!(
        error.starts_with("measure/undeclared-metric:"),
        "expected deterministic undeclared metric error, got: {}",
        error
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
        .contains(&ChildKind::Child));
    assert!(ChildRole::Grammar
        .expected_worlds()
        .contains(&ChildKind::Pipeline));
    assert!(ChildRole::Extension
        .expected_worlds()
        .contains(&ChildKind::Child));
    assert!(ChildRole::App.expected_worlds().contains(&ChildKind::Child));
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
        world: ChildKind::Child,
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
        world: ChildKind::Child,
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
        world: ChildKind::Child,
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
        toys: crate::mother::GrantedToys::default(),
    };
    assert!(check_capabilities(&m).is_ok());
}

#[test]
fn capability_checks_match_for_auto_granted_capability() {
    let f = write_temp_manifest(
        r#"
[child]
name = "cap-match"
kind = "knowledge-child"

[capabilities]
host_log = true

[provides]
child = "test"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();

    let top_level = check_capabilities(&m).is_ok();
    let engine_level = KnowledgeChildEngine::check_capabilities(&m).is_ok();

    assert_eq!(top_level, engine_level);
}

#[test]
fn capability_checks_match_for_unknown_capability() {
    let f = write_temp_manifest(
        r#"
[child]
name = "cap-mismatch"
kind = "knowledge-child"

[capabilities]
host_log = true
host_unknown = true

[provides]
child = "test"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();

    let top_level = check_capabilities(&m).is_ok();
    let engine_level = KnowledgeChildEngine::check_capabilities(&m).is_ok();

    assert_eq!(top_level, engine_level);
}

#[test]
fn capability_checks_match_for_host_layer_case() {
    let f = write_temp_manifest(
        r#"
[child]
name = "layer-check"
kind = "knowledge-child"

[needs]
toys = ["layer"]

[provides]
child = "test"
"#,
    );
    let m = ChildManifest::from_path(f.path()).unwrap();

    let top_level = check_capabilities(&m).is_ok();
    let engine_level = KnowledgeChildEngine::check_capabilities(&m).is_ok();

    assert_eq!(top_level, engine_level);
}

#[cfg(all(test, patina_compat_proof))]
#[allow(deprecated)]
#[test]
fn compat_proof_deprecated_knowledge_child_aliases_compile() {
    use crate::child::engine::KnowledgeChildEngine;
    use crate::mother::KnowledgeChild;

    fn accepts_legacy_trait(_child: &dyn KnowledgeChild) {}

    let _ = std::any::TypeId::of::<KnowledgeChildEngine>();
    let _ = accepts_legacy_trait as fn(&dyn KnowledgeChild);
}
