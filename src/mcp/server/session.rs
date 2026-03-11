//! Session lifecycle MCP handlers

use serde::Deserialize;

use super::super::protocol::{Request, Response};

#[derive(Deserialize)]
pub(super) struct SessionArgs {
    pub title: Option<String>,
    pub session: Option<String>,
    pub adapter: Option<String>,
    pub note: Option<String>,
}

macro_rules! require {
    ($req:expr, $field:expr, $tool:expr, $name:expr) => {
        match $field.as_deref() {
            Some(v) if !v.is_empty() => v,
            _ => {
                return Response::error(
                    $req.id.clone(),
                    super::ERR_INVALID_PARAMS,
                    &format!("{} requires '{}' parameter", $tool, $name),
                );
            }
        }
    };
}

pub(super) fn handle(req: &Request, name: &str, args: SessionArgs) -> Response {
    let project_root = match patina::session::SessionManager::find_project_root() {
        Ok(root) => root,
        Err(e) => return Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
    };

    match name {
        "session.start" => {
            let title = require!(req, args.title, "session.start", "title");
            let adapter = args
                .adapter
                .or_else(|| std::env::var("PATINA_AI_INTERFACE").ok())
                .unwrap_or_else(|| "opencode".to_string());
            let result = crate::commands::session::start_session_value(
                &project_root,
                crate::commands::session::SessionStartRequest {
                    title: title.to_string(),
                    adapter: Some(adapter.clone()),
                    interface_kind: patina::session::InterfaceKind::from_adapter_name(&adapter),
                    write_compatibility_projection: false,
                },
            );
            session_response(req, result)
        }
        "session.update" => {
            let handle = match resolve_session(&project_root, args.session.as_deref()) {
                Ok(handle) => handle,
                Err(e) => return Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            };
            session_response(
                req,
                crate::commands::session::update_live_session_value(&project_root, &handle),
            )
        }
        "session.end" => {
            let handle = match resolve_session(&project_root, args.session.as_deref()) {
                Ok(handle) => handle,
                Err(e) => return Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
            };
            session_response(
                req,
                crate::commands::session::end_live_session_value(
                    &project_root,
                    &handle,
                    args.note.as_deref(),
                ),
            )
        }
        "session.list" => {
            session_response(req, crate::commands::session::list_sessions_value(&project_root))
        }
        _ => Response::error(
            req.id.clone(),
            super::ERR_INVALID_PARAMS,
            &format!("Unknown tool: {}", name),
        ),
    }
}

fn session_response<T: serde::Serialize>(
    req: &Request,
    result: anyhow::Result<T>,
) -> Response {
    match result {
        Ok(result) => {
            let text = serde_json::to_string_pretty(&result).unwrap_or_default();
            Response::success(
                req.id.clone(),
                serde_json::json!({
                    "content": [{ "type": "text", "text": text }]
                }),
            )
        }
        Err(e) => Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string()),
    }
}

fn resolve_session(
    project_root: &std::path::Path,
    selector: Option<&str>,
) -> anyhow::Result<patina::session::LiveSessionHandle> {
    if let Some(selector) = selector {
        return load_session(project_root, selector)?
            .ok_or_else(|| anyhow::anyhow!("No active session found for selector '{}'", selector));
    }

    if let Some(runtime_id) = std::env::var("PATINA_SESSION_RUNTIME_ID").ok().filter(|v| !v.is_empty()) {
        if let Some(handle) = patina::session::load_session(project_root, &runtime_id)? {
            return Ok(handle);
        }
    }

    if let Some(file_id) = std::env::var("PATINA_SESSION_ID").ok().filter(|v| !v.is_empty()) {
        if let Some(handle) = patina::session::load_session_by_file_id(project_root, &file_id)? {
            return Ok(handle);
        }
    }

    let mut sessions = patina::session::list_active_sessions(project_root)?;
    if let Some(adapter) = std::env::var("PATINA_AI_INTERFACE").ok().filter(|v| !v.is_empty()) {
        sessions.retain(|handle| handle.adapter_name == adapter);
    }

    match sessions.len() {
        1 => Ok(sessions.remove(0)),
        0 => anyhow::bail!("No active session found"),
        _ => {
            let choices = sessions
                .iter()
                .map(|handle| format!("{} ({})", handle.file_id, handle.title))
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!(
                "Multiple active sessions match. Retry with session=<id>. Choices: {}",
                choices
            )
        }
    }
}

fn load_session(
    project_root: &std::path::Path,
    selector: &str,
) -> anyhow::Result<Option<patina::session::LiveSessionHandle>> {
    if let Some(handle) = patina::session::load_session(project_root, selector)? {
        return Ok(Some(handle));
    }
    patina::session::load_session_by_file_id(project_root, selector)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_requires_title() {
        let req = Request {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".to_string(),
            params: serde_json::json!({}),
        };

        let response = handle(
            &req,
            "session.start",
            SessionArgs {
                title: None,
                session: None,
                adapter: None,
                note: None,
            },
        );

        assert!(response.error.is_some());
        assert!(response
            .error
            .unwrap()
            .message
            .contains("session.start requires 'title'"));
    }
}
