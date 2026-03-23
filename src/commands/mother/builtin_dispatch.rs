use mother_crate::http_daemon::{json_error, HttpResponse};
use patina::secrets;

pub(super) fn handle_builtin_child_request(
    child_name: &str,
    action: &str,
    body: &[u8],
) -> Option<HttpResponse> {
    match (child_name, action) {
        ("spec-manager", "health") => Some(HttpResponse::json(
            200,
            &serde_json::json!({"status": "healthy"}),
        )),
        ("spec-manager", "dispatch") => {
            let payload = if body.is_empty() {
                serde_json::Value::Null
            } else {
                match serde_json::from_slice::<serde_json::Value>(body) {
                    Ok(v) => v,
                    Err(e) => return Some(json_error(400, &format!("Invalid JSON: {}", e))),
                }
            };
            let command_value = payload
                .get("command")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let command: crate::commands::spec::SpecCommands =
                match serde_json::from_value(command_value) {
                    Ok(command) => command,
                    Err(e) => {
                        return Some(json_error(
                            400,
                            &format!("Invalid spec-manager command payload: {}", e),
                        ));
                    }
                };

            match crate::commands::spec::execute_value(command) {
                Ok(value) => Some(HttpResponse::json(200, &value)),
                Err(e) => Some(json_error(400, &e.to_string())),
            }
        }
        ("lake-manager", "health") => Some(HttpResponse::json(
            200,
            &serde_json::json!({"status": "healthy"}),
        )),
        ("lake-manager", "dispatch") => {
            let payload = if body.is_empty() {
                serde_json::Value::Null
            } else {
                match serde_json::from_slice::<serde_json::Value>(body) {
                    Ok(v) => v,
                    Err(e) => return Some(json_error(400, &format!("Invalid JSON: {}", e))),
                }
            };
            let command_value = payload
                .get("command")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let command: crate::commands::lake::LakeCommands =
                match serde_json::from_value(command_value) {
                    Ok(command) => command,
                    Err(e) => {
                        return Some(json_error(
                            400,
                            &format!("Invalid lake-manager command payload: {}", e),
                        ));
                    }
                };

            match crate::commands::lake::execute_value(command) {
                Ok(value) => Some(HttpResponse::json(200, &value)),
                Err(e) => Some(json_error(400, &e.to_string())),
            }
        }
        ("doctor", "health") => Some(HttpResponse::json(
            200,
            &serde_json::json!({"status": "healthy"}),
        )),
        ("doctor", "run") => match crate::commands::doctor::execute_value() {
            Ok(value) => {
                let exit_code = value.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0);
                Some(HttpResponse::json(
                    200,
                    &serde_json::json!({
                        "child": "doctor",
                        "text": "",
                        "data": value,
                        "exit_code": exit_code,
                    }),
                ))
            }
            Err(e) => Some(json_error(400, &e.to_string())),
        },
        ("secrets-authority", "health") => Some(HttpResponse::json(
            200,
            &serde_json::json!({"status": "healthy"}),
        )),
        ("secrets-authority", "dispatch") => {
            let payload = if body.is_empty() {
                serde_json::Value::Null
            } else {
                match serde_json::from_slice::<serde_json::Value>(body) {
                    Ok(v) => v,
                    Err(e) => return Some(json_error(400, &format!("Invalid JSON: {}", e))),
                }
            };
            Some(handle_secrets_authority_dispatch(payload))
        }
        _ => None,
    }
}

fn handle_secrets_authority_dispatch(payload: serde_json::Value) -> HttpResponse {
    let op = payload
        .get("op")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    let project_root = payload
        .get("project_root")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from);

    match op {
        "add_secret" => {
            let name = match payload.get("name").and_then(|v| v.as_str()) {
                Some(value) => value,
                None => return json_error(400, "Missing 'name' for add_secret"),
            };
            let value = match payload.get("value").and_then(|v| v.as_str()) {
                Some(value) => value,
                None => return json_error(400, "Missing 'value' for add_secret"),
            };
            let env_name = payload.get("env").and_then(|v| v.as_str());
            let global = payload
                .get("global")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            match secrets::add_secret(name, value, env_name, global, project_root.as_deref()) {
                Ok(result) => HttpResponse::json(
                    200,
                    &serde_json::json!({
                        "status": "ok",
                        "env_var": result.env_var,
                        "created_vault": result.created_vault,
                    }),
                ),
                Err(error) => json_error(400, &error.to_string()),
            }
        }
        "remove_secret" => {
            let name = match payload.get("name").and_then(|v| v.as_str()) {
                Some(value) => value,
                None => return json_error(400, "Missing 'name' for remove_secret"),
            };
            let global = payload
                .get("global")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            match secrets::remove_secret(name, global, project_root.as_deref()) {
                Ok(()) => HttpResponse::json(200, &serde_json::json!({"status": "ok"})),
                Err(error) => json_error(400, &error.to_string()),
            }
        }
        "add_recipient" => {
            let key = match payload.get("key").and_then(|v| v.as_str()) {
                Some(value) => value,
                None => return json_error(400, "Missing 'key' for add_recipient"),
            };
            let Some(project_root) = project_root.as_deref() else {
                return json_error(400, "Missing 'project_root' for add_recipient");
            };

            match secrets::add_recipient(project_root, key) {
                Ok(()) => HttpResponse::json(200, &serde_json::json!({"status": "ok"})),
                Err(error) => json_error(400, &error.to_string()),
            }
        }
        "remove_recipient" => {
            let key = match payload.get("key").and_then(|v| v.as_str()) {
                Some(value) => value,
                None => return json_error(400, "Missing 'key' for remove_recipient"),
            };
            let Some(project_root) = project_root.as_deref() else {
                return json_error(400, "Missing 'project_root' for remove_recipient");
            };

            match secrets::remove_recipient(project_root, key) {
                Ok(()) => HttpResponse::json(200, &serde_json::json!({"status": "ok"})),
                Err(error) => json_error(400, &error.to_string()),
            }
        }
        "list_recipients" => {
            let Some(project_root) = project_root.as_deref() else {
                return json_error(400, "Missing 'project_root' for list_recipients");
            };

            match secrets::list_recipients(project_root) {
                Ok(recipients) => HttpResponse::json(
                    200,
                    &serde_json::json!({
                        "status": "ok",
                        "recipients": recipients,
                    }),
                ),
                Err(error) => json_error(400, &error.to_string()),
            }
        }
        "status" => match secrets::check_status(project_root.as_deref()) {
            Ok(status) => HttpResponse::json(
                200,
                &serde_json::json!({
                    "status": "ok",
                    "identity_source": status.identity_source.map(|s| s.to_string()),
                    "recipient_key": status.recipient_key,
                    "global": {
                        "exists": status.global.exists,
                        "secret_count": status.global.secret_count,
                        "recipient_count": status.global.recipient_count,
                        "secret_names": status.global.secret_names,
                    },
                    "project": status.project.map(|p| serde_json::json!({
                        "exists": p.exists,
                        "secret_count": p.secret_count,
                        "recipient_count": p.recipient_count,
                        "secret_names": p.secret_names,
                    })),
                }),
            ),
            Err(error) => json_error(400, &error.to_string()),
        },
        "lock_session" => match secrets::lock_session() {
            Ok(()) => HttpResponse::json(200, &serde_json::json!({"status": "ok"})),
            Err(error) => json_error(400, &error.to_string()),
        },
        _ => json_error(400, "Unknown secrets-authority operation"),
    }
}
