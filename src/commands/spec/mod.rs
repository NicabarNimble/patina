//! Spec lifecycle management
//!
//! This module follows the dependable-rust pattern:
//! - Public interface (this file): clean API for spec operations
//! - Internal implementation: all logic in internal/

#[allow(dead_code)]
pub(crate) mod internal;

#[allow(unused_imports)]
// Data types and functions re-exported for session integration (Phase 5)
pub(crate) use internal::{get_all_specs, get_blocked_specs, ListFilters};

#[allow(unused_imports)]
// Query data functions re-exported for MCP (Phase 6)
pub(crate) use internal::{
    check_spec_value, get_ready_specs, handoff_spec_value, history_spec_value, next_spec_value,
    packet_spec_value, prompt_spec_value, show_spec_value,
};

#[allow(unused_imports)]
// Mutation _value() functions re-exported for MCP (Phase 6)
pub(crate) use internal::{
    abandon_spec_value, block_spec_value, complete_spec_value, create_spec_value, pause_spec_value,
    promote_spec_value, rename_spec_value, reopen_spec_value, resume_spec_value, set_spec_value,
    split_spec_value,
};

use anyhow::Result;
pub use patina::spec::SpecCommands;
use patina_protocol::{
    BuiltinChild, BuiltinChildAction, BuiltinChildRequest, BuiltinChildResult, SpecDispatchRequest,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum SpecBackendMode {
    Off,
    Observe,
    Execute,
}

impl Default for SpecBackendMode {
    fn default() -> Self {
        Self::Off
    }
}

fn parse_spec_backend_mode(raw: &str) -> SpecBackendMode {
    match raw.trim().to_ascii_lowercase().as_str() {
        "off" | "legacy" => SpecBackendMode::Off,
        "observe" | "slate-observe" => SpecBackendMode::Observe,
        "execute" | "slate-execute" => SpecBackendMode::Execute,
        _ => SpecBackendMode::Off,
    }
}

#[derive(Debug, Default, Deserialize)]
struct SpecBackendManifest {
    #[serde(default)]
    spec: Option<SpecBackendManifestSpec>,
}

#[derive(Debug, Default, Deserialize)]
struct SpecBackendManifestSpec {
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    backend: Option<String>,
}

fn parse_manifest_spec_backend_mode(content: &str) -> Option<String> {
    let parsed = toml::from_str::<SpecBackendManifest>(content).ok()?;
    let spec = parsed.spec?;
    spec.mode
        .or(spec.backend)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn load_manifest_spec_backend_mode() -> Option<String> {
    let project_root = patina::session::SessionManager::find_project_root().ok()?;
    let manifest_path = project_root.join(".patina/manifest.toml");
    let raw = std::fs::read_to_string(manifest_path).ok()?;
    parse_manifest_spec_backend_mode(&raw)
}

fn resolve_spec_backend_mode() -> SpecBackendMode {
    if let Ok(raw) = std::env::var("PATINA_SPEC_BACKEND") {
        return parse_spec_backend_mode(&raw);
    }

    if let Some(raw) = load_manifest_spec_backend_mode() {
        return parse_spec_backend_mode(&raw);
    }

    SpecBackendMode::Off
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpecDispatchEnvelope {
    command: SpecCommands,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    origin_project: Option<String>,
    #[serde(default)]
    backend_mode: SpecBackendMode,
}

fn build_spec_dispatch_envelope(
    command: SpecCommands,
    project: Option<String>,
    origin_project: Option<String>,
) -> SpecDispatchEnvelope {
    SpecDispatchEnvelope {
        command,
        project,
        origin_project,
        backend_mode: resolve_spec_backend_mode(),
    }
}

pub fn execute(mut command: SpecCommands, project: Option<String>) -> Result<()> {
    if std::env::var("PATINA_SPEC_DIRECT").as_deref() == Ok("1") {
        if let Ok(raw) = std::env::var("PATINA_SPEC_DIRECT_COMMAND_JSON") {
            command = serde_json::from_str(&raw)?;
        }
        let payload = execute_value(command)?;
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    let caller_project = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.display().to_string());

    let embedded_create_project = match &command {
        SpecCommands::Create { project, .. } => project.clone(),
        _ => None,
    };

    let effective_project = project
        .clone()
        .or_else(|| embedded_create_project.clone())
        .or_else(|| caller_project.clone());

    let mut origin_project = if project.is_some() || embedded_create_project.is_some() {
        caller_project.clone()
    } else {
        None
    };

    if let SpecCommands::Create {
        origin_project: embedded_origin,
        ..
    } = &mut command
    {
        if embedded_origin.is_none() {
            *embedded_origin = origin_project.clone();
        }
        if embedded_origin.is_some() {
            origin_project = embedded_origin.clone();
        }
    }

    if !command.wants_json() {
        match &command {
            SpecCommands::Complete { id, .. } => {
                if !confirm(&format!("Complete spec '{}' now?", id))? {
                    println!("Aborted.");
                    return Ok(());
                }
            }
            SpecCommands::Abandon { id, .. } => {
                if !confirm(&format!("Abandon spec '{}' now?", id))? {
                    println!("Aborted.");
                    return Ok(());
                }
            }
            _ => {}
        }
    }

    let envelope = build_spec_dispatch_envelope(command.clone(), effective_project, origin_project);

    let protocol_request = BuiltinChildRequest::new(
        BuiltinChild::SpecManager,
        BuiltinChildAction::SpecDispatch(SpecDispatchRequest {
            command: serde_json::to_value(envelope)?,
        }),
    );
    let client = patina::mother::control_plane_client();
    let response = client.child_action_typed(&protocol_request).map_err(|e| {
        anyhow::anyhow!(
            "spec-manager unavailable via Mother (start with `patina mother start`): {}",
            e
        )
    })?;
    let response = match response.result {
        BuiltinChildResult::Dispatch { payload } => payload,
        other => {
            anyhow::bail!("Unexpected typed response from spec-manager: {:?}", other);
        }
    };

    if let Some(text) = response.get("text").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            println!("{}", text);
        }
    }

    if let Some(data) = response.get("data") {
        if response
            .get("json")
            .and_then(|v| v.as_bool())
            .unwrap_or_else(|| command.wants_json())
            || response.get("text").is_none_or(|v| v.is_null())
        {
            println!("{}", serde_json::to_string_pretty(data)?);
        }
    }

    Ok(())
}

#[allow(dead_code)]
pub(crate) fn execute_value(command: SpecCommands) -> Result<Value> {
    patina::spec::execute_command_value(command)
}

fn confirm(prompt: &str) -> Result<bool> {
    use std::io::{stdin, stdout, Write};

    print!("{} [y/N] ", prompt);
    stdout().flush()?;
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    Ok(trimmed.eq_ignore_ascii_case("y") || trimmed.eq_ignore_ascii_case("yes"))
}

#[cfg(test)]
mod tests {
    use super::{
        build_spec_dispatch_envelope, parse_manifest_spec_backend_mode, parse_spec_backend_mode,
        resolve_spec_backend_mode, SpecBackendMode, SpecCommands,
    };
    use clap::Parser;
    use std::path::Path;

    fn with_temp_patina_project<T>(f: impl FnOnce(&Path) -> T) -> T {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp = tempfile::TempDir::new().expect("temp project");
        let project_root = temp.path().join("project");
        std::fs::create_dir_all(project_root.join(".patina")).expect("create .patina");
        std::fs::create_dir_all(project_root.join("layer")).expect("create layer");

        let old_cwd = std::env::current_dir().expect("cwd");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&project_root)));
        std::env::set_current_dir(old_cwd).expect("restore cwd");

        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    // Minimal CLI struct for testing SpecCommands parsing
    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        command: SpecCommands,
    }

    fn parse(args: &[&str]) -> Result<SpecCommands, clap::Error> {
        TestCli::try_parse_from(std::iter::once("patina-spec").chain(args.iter().copied()))
            .map(|cli| cli.command)
    }

    #[test]
    fn create_basic() {
        let cmd = parse(&["create", "feat", "my-feature"]).unwrap();
        match cmd {
            SpecCommands::Create {
                r#type,
                id,
                title,
                json,
                ..
            } => {
                assert_eq!(r#type, "feat");
                assert_eq!(id, "my-feature");
                assert!(title.is_none());
                assert!(!json);
            }
            _ => panic!("expected Create"),
        }
    }

    #[test]
    fn create_with_options() {
        let cmd = parse(&[
            "create",
            "fix",
            "my-bug",
            "--title",
            "Fix the bug",
            "--blocked-by",
            "other-spec",
            "--json",
        ])
        .unwrap();
        match cmd {
            SpecCommands::Create {
                r#type,
                id,
                title,
                blocked_by,
                json,
                ..
            } => {
                assert_eq!(r#type, "fix");
                assert_eq!(id, "my-bug");
                assert_eq!(title.as_deref(), Some("Fix the bug"));
                assert_eq!(blocked_by, vec!["other-spec"]);
                assert!(json);
            }
            _ => panic!("expected Create"),
        }
    }

    #[test]
    fn create_with_cross_project_flags() {
        let cmd = parse(&[
            "create",
            "feat",
            "cross-proj-spec",
            "--project",
            "/tmp/other-project",
            "--force-cross-project",
        ])
        .unwrap();
        match cmd {
            SpecCommands::Create {
                project,
                force_cross_project,
                ..
            } => {
                assert_eq!(project.as_deref(), Some("/tmp/other-project"));
                assert!(force_cross_project);
            }
            _ => panic!("expected Create"),
        }
    }

    #[test]
    fn archive_with_id() {
        let cmd = parse(&["archive", "my-spec"]).unwrap();
        match cmd {
            SpecCommands::Archive { id, stale, dry_run } => {
                assert_eq!(id.as_deref(), Some("my-spec"));
                assert!(!stale);
                assert!(!dry_run);
            }
            _ => panic!("expected Archive"),
        }
    }

    #[test]
    fn archive_stale_no_id() {
        let cmd = parse(&["archive", "--stale"]).unwrap();
        match cmd {
            SpecCommands::Archive { id, stale, .. } => {
                assert!(id.is_none());
                assert!(stale);
            }
            _ => panic!("expected Archive"),
        }
    }

    #[test]
    fn archive_stale_dry_run() {
        let cmd = parse(&["archive", "--stale", "--dry-run"]).unwrap();
        match cmd {
            SpecCommands::Archive { id, stale, dry_run } => {
                assert!(id.is_none());
                assert!(stale);
                assert!(dry_run);
            }
            _ => panic!("expected Archive"),
        }
    }

    #[test]
    fn archive_no_id_no_stale_still_parses() {
        // clap accepts this — validation happens at dispatch time in main.rs
        let cmd = parse(&["archive"]).unwrap();
        match cmd {
            SpecCommands::Archive { id, stale, .. } => {
                assert!(id.is_none());
                assert!(!stale);
            }
            _ => panic!("expected Archive"),
        }
    }

    #[test]
    fn set_basic() {
        let cmd = parse(&["set", "my-spec", "beliefs", "+some-belief"]).unwrap();
        match cmd {
            SpecCommands::Set {
                id,
                field,
                value,
                json,
            } => {
                assert_eq!(id, "my-spec");
                assert_eq!(field, "beliefs");
                assert_eq!(value, "+some-belief");
                assert!(!json);
            }
            _ => panic!("expected Set"),
        }
    }

    #[test]
    fn set_with_json() {
        let cmd = parse(&["set", "my-spec", "target", "v0.33.0", "--json"]).unwrap();
        match cmd {
            SpecCommands::Set {
                id,
                field,
                value,
                json,
            } => {
                assert_eq!(id, "my-spec");
                assert_eq!(field, "target");
                assert_eq!(value, "v0.33.0");
                assert!(json);
            }
            _ => panic!("expected Set"),
        }
    }

    #[test]
    fn backend_mode_parser_maps_expected_aliases() {
        assert_eq!(parse_spec_backend_mode("off"), SpecBackendMode::Off);
        assert_eq!(parse_spec_backend_mode("legacy"), SpecBackendMode::Off);
        assert_eq!(parse_spec_backend_mode("observe"), SpecBackendMode::Observe);
        assert_eq!(
            parse_spec_backend_mode("slate-observe"),
            SpecBackendMode::Observe
        );
        assert_eq!(parse_spec_backend_mode("execute"), SpecBackendMode::Execute);
        assert_eq!(
            parse_spec_backend_mode("slate-execute"),
            SpecBackendMode::Execute
        );
    }

    #[test]
    fn backend_mode_parser_fails_closed_to_off() {
        assert_eq!(parse_spec_backend_mode(""), SpecBackendMode::Off);
        assert_eq!(
            parse_spec_backend_mode("unknown-mode"),
            SpecBackendMode::Off
        );
    }

    #[test]
    fn manifest_backend_mode_parser_reads_spec_mode() {
        let manifest = r#"
[project]
schema = 1

[needs]
children = ["spec-manager"]

[spec]
mode = "observe"
"#;

        assert_eq!(
            parse_manifest_spec_backend_mode(manifest).as_deref(),
            Some("observe")
        );
    }

    #[test]
    fn manifest_backend_mode_parser_accepts_backend_alias() {
        let manifest = r#"
[project]
schema = 1

[needs]
children = ["spec-manager"]

[spec]
backend = "execute"
"#;

        assert_eq!(
            parse_manifest_spec_backend_mode(manifest).as_deref(),
            Some("execute")
        );
    }

    #[test]
    fn manifest_backend_mode_parser_returns_none_when_missing() {
        let manifest = r#"
[project]
schema = 1

[needs]
children = ["spec-manager"]
"#;

        assert!(parse_manifest_spec_backend_mode(manifest).is_none());
    }

    #[test]
    fn resolve_backend_mode_reads_manifest_execute_and_envelope_uses_it() {
        with_temp_patina_project(|project_root| {
            std::fs::write(
                project_root.join(".patina/manifest.toml"),
                r#"
[project]
schema = 1

[needs]
children = ["spec-manager", "slate-manager"]

[spec]
mode = "execute"
"#,
            )
            .expect("write manifest");

            let old_backend = std::env::var("PATINA_SPEC_BACKEND").ok();
            std::env::remove_var("PATINA_SPEC_BACKEND");
            std::env::set_current_dir(project_root.join("layer")).expect("cd project subdir");

            let mode = resolve_spec_backend_mode();
            assert_eq!(mode, SpecBackendMode::Execute);

            let envelope =
                build_spec_dispatch_envelope(SpecCommands::Next { json: true }, None, None);
            assert_eq!(envelope.backend_mode, SpecBackendMode::Execute);

            match old_backend {
                Some(value) => std::env::set_var("PATINA_SPEC_BACKEND", value),
                None => std::env::remove_var("PATINA_SPEC_BACKEND"),
            }
        });
    }

    #[test]
    fn resolve_backend_mode_env_overrides_manifest() {
        with_temp_patina_project(|project_root| {
            std::fs::write(
                project_root.join(".patina/manifest.toml"),
                r#"
[project]
schema = 1

[needs]
children = ["spec-manager", "slate-manager"]

[spec]
mode = "execute"
"#,
            )
            .expect("write manifest");

            let old_backend = std::env::var("PATINA_SPEC_BACKEND").ok();
            std::env::set_var("PATINA_SPEC_BACKEND", "off");
            std::env::set_current_dir(project_root).expect("cd project root");

            assert_eq!(resolve_spec_backend_mode(), SpecBackendMode::Off);

            match old_backend {
                Some(value) => std::env::set_var("PATINA_SPEC_BACKEND", value),
                None => std::env::remove_var("PATINA_SPEC_BACKEND"),
            }
        });
    }
}
