use anyhow::Result;
use chrono::{Duration, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

const HANDSHAKE_TTL_SECS: i64 = 120;
const HANDSHAKE_STORE_CAP: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InterfaceControlOperation {
    HandshakeV1,
    EnvelopeResolveV1,
    EnvelopeHeartbeatV1,
    EnvelopeEndV1,
}

impl InterfaceControlOperation {
    fn parse(operation_id: &str) -> Option<Self> {
        match operation_id {
            "patina:interface/handshake.v1" => Some(Self::HandshakeV1),
            "patina:interface/envelope.resolve.v1" => Some(Self::EnvelopeResolveV1),
            "patina:interface/envelope.heartbeat.v1" => Some(Self::EnvelopeHeartbeatV1),
            "patina:interface/envelope.end.v1" => Some(Self::EnvelopeEndV1),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::HandshakeV1 => "patina:interface/handshake.v1",
            Self::EnvelopeResolveV1 => "patina:interface/envelope.resolve.v1",
            Self::EnvelopeHeartbeatV1 => "patina:interface/envelope.heartbeat.v1",
            Self::EnvelopeEndV1 => "patina:interface/envelope.end.v1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum HandshakeInterfaceKind {
    Hitl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum LaunchIntent {
    AttachOrCreate,
    AttachOnly,
    CreateOnly,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct HandshakeArgs {
    #[serde(alias = "protocol_version")]
    protocol_version: String,
    #[serde(alias = "cli_version")]
    cli_version: String,
    #[serde(alias = "project_uid")]
    project_uid: String,
    #[serde(alias = "project_root")]
    project_root: String,
    #[serde(alias = "interface_name")]
    interface_name: String,
    #[serde(alias = "interface_kind")]
    interface_kind: HandshakeInterfaceKind,
    #[serde(alias = "launch_intent")]
    launch_intent: LaunchIntent,
    #[serde(alias = "requested_session")]
    requested_session: Option<String>,
    #[serde(alias = "tmux_mode")]
    tmux_mode: Option<String>,
    tty: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ResolveArgs {
    #[serde(alias = "handshake_id")]
    handshake_id: String,
    title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct HeartbeatArgs {
    #[serde(alias = "envelope_id")]
    envelope_id: String,
    pid: u32,
    #[serde(alias = "tmux_lane")]
    tmux_lane: String,
    #[serde(alias = "observed_at")]
    observed_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct EndArgs {
    #[serde(alias = "envelope_id")]
    envelope_id: String,
    reason: String,
}

#[derive(Debug, Clone)]
struct HandshakeRecord {
    project_uid: String,
    project_root: PathBuf,
    interface_name: String,
    launch_intent: LaunchIntent,
    requested_session: Option<String>,
    tmux_mode: Option<String>,
    tty: bool,
    expires_at: chrono::DateTime<Utc>,
}

enum ResolveDecision {
    Attach(patina::session::LiveSessionHandle),
    Create,
    Choose(Vec<patina::session::LiveSessionHandle>),
    Reject(String),
}

fn handshake_store() -> &'static Mutex<HashMap<String, HandshakeRecord>> {
    static STORE: OnceLock<Mutex<HashMap<String, HandshakeRecord>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn invalid_request(message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(mother_crate::http_api::LifecycleError::invalid_request(
        message.into(),
    ))
}

fn ok_response(
    operation: InterfaceControlOperation,
    result: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "adapter": "native",
        "operation_id": operation.as_str(),
        "status": "ok",
        "result": result,
    })
}

pub(super) fn dispatch_interface_control_call(
    request: mother_crate::http_api::InterfaceControlCallRequest,
) -> Result<serde_json::Value> {
    let operation = InterfaceControlOperation::parse(&request.operation_id).ok_or_else(|| {
        invalid_request(format!(
            "unsupported interface operation '{}'",
            request.operation_id
        ))
    })?;

    match operation {
        InterfaceControlOperation::HandshakeV1 => handle_handshake(operation, request.args),
        InterfaceControlOperation::EnvelopeResolveV1 => handle_resolve(operation, request.args),
        InterfaceControlOperation::EnvelopeHeartbeatV1 => handle_heartbeat(operation, request.args),
        InterfaceControlOperation::EnvelopeEndV1 => handle_end(operation, request.args),
    }
}

fn handle_handshake(
    operation: InterfaceControlOperation,
    args: serde_json::Value,
) -> Result<serde_json::Value> {
    prune_handshakes();

    let args: HandshakeArgs = serde_json::from_value(args)
        .map_err(|error| invalid_request(format!("invalid handshake args: {}", error)))?;

    if args.protocol_version.trim().is_empty() {
        return Err(invalid_request("protocol_version is required"));
    }
    if args.cli_version.trim().is_empty() {
        return Err(invalid_request("cli_version is required"));
    }

    match args.interface_kind {
        HandshakeInterfaceKind::Hitl => {}
    }

    let interface_name = args.interface_name.trim().to_string();
    if interface_name.is_empty() {
        return Err(invalid_request("interface_name is required"));
    }

    let project_root = PathBuf::from(args.project_root.trim());
    if project_root.as_os_str().is_empty() {
        return Err(invalid_request("project_root is required"));
    }

    let normalized_uid = patina::project::register_with_mother(&project_root)
        .map_err(|error| invalid_request(format!("project registration failed: {}", error)))?;
    if normalized_uid != args.project_uid {
        return Err(invalid_request(format!(
            "project_uid mismatch: expected '{}' from project root",
            normalized_uid
        )));
    }

    let handshake_id = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::seconds(HANDSHAKE_TTL_SECS);

    let mut store = handshake_store()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if store.len() >= HANDSHAKE_STORE_CAP {
        trim_oldest_handshakes(&mut store);
    }
    store.insert(
        handshake_id.clone(),
        HandshakeRecord {
            project_uid: normalized_uid.clone(),
            project_root,
            interface_name,
            launch_intent: args.launch_intent,
            requested_session: args
                .requested_session
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            tmux_mode: args.tmux_mode,
            tty: args.tty,
            expires_at,
        },
    );

    Ok(ok_response(
        operation,
        serde_json::json!({
            "handshake_id": handshake_id,
            "mother_version": env!("CARGO_PKG_VERSION"),
            "project_uid": normalized_uid,
            "control_plane_ready": true,
            "expires_at": expires_at.to_rfc3339(),
        }),
    ))
}

fn handle_resolve(
    operation: InterfaceControlOperation,
    args: serde_json::Value,
) -> Result<serde_json::Value> {
    prune_handshakes();

    let args: ResolveArgs = serde_json::from_value(args)
        .map_err(|error| invalid_request(format!("invalid resolve args: {}", error)))?;
    if args.handshake_id.trim().is_empty() {
        return Err(invalid_request("handshake_id is required"));
    }

    let handshake = {
        let store = handshake_store()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        store.get(args.handshake_id.trim()).cloned()
    }
    .ok_or_else(|| invalid_request("unknown or expired handshake_id"))?;

    let sessions = active_interface_sessions(&handshake)?;
    let decision = decide_resolution(
        handshake.launch_intent,
        handshake.requested_session.as_deref(),
        sessions,
    );

    let result = match decision {
        ResolveDecision::Attach(handle) => serde_json::json!({
            "decision": "attach",
            "identity": identity_from_handle(&handle, &handshake.project_uid, &handshake.interface_name),
            "tmux_mode": handshake.tmux_mode,
            "tty": handshake.tty,
        }),
        ResolveDecision::Create => {
            let title = args
                .title
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| format!("{} session", handshake.interface_name));
            let handle = create_session(&handshake, &title)?;
            serde_json::json!({
                "decision": "create",
                "identity": identity_from_handle(&handle, &handshake.project_uid, &handshake.interface_name),
                "tmux_mode": handshake.tmux_mode,
                "tty": handshake.tty,
            })
        }
        ResolveDecision::Choose(candidates) => {
            let choices = candidates
                .iter()
                .map(|handle| {
                    identity_from_handle(handle, &handshake.project_uid, &handshake.interface_name)
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "decision": "choose",
                "choices": choices,
                "tmux_mode": handshake.tmux_mode,
                "tty": handshake.tty,
            })
        }
        ResolveDecision::Reject(reason) => serde_json::json!({
            "decision": "reject",
            "reason": reason,
            "tmux_mode": handshake.tmux_mode,
            "tty": handshake.tty,
        }),
    };

    Ok(ok_response(operation, result))
}

fn handle_heartbeat(
    operation: InterfaceControlOperation,
    args: serde_json::Value,
) -> Result<serde_json::Value> {
    let args: HeartbeatArgs = serde_json::from_value(args)
        .map_err(|error| invalid_request(format!("invalid heartbeat args: {}", error)))?;
    if args.envelope_id.trim().is_empty() {
        return Err(invalid_request("envelope_id is required"));
    }

    Ok(ok_response(
        operation,
        serde_json::json!({
            "ok": true,
            "detail": "heartbeat acknowledged",
            "envelope_id": args.envelope_id,
            "pid": args.pid,
            "tmux_lane": args.tmux_lane,
            "observed_at": args.observed_at,
        }),
    ))
}

fn handle_end(
    operation: InterfaceControlOperation,
    args: serde_json::Value,
) -> Result<serde_json::Value> {
    let args: EndArgs = serde_json::from_value(args)
        .map_err(|error| invalid_request(format!("invalid end args: {}", error)))?;
    if args.envelope_id.trim().is_empty() {
        return Err(invalid_request("envelope_id is required"));
    }

    Ok(ok_response(
        operation,
        serde_json::json!({
            "ok": true,
            "detail": "envelope ended",
            "envelope_id": args.envelope_id,
            "reason": args.reason,
        }),
    ))
}

fn active_interface_sessions(
    handshake: &HandshakeRecord,
) -> Result<Vec<patina::session::LiveSessionHandle>> {
    let expected_kind = interface_kind_from_name(&handshake.interface_name)?;
    Ok(
        patina::session::list_active_sessions(&handshake.project_root)?
            .into_iter()
            .filter(|session| {
                session.interface_name == handshake.interface_name
                    && session.interface_kind == expected_kind
            })
            .collect(),
    )
}

fn interface_kind_from_name(interface_name: &str) -> Result<patina::session::InterfaceKind> {
    let kind = patina::session::InterfaceKind::from_interface_name(interface_name);
    if kind == patina::session::InterfaceKind::Unknown {
        return Err(invalid_request(format!(
            "unsupported interface_name '{}' for HITL",
            interface_name
        )));
    }
    Ok(kind)
}

fn create_session(
    handshake: &HandshakeRecord,
    title: &str,
) -> Result<patina::session::LiveSessionHandle> {
    let interface_kind = interface_kind_from_name(&handshake.interface_name)?;
    let start = patina::session::begin_session(
        Path::new(&handshake.project_root),
        patina::session::BeginSessionRequest {
            title: title.to_string(),
            interface_name: handshake.interface_name.clone(),
            interface_kind,
            voice_uid: None,
            work_spec: std::env::var("PATINA_WORK_SPEC").ok(),
            continuity_uid: None,
            takeover_from_runtime: None,
            takeover_user_verified: None,
            parent_runtime_id: None,
            handoff_from_runtime_id: None,
            participant: Some(patina::session::SessionParticipant {
                participant_id: format!("{}-{}", handshake.interface_name, std::process::id()),
                role: "interface".to_string(),
                interface_kind,
                interface_name: Some(handshake.interface_name.clone()),
                display_name: std::env::var("USER")
                    .ok()
                    .or_else(|| std::env::var("LOGNAME").ok())
                    .or_else(|| Some(handshake.interface_name.clone())),
            }),
        },
    )?;

    if let Err(error) = patina::interface::session_writer_action(
        &start.handle,
        "init-session",
        serde_json::json!({
            "session_runtime_id": start.handle.runtime_id,
            "session_file_id": start.handle.file_id,
            "artifact_path": start.handle.artifact_path,
            "branch": start.handle.branch,
            "start_tag": start.handle.start_tag,
        }),
    ) {
        eprintln!(
            "[mother-hitl] warning: session-writer init failed for '{}': {}",
            start.handle.runtime_id, error
        );
        if let Err(event_error) =
            emit_session_writer_init_failed_event(&handshake.project_root, &start.handle, &error)
        {
            eprintln!(
                "[mother-hitl] warning: failed to emit session-writer failure event for '{}': {}",
                start.handle.runtime_id, event_error
            );
        }
    }

    Ok(start.handle)
}

fn emit_session_writer_init_failed_event(
    project_root: &Path,
    handle: &patina::session::LiveSessionHandle,
    error: &anyhow::Error,
) -> Result<()> {
    let conn = patina::eventlog::open_events_db_at(project_root)?;
    let timestamp = Utc::now().to_rfc3339();
    let payload = serde_json::json!({
        "session_id": handle.file_id,
        "runtime_id": handle.runtime_id,
        "interface": handle.interface_name,
        "artifact": handle.artifact_path,
        "error": error.to_string(),
    });

    patina::eventlog::insert_event(
        &conn,
        "session-writer.init.failed",
        &timestamp,
        &handle.file_id,
        Some(&handle.artifact_path.display().to_string()),
        &payload.to_string(),
    )?;

    Ok(())
}

fn identity_from_handle(
    handle: &patina::session::LiveSessionHandle,
    project_uid: &str,
    interface_name: &str,
) -> serde_json::Value {
    serde_json::json!({
        "envelope_id": handle.runtime_id,
        "session_runtime_id": handle.runtime_id,
        "session_file_id": handle.file_id,
        "tmux_lane": interface_name,
        "interface_name": interface_name,
        "project_uid": project_uid,
    })
}

fn decide_resolution(
    launch_intent: LaunchIntent,
    requested_session: Option<&str>,
    sessions: Vec<patina::session::LiveSessionHandle>,
) -> ResolveDecision {
    if let Some(selector) = requested_session {
        let selector = selector.trim();
        if selector.is_empty() {
            return ResolveDecision::Reject("requested_session must not be empty".to_string());
        }

        if launch_intent == LaunchIntent::CreateOnly {
            return ResolveDecision::Reject(
                "requested_session cannot be used with launch_intent=create-only".to_string(),
            );
        }

        if let Some(handle) = sessions
            .into_iter()
            .find(|session| session.runtime_id == selector || session.file_id == selector)
        {
            return ResolveDecision::Attach(handle);
        }

        return ResolveDecision::Reject(format!("requested session '{}' is not active", selector));
    }

    match launch_intent {
        LaunchIntent::CreateOnly => ResolveDecision::Create,
        LaunchIntent::AttachOnly => match sessions.len() {
            0 => ResolveDecision::Reject(
                "launch_intent=attach-only but no active envelope exists".to_string(),
            ),
            1 => ResolveDecision::Attach(sessions.into_iter().next().expect("singleton")),
            _ => ResolveDecision::Choose(sessions),
        },
        LaunchIntent::AttachOrCreate => match sessions.len() {
            0 => ResolveDecision::Create,
            1 => ResolveDecision::Attach(sessions.into_iter().next().expect("singleton")),
            _ => ResolveDecision::Choose(sessions),
        },
    }
}

fn prune_handshakes() {
    let now = Utc::now();
    let mut store = handshake_store()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    store.retain(|_, record| record.expires_at > now);
    if store.len() > HANDSHAKE_STORE_CAP {
        trim_oldest_handshakes(&mut store);
    }
}

fn trim_oldest_handshakes(store: &mut HashMap<String, HandshakeRecord>) {
    if store.len() <= HANDSHAKE_STORE_CAP {
        return;
    }

    let overflow = store.len().saturating_sub(HANDSHAKE_STORE_CAP);
    let mut entries = store
        .iter()
        .map(|(id, record)| (id.clone(), record.expires_at))
        .collect::<Vec<_>>();
    entries.sort_by_key(|(_, expires_at)| *expires_at);

    for (id, _) in entries.into_iter().take(overflow) {
        store.remove(&id);
    }
}

#[cfg(test)]
fn clear_handshakes() {
    handshake_store()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handle(id: &str) -> patina::session::LiveSessionHandle {
        patina::session::LiveSessionHandle {
            runtime_id: format!("runtime-{}", id),
            file_id: id.to_string(),
            title: format!("session {}", id),
            interface_name: "pi".to_string(),
            interface_kind: patina::session::InterfaceKind::Pi,
            voice_uid: None,
            artifact_path: std::path::PathBuf::from(format!("/tmp/{}.md", id)),
            branch: "patina".to_string(),
            starting_commit: "deadbeef".to_string(),
            start_tag: format!("session-{}-pi-start", id),
            end_tag: None,
            created_at: "2026-04-16T00:00:00Z".to_string(),
            updated_at: "2026-04-16T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn rejects_unknown_operation_id() {
        let error =
            dispatch_interface_control_call(mother_crate::http_api::InterfaceControlCallRequest {
                operation_id: "patina:interface/unknown.v1".to_string(),
                args: serde_json::json!([]),
                correlation: None,
            })
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("unsupported interface operation"),
            "got: {}",
            error
        );
    }

    #[test]
    fn decide_resolution_returns_choose_for_ambiguous_attach_or_create() {
        let decision = decide_resolution(
            LaunchIntent::AttachOrCreate,
            None,
            vec![make_handle("A"), make_handle("B")],
        );

        match decision {
            ResolveDecision::Choose(choices) => assert_eq!(choices.len(), 2),
            _ => panic!("expected choose decision"),
        }
    }

    #[test]
    fn handshake_then_resolve_creates_session_when_none_active() {
        crate::test_support::with_temp_patina_home(|_| {
            clear_handshakes();
            let temp = tempfile::TempDir::new().unwrap();
            let project_root = temp.path().join("project");
            std::fs::create_dir_all(&project_root).unwrap();
            let project_uid = patina::project::create_uid_if_missing(&project_root).unwrap();

            let handshake = dispatch_interface_control_call(
                mother_crate::http_api::InterfaceControlCallRequest {
                    operation_id: "patina:interface/handshake.v1".to_string(),
                    args: serde_json::json!({
                        "protocol-version": "0.1",
                        "cli-version": "0.62.0",
                        "project-uid": project_uid,
                        "project-root": project_root.display().to_string(),
                        "interface-name": "pi",
                        "interface-kind": "hitl",
                        "launch-intent": "attach-or-create",
                        "tty": true
                    }),
                    correlation: None,
                },
            )
            .expect("handshake should succeed");

            let handshake_id = handshake
                .get("result")
                .and_then(|result| result.get("handshake_id"))
                .and_then(|value| value.as_str())
                .expect("handshake_id")
                .to_string();

            let resolve = dispatch_interface_control_call(
                mother_crate::http_api::InterfaceControlCallRequest {
                    operation_id: "patina:interface/envelope.resolve.v1".to_string(),
                    args: serde_json::json!({
                        "handshake-id": handshake_id,
                        "title": "PI session"
                    }),
                    correlation: None,
                },
            )
            .expect("resolve should succeed");

            assert_eq!(
                resolve
                    .get("result")
                    .and_then(|result| result.get("decision"))
                    .and_then(|value| value.as_str()),
                Some("create")
            );

            let active = patina::session::list_active_sessions(&project_root).unwrap();
            assert_eq!(active.len(), 1);
        });
    }
}
