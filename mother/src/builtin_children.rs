use crate::http_daemon::{json_error, HttpResponse};
use crate::secrets_authority_api;

pub trait BuiltinChildExecutor {
    fn spec_dispatch(
        &self,
        command_payload: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value>;
    fn lake_dispatch(
        &self,
        command_payload: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value>;
    fn doctor_run(&self) -> anyhow::Result<serde_json::Value>;
}

pub fn handle_builtin_child_request(
    child_name: &str,
    action: &str,
    body: &[u8],
    executor: &dyn BuiltinChildExecutor,
    secrets_backend: &dyn secrets_authority_api::SecretsAuthorityBackend,
) -> Option<HttpResponse> {
    match (child_name, action) {
        ("spec-manager", "health") => Some(HttpResponse::json(
            200,
            &serde_json::json!({"status": "healthy"}),
        )),
        ("spec-manager", "dispatch") => {
            let payload = match parse_payload(body) {
                Ok(payload) => payload,
                Err(error) => return Some(error),
            };
            let command_value = payload
                .get("command")
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            match executor.spec_dispatch(command_value) {
                Ok(value) => Some(HttpResponse::json(200, &value)),
                Err(e) => Some(json_error(400, &e.to_string())),
            }
        }
        ("lake-manager", "health") => Some(HttpResponse::json(
            200,
            &serde_json::json!({"status": "healthy"}),
        )),
        ("lake-manager", "dispatch") => {
            let payload = match parse_payload(body) {
                Ok(payload) => payload,
                Err(error) => return Some(error),
            };
            let command_value = payload
                .get("command")
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            match executor.lake_dispatch(command_value) {
                Ok(value) => Some(HttpResponse::json(200, &value)),
                Err(e) => Some(json_error(400, &e.to_string())),
            }
        }
        ("doctor", "health") => Some(HttpResponse::json(
            200,
            &serde_json::json!({"status": "healthy"}),
        )),
        ("doctor", "run") => match executor.doctor_run() {
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
            let payload = match parse_payload(body) {
                Ok(payload) => payload,
                Err(error) => return Some(error),
            };
            Some(secrets_authority_api::dispatch(payload, secrets_backend))
        }
        _ => None,
    }
}

fn parse_payload(body: &[u8]) -> Result<serde_json::Value, HttpResponse> {
    if body.is_empty() {
        return Ok(serde_json::Value::Null);
    }

    match serde_json::from_slice::<serde_json::Value>(body) {
        Ok(v) => Ok(v),
        Err(e) => Err(json_error(400, &format!("Invalid JSON: {}", e))),
    }
}
