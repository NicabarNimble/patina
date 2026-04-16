use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Clone, Deserialize)]
pub struct RivetDispatchRequest {
    pub child: String,
    pub operation_id: String,
    #[serde(default = "default_rivet_dispatch_args")]
    pub args: serde_json::Value,
    #[serde(default)]
    pub correlation: Option<crate::CallCorrelation>,
    #[serde(default)]
    pub delivery: Option<crate::pando::PandoDeliveryPolicy>,
    #[serde(default, rename = "dead-letter", alias = "dead_letter")]
    pub dead_letter: Option<RivetDispatchDeadLetter>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RivetDispatchDeadLetter {
    pub child: String,
    #[serde(default)]
    pub operation_id: Option<String>,
}

impl RivetDispatchRequest {
    pub fn delivery_policy(&self) -> crate::pando::PandoDeliveryPolicy {
        self.delivery
            .unwrap_or(crate::pando::PandoDeliveryPolicy::Required)
    }
}

fn default_rivet_dispatch_args() -> serde_json::Value {
    serde_json::json!([])
}

pub fn handle_rivet_dispatch(
    request: &HttpRequest,
    runtime: &(impl RivetApi + ?Sized),
) -> HttpResponse {
    if request.body.is_empty() {
        return super::lifecycle::lifecycle_error(400, "invalid_request", "missing request body");
    }

    let payload: RivetDispatchRequest = match serde_json::from_slice(&request.body) {
        Ok(value) => value,
        Err(error) => {
            return super::lifecycle::lifecycle_error(
                400,
                "invalid_request",
                &format!("invalid JSON: {}", error),
            );
        }
    };

    if payload.child.trim().is_empty() {
        return super::lifecycle::lifecycle_error(400, "invalid_request", "child is required");
    }
    if payload.operation_id.trim().is_empty() {
        return super::lifecycle::lifecycle_error(
            400,
            "invalid_request",
            "operation_id is required",
        );
    }
    if matches!(
        payload.delivery_policy(),
        crate::pando::PandoDeliveryPolicy::DeadLetter
    ) {
        let Some(dead_letter) = payload.dead_letter.as_ref() else {
            return super::lifecycle::lifecycle_error(
                400,
                "invalid_request",
                "dead-letter policy requires dead-letter target",
            );
        };
        if dead_letter.child.trim().is_empty() {
            return super::lifecycle::lifecycle_error(
                400,
                "invalid_request",
                "dead-letter child is required",
            );
        }
    }

    match runtime.rivet_dispatch(payload) {
        Ok(response) => HttpResponse::json(200, &response),
        Err(error) => super::lifecycle::lifecycle_error_from_anyhow(&error),
    }
}
