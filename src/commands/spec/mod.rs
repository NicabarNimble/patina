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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpecDispatchEnvelope {
    command: SpecCommands,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    origin_project: Option<String>,
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

    let envelope = SpecDispatchEnvelope {
        command: command.clone(),
        project: effective_project,
        origin_project,
    };

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
    use super::SpecCommands;
    use clap::Parser;

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
}
