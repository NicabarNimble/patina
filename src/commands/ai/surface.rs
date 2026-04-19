use anyhow::{anyhow, bail, Result};
use std::io::IsTerminal as _;

use patina::interface::{self};
use patina::project;
use patina::workspace;

use crate::commands::launch::internal::{self as launch_internal, BranchAction};

#[derive(Debug, Clone)]
pub struct AiSetupRequest {
    pub interface: Option<String>,
    pub path: Option<String>,
    pub force: bool,
    pub all: bool,
}

#[derive(Debug, Clone)]
pub struct AiRefreshRequest {
    pub interface: Option<String>,
    pub path: Option<String>,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct AiLaunchRequest {
    pub interface_name: String,
    pub title: Option<String>,
    pub requested_session: Option<String>,
    pub voice: Option<String>,
    pub path: Option<String>,
    pub set_default: bool,
    pub tmux: bool,
    pub no_tmux: bool,
    pub decision_path: Option<String>,
}

pub fn launch_default() -> Result<()> {
    let project_root = launch_internal::resolve_project_path(None)?;
    let interface_name = interface::resolve_preferred_ai_interface(&project_root)?;
    launch(AiLaunchRequest {
        interface_name,
        title: None,
        requested_session: None,
        voice: None,
        path: None,
        set_default: false,
        tmux: false,
        no_tmux: false,
        decision_path: Some("direct".to_string()),
    })
}

pub fn setup(request: AiSetupRequest) -> Result<()> {
    let project_path = launch_internal::resolve_project_path(request.path.as_deref())?;

    let (default_interface, prepared) = if request.all {
        let result = interface::ensure_ai_surface(interface::AiSurfaceRequest {
            project_root: &project_path,
            force: request.force,
            default_interface: request.interface.as_deref(),
        })?;
        (result.default_interface, result.prepared)
    } else if let Some(interface_name) = request.interface.as_deref() {
        let config = interface::ensure_ai_project_config(&project_path, Some(interface_name))?;
        let prepared = vec![interface::prepare_ai_bundle(
            &project_path,
            interface_name,
            request.force,
        )?];
        (config.default_interface, prepared)
    } else {
        let config = interface::ensure_ai_project_config(&project_path, None)?;
        let prepared = vec![interface::prepare_ai_bundle(
            &project_path,
            &config.default_interface,
            request.force,
        )?];
        (config.default_interface, prepared)
    };

    println!(
        "Patina AI bundles deployed. Default interface: {}",
        default_interface
    );
    for prepared in prepared {
        println!("  {}:", prepared.display_name);
        println!("    Context: {}", prepared.bootstrap.context_path.display());
        println!(
            "    Bootstrap: {}",
            prepared.bootstrap.bootstrap_path.display()
        );
        if let Some(backup_snapshot) = prepared.bootstrap.backup_snapshot {
            println!("    Backup: {}", backup_snapshot.display());
        }
    }

    Ok(())
}

pub fn refresh(request: AiRefreshRequest) -> Result<()> {
    let project_path = launch_internal::resolve_project_path(request.path.as_deref())?;
    interface::ensure_ai_project_config(&project_path, None)?;

    let prepared = if let Some(interface_name) = request.interface.as_deref() {
        vec![interface::prepare_ai_bundle(
            &project_path,
            interface_name,
            request.force,
        )?]
    } else {
        interface::ensure_ai_surface(interface::AiSurfaceRequest {
            project_root: &project_path,
            force: request.force,
            default_interface: None,
        })?
        .prepared
    };

    println!("Patina AI bundles refreshed.");
    for prepared in prepared {
        println!("  {}:", prepared.display_name);
        println!("    Context: {}", prepared.bootstrap.context_path.display());
        println!(
            "    Bootstrap: {}",
            prepared.bootstrap.bootstrap_path.display()
        );
        println!(
            "    Status: {}",
            if prepared.current {
                "already current"
            } else {
                "refreshed"
            }
        );
        if let Some(backup_snapshot) = prepared.bootstrap.backup_snapshot {
            println!("    Backup: {}", backup_snapshot.display());
        }
    }

    Ok(())
}

pub fn launch(request: AiLaunchRequest) -> Result<()> {
    ensure_workspace_ready()?;

    let mut project_path = launch_internal::resolve_project_path(request.path.as_deref())?;
    let interface_name = request.interface_name.to_ascii_lowercase();
    if !interface::is_supported_ai_interface(&interface_name) {
        bail!(
            "Unsupported Patina AI interface '{}'. Choose one of: {}.",
            interface_name,
            interface::supported_ai_interfaces().join(", ")
        );
    }

    let interface_info = patina::interface::launch::get(&interface_name)?;
    if !interface_info.detected {
        bail!(
            "Interface '{}' ({}) is not installed.",
            interface_name,
            interface_info.display
        );
    }

    if !project::is_patina_project(&project_path) {
        match launch_internal::prompt_are_you_lost(&project_path, Some(&interface_name))? {
            Some(_) => {
                project_path = launch_internal::resolve_project_path(Some(
                    project_path.to_string_lossy().as_ref(),
                ))?;
            }
            None => return Ok(()),
        }
    }

    match launch_internal::ensure_on_patina_branch()? {
        BranchAction::AlreadyOnPatina
        | BranchAction::Switched { .. }
        | BranchAction::StashedAndSwitched { .. }
        | BranchAction::Rebased { .. }
        | BranchAction::NotGitRepo
        | BranchAction::NoPatinaExists => {}
        BranchAction::RebaseConflicts => {
            bail!("Please resolve rebase conflicts before launching.");
        }
    }

    let config_result = interface::ensure_ai_project_config(
        &project_path,
        if request.set_default {
            Some(&interface_name)
        } else {
            None
        },
    )?;

    let bundle_before = interface::bundle_deployment_status(&interface_name, &project_path).ok();
    let tool_version_observed = interface::launch::detect_version(&interface_name);

    let (iface, bootstrap) =
        crate::commands::interface::ensure_ready(&interface_name, &project_path, false)?;

    let bundle_after = interface::bundle_deployment_status(&interface_name, &project_path).ok();

    if request.set_default || config_result.default_interface.is_empty() {
        interface::set_project_default_interface(&project_path, &interface_name)?;
    }

    let resolved_voice_uid = resolve_voice_uid(request.voice.as_deref(), &project_path);
    let requested_session = request.requested_session.clone();
    let requested_title = request.title.clone();

    let (checkin, envelope_id, tmux_lane) = resolve_checkin_via_mother(
        &project_path,
        &interface_name,
        requested_title,
        requested_session,
    )?;

    if !checkin.attached_existing {
        record_ai_session_started(&project_path, &interface_name, &checkin)?;
        if config_result.default_interface.is_empty() {
            interface::set_project_default_interface(&project_path, &interface_name)?;
        }
    }

    let decision_path = request
        .decision_path
        .as_deref()
        .unwrap_or("direct")
        .to_string();

    if let Err(error) =
        remember_project_last_interface(&project_path, &interface_name, &decision_path)
    {
        eprintln!(
            "[ai] warning: failed to persist last interface '{}' for project '{}': {}",
            interface_name,
            project_path.display(),
            error
        );
    }

    if let Err(error) = record_interface_launch_decision(
        &project_path,
        &interface_name,
        &checkin,
        &decision_path,
        bundle_before.as_ref(),
        bundle_after.as_ref(),
        tool_version_observed.as_deref(),
    ) {
        eprintln!(
            "[ai] warning: failed to write interface launch event for '{}' in '{}': {}",
            interface_name,
            project_path.display(),
            error
        );
    }

    println!(
        "Patina AI {} {}",
        if checkin.attached_existing {
            "attached to"
        } else {
            "started"
        },
        checkin.session_file_id
    );
    println!("  Interface: {}", iface.display_name());
    println!("  Context: {}", bootstrap.context_path.display());
    println!("  Bootstrap: {}", bootstrap.bootstrap_path.display());
    if let Some(backup_snapshot) = &bootstrap.backup_snapshot {
        println!("  Backup: {}", backup_snapshot.display());
    }
    println!("  Artifact: {}", checkin.artifact_path.display());

    let mut env = vec![
        (
            "PATINA_SESSION_RUNTIME_ID".to_string(),
            checkin.session_runtime_id.clone(),
        ),
        (
            "PATINA_SESSION_ID".to_string(),
            checkin.session_file_id.clone(),
        ),
        ("PATINA_AI_INTERFACE".to_string(), interface_name.clone()),
        (
            "PATINA_SESSION_ARTIFACT".to_string(),
            checkin.artifact_path.display().to_string(),
        ),
    ];
    if let Some(voice_uid) = checkin.voice_uid.as_ref().or(resolved_voice_uid.as_ref()) {
        env.push(("PATINA_VOICE_UID".to_string(), voice_uid.clone()));
    }
    env.push(("PATINA_ENVELOPE_ID".to_string(), envelope_id));
    env.push(("PATINA_TMUX_LANE".to_string(), tmux_lane));

    let bundle = interface::interface_bundle(&interface_name)?;

    let tmux_mode = if request.no_tmux {
        interface::TmuxLaunchMode::Off
    } else if request.tmux {
        interface::TmuxLaunchMode::Force
    } else {
        match bundle.tmux_policy {
            interface::BundleTmuxPolicy::Auto => interface::TmuxLaunchMode::Auto,
        }
    };
    let tmux_session_name = Some(interface::derive_interface_session_name(
        &project_path,
        &interface_name,
        Some(&checkin.session_file_id),
    ));

    iface.launch(interface::LaunchRequest {
        project_root: project_path,
        env,
        tmux_mode,
        tmux_session_name,
    })
}

trait MotherControlClient {
    fn ready(&self) -> Result<()>;
    fn interface_control_call(
        &self,
        operation_id: &str,
        args: serde_json::Value,
        correlation: Option<serde_json::Value>,
    ) -> Result<serde_json::Value>;
}

impl MotherControlClient for patina::mother::Client {
    fn ready(&self) -> Result<()> {
        patina::mother::Client::ready(self)
    }

    fn interface_control_call(
        &self,
        operation_id: &str,
        args: serde_json::Value,
        correlation: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        patina::mother::Client::interface_control_call(self, operation_id, args, correlation)
    }
}

fn resolve_checkin_via_mother(
    project_root: &std::path::Path,
    interface_name: &str,
    title: Option<String>,
    requested_session: Option<String>,
) -> Result<(interface::CheckInResult, String, String)> {
    let client = patina::mother::control_plane_client();
    resolve_checkin_via_mother_with_client(
        project_root,
        interface_name,
        title,
        requested_session,
        &client,
    )
}

fn resolve_checkin_via_mother_with_client(
    project_root: &std::path::Path,
    interface_name: &str,
    title: Option<String>,
    requested_session: Option<String>,
    client: &dyn MotherControlClient,
) -> Result<(interface::CheckInResult, String, String)> {
    client.ready().map_err(|error| {
        anyhow!(
            "Mother daemon required for HITL launch (ready probe failed). Start with `patina mother start` and retry: {}",
            error
        )
    })?;

    let project_uid = project::create_uid_if_missing(project_root)?;
    let launch_id = uuid::Uuid::new_v4().to_string();
    let correlation = serde_json::json!({
        "project_uid": project_uid,
        "interface": interface_name,
        "launch_id": launch_id,
    });

    let protocol = format!(
        "{}.{}",
        patina_protocol::CURRENT_PROTOCOL_VERSION.major,
        patina_protocol::CURRENT_PROTOCOL_VERSION.minor
    );
    let launch_intent = if requested_session.is_some() {
        "attach-only"
    } else {
        "attach-or-create"
    };

    let handshake = client.interface_control_call(
        "patina:interface/handshake.v1",
        serde_json::json!({
            "protocol_version": protocol,
            "cli_version": env!("CARGO_PKG_VERSION"),
            "project_uid": project_uid,
            "project_root": project_root.display().to_string(),
            "interface_name": interface_name,
            "interface_kind": "hitl",
            "launch_intent": launch_intent,
            "requested_session": requested_session,
            "tty": std::io::stdin().is_terminal() && std::io::stdout().is_terminal(),
        }),
        Some(correlation.clone()),
    )?;

    let handshake_id = handshake
        .get("result")
        .and_then(|result| result.get("handshake_id"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow!("Mother handshake response missing result.handshake_id"))?
        .to_string();

    let resolve = client.interface_control_call(
        "patina:interface/envelope.resolve.v1",
        serde_json::json!({
            "handshake_id": handshake_id,
            "title": title,
        }),
        Some(correlation),
    )?;

    let result = resolve
        .get("result")
        .ok_or_else(|| anyhow!("Mother resolve response missing result payload"))?;

    let decision = result
        .get("decision")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow!("Mother resolve response missing result.decision"))?;

    match decision {
        "attach" | "create" => {
            let identity = result
                .get("identity")
                .ok_or_else(|| anyhow!("Mother resolve response missing result.identity"))?;
            let envelope_id = identity
                .get("envelope_id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| anyhow!("Mother resolve response missing identity.envelope_id"))?
                .to_string();
            let runtime_id = identity
                .get("session_runtime_id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| {
                    anyhow!("Mother resolve response missing identity.session_runtime_id")
                })?
                .to_string();
            let file_id = identity
                .get("session_file_id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| anyhow!("Mother resolve response missing identity.session_file_id"))?
                .to_string();
            let tmux_lane = identity
                .get("tmux_lane")
                .and_then(|value| value.as_str())
                .unwrap_or(interface_name)
                .to_string();

            let handle = match patina::session::load_session(project_root, &runtime_id)? {
                Some(handle) => Some(handle),
                None => patina::session::load_session_by_file_id(project_root, &file_id)?,
            }
            .ok_or_else(|| {
                anyhow!(
                    "Mother resolved session '{}', but no local session artifact was found",
                    runtime_id
                )
            })?;

            let checkin = interface::CheckInResult {
                voice_uid: handle.voice_uid,
                session_runtime_id: runtime_id,
                session_file_id: file_id,
                artifact_path: handle.artifact_path,
                attached_existing: decision == "attach",
            };

            Ok((checkin, envelope_id, tmux_lane))
        }
        "choose" => {
            let choices = result
                .get("choices")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default();
            let formatted = choices
                .iter()
                .filter_map(|entry| {
                    let file_id = entry
                        .get("session_file_id")
                        .and_then(|value| value.as_str())?;
                    let runtime_id = entry
                        .get("session_runtime_id")
                        .and_then(|value| value.as_str())
                        .unwrap_or("unknown");
                    Some(format!("{} ({})", file_id, runtime_id))
                })
                .collect::<Vec<_>>()
                .join(", ");
            bail!(
                "Multiple active {} sessions exist. Use `patina ai list` and retry with `--session <id>`.\nChoices: {}",
                interface_name,
                formatted
            );
        }
        "reject" => {
            let reason = result
                .get("reason")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown rejection");
            bail!("Mother rejected HITL envelope resolution: {}", reason);
        }
        other => {
            bail!(
                "Mother resolve returned unsupported decision '{}'. Expected attach|create|choose|reject.",
                other
            );
        }
    }
}

fn ensure_workspace_ready() -> Result<()> {
    if workspace::is_first_run() {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!(" Welcome to Patina AI");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        workspace::setup()?;
        println!();
    }

    Ok(())
}

fn record_ai_session_started(
    project_root: &std::path::Path,
    interface_name: &str,
    checkin: &interface::CheckInResult,
) -> Result<()> {
    let conn = patina::eventlog::open_events_db_at(project_root)?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let payload = serde_json::json!({
        "session_id": checkin.session_file_id,
        "runtime_id": checkin.session_runtime_id,
        "voice_uid": checkin.voice_uid,
        "interface": interface_name,
        "artifact": checkin.artifact_path,
        "attached_existing": false,
    });
    patina::eventlog::insert_event(
        &conn,
        "session.started",
        &timestamp,
        &checkin.session_file_id,
        Some(&checkin.artifact_path.display().to_string()),
        &payload.to_string(),
    )?;
    Ok(())
}

fn remember_project_last_interface(
    project_root: &std::path::Path,
    interface_name: &str,
    launch_mode: &str,
) -> Result<()> {
    let project_uid = project::create_uid_if_missing(project_root)?;
    let key_prefix = format!("project:{}", project_uid);
    let runtime = patina::mother::MotherRuntimeStore::default();

    runtime.put_state(
        "interface-launch",
        &format!("{}:last_interface", key_prefix),
        interface_name,
    )?;
    runtime.put_state(
        "interface-launch",
        &format!("{}:last_launch_at", key_prefix),
        &chrono::Utc::now().to_rfc3339(),
    )?;
    runtime.put_state(
        "interface-launch",
        &format!("{}:last_launch_mode", key_prefix),
        launch_mode,
    )?;

    Ok(())
}

fn derive_bundle_action(
    before: Option<&interface::BundleDeploymentStatus>,
    after: Option<&interface::BundleDeploymentStatus>,
) -> &'static str {
    let Some(after) = after else {
        return "unknown";
    };

    let Some(before) = before else {
        return if after.current { "prepare" } else { "unknown" };
    };

    if before.current && after.current {
        return "noop";
    }

    if !before.current && after.current {
        if !before.stale_paths.is_empty() {
            return "refresh";
        }
        return "prepare";
    }

    "unknown"
}

fn record_interface_launch_decision(
    project_root: &std::path::Path,
    interface_name: &str,
    checkin: &interface::CheckInResult,
    decision_path: &str,
    bundle_before: Option<&interface::BundleDeploymentStatus>,
    bundle_after: Option<&interface::BundleDeploymentStatus>,
    tool_version_observed: Option<&str>,
) -> Result<()> {
    let conn = patina::eventlog::open_events_db_at(project_root)?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let payload = serde_json::json!({
        "project_uid": project::get_uid(project_root),
        "interface": interface_name,
        "session_id": checkin.session_file_id,
        "runtime_id": checkin.session_runtime_id,
        "decision_path": decision_path,
        "bundle_version_before": bundle_before.map(|status| status.expected_version.as_str()),
        "bundle_version_after": bundle_after.map(|status| status.expected_version.as_str()),
        "bundle_current_before": bundle_before.map(|status| status.current),
        "bundle_current_after": bundle_after.map(|status| status.current),
        "tool_version_observed": tool_version_observed,
        "action": derive_bundle_action(bundle_before, bundle_after),
        "attached_existing": checkin.attached_existing,
        "artifact": checkin.artifact_path,
    });

    patina::eventlog::insert_event(
        &conn,
        "interface.launch",
        &timestamp,
        &checkin.session_file_id,
        Some(&checkin.artifact_path.display().to_string()),
        &payload.to_string(),
    )?;

    Ok(())
}

fn resolve_voice_uid(explicit: Option<&str>, project_root: &std::path::Path) -> Option<String> {
    let old_voice_binding = project_root.join(".patina/persona");
    let new_voice_binding = project::voice_path(project_root);
    if old_voice_binding.exists() && !new_voice_binding.exists() {
        if let Err(error) = std::fs::rename(&old_voice_binding, &new_voice_binding) {
            eprintln!(
                "[ai] warning: failed to migrate project voice binding '{}' -> '{}': {}",
                old_voice_binding.display(),
                new_voice_binding.display(),
                error
            );
        }
    }

    // Launch-time voice scope precedence:
    // 1) explicit CLI flag, 2) project binding `.patina/voice`, 3) none.
    let resolved = explicit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| project::get_voice(project_root));

    if let Some(voice_uid) = resolved.as_deref() {
        let old_voice_dir = patina::paths::patina_home()
            .join("mother")
            .join("persona")
            .join(voice_uid);
        let new_voice_dir = patina::paths::mother::voice::voice_dir(voice_uid).ok();

        if let Some(new_voice_dir) = new_voice_dir {
            if old_voice_dir.exists() && !new_voice_dir.exists() {
                if let Some(parent) = new_voice_dir.parent() {
                    if let Err(error) = std::fs::create_dir_all(parent) {
                        eprintln!(
                            "[ai] warning: failed to prepare voice dir '{}' for migration: {}",
                            parent.display(),
                            error
                        );
                    }
                }
                if let Err(error) = std::fs::rename(&old_voice_dir, &new_voice_dir) {
                    eprintln!(
                        "[ai] warning: failed to migrate voice store '{}' -> '{}': {}",
                        old_voice_dir.display(),
                        new_voice_dir.display(),
                        error
                    );
                }
            } else if old_voice_dir.exists() && new_voice_dir.exists() {
                eprintln!(
                    "[ai] warning: both legacy and current voice stores exist for '{}'; preferring '{}'",
                    voice_uid,
                    new_voice_dir.display()
                );
            }
        }
    }

    resolved
}

#[cfg(test)]
mod tests {
    use super::*;
    use patina::project::{self, ProjectConfig};
    use std::cell::RefCell;
    use std::fs;
    use tempfile::TempDir;

    fn setup_project() -> TempDir {
        let temp = TempDir::new().unwrap();
        let mut config = ProjectConfig::with_name("patina");
        config.interfaces.allowed = vec![];
        config.interfaces.default = String::new();
        project::save(temp.path(), &config).unwrap();
        temp
    }

    fn with_temp_env<T>(temp: &TempDir, f: impl FnOnce() -> T) -> T {
        let _guard = patina::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let patina_home = temp.path().join("patina-home");
        fs::create_dir_all(&patina_home).unwrap();

        let old_dir = std::env::current_dir().ok();
        let old_patina_home = std::env::var_os("PATINA_HOME");
        std::env::set_current_dir(temp.path()).unwrap();
        unsafe {
            std::env::set_var("PATINA_HOME", &patina_home);
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        if let Some(path) = old_dir {
            let _ = std::env::set_current_dir(path);
        }
        match old_patina_home {
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

    #[test]
    fn setup_accepts_legacy_interface_argument_and_sets_default() {
        let temp = setup_project();

        with_temp_env(&temp, || {
            setup(AiSetupRequest {
                interface: Some("gemini".to_string()),
                path: Some(temp.path().display().to_string()),
                force: false,
                all: false,
            })
            .unwrap();
        });

        let config = project::load_with_migration(temp.path()).unwrap();
        assert_eq!(config.interfaces.default, "gemini");
        assert!(temp.path().join(".gemini").exists());
    }

    #[test]
    fn setup_defaults_to_project_default_interface_only() {
        let temp = setup_project();
        let mut config = project::load_with_migration(temp.path()).unwrap();
        config.interfaces.default = "gemini".to_string();
        project::save(temp.path(), &config).unwrap();

        with_temp_env(&temp, || {
            setup(AiSetupRequest {
                interface: None,
                path: Some(temp.path().display().to_string()),
                force: false,
                all: false,
            })
            .unwrap();
        });

        assert!(temp.path().join(".gemini").exists());
        assert!(!temp.path().join(".claude").exists());
        assert!(!temp.path().join(".opencode").exists());
    }

    #[test]
    fn setup_all_prewarms_all_interfaces() {
        let temp = setup_project();

        with_temp_env(&temp, || {
            setup(AiSetupRequest {
                interface: None,
                path: Some(temp.path().display().to_string()),
                force: false,
                all: true,
            })
            .unwrap();
        });

        assert!(temp.path().join(".claude").exists());
        assert!(temp.path().join(".opencode").exists());
        assert!(temp.path().join(".gemini").exists());
        assert!(temp.path().join(".pi").exists());
    }

    #[test]
    fn refresh_can_target_single_bundle_without_changing_default() {
        let temp = setup_project();

        with_temp_env(&temp, || {
            setup(AiSetupRequest {
                interface: Some("claude".to_string()),
                path: Some(temp.path().display().to_string()),
                force: false,
                all: false,
            })
            .unwrap();
        });

        with_temp_env(&temp, || {
            refresh(AiRefreshRequest {
                interface: Some("gemini".to_string()),
                path: Some(temp.path().display().to_string()),
                force: false,
            })
            .unwrap();
        });

        let config = project::load_with_migration(temp.path()).unwrap();
        assert_eq!(config.interfaces.default, "claude");
        assert!(temp.path().join(".gemini/commands/spec.toml").exists());
    }

    #[test]
    fn resolve_voice_uid_prefers_explicit_over_project_binding() {
        let temp = setup_project();
        let voice_path = project::voice_path(temp.path());
        fs::write(&voice_path, "voice-project\n").unwrap();

        let resolved = resolve_voice_uid(Some("voice-cli"), temp.path());
        assert_eq!(resolved.as_deref(), Some("voice-cli"));
    }

    #[test]
    fn resolve_voice_uid_falls_back_to_project_binding() {
        let temp = setup_project();
        let voice_path = project::voice_path(temp.path());
        fs::write(&voice_path, "voice-project\n").unwrap();

        let resolved = resolve_voice_uid(None, temp.path());
        assert_eq!(resolved.as_deref(), Some("voice-project"));
    }

    #[test]
    fn resolve_voice_uid_migrates_legacy_project_binding() {
        let temp = setup_project();
        fs::write(temp.path().join(".patina/persona"), "voice-project\n").unwrap();

        let resolved = resolve_voice_uid(None, temp.path());
        assert_eq!(resolved.as_deref(), Some("voice-project"));
        assert!(project::voice_path(temp.path()).exists());
        assert!(!temp.path().join(".patina/persona").exists());
    }

    struct MockMotherControlClient {
        ready_result: Result<()>,
        responses: RefCell<Vec<serde_json::Value>>,
    }

    impl MotherControlClient for MockMotherControlClient {
        fn ready(&self) -> Result<()> {
            self.ready_result
                .as_ref()
                .map(|_| ())
                .map_err(|error| anyhow::anyhow!(error.to_string()))
        }

        fn interface_control_call(
            &self,
            _operation_id: &str,
            _args: serde_json::Value,
            _correlation: Option<serde_json::Value>,
        ) -> Result<serde_json::Value> {
            Ok(self.responses.borrow_mut().remove(0))
        }
    }

    fn create_live_session(
        temp: &TempDir,
        interface_name: &str,
    ) -> patina::session::LiveSessionHandle {
        patina::session::begin_session(
            temp.path(),
            patina::session::BeginSessionRequest {
                title: format!("{} session", interface_name),
                interface_name: interface_name.to_string(),
                interface_kind: patina::session::InterfaceKind::from_interface_name(interface_name),
                voice_uid: None,
                work_spec: None,
                continuity_uid: None,
                takeover_from_runtime: None,
                takeover_user_verified: None,
                parent_runtime_id: None,
                handoff_from_runtime_id: None,
                participant: None,
            },
        )
        .expect("create live session")
        .handle
    }

    #[test]
    fn resolve_checkin_via_mother_ready_failure_is_actionable() {
        let temp = setup_project();
        with_temp_env(&temp, || {
            let client = MockMotherControlClient {
                ready_result: Err(anyhow::anyhow!("not ready")),
                responses: RefCell::new(vec![]),
            };
            let error =
                resolve_checkin_via_mother_with_client(temp.path(), "pi", None, None, &client)
                    .unwrap_err();

            assert!(error
                .to_string()
                .contains("Mother daemon required for HITL launch"));
        });
    }

    #[test]
    fn resolve_checkin_via_mother_attach_branch_returns_existing_handle() {
        let temp = setup_project();
        with_temp_env(&temp, || {
            let live = create_live_session(&temp, "pi");
            let client = MockMotherControlClient {
                ready_result: Ok(()),
                responses: RefCell::new(vec![
                    serde_json::json!({
                        "result": { "handshake_id": "hs-1" }
                    }),
                    serde_json::json!({
                        "result": {
                            "decision": "attach",
                            "identity": {
                                "envelope_id": "env-1",
                                "session_runtime_id": live.runtime_id,
                                "session_file_id": live.file_id,
                                "tmux_lane": "pi",
                                "interface_name": "pi",
                                "project_uid": project::create_uid_if_missing(temp.path()).unwrap(),
                            }
                        }
                    }),
                ]),
            };

            let (checkin, envelope, lane) =
                resolve_checkin_via_mother_with_client(temp.path(), "pi", None, None, &client)
                    .expect("attach branch");

            assert!(checkin.attached_existing);
            assert_eq!(envelope, "env-1");
            assert_eq!(lane, "pi");
        });
    }

    #[test]
    fn resolve_checkin_via_mother_create_branch_sets_attached_false() {
        let temp = setup_project();
        with_temp_env(&temp, || {
            let live = create_live_session(&temp, "pi");
            let client = MockMotherControlClient {
                ready_result: Ok(()),
                responses: RefCell::new(vec![
                    serde_json::json!({
                        "result": { "handshake_id": "hs-2" }
                    }),
                    serde_json::json!({
                        "result": {
                            "decision": "create",
                            "identity": {
                                "envelope_id": "env-2",
                                "session_runtime_id": live.runtime_id,
                                "session_file_id": live.file_id,
                                "tmux_lane": "pi",
                                "interface_name": "pi",
                                "project_uid": project::create_uid_if_missing(temp.path()).unwrap(),
                            }
                        }
                    }),
                ]),
            };

            let (checkin, _, _) =
                resolve_checkin_via_mother_with_client(temp.path(), "pi", None, None, &client)
                    .expect("create branch");

            assert!(!checkin.attached_existing);
        });
    }

    #[test]
    fn resolve_checkin_via_mother_choose_branch_errors_with_choices() {
        let temp = setup_project();
        with_temp_env(&temp, || {
            let client = MockMotherControlClient {
                ready_result: Ok(()),
                responses: RefCell::new(vec![
                    serde_json::json!({
                        "result": { "handshake_id": "hs-3" }
                    }),
                    serde_json::json!({
                        "result": {
                            "decision": "choose",
                            "choices": [
                                {
                                    "session_file_id": "20260416-1",
                                    "session_runtime_id": "runtime-1"
                                }
                            ]
                        }
                    }),
                ]),
            };

            let error =
                resolve_checkin_via_mother_with_client(temp.path(), "pi", None, None, &client)
                    .unwrap_err();

            assert!(error
                .to_string()
                .contains("Multiple active pi sessions exist"));
        });
    }

    #[test]
    fn resolve_checkin_via_mother_reject_branch_surfaces_reason() {
        let temp = setup_project();
        with_temp_env(&temp, || {
            let client = MockMotherControlClient {
                ready_result: Ok(()),
                responses: RefCell::new(vec![
                    serde_json::json!({
                        "result": { "handshake_id": "hs-4" }
                    }),
                    serde_json::json!({
                        "result": {
                            "decision": "reject",
                            "reason": "policy denied"
                        }
                    }),
                ]),
            };

            let error =
                resolve_checkin_via_mother_with_client(temp.path(), "pi", None, None, &client)
                    .unwrap_err();

            assert!(error
                .to_string()
                .contains("Mother rejected HITL envelope resolution: policy denied"));
        });
    }

    #[test]
    fn launch_explicit_interface_hard_fails_when_detect_missing() {
        let temp = setup_project();

        with_temp_env(&temp, || {
            let old_path = std::env::var_os("PATH");
            unsafe {
                std::env::set_var("PATH", "/definitely/missing/path");
            }

            let result = launch(AiLaunchRequest {
                interface_name: "claude".to_string(),
                title: None,
                requested_session: None,
                voice: None,
                path: Some(temp.path().display().to_string()),
                set_default: false,
                tmux: false,
                no_tmux: false,
                decision_path: Some("direct".to_string()),
            });

            assert!(result.is_err());
            let message = result.unwrap_err().to_string();
            assert!(message.contains("Interface 'claude' (Claude Code) is not installed"));

            match old_path {
                Some(value) => unsafe {
                    std::env::set_var("PATH", value);
                },
                None => unsafe {
                    std::env::remove_var("PATH");
                },
            }
        });
    }
}
