use anyhow::{bail, Result};
use serde_json::json;
use std::fs;
use std::path::Path;

use patina::interface::adapter as load_adapter;
use patina::session::{self, ArchiveSessionRequest, SessionManager};
use patina::git;

use super::AiSessionCommands;

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

    println!("Started AI session {}", result.session_id);
    println!("  Interface: {}", result.adapter);
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

    println!("Updated AI session {}", result.session_id);
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

    println!("Archived AI session {}", result.session_id);
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

fn resolve_native_session_adapter(project_root: &Path, adapter: Option<&str>) -> Result<String> {
    let resolved = adapter
        .map(ToOwned::to_owned)
        .or_else(current_interface_adapter)
        .unwrap_or(patina::interface::resolve_preferred_ai_interface(project_root)?);
    patina::interface::ensure_ai_project_config(project_root, None)?;
    let _ = load_adapter(&resolved).map_err(|_| {
        anyhow::anyhow!(
            "Unsupported Patina AI interface '{}'. Choose one of: {}.",
            resolved,
            patina::interface::supported_ai_interfaces().join(", ")
        )
    })?;
    Ok(resolved)
}

fn current_interface_adapter() -> Option<String> {
    std::env::var("PATINA_AI_INTERFACE")
        .ok()
        .filter(|value| !value.is_empty())
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
    use std::path::PathBuf;

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
}
