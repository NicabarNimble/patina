use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct InterfaceControlCorrelation {
    #[serde(default)]
    pub project_uid: Option<String>,
    #[serde(default)]
    pub interface: Option<String>,
    #[serde(default)]
    pub launch_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InterfaceControlCallRequest {
    pub operation_id: String,
    #[serde(default = "default_interface_call_args")]
    pub args: serde_json::Value,
    #[serde(default)]
    pub correlation: Option<InterfaceControlCorrelation>,
}

fn default_interface_call_args() -> serde_json::Value {
    serde_json::json!([])
}

pub fn handle_interface_control_call(
    request: &HttpRequest,
    runtime: &(impl InterfaceControlApi + ?Sized),
) -> HttpResponse {
    if request.body.is_empty() {
        return super::lifecycle::lifecycle_error(400, "invalid_request", "missing request body");
    }

    let payload: InterfaceControlCallRequest = match serde_json::from_slice(&request.body) {
        Ok(value) => value,
        Err(error) => {
            return super::lifecycle::lifecycle_error(
                400,
                "invalid_request",
                &format!("invalid JSON: {}", error),
            );
        }
    };

    if payload.operation_id.trim().is_empty() {
        return super::lifecycle::lifecycle_error(
            400,
            "invalid_request",
            "operation_id is required",
        );
    }

    match runtime.interface_control_call(payload) {
        Ok(response) => HttpResponse::json(200, &response),
        Err(error) => super::lifecycle::lifecycle_error_from_anyhow(&error),
    }
}
