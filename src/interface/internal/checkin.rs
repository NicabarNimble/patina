use anyhow::Result;
use serde_json::json;
use std::path::PathBuf;

use crate::project;
use crate::session::{self, BeginSessionRequest, InterfaceKind, SessionParticipant};

#[derive(Debug, Clone)]
pub struct InterfaceCapabilities {
    pub bootstrap: bool,
    pub durable_sessions: bool,
}

impl Default for InterfaceCapabilities {
    fn default() -> Self {
        Self {
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
pub struct CheckInResult {
    pub persona_uid: Option<String>,
    pub session_runtime_id: String,
    pub session_file_id: String,
    pub artifact_path: PathBuf,
    pub attached_existing: bool,
}

pub fn check_in(request: &InterfaceCheckIn) -> Result<CheckInResult> {
    let _project_uid = match &request.project_uid {
        Some(uid) => uid.clone(),
        None => project::create_uid_if_missing(&request.project_root)?,
    };

    if let Some(selector) = &request.requested_session {
        if let Some(handle) = load_requested_session(&request.project_root, selector)? {
            if !persona_matches(
                request.requested_persona.as_deref(),
                handle.persona_uid.as_deref(),
            ) {
                anyhow::bail!(
                    "Requested session '{}' persona does not match requested persona scope",
                    selector
                );
            }
            return Ok(result_from_handle(request, handle, true));
        }
    }

    if let Some(handle) =
        session::load_current_interface_session(&request.project_root, &request.adapter_name)?
    {
        if persona_matches(
            request.requested_persona.as_deref(),
            handle.persona_uid.as_deref(),
        ) {
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
                display_name: std::env::var("USER")
                    .ok()
                    .or_else(|| std::env::var("LOGNAME").ok())
                    .or_else(|| Some(request.adapter_name.clone())),
            }),
        },
    )?;

    if let Err(error) = spawn_session_writer(&start.handle) {
        eprintln!(
            "[check-in] warning: failed to spawn session-writer for '{}': {}",
            start.handle.runtime_id, error
        );
    }

    Ok(result_from_handle(request, start.handle, false))
}

fn spawn_session_writer(handle: &session::LiveSessionHandle) -> Result<()> {
    let payload = json!({
        "session_runtime_id": handle.runtime_id,
        "session_file_id": handle.file_id,
        "artifact_path": handle.artifact_path,
        "branch": handle.branch,
        "start_tag": handle.start_tag,
    });
    let _ = session_writer_action(handle, "init-session", payload)?;
    Ok(())
}

pub fn session_writer_action(
    handle: &session::LiveSessionHandle,
    action: &str,
    payload: serde_json::Value,
) -> Result<Option<serde_json::Value>> {
    let Some(child) = load_session_writer_knowledge_child()? else {
        return Ok(None);
    };
    let response = child.handle(&crate::mother::ChildRequest {
        action: action.to_string(),
        payload,
    })?;

    let runtime = crate::mother::KnowledgeRuntimeStore::default();
    let key = format!("session:{}:child", handle.runtime_id);
    runtime.put_state(
        "session-writer",
        &key,
        &json!({
            "child": child.name(),
            "action": action,
            "response": response.payload,
            "artifact_path": handle.artifact_path,
        })
        .to_string(),
    )?;

    // keep latest pointer for operator visibility
    runtime.put_state(
        "session-writer",
        "latest-session-child",
        &json!({
            "runtime_id": handle.runtime_id,
            "file_id": handle.file_id,
            "action": action,
            "artifact_path": handle.artifact_path,
        })
        .to_string(),
    )?;

    Ok(Some(response.payload))
}

fn load_session_writer_knowledge_child() -> Result<Option<Box<dyn crate::mother::KnowledgeChild>>> {
    let engine = crate::child::engine::KnowledgeChildEngine::new()?;

    let mut candidates: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();
    let installed_dir = crate::paths::child::children_dir();
    if installed_dir.exists() {
        for entry in std::fs::read_dir(&installed_dir)? {
            let entry = entry?;
            let manifest_path = entry.path();
            if manifest_path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let wasm_path = manifest_path.with_extension("wasm");
            if wasm_path.exists() {
                candidates.push((wasm_path, manifest_path));
            }
        }
    }

    if let Some(manifest_path) = crate::child::engine::ChildManifest::resolve_child_manifest_path(
        std::path::Path::new("children/session-writer"),
    ) {
        candidates.push((
            std::path::PathBuf::from(
                "target/wasm32-wasip1/debug/patina_ai_child_session_writer.wasm",
            ),
            manifest_path.clone(),
        ));
        candidates.push((
            std::path::PathBuf::from(
                "target/wasm32-wasip1/release/patina_ai_child_session_writer.wasm",
            ),
            manifest_path,
        ));
    }

    for (wasm_path, manifest_path) in candidates {
        if !wasm_path.exists() || !manifest_path.exists() {
            continue;
        }
        let manifest = match crate::child::engine::ChildManifest::from_path(&manifest_path) {
            Ok(manifest) => manifest,
            Err(_) => continue,
        };
        if manifest.provides.child.as_deref() != Some("session-writer") {
            continue;
        }
        let wasm = std::fs::read(&wasm_path)?;
        let component = engine.load_component(&wasm)?;
        let child = engine.instantiate_child(&component, &manifest, None)?;
        return Ok(Some(child));
    }

    Ok(None)
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
    _request: &InterfaceCheckIn,
    handle: session::LiveSessionHandle,
    attached_existing: bool,
) -> CheckInResult {
    CheckInResult {
        persona_uid: handle.persona_uid.clone(),
        session_runtime_id: handle.runtime_id,
        session_file_id: handle.file_id,
        artifact_path: handle.artifact_path,
        attached_existing,
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
                && persona_matches(
                    request.requested_persona.as_deref(),
                    handle.persona_uid.as_deref(),
                )
        })
        .collect())
}

fn persona_matches(requested: Option<&str>, candidate: Option<&str>) -> bool {
    match requested {
        Some(expected) => candidate == Some(expected),
        None => true,
    }
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
    use crate::mother::{KnowledgeRuntimeStore, MotherSessionRecord, MotherSessionStatus};
    use std::fs;
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

    #[test]
    fn check_in_prefers_current_interface_pointer_over_ambiguous_matches() {
        let temp = tempfile::TempDir::new().unwrap();
        with_temp_patina_home(&temp, || {
            let project_uid = project::create_uid_if_missing(temp.path()).unwrap();
            let store = KnowledgeRuntimeStore::default();

            let first = MotherSessionRecord {
                runtime_id: "runtime-first".to_string(),
                project_uid: project_uid.clone(),
                file_id: "20260312-000230-AAAA".to_string(),
                title: "First OpenCode session".to_string(),
                persona_uid: None,
                status: MotherSessionStatus::Active,
                interface_kind: "opencode".to_string(),
                adapter_name: "opencode".to_string(),
                branch: Some("patina".to_string()),
                start_tag: Some("session-20260312-000230-AAAA-opencode-start".to_string()),
                end_tag: None,
                parent_runtime_id: None,
                handoff_from_runtime_id: None,
                created_at: "2026-03-12T00:02:30Z".to_string(),
                updated_at: "2026-03-12T00:02:30Z".to_string(),
            };
            let second = MotherSessionRecord {
                runtime_id: "runtime-second".to_string(),
                project_uid,
                file_id: "20260312-000231-BBBB".to_string(),
                title: "Second OpenCode session".to_string(),
                persona_uid: None,
                status: MotherSessionStatus::Active,
                interface_kind: "opencode".to_string(),
                adapter_name: "opencode".to_string(),
                branch: Some("patina".to_string()),
                start_tag: Some("session-20260312-000231-BBBB-opencode-start".to_string()),
                end_tag: None,
                parent_runtime_id: None,
                handoff_from_runtime_id: None,
                created_at: "2026-03-12T00:02:31Z".to_string(),
                updated_at: "2026-03-12T00:02:31Z".to_string(),
            };

            for record in [&first, &second] {
                store.create_mother_session(record, &[]).unwrap();
                let artifact_path = session::durable_session_path(temp.path(), &record.file_id);
                fs::create_dir_all(artifact_path.parent().unwrap()).unwrap();
                fs::write(&artifact_path, format!("session {}", record.file_id)).unwrap();
            }

            let pointer_path = temp
                .path()
                .join(".patina/local/interface-sessions/opencode.toml");
            fs::create_dir_all(pointer_path.parent().unwrap()).unwrap();
            fs::write(
                &pointer_path,
                "adapter = \"opencode\"\nruntime_id = \"runtime-second\"\nfile_id = \"20260312-000231-BBBB\"\n",
            )
            .unwrap();

            let result = check_in(&InterfaceCheckIn {
                interface_kind: InterfaceKind::OpenCode,
                adapter_name: "opencode".to_string(),
                project_root: temp.path().to_path_buf(),
                project_uid: None,
                requested_persona: None,
                requested_session: None,
                title: None,
                capabilities: InterfaceCapabilities::default(),
            })
            .unwrap();

            assert!(result.attached_existing);
            assert_eq!(result.session_runtime_id, second.runtime_id);
            assert_eq!(result.session_file_id, second.file_id);
            assert_ne!(result.session_runtime_id, first.runtime_id);
        });
    }

    fn with_temp_patina_home<T>(temp: &tempfile::TempDir, f: impl FnOnce() -> T) -> T {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let patina_home = temp.path().join("patina-home");
        fs::create_dir_all(&patina_home).unwrap();

        let old_patina_home = std::env::var_os("PATINA_HOME");
        unsafe {
            std::env::set_var("PATINA_HOME", &patina_home);
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
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
    fn check_in_persona_scope_ignores_pointer_mismatch_and_reattaches_matching_persona() {
        let temp = tempfile::TempDir::new().unwrap();
        with_temp_patina_home(&temp, || {
            let project_uid = project::create_uid_if_missing(temp.path()).unwrap();
            let store = KnowledgeRuntimeStore::default();

            let none_persona = MotherSessionRecord {
                runtime_id: "runtime-none-persona".to_string(),
                project_uid: project_uid.clone(),
                file_id: "20260312-100000-AAAA".to_string(),
                title: "OpenCode none persona".to_string(),
                persona_uid: None,
                status: MotherSessionStatus::Active,
                interface_kind: "opencode".to_string(),
                adapter_name: "opencode".to_string(),
                branch: Some("patina".to_string()),
                start_tag: Some("session-20260312-100000-AAAA-opencode-start".to_string()),
                end_tag: None,
                parent_runtime_id: None,
                handoff_from_runtime_id: None,
                created_at: "2026-03-12T10:00:00Z".to_string(),
                updated_at: "2026-03-12T10:00:00Z".to_string(),
            };
            let persona_one = MotherSessionRecord {
                runtime_id: "runtime-persona-one".to_string(),
                project_uid,
                file_id: "20260312-100001-BBBB".to_string(),
                title: "OpenCode persona one".to_string(),
                persona_uid: Some("persona-1".to_string()),
                status: MotherSessionStatus::Active,
                interface_kind: "opencode".to_string(),
                adapter_name: "opencode".to_string(),
                branch: Some("patina".to_string()),
                start_tag: Some("session-20260312-100001-BBBB-opencode-start".to_string()),
                end_tag: None,
                parent_runtime_id: None,
                handoff_from_runtime_id: None,
                created_at: "2026-03-12T10:00:01Z".to_string(),
                updated_at: "2026-03-12T10:00:01Z".to_string(),
            };

            for record in [&none_persona, &persona_one] {
                store.create_mother_session(record, &[]).unwrap();
                let artifact_path = session::durable_session_path(temp.path(), &record.file_id);
                fs::create_dir_all(artifact_path.parent().unwrap()).unwrap();
                fs::write(&artifact_path, format!("session {}", record.file_id)).unwrap();
            }

            let pointer_path = temp
                .path()
                .join(".patina/local/interface-sessions/opencode.toml");
            fs::create_dir_all(pointer_path.parent().unwrap()).unwrap();
            fs::write(
                &pointer_path,
                "adapter = \"opencode\"\nruntime_id = \"runtime-none-persona\"\nfile_id = \"20260312-100000-AAAA\"\n",
            )
            .unwrap();

            let result = check_in(&InterfaceCheckIn {
                interface_kind: InterfaceKind::OpenCode,
                adapter_name: "opencode".to_string(),
                project_root: temp.path().to_path_buf(),
                project_uid: None,
                requested_persona: Some("persona-1".to_string()),
                requested_session: None,
                title: None,
                capabilities: InterfaceCapabilities::default(),
            })
            .unwrap();

            assert!(result.attached_existing);
            assert_eq!(result.session_runtime_id, persona_one.runtime_id);
            assert_eq!(result.session_file_id, persona_one.file_id);
            assert_eq!(result.persona_uid.as_deref(), Some("persona-1"));
        });
    }

    #[test]
    fn check_in_rejects_requested_session_when_persona_scope_mismatches() {
        let temp = tempfile::TempDir::new().unwrap();
        with_temp_patina_home(&temp, || {
            let project_uid = project::create_uid_if_missing(temp.path()).unwrap();
            let store = KnowledgeRuntimeStore::default();

            let record = MotherSessionRecord {
                runtime_id: "runtime-none-persona".to_string(),
                project_uid,
                file_id: "20260312-100000-AAAA".to_string(),
                title: "OpenCode none persona".to_string(),
                persona_uid: None,
                status: MotherSessionStatus::Active,
                interface_kind: "opencode".to_string(),
                adapter_name: "opencode".to_string(),
                branch: Some("patina".to_string()),
                start_tag: Some("session-20260312-100000-AAAA-opencode-start".to_string()),
                end_tag: None,
                parent_runtime_id: None,
                handoff_from_runtime_id: None,
                created_at: "2026-03-12T10:00:00Z".to_string(),
                updated_at: "2026-03-12T10:00:00Z".to_string(),
            };
            store.create_mother_session(&record, &[]).unwrap();
            let artifact_path = session::durable_session_path(temp.path(), &record.file_id);
            fs::create_dir_all(artifact_path.parent().unwrap()).unwrap();
            fs::write(&artifact_path, format!("session {}", record.file_id)).unwrap();

            let error = check_in(&InterfaceCheckIn {
                interface_kind: InterfaceKind::OpenCode,
                adapter_name: "opencode".to_string(),
                project_root: temp.path().to_path_buf(),
                project_uid: None,
                requested_persona: Some("persona-1".to_string()),
                requested_session: Some(record.runtime_id.clone()),
                title: None,
                capabilities: InterfaceCapabilities::default(),
            })
            .unwrap_err();

            assert!(error
                .to_string()
                .contains("persona does not match requested persona scope"));
        });
    }
}
