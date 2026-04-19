use anyhow::Result;
use serde_json::json;
use std::path::PathBuf;

use crate::session;

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
pub struct CheckInResult {
    pub voice_uid: Option<String>,
    pub session_runtime_id: String,
    pub session_file_id: String,
    pub artifact_path: PathBuf,
    pub attached_existing: bool,
}

/// Execute a session-writer action for a resolved live session.
///
/// HITL envelope/session resolution is Mother-authoritative and handled through
/// `/api/interface/call` (`handshake` + `envelope.resolve`). This module only
/// retains shared session-writer integration helpers used by launch paths.
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

    let runtime = crate::mother::MotherRuntimeStore::default();
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

fn load_session_writer_knowledge_child() -> Result<Option<Box<dyn crate::mother::Child>>> {
    let engine = crate::child::engine::ChildEngine::new()?;

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
