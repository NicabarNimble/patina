use anyhow::{bail, Result};

use patina::interface::{self, check_in, InterfaceCheckIn};
use patina::project;
use patina::workspace;

use crate::commands::launch::internal::{self as launch_internal, BranchAction};

#[derive(Debug, Clone)]
pub struct AiSetupRequest {
    pub interface: Option<String>,
    pub path: Option<String>,
    pub force: bool,
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
    pub persona: Option<String>,
    pub path: Option<String>,
    pub set_default: bool,
    pub tmux: bool,
    pub no_tmux: bool,
}

pub fn launch_default() -> Result<()> {
    let project_root = launch_internal::resolve_project_path(None)?;
    let interface_name = interface::resolve_preferred_ai_interface(&project_root)?;
    launch(AiLaunchRequest {
        interface_name,
        title: None,
        requested_session: None,
        persona: None,
        path: None,
        set_default: false,
        tmux: false,
        no_tmux: false,
    })
}

pub fn setup(request: AiSetupRequest) -> Result<()> {
    let project_path = launch_internal::resolve_project_path(request.path.as_deref())?;
    let result = interface::ensure_ai_surface(interface::AiSurfaceRequest {
        project_root: &project_path,
        force: request.force,
        default_interface: request.interface.as_deref(),
    })?;

    println!(
        "Patina AI bundles deployed. Default interface: {}",
        result.default_interface
    );
    for prepared in result.prepared {
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

    if let Err(error) = launch_internal::ensure_mother_running() {
        eprintln!(
            "Warning: Mother daemon unavailable ({}). Continuing with local Mother runtime check-in.",
            error
        );
    }

    if !project_path.join(".patina").exists() {
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

    let (adapter, bootstrap) =
        crate::commands::interface::ensure_ready(&interface_name, &project_path, false)?;

    if request.set_default || config_result.default_interface.is_empty() {
        interface::set_project_default_interface(&project_path, &interface_name)?;
    }

    let resolved_persona_uid = resolve_persona_uid(request.persona.as_deref(), &project_path);

    let checkin = check_in(&InterfaceCheckIn {
        interface_kind: adapter.interface_kind(),
        adapter_name: interface_name.clone(),
        project_root: project_path.clone(),
        project_uid: project::get_uid(&project_path),
        requested_persona: resolved_persona_uid.clone(),
        requested_session: request.requested_session,
        title: request.title,
        capabilities: adapter.capabilities(),
    })?;

    if !checkin.attached_existing {
        record_ai_session_started(&project_path, &interface_name, &checkin)?;
        if config_result.default_interface.is_empty() {
            interface::set_project_default_interface(&project_path, &interface_name)?;
        }
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
    println!("  Interface: {}", adapter.display_name());
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
    if let Some(persona_uid) = checkin.persona_uid.as_ref() {
        env.push(("PATINA_PERSONA_UID".to_string(), persona_uid.clone()));
    }

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
    ));

    adapter.launch(interface::LaunchRequest {
        project_root: project_path,
        env,
        tmux_mode,
        tmux_session_name,
    })
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
        "persona_uid": checkin.persona_uid,
        "adapter": interface_name,
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

fn resolve_persona_uid(explicit: Option<&str>, project_root: &std::path::Path) -> Option<String> {
    explicit
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| project::get_persona(project_root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use patina::project::{self, ProjectConfig};
    use std::fs;
    use tempfile::TempDir;

    fn setup_project() -> TempDir {
        let temp = TempDir::new().unwrap();
        let mut config = ProjectConfig::with_name("patina");
        config.adapters.allowed = vec![];
        config.adapters.default = String::new();
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
            })
            .unwrap();
        });

        let config = project::load_with_migration(temp.path()).unwrap();
        assert_eq!(config.adapters.default, "gemini");
        assert!(temp.path().join(".claude").exists());
        assert!(temp.path().join(".opencode").exists());
        assert!(temp.path().join(".gemini").exists());
    }

    #[test]
    fn refresh_can_target_single_bundle_without_changing_default() {
        let temp = setup_project();

        with_temp_env(&temp, || {
            setup(AiSetupRequest {
                interface: Some("claude".to_string()),
                path: Some(temp.path().display().to_string()),
                force: false,
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
        assert_eq!(config.adapters.default, "claude");
        assert!(temp.path().join(".gemini/commands/spec.toml").exists());
    }

    #[test]
    fn resolve_persona_uid_prefers_explicit_over_project_binding() {
        let temp = setup_project();
        let persona_path = project::persona_path(temp.path());
        fs::write(&persona_path, "persona-project\n").unwrap();

        let resolved = resolve_persona_uid(Some("persona-cli"), temp.path());
        assert_eq!(resolved.as_deref(), Some("persona-cli"));
    }

    #[test]
    fn resolve_persona_uid_falls_back_to_project_binding() {
        let temp = setup_project();
        let persona_path = project::persona_path(temp.path());
        fs::write(&persona_path, "persona-project\n").unwrap();

        let resolved = resolve_persona_uid(None, temp.path());
        assert_eq!(resolved.as_deref(), Some("persona-project"));
    }
}
