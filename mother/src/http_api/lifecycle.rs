use serde::Deserialize;

use super::*;

#[derive(Deserialize)]
struct LifecycleNameRequest {
    name: String,
}

#[derive(Deserialize, Default)]
struct LifecycleRefreshRequest {}

#[derive(Deserialize, Default)]
struct LifecycleWarmupRequest {}

pub(super) fn lifecycle_error(status: u16, code: &str, detail: &str) -> HttpResponse {
    HttpResponse::json(
        status,
        &serde_json::json!({
            "error": code,
            "code": status,
            "detail": detail,
        }),
    )
}

pub(super) fn lifecycle_error_from_anyhow(error: &anyhow::Error) -> HttpResponse {
    let detail = error.to_string();
    if let Some(value) = detail.strip_prefix("invalid_request: ") {
        return lifecycle_error(400, "invalid_request", value);
    }
    if let Some(value) = detail.strip_prefix("child_not_found: ") {
        return lifecycle_error(404, "child_not_found", value);
    }
    if let Some(value) = detail.strip_prefix("pando_not_found: ") {
        return lifecycle_error(404, "pando_not_found", value);
    }
    if let Some(value) = detail.strip_prefix("operation_in_progress: ") {
        return lifecycle_error(409, "operation_in_progress", value);
    }
    if let Some(value) = detail.strip_prefix("resource_exhausted: ") {
        return lifecycle_error(429, "resource_exhausted", value);
    }
    if let Some(value) = detail.strip_prefix("internal_error: ") {
        return lifecycle_error(500, "internal_error", value);
    }
    lifecycle_error(500, "internal_error", &detail)
}

pub fn handle_lifecycle_load_pando(
    request: &HttpRequest,
    runtime: &dyn ApiRuntime,
) -> HttpResponse {
    if request.body.is_empty() {
        return lifecycle_error(400, "invalid_request", "missing request body");
    }
    let payload: LifecycleNameRequest = match serde_json::from_slice(&request.body) {
        Ok(v) => v,
        Err(e) => {
            return lifecycle_error(400, "invalid_request", &format!("invalid JSON: {}", e));
        }
    };
    if payload.name.trim().is_empty() {
        return lifecycle_error(400, "invalid_request", "name is required");
    }
    match runtime.lifecycle_load_pando(&payload.name) {
        Ok(response) => HttpResponse::json(200, &response),
        Err(e) => lifecycle_error_from_anyhow(&e),
    }
}

pub fn handle_lifecycle_refresh(request: &HttpRequest, runtime: &dyn ApiRuntime) -> HttpResponse {
    if !request.body.is_empty()
        && serde_json::from_slice::<LifecycleRefreshRequest>(&request.body).is_err()
    {
        return lifecycle_error(400, "invalid_request", "invalid JSON");
    }
    match runtime.lifecycle_refresh() {
        Ok(response) => HttpResponse::json(200, &response),
        Err(e) => lifecycle_error_from_anyhow(&e),
    }
}

pub fn handle_lifecycle_reload_child(
    request: &HttpRequest,
    runtime: &dyn ApiRuntime,
) -> HttpResponse {
    if request.body.is_empty() {
        return lifecycle_error(400, "invalid_request", "missing request body");
    }
    let payload: LifecycleNameRequest = match serde_json::from_slice(&request.body) {
        Ok(v) => v,
        Err(e) => {
            return lifecycle_error(400, "invalid_request", &format!("invalid JSON: {}", e));
        }
    };
    if payload.name.trim().is_empty() {
        return lifecycle_error(400, "invalid_request", "name is required");
    }
    match runtime.lifecycle_reload_child(&payload.name) {
        Ok(response) => HttpResponse::json(200, &response),
        Err(e) => lifecycle_error_from_anyhow(&e),
    }
}

pub fn handle_lifecycle_warmup_children(
    request: &HttpRequest,
    runtime: &dyn ApiRuntime,
) -> HttpResponse {
    if !request.body.is_empty()
        && serde_json::from_slice::<LifecycleWarmupRequest>(&request.body).is_err()
    {
        return lifecycle_error(400, "invalid_request", "invalid JSON");
    }

    match runtime.lifecycle_warmup_children() {
        Ok(response) => HttpResponse::json(200, &response),
        Err(e) => lifecycle_error_from_anyhow(&e),
    }
}
