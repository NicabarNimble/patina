use anyhow::{bail, Context, Result};
use chrono::{Local, Utc};
use std::path::{Path, PathBuf};

use crate::git;
use crate::mother::{
    InterfaceKindId, MotherRuntimeStore, MotherSessionParticipant, MotherSessionRecord,
    MotherSessionStatus, ProjectUid, VoiceUid,
};
use crate::project;
use crate::session::{
    ArchiveSessionRequest, BeginSessionRequest, InterfaceKind, SessionParticipant, SessionStart,
};

use super::artifact::{initial_document, parse_session_ids, rewrite_document_status, NewDocument};
use super::ids::{generate_file_id, generate_runtime_id};
use super::projection;

#[derive(Debug, Clone)]
pub struct LiveSessionHandle {
    pub runtime_id: String,
    pub file_id: String,
    pub title: String,
    pub interface_name: String,
    pub interface_kind: InterfaceKind,
    pub voice_uid: Option<String>,
    pub artifact_path: PathBuf,
    pub branch: String,
    pub starting_commit: String,
    pub start_tag: String,
    pub end_tag: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn begin_session(project_root: &Path, request: BeginSessionRequest) -> Result<SessionStart> {
    let store = MotherRuntimeStore::default();
    let project_uid = project::register_with_mother(project_root)?;
    let created_local = Local::now();
    let created_at = Utc::now().to_rfc3339();
    let runtime_id = generate_runtime_id();
    let file_id = generate_file_id(created_local);
    let branch = git::current_branch().unwrap_or_else(|_| "none".to_string());
    let starting_commit = git::head_sha().unwrap_or_else(|_| "none".to_string());
    let start_tag = format!("session-{}-{}-start", file_id, request.interface_name);

    if git::is_git_repo().unwrap_or(false) {
        if git::tag_exists(&start_tag).unwrap_or(false) {
            bail!("Session tag {} already exists.", start_tag);
        }
        git::create_tag(&start_tag, &format!("Session start: {}", request.title))
            .with_context(|| format!("creating {}", start_tag))?;
    }

    let participant = request
        .participant
        .clone()
        .unwrap_or_else(|| default_participant(&request.interface_name, request.interface_kind));
    let continuity_uid = request
        .continuity_uid
        .clone()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| Some(runtime_id.clone()));
    let work_spec = request
        .work_spec
        .clone()
        .filter(|value| !value.trim().is_empty())
        .or_else(resolve_work_spec_hint)
        .or_else(|| Some("unspecified".to_string()));

    let document = initial_document(NewDocument {
        file_id: file_id.clone(),
        runtime_id: runtime_id.clone(),
        continuity_uid,
        work_spec,
        title: request.title.clone(),
        interface_name: request.interface_name.clone(),
        interface_kind: request.interface_kind,
        voice_uid: request.voice_uid.clone(),
        project_uid: project_uid.clone(),
        branch: branch.clone(),
        starting_commit: starting_commit.clone(),
        start_tag: start_tag.clone(),
        parent_session: request.parent_runtime_id.clone(),
        handoff_from: request.handoff_from_runtime_id.clone(),
        takeover_from_runtime: request.takeover_from_runtime.clone(),
        takeover_user_verified: request.takeover_user_verified,
        participants: vec![participant.clone()],
        created_at: created_at.clone(),
        created_local,
    })?;
    let rendered = document.render()?;
    let artifact_path = projection::write_durable_artifact(project_root, &file_id, &rendered)?;

    let record = MotherSessionRecord {
        runtime_id: runtime_id.clone(),
        project_uid,
        file_id: file_id.clone(),
        title: request.title.clone(),
        voice_uid: request.voice_uid.clone(),
        status: MotherSessionStatus::Active,
        interface_kind: request.interface_kind.as_str().to_string(),
        interface_name: request.interface_name.clone(),
        branch: Some(branch.clone()),
        start_tag: Some(start_tag.clone()),
        end_tag: None,
        parent_runtime_id: request.parent_runtime_id,
        handoff_from_runtime_id: request.handoff_from_runtime_id,
        starting_commit: Some(starting_commit.clone()),
        created_at: created_at.clone(),
        updated_at: created_at.clone(),
    };
    store.create_mother_session(
        &record,
        &[MotherSessionParticipant {
            session_runtime_id: runtime_id.clone(),
            participant_id: participant.participant_id.clone(),
            role: participant.role.clone(),
            interface_kind: Some(participant.interface_kind.as_str().to_string()),
            interface_name: participant.interface_name.clone(),
            display_name: participant.display_name.clone(),
            joined_at: created_at.clone(),
            left_at: None,
        }],
    )?;

    if request.interface_kind != crate::session::InterfaceKind::LegacyCli {
        projection::write_native_interface_session(
            project_root,
            &request.interface_name,
            &runtime_id,
            &file_id,
        )?;
    }

    Ok(SessionStart {
        handle: map_record(record, artifact_path, starting_commit),
        document: rendered,
    })
}

pub fn sync_session_document(
    project_root: &Path,
    runtime_id: &str,
    markdown: &str,
) -> Result<LiveSessionHandle> {
    let Some(record) = load_record(runtime_id)? else {
        bail!("Unknown live session {}", runtime_id);
    };
    let updated_at = Utc::now().to_rfc3339();
    let refreshed = rewrite_document_status(
        markdown,
        record.status.as_str(),
        &updated_at,
        record.end_tag.as_deref(),
    )?;
    let artifact_path =
        projection::write_durable_artifact(project_root, &record.file_id, &refreshed)?;
    let store = MotherRuntimeStore::default();
    store.touch_mother_session(runtime_id, &updated_at)?;
    let Some(updated) = store.get_mother_session(runtime_id)? else {
        bail!("live session {} disappeared during sync", runtime_id);
    };
    Ok(map_record(
        updated,
        artifact_path,
        git::head_sha().unwrap_or_else(|_| "none".to_string()),
    ))
}

pub fn archive_session(
    project_root: &Path,
    request: ArchiveSessionRequest,
) -> Result<LiveSessionHandle> {
    let Some(record) = load_record(&request.runtime_id)? else {
        bail!("Unknown live session {}", request.runtime_id);
    };
    let updated_at = Utc::now().to_rfc3339();
    let archived = rewrite_document_status(
        &request.markdown,
        MotherSessionStatus::Archived.as_str(),
        &updated_at,
        request.end_tag.as_deref(),
    )?;
    let artifact_path =
        projection::write_durable_artifact(project_root, &record.file_id, &archived)?;
    let store = MotherRuntimeStore::default();
    store.finish_mother_session(
        &request.runtime_id,
        MotherSessionStatus::Archived,
        request.end_tag.as_deref(),
        &updated_at,
    )?;
    if record.interface_kind != crate::session::InterfaceKind::LegacyCli.as_str() {
        projection::clear_native_interface_session(
            project_root,
            &record.interface_name,
            Some(&request.runtime_id),
        )?;
    }
    let Some(updated) = store.get_mother_session(&request.runtime_id)? else {
        bail!(
            "live session {} disappeared during archive",
            request.runtime_id
        );
    };
    Ok(map_record(updated, artifact_path, record.starting_commit()))
}

pub fn list_active_sessions(project_root: &Path) -> Result<Vec<LiveSessionHandle>> {
    let store = MotherRuntimeStore::default();
    let Some(project_uid_raw) = project::get_uid(project_root) else {
        return Ok(Vec::new());
    };
    let project_uid = ProjectUid::new(project_uid_raw)?;
    let records = store.list_active_mother_sessions(&project_uid)?;
    Ok(records
        .into_iter()
        .map(|record| {
            let artifact_path = projection::durable_session_path(project_root, &record.file_id);
            map_record(
                record,
                artifact_path,
                git::head_sha().unwrap_or_else(|_| "none".to_string()),
            )
        })
        .collect())
}

pub fn find_active_interface_session(
    project_root: &Path,
    interface_name: &str,
    interface_kind: InterfaceKind,
    voice_uid: Option<&str>,
) -> Result<Option<LiveSessionHandle>> {
    let store = MotherRuntimeStore::default();
    let Some(project_uid_raw) = project::get_uid(project_root) else {
        return Ok(None);
    };
    let project_uid = ProjectUid::new(project_uid_raw)?;
    let voice_uid = match voice_uid {
        Some(value) => Some(VoiceUid::new(value.to_string())?),
        None => None,
    };
    let interface_kind_id = InterfaceKindId::new(interface_kind.as_str().to_string())?;
    let Some(record) = store.find_active_mother_session_for_interface(
        &project_uid,
        interface_name,
        &interface_kind_id,
        voice_uid.as_ref(),
    )?
    else {
        return Ok(None);
    };
    let artifact_path = projection::durable_session_path(project_root, &record.file_id);
    Ok(Some(map_record(
        record,
        artifact_path,
        git::head_sha().unwrap_or_else(|_| "none".to_string()),
    )))
}

pub fn load_session(project_root: &Path, runtime_id: &str) -> Result<Option<LiveSessionHandle>> {
    let Some(record) = load_record(runtime_id)? else {
        return Ok(None);
    };
    let artifact_path = projection::durable_session_path(project_root, &record.file_id);
    Ok(Some(map_record(
        record,
        artifact_path,
        git::head_sha().unwrap_or_else(|_| "none".to_string()),
    )))
}

pub fn load_session_by_file_id(
    project_root: &Path,
    file_id: &str,
) -> Result<Option<LiveSessionHandle>> {
    let store = MotherRuntimeStore::default();
    let Some(record) = store.get_mother_session_by_file_id(file_id)? else {
        return Ok(None);
    };
    let artifact_path = projection::durable_session_path(project_root, &record.file_id);
    Ok(Some(map_record(
        record,
        artifact_path,
        git::head_sha().unwrap_or_else(|_| "none".to_string()),
    )))
}

pub fn current_session_file_id(project_root: &Path) -> Result<Option<String>> {
    if let Some((_, file_id)) = read_compatibility_ids(project_root)? {
        return Ok(Some(file_id));
    }

    let sessions = list_active_sessions(project_root)?;
    Ok(sessions.into_iter().next().map(|session| session.file_id))
}

pub fn current_session_runtime_id(project_root: &Path) -> Result<Option<String>> {
    if let Some((runtime_id, _)) = read_compatibility_ids(project_root)? {
        return Ok(Some(runtime_id));
    }

    let sessions = list_active_sessions(project_root)?;
    Ok(sessions
        .into_iter()
        .next()
        .map(|session| session.runtime_id))
}

pub fn load_current_interface_session(
    project_root: &Path,
    interface_name: &str,
) -> Result<Option<LiveSessionHandle>> {
    let Some(pointer) = projection::read_native_interface_session(project_root, interface_name)?
    else {
        return Ok(None);
    };

    let Some(record) = load_record(&pointer.runtime_id)? else {
        projection::clear_native_interface_session(
            project_root,
            interface_name,
            Some(&pointer.runtime_id),
        )?;
        return Ok(None);
    };

    if record.interface_name != interface_name {
        projection::clear_native_interface_session(
            project_root,
            interface_name,
            Some(&pointer.runtime_id),
        )?;
        return Ok(None);
    }

    if record.status != MotherSessionStatus::Active {
        projection::clear_native_interface_session(
            project_root,
            interface_name,
            Some(&pointer.runtime_id),
        )?;
        return Ok(None);
    }

    let artifact_path = projection::durable_session_path(project_root, &record.file_id);
    Ok(Some(map_record(
        record,
        artifact_path,
        git::head_sha().unwrap_or_else(|_| "none".to_string()),
    )))
}

fn read_compatibility_ids(project_root: &Path) -> Result<Option<(String, String)>> {
    let path = projection::active_session_path(project_root);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    Ok(parse_session_ids(&content))
}

fn load_record(runtime_id: &str) -> Result<Option<MotherSessionRecord>> {
    MotherRuntimeStore::default().get_mother_session(runtime_id)
}

fn resolve_work_spec_hint() -> Option<String> {
    ["PATINA_WORK_SPEC", "PATINA_SPEC_ID"]
        .iter()
        .find_map(|key| {
            std::env::var(key)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
}

fn default_participant(interface_name: &str, interface_kind: InterfaceKind) -> SessionParticipant {
    let participant_id = format!(
        "{}-{}",
        std::env::var("USER").unwrap_or_else(|_| "operator".to_string()),
        std::process::id()
    );
    SessionParticipant {
        participant_id,
        role: "operator".to_string(),
        interface_kind,
        interface_name: Some(interface_name.to_string()),
        display_name: std::env::var("USER").ok(),
    }
}

fn map_record(
    record: MotherSessionRecord,
    artifact_path: PathBuf,
    fallback_commit: String,
) -> LiveSessionHandle {
    let starting_commit = if let Some(commit) = record.starting_commit.clone() {
        commit
    } else {
        read_starting_commit_from_artifact(&artifact_path).unwrap_or(fallback_commit)
    };

    LiveSessionHandle {
        runtime_id: record.runtime_id,
        file_id: record.file_id,
        title: record.title,
        interface_name: record.interface_name,
        interface_kind: map_interface_kind(&record.interface_kind),
        voice_uid: record.voice_uid,
        artifact_path,
        branch: record.branch.unwrap_or_else(|| "none".to_string()),
        starting_commit,
        start_tag: record.start_tag.unwrap_or_default(),
        end_tag: record.end_tag,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

fn read_starting_commit_from_artifact(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_frontmatter = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "---" {
            if in_frontmatter {
                break;
            }
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter && trimmed.starts_with("starting_commit:") {
            return trimmed
                .split_once(':')
                .map(|(_, value)| value.trim().to_string())
                .filter(|value| !value.is_empty());
        }
    }
    None
}

fn map_interface_kind(kind: &str) -> InterfaceKind {
    match kind {
        "legacy-cli" => InterfaceKind::LegacyCli,
        "opencode" => InterfaceKind::OpenCode,
        "gemini" => InterfaceKind::Gemini,
        "claude" => InterfaceKind::Claude,
        "pi" => InterfaceKind::Pi,
        _ => InterfaceKind::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{self, ProjectConfig};
    use tempfile::TempDir;

    fn with_test_env<T>(f: impl FnOnce(&TempDir) -> T) -> T {
        let _guard = crate::test_support::env_test_mutex()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let temp = TempDir::new().unwrap();
        let patina_home = temp.path().join("patina-home");
        std::fs::create_dir_all(&patina_home).unwrap();

        let old_home = std::env::var_os("HOME");
        let old_patina_home = std::env::var_os("PATINA_HOME");
        let old_cwd = std::env::current_dir().ok();
        unsafe {
            std::env::set_var("HOME", temp.path());
            std::env::set_var("PATINA_HOME", &patina_home);
        }
        std::env::set_current_dir(temp.path()).unwrap();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&temp)));

        match old_home {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }
        match old_patina_home {
            Some(value) => unsafe { std::env::set_var("PATINA_HOME", value) },
            None => unsafe { std::env::remove_var("PATINA_HOME") },
        }
        if let Some(path) = old_cwd {
            let _ = std::env::set_current_dir(path);
        }

        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    #[test]
    fn load_current_interface_session_clears_non_active_pointer() {
        with_test_env(|temp| {
            let mut config = ProjectConfig::with_name("patina");
            config.interfaces.allowed = vec!["opencode".to_string()];
            config.interfaces.default = "opencode".to_string();
            project::save(temp.path(), &config).unwrap();

            let started = begin_session(
                temp.path(),
                BeginSessionRequest {
                    title: "test".to_string(),
                    interface_name: "opencode".to_string(),
                    interface_kind: InterfaceKind::OpenCode,
                    voice_uid: None,
                    work_spec: Some("interface-session-artifacts".to_string()),
                    continuity_uid: None,
                    takeover_from_runtime: None,
                    takeover_user_verified: None,
                    parent_runtime_id: None,
                    handoff_from_runtime_id: None,
                    participant: None,
                },
            )
            .unwrap();

            let markdown = std::fs::read_to_string(&started.handle.artifact_path).unwrap();
            archive_session(
                temp.path(),
                ArchiveSessionRequest {
                    runtime_id: started.handle.runtime_id.clone(),
                    markdown,
                    end_tag: Some("end-test".to_string()),
                },
            )
            .unwrap();

            projection::write_native_interface_session(
                temp.path(),
                "opencode",
                &started.handle.runtime_id,
                &started.handle.file_id,
            )
            .unwrap();

            let loaded = load_current_interface_session(temp.path(), "opencode").unwrap();
            assert!(loaded.is_none());
            assert!(
                projection::read_native_interface_session(temp.path(), "opencode")
                    .unwrap()
                    .is_none()
            );
        });
    }
}
