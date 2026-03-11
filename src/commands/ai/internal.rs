use anyhow::{bail, Result};
use serde_json::json;
use std::fs;
use std::io::IsTerminal;
use std::path::Path;

use patina::interface::{
    self, adapter as load_adapter, check_in, CheckInResult, InterfaceCheckIn, LaunchPolicy,
};
use patina::project;
use patina::session::{self, ArchiveSessionRequest, SessionManager};
use patina::{git, workspace};

use super::AiSessionCommands;
use crate::commands::launch::internal::{self as launch_internal, BranchAction};

pub fn launch_default() -> Result<()> {
    let project_root = launch_internal::resolve_project_path(None)?;
    let adapter_name = preferred_adapter(&project_root)?;
    launch(adapter_name, None, None, None, None, false)
}

pub fn launch(
    adapter_name: &str,
    title: Option<String>,
    requested_session: Option<String>,
    persona: Option<String>,
    path: Option<String>,
    no_tmux: bool,
) -> Result<()> {
    ensure_workspace_ready()?;

    let mut project_path = launch_internal::resolve_project_path(path.as_deref())?;
    let adapter_info = patina::adapters::launch::get(adapter_name)?;
    if !adapter_info.detected {
        bail!(
            "Adapter '{}' ({}) is not installed.",
            adapter_name,
            adapter_info.display
        );
    }

    if let Err(error) = launch_internal::ensure_mother_running() {
        eprintln!(
            "Warning: Mother daemon unavailable ({}). Continuing with local Mother runtime check-in.",
            error
        );
    }

    if !project_path.join(".patina").exists() {
        match launch_internal::prompt_are_you_lost(&project_path, Some(adapter_name))? {
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

    let (adapter, bootstrap) =
        crate::commands::interface::ensure_ready(adapter_name, &project_path, false)?;
    let checkin = check_in(&InterfaceCheckIn {
        interface_kind: adapter.interface_kind(),
        adapter_name: adapter_name.to_string(),
        project_root: project_path.clone(),
        project_uid: project::get_uid(&project_path),
        requested_persona: persona,
        requested_session,
        title,
        capabilities: adapter.capabilities(),
    })?;

    if !checkin.attached_existing {
        record_ai_session_started(&project_path, adapter_name, &checkin)?;
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
    println!("  Adapter: {}", adapter.display_name());
    println!("  Context: {}", bootstrap.context_path.display());
    println!("  Bootstrap: {}", bootstrap.bootstrap_path.display());
    if let Some(backup_snapshot) = &bootstrap.backup_snapshot {
        println!("  Backup: {}", backup_snapshot.display());
    }
    println!("  Artifact: {}", checkin.artifact_path.display());

    let env_disabled = std::env::var("PATINA_TMUX")
        .map(|value| value == "0")
        .unwrap_or(false);
    let is_tty = std::io::stdout().is_terminal();
    let inside_tmux = std::env::var_os("TMUX").is_some();
    let tmux_in_path = which::which("tmux").is_ok();
    let (tmux_version_ok, detected_version) = if tmux_in_path {
        interface::check_tmux_version()
    } else {
        (false, String::new())
    };
    let decision = interface::resolve_tmux_decision(
        no_tmux,
        env_disabled,
        is_tty,
        inside_tmux,
        tmux_in_path,
        tmux_version_ok,
    );

    match &decision {
        interface::TmuxDecision::Off(interface::OffReason::NotInPath) => {
            eprintln!(
                "Warning: tmux not found — launching {} directly",
                adapter.display_name()
            );
        }
        interface::TmuxDecision::Off(interface::OffReason::TmuxTooOld) => {
            eprintln!(
                "Warning: tmux {} too old (need ≥ 1.9) — launching {} directly",
                detected_version,
                adapter.display_name()
            );
        }
        _ => {}
    }

    let session_name = match &checkin.launch_policy {
        LaunchPolicy::TmuxPreferred { session_name } => session_name.clone(),
        LaunchPolicy::Direct => {
            interface::derive_interface_session_name(&project_path, adapter_name)
        }
    };
    let env = vec![
        (
            "PATINA_SESSION_RUNTIME_ID".to_string(),
            checkin.session_runtime_id.clone(),
        ),
        (
            "PATINA_SESSION_ID".to_string(),
            checkin.session_file_id.clone(),
        ),
        ("PATINA_AI_INTERFACE".to_string(), adapter_name.to_string()),
        (
            "PATINA_SESSION_ARTIFACT".to_string(),
            checkin.artifact_path.display().to_string(),
        ),
    ];

    adapter.launch(interface::LaunchRequest {
        project_root: project_path,
        tmux_decision: decision,
        session_name,
        env,
    })
}

pub fn list(json_output: bool) -> Result<()> {
    let project_root = SessionManager::find_project_root()?;
    let sessions = session::list_active_sessions(&project_root)?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &sessions
                    .iter()
                    .map(|session| {
                        json!({
                            "runtime_id": session.runtime_id,
                            "file_id": session.file_id,
                            "title": session.title,
                            "adapter": session.adapter_name,
                            "interface": session.interface_kind.as_str(),
                            "artifact": session.artifact_path,
                            "created_at": session.created_at,
                            "updated_at": session.updated_at,
                        })
                    })
                    .collect::<Vec<_>>()
            )?
        );
        return Ok(());
    }

    if sessions.is_empty() {
        println!("No active AI sessions.");
        return Ok(());
    }

    for session in sessions {
        println!(
            "{}  {}  {}  {}",
            session.file_id,
            session.adapter_name,
            session.title,
            session.artifact_path.display()
        );
    }

    Ok(())
}

pub fn session(command: AiSessionCommands) -> Result<()> {
    match command {
        AiSessionCommands::Start {
            title,
            adapter,
            json,
        } => start_session(&title, adapter, json),
        AiSessionCommands::Update { session, json } => update_session(session, json),
        AiSessionCommands::End {
            session,
            note,
            json,
        } => end_session(session, note, json),
        AiSessionCommands::List { json } => list(json),
    }
}

pub fn end(
    session_selector: Option<String>,
    note: Option<String>,
    json_output: bool,
) -> Result<()> {
    let project_root = SessionManager::find_project_root()?;
    let handle = match session_selector {
        Some(selector) => session::load_session(&project_root, &selector)?.or_else(|| {
            session::load_session_by_file_id(&project_root, &selector)
                .ok()
                .flatten()
        }),
        None => resolve_default_end_session(&project_root)?,
    }
    .ok_or_else(|| anyhow::anyhow!("No active AI session found"))?;

    let mut markdown = fs::read_to_string(&handle.artifact_path).unwrap_or_else(|_| {
        format!(
            "---\nid: {}\nruntime_id: {}\nstatus: active\n---\n\n## Outcome\n",
            handle.file_id, handle.runtime_id
        )
    });
    if let Some(note) = note {
        markdown = append_outcome_note(&markdown, &note);
    }

    let end_tag = format!("session-{}-{}-end", handle.file_id, handle.adapter_name);
    if git::is_git_repo().unwrap_or(false) && !git::tag_exists(&end_tag).unwrap_or(false) {
        let _ = git::create_tag(&end_tag, &format!("Session end: {}", handle.title));
    }

    session::archive_session(
        &project_root,
        ArchiveSessionRequest {
            runtime_id: handle.runtime_id.clone(),
            markdown,
            end_tag: Some(end_tag.clone()),
        },
    )?;
    record_ai_session_ended(&project_root, &handle, &end_tag)?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "runtime_id": handle.runtime_id,
                "file_id": handle.file_id,
                "end_tag": end_tag,
                "artifact": handle.artifact_path,
            }))?
        );
        return Ok(());
    }

    println!("Archived AI session {}", handle.file_id);
    println!("  Artifact: {}", handle.artifact_path.display());
    println!("  Tag: {}", end_tag);
    Ok(())
}

fn start_session(title: &str, adapter: Option<String>, json_output: bool) -> Result<()> {
    let project_root = SessionManager::find_project_root()?;
    let adapter_name = resolve_native_session_adapter(&project_root, adapter.as_deref())?;
    let result = crate::commands::session::start_session_value(
        &project_root,
        crate::commands::session::SessionStartRequest::native(title, &adapter_name),
    )?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    println!("Started native AI session {}", result.session_id);
    println!("  Adapter: {}", result.adapter);
    println!("  Artifact: {}", result.artifact_path);
    Ok(())
}

fn update_session(session_selector: Option<String>, json_output: bool) -> Result<()> {
    let project_root = SessionManager::find_project_root()?;
    let handle = crate::commands::session::resolve_live_session(
        &project_root,
        session_selector.as_deref(),
        current_interface_adapter().as_deref(),
    )?;
    let result = crate::commands::session::update_live_session_value(&project_root, &handle)?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    println!("Updated native AI session {}", result.session_id);
    println!("  Artifact: {}", result.artifact_path);
    Ok(())
}

fn end_session(
    session_selector: Option<String>,
    note: Option<String>,
    json_output: bool,
) -> Result<()> {
    let project_root = SessionManager::find_project_root()?;
    let handle = crate::commands::session::resolve_live_session(
        &project_root,
        session_selector.as_deref(),
        current_interface_adapter().as_deref(),
    )?;
    let result =
        crate::commands::session::end_live_session_value(&project_root, &handle, note.as_deref())?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    println!("Archived native AI session {}", result.session_id);
    println!("  Artifact: {}", result.artifact_path);
    println!("  Tag: {}", result.end_tag);
    Ok(())
}

fn append_outcome_note(markdown: &str, note: &str) -> String {
    if let Some((head, tail)) = markdown.split_once("## Outcome\n") {
        return format!("{head}## Outcome\n{note}\n\n{tail}");
    }
    format!("{markdown}\n\n## Outcome\n{note}\n")
}

fn record_ai_session_started(
    project_root: &Path,
    adapter_name: &str,
    checkin: &CheckInResult,
) -> Result<()> {
    let conn = patina::eventlog::open_events_db_at(project_root)?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let payload = json!({
        "session_id": checkin.session_file_id,
        "runtime_id": checkin.session_runtime_id,
        "adapter": adapter_name,
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

fn record_ai_session_ended(
    project_root: &Path,
    handle: &session::LiveSessionHandle,
    end_tag: &str,
) -> Result<()> {
    let conn = patina::eventlog::open_events_db_at(project_root)?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    let payload = json!({
        "session_id": handle.file_id,
        "runtime_id": handle.runtime_id,
        "adapter": handle.adapter_name,
        "end_tag": end_tag,
        "artifact": handle.artifact_path,
    });
    patina::eventlog::insert_event(
        &conn,
        "session.ended",
        &timestamp,
        &handle.file_id,
        Some(&handle.artifact_path.display().to_string()),
        &payload.to_string(),
    )?;
    Ok(())
}

fn preferred_adapter(project_root: &std::path::Path) -> Result<&'static str> {
    let config = project::load_with_migration(project_root)?;
    for candidate in [config.adapters.default.as_str(), "opencode", "gemini"] {
        if !matches!(candidate, "opencode" | "gemini") {
            continue;
        }
        if !config
            .adapters
            .allowed
            .iter()
            .any(|allowed| allowed == candidate)
        {
            continue;
        }
        let info = patina::adapters::launch::get(candidate)?;
        if info.detected {
            return Ok(if candidate == "opencode" {
                "opencode"
            } else {
                "gemini"
            });
        }
    }
    bail!(
        "No supported `patina ai` adapter is both allowed for this project and installed. Allowed adapters: {:?}",
        config.adapters.allowed
    );
}

fn resolve_native_session_adapter(project_root: &Path, adapter: Option<&str>) -> Result<String> {
    let resolved = adapter
        .map(ToOwned::to_owned)
        .or_else(current_interface_adapter)
        .unwrap_or(preferred_adapter(project_root)?.to_string());
    ensure_native_adapter_allowed(project_root, &resolved)?;
    let _ = load_adapter(&resolved).map_err(|_| {
        anyhow::anyhow!(
            "Adapter '{}' does not use the native `patina ai` session path.\nChoose one of: opencode, gemini.",
            resolved
        )
    })?;
    Ok(resolved)
}

fn current_interface_adapter() -> Option<String> {
    std::env::var("PATINA_AI_INTERFACE")
        .ok()
        .filter(|value| !value.is_empty())
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

fn ensure_native_adapter_allowed(project_path: &Path, adapter_name: &str) -> Result<()> {
    let config = project::load_with_migration(project_path)?;
    if !config.adapters.allowed.contains(&adapter_name.to_string()) {
        bail!(
            "Adapter '{}' is not allowed for this project.\nAllowed: {:?}\n\nTo add it, run: patina adapter add {}",
            adapter_name,
            config.adapters.allowed,
            adapter_name
        );
    }

    Ok(())
}

fn resolve_default_end_session(
    project_root: &std::path::Path,
) -> Result<Option<session::LiveSessionHandle>> {
    let active = session::list_active_sessions(project_root)?;
    choose_single_session(active)
}

fn choose_single_session(
    mut active: Vec<session::LiveSessionHandle>,
) -> Result<Option<session::LiveSessionHandle>> {
    match active.len() {
        0 => Ok(None),
        1 => Ok(active.pop()),
        _ => {
            let choices = active
                .iter()
                .map(|handle| {
                    format!(
                        "{}:{} ({})",
                        handle.adapter_name, handle.file_id, handle.title
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            bail!(
                "Multiple active AI sessions exist. Use `patina ai list` and retry with `patina ai end --session <id>`.\nChoices: {}",
                choices
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use patina::project::{self, ProjectConfig};
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn handle(file_id: &str, adapter_name: &str) -> session::LiveSessionHandle {
        session::LiveSessionHandle {
            runtime_id: format!("runtime-{file_id}"),
            file_id: file_id.to_string(),
            title: format!("{adapter_name} session"),
            adapter_name: adapter_name.to_string(),
            interface_kind: session::InterfaceKind::from_adapter_name(adapter_name),
            persona_uid: None,
            artifact_path: PathBuf::from(format!("/tmp/{file_id}.md")),
            branch: "patina".to_string(),
            starting_commit: "deadbeef".to_string(),
            start_tag: format!("session-{file_id}-{adapter_name}-start"),
            end_tag: None,
            created_at: "2026-03-11T00:00:00Z".to_string(),
            updated_at: "2026-03-11T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn choose_single_session_returns_only_active_session() {
        let selected = choose_single_session(vec![handle("20260311-100000-AAAA", "opencode")])
            .unwrap()
            .unwrap();
        assert_eq!(selected.file_id, "20260311-100000-AAAA");
    }

    #[test]
    fn choose_single_session_rejects_ambiguous_end_without_selector() {
        let error = choose_single_session(vec![
            handle("20260311-100000-AAAA", "opencode"),
            handle("20260311-100001-BBBB", "gemini"),
        ])
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("Multiple active AI sessions exist"));
        assert!(error.to_string().contains("patina ai end --session <id>"));
    }

    fn setup_project(adapter_name: &str) -> TempDir {
        let temp = TempDir::new().unwrap();
        let mut config = ProjectConfig::with_name("patina");
        config.adapters.allowed = vec!["claude".to_string(), adapter_name.to_string()];
        config.adapters.default = adapter_name.to_string();
        project::save(temp.path(), &config).unwrap();
        temp
    }

    fn with_temp_patina_home<T>(temp: &TempDir, f: impl FnOnce() -> T) -> T {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let patina_home = temp.path().join("patina-home");
        std::fs::create_dir_all(&patina_home).unwrap();

        let old = std::env::var_os("PATINA_HOME");
        unsafe {
            std::env::set_var("PATINA_HOME", &patina_home);
        }
        let result = f();
        match old {
            Some(value) => unsafe {
                std::env::set_var("PATINA_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("PATINA_HOME");
            },
        }
        result
    }

    #[test]
    fn setup_native_interface_creates_projection_for_opencode() {
        let temp = setup_project("opencode");

        with_temp_patina_home(&temp, || {
            crate::commands::interface::setup(
                "opencode",
                Some(temp.path().display().to_string()),
                false,
            )
            .unwrap();
        });

        assert!(temp
            .path()
            .join(".opencode/commands/session-start.md")
            .exists());
        assert!(temp.path().join("AGENTS.md").exists());
        assert!(!temp.path().join("OPENCODE.md").exists());
        assert!(!temp.path().join(".opencode/PATINA.md").exists());
    }

    #[test]
    fn ai_setup_alias_creates_projection_for_opencode() {
        let temp = setup_project("opencode");

        with_temp_patina_home(&temp, || {
            crate::commands::ai::execute(Some(crate::commands::ai::AiCommands::Setup {
                adapter: "opencode".to_string(),
                path: Some(temp.path().display().to_string()),
                force: false,
            }))
            .unwrap();
        });

        assert!(temp.path().join("AGENTS.md").exists());
        assert!(temp
            .path()
            .join(".opencode/commands/session-start.md")
            .exists());
    }

    #[test]
    fn setup_rejects_claude_native_setup_path() {
        let temp = setup_project("claude");
        let error = with_temp_patina_home(&temp, || {
            crate::commands::interface::setup(
                "claude",
                Some(temp.path().display().to_string()),
                false,
            )
            .unwrap_err()
        });

        assert!(error
            .to_string()
            .contains("does not use the native Patina interface setup path"));
    }
}
