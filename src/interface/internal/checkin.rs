use anyhow::Result;
use std::path::PathBuf;

use crate::project;
use crate::session::{self, BeginSessionRequest, InterfaceKind, SessionParticipant};

use super::tmux::derive_interface_session_name;

#[derive(Debug, Clone)]
pub struct InterfaceCapabilities {
    pub tmux: bool,
    pub bootstrap: bool,
    pub durable_sessions: bool,
}

impl Default for InterfaceCapabilities {
    fn default() -> Self {
        Self {
            tmux: true,
            bootstrap: true,
            durable_sessions: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceCheckIn {
    pub interface_kind: InterfaceKind,
    pub adapter_name: String,
    pub project_root: PathBuf,
    pub project_uid: Option<String>,
    pub requested_persona: Option<String>,
    pub requested_session: Option<String>,
    pub title: Option<String>,
    pub capabilities: InterfaceCapabilities,
}

#[derive(Debug, Clone)]
pub enum LaunchPolicy {
    TmuxPreferred { session_name: String },
    Direct,
}

#[derive(Debug, Clone)]
pub struct CheckInResult {
    pub persona_uid: Option<String>,
    pub session_runtime_id: String,
    pub session_file_id: String,
    pub artifact_path: PathBuf,
    pub attached_existing: bool,
    pub launch_policy: LaunchPolicy,
}

pub fn check_in(request: &InterfaceCheckIn) -> Result<CheckInResult> {
    let _project_uid = match &request.project_uid {
        Some(uid) => uid.clone(),
        None => project::create_uid_if_missing(&request.project_root)?,
    };

    if let Some(selector) = &request.requested_session {
        if let Some(handle) = load_requested_session(&request.project_root, selector)? {
            return Ok(result_from_handle(request, handle, true));
        }
    }

    let active_interface_sessions = active_interface_sessions(request)?;
    if let Some(handle) = select_reusable_session(&request.adapter_name, active_interface_sessions)?
    {
        return Ok(result_from_handle(request, handle, true));
    }

    let title = request
        .title
        .clone()
        .unwrap_or_else(|| format!("{} session", request.adapter_name));
    let start = session::begin_session(
        &request.project_root,
        BeginSessionRequest {
            title,
            adapter_name: request.adapter_name.clone(),
            interface_kind: request.interface_kind,
            persona_uid: request.requested_persona.clone(),
            parent_runtime_id: None,
            handoff_from_runtime_id: None,
            participant: Some(SessionParticipant {
                participant_id: format!("{}-{}", request.adapter_name, std::process::id()),
                role: "interface".to_string(),
                interface_kind: request.interface_kind,
                adapter_name: Some(request.adapter_name.clone()),
                display_name: Some(request.adapter_name.clone()),
            }),
        },
    )?;

    Ok(result_from_handle(request, start.handle, false))
}

fn load_requested_session(
    project_root: &std::path::Path,
    selector: &str,
) -> Result<Option<session::LiveSessionHandle>> {
    if let Some(handle) = session::load_session(project_root, selector)? {
        return Ok(Some(handle));
    }
    session::load_session_by_file_id(project_root, selector)
}

fn result_from_handle(
    request: &InterfaceCheckIn,
    handle: session::LiveSessionHandle,
    attached_existing: bool,
) -> CheckInResult {
    let launch_policy = if request.capabilities.tmux {
        LaunchPolicy::TmuxPreferred {
            session_name: derive_interface_session_name(
                &request.project_root,
                &request.adapter_name,
            ),
        }
    } else {
        LaunchPolicy::Direct
    };

    CheckInResult {
        persona_uid: handle.persona_uid.clone(),
        session_runtime_id: handle.runtime_id,
        session_file_id: handle.file_id,
        artifact_path: handle.artifact_path,
        attached_existing,
        launch_policy,
    }
}

fn active_interface_sessions(
    request: &InterfaceCheckIn,
) -> Result<Vec<session::LiveSessionHandle>> {
    Ok(session::list_active_sessions(&request.project_root)?
        .into_iter()
        .filter(|handle| {
            handle.adapter_name == request.adapter_name
                && handle.interface_kind == request.interface_kind
        })
        .collect())
}

fn select_reusable_session(
    adapter_name: &str,
    mut sessions: Vec<session::LiveSessionHandle>,
) -> Result<Option<session::LiveSessionHandle>> {
    match sessions.len() {
        0 => Ok(None),
        1 => Ok(sessions.pop()),
        _ => {
            let choices = sessions
                .iter()
                .map(|handle| format!("{} ({})", handle.file_id, handle.title))
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!(
                "Multiple active {} sessions exist. Use `patina ai list` and retry with `--session <id>`.\nChoices: {}",
                adapter_name,
                choices
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn handle(
        file_id: &str,
        adapter_name: &str,
        interface_kind: InterfaceKind,
    ) -> session::LiveSessionHandle {
        session::LiveSessionHandle {
            runtime_id: format!("runtime-{file_id}"),
            file_id: file_id.to_string(),
            title: format!("{adapter_name} session"),
            adapter_name: adapter_name.to_string(),
            interface_kind,
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
    fn select_reusable_session_allows_same_interface_singleton() {
        let selected = select_reusable_session(
            "opencode",
            vec![handle(
                "20260311-100000-AAAA",
                "opencode",
                InterfaceKind::OpenCode,
            )],
        )
        .unwrap()
        .unwrap();

        assert_eq!(selected.adapter_name, "opencode");
        assert_eq!(selected.interface_kind, InterfaceKind::OpenCode);
    }

    #[test]
    fn select_reusable_session_rejects_ambiguous_same_interface_matches() {
        let error = select_reusable_session(
            "opencode",
            vec![
                handle("20260311-100000-AAAA", "opencode", InterfaceKind::OpenCode),
                handle("20260311-100001-BBBB", "opencode", InterfaceKind::OpenCode),
            ],
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("Multiple active opencode sessions exist"));
        assert!(error.to_string().contains("--session <id>"));
    }
}
