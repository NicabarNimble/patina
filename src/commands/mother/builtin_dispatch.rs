use mother_crate::http_daemon::{json_error, HttpResponse};
use mother_crate::secrets_authority_api as secrets_api;

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
    secrets_api::dispatch(
        payload,
        &mother_crate::secrets_authority_backend::MotherSecretsAuthorityBackend,
    )
}
