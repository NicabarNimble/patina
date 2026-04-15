use patina_protocol::{BuiltinChild, BuiltinChildAction, BuiltinChildRequest};

use super::*;

pub fn handle_child_request(
    request: &HttpRequest,
    runtime: &(impl ChildApi + ?Sized),
) -> HttpResponse {
    let parts: Vec<&str> = request.path[1..].split('/').collect();
    if parts.len() != 3 {
        return json_error(400, "Expected /child/{name}/{action}");
    }
    let child_name = parts[1];
    let action = parts[2];

    if BuiltinChild::from_route(child_name).is_some() {
        match BuiltinChildRequest::from_http_parts(child_name, action, &request.body) {
            Ok(builtin) => {
                return match builtin.action {
                    BuiltinChildAction::Health => {
                        HttpResponse::json(200, &serde_json::json!({ "status": "healthy" }))
                    }
                    BuiltinChildAction::SpecDispatch(dispatch) => {
                        match runtime.builtin_spec_dispatch(dispatch) {
                            Ok(payload) => HttpResponse::json(200, &payload),
                            Err(e) => json_error(400, &e.to_string()),
                        }
                    }
                    BuiltinChildAction::LakeDispatch(dispatch) => {
                        match runtime.builtin_lake_dispatch(dispatch) {
                            Ok(payload) => HttpResponse::json(200, &payload),
                            Err(e) => json_error(400, &e.to_string()),
                        }
                    }
                    BuiltinChildAction::DoctorRun(_) => match runtime.builtin_doctor_run() {
                        Ok(result) => HttpResponse::json(
                            200,
                            &serde_json::json!({
                                "child": "doctor",
                                "text": "",
                                "data": result.data,
                                "exit_code": result.exit_code,
                            }),
                        ),
                        Err(e) => json_error(400, &e.to_string()),
                    },
                    BuiltinChildAction::SecretsDispatch(dispatch) => {
                        runtime.builtin_secrets_dispatch(dispatch.operation.into_payload())
                    }
                };
            }
            Err(message) if message.starts_with("Unsupported action") => {}
            Err(message) => return json_error(400, &message),
        }
    }

    if action == "health" {
        return match runtime.child_health(child_name) {
            Ok(health) => {
                let status = match health {
                    crate::ChildHealth::Healthy => "healthy",
                    crate::ChildHealth::Degraded(_) => "degraded",
                    crate::ChildHealth::Unhealthy(_) => "unhealthy",
                };
                HttpResponse::json(200, &serde_json::json!({ "status": status }))
            }
            Err(e) => json_error(404, &e.to_string()),
        };
    }

    let payload = if request.body.is_empty() {
        serde_json::Value::Null
    } else {
        match serde_json::from_slice(&request.body) {
            Ok(v) => v,
            Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
        }
    };

    if action == "call" {
        let operation_id = payload
            .get("operation_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();
        if operation_id.is_empty() {
            return json_error(400, "Missing operation_id for child typed call");
        }
        let args = payload
            .get("args")
            .cloned()
            .unwrap_or(serde_json::json!([]));
        let correlation = match payload.get("correlation") {
            Some(value) => match serde_json::from_value::<crate::CallCorrelation>(value.clone()) {
                Ok(parsed) => Some(parsed),
                Err(error) => {
                    return json_error(400, &format!("Invalid correlation payload: {}", error));
                }
            },
            None => None,
        };
        return match runtime.child_call(child_name, operation_id, args, correlation) {
            Ok(payload) => HttpResponse::json(200, &payload),
            Err(e) => json_error(404, &e.to_string()),
        };
    }

    match runtime.child_handle(child_name, action.to_string(), payload) {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(e) => json_error(404, &e.to_string()),
    }
}
