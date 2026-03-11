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
                crate::commands::session::SessionStartRequest::native(title, &adapter),
            );
            session_response(req, result)
        }
        "session.update" => {
            let handle = match crate::commands::session::resolve_live_session(
                &project_root,
                args.session.as_deref(),
                None,
            ) {
                Ok(handle) => handle,
                Err(e) => {
                    return Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string())
                }
            };
            session_response(
                req,
                crate::commands::session::update_live_session_value(&project_root, &handle),
            )
        }
        "session.end" => {
            let handle = match crate::commands::session::resolve_live_session(
                &project_root,
                args.session.as_deref(),
                None,
            ) {
                Ok(handle) => handle,
                Err(e) => {
                    return Response::error(req.id.clone(), super::ERR_INTERNAL, &e.to_string())
                }
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
        "session.list" => session_response(
            req,
            crate::commands::session::list_sessions_value(&project_root),
        ),
        _ => Response::error(
            req.id.clone(),
            super::ERR_INVALID_PARAMS,
            &format!("Unknown tool: {}", name),
        ),
    }
}

fn session_response<T: serde::Serialize>(req: &Request, result: anyhow::Result<T>) -> Response {
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

#[cfg(test)]
mod tests {
    use super::*;
    use patina::project::{self, ProjectConfig};
    use tempfile::TempDir;

    fn setup_project() -> TempDir {
        let temp = TempDir::new().unwrap();
        let mut config = ProjectConfig::with_name("patina");
        config.adapters.allowed = vec!["opencode".to_string()];
        config.adapters.default = "opencode".to_string();
        project::save(temp.path(), &config).unwrap();
        std::fs::create_dir_all(temp.path().join(".patina/local/data")).unwrap();
        temp
    }

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

    #[test]
    fn session_start_handler_preserves_native_interface_semantics() {
        let temp = setup_project();
        let old_dir = std::env::current_dir().unwrap();
        let patina_home = temp.path().join("patina-home");
        std::fs::create_dir_all(&patina_home).unwrap();
        let old_patina_home = std::env::var_os("PATINA_HOME");
        std::env::set_current_dir(temp.path()).unwrap();
        unsafe {
            std::env::set_var("PATINA_HOME", &patina_home);
        }

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
                title: Some("Native MCP session".to_string()),
                session: None,
                adapter: Some("opencode".to_string()),
                note: None,
            },
        );

        std::env::set_current_dir(old_dir).unwrap();
        match old_patina_home {
            Some(value) => unsafe {
                std::env::set_var("PATINA_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("PATINA_HOME");
            },
        }

        let text = response.result.unwrap()["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        let payload: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(payload["interface"].as_str(), Some("opencode"));
        assert!(payload["active_session_path"].is_null());
    }
}
