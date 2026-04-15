use serde::Deserialize;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleErrorCode {
    InvalidRequest,
    ChildNotFound,
    PandoNotFound,
    OperationInProgress,
    ResourceExhausted,
    InternalError,
}

impl LifecycleErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::ChildNotFound => "child_not_found",
            Self::PandoNotFound => "pando_not_found",
            Self::OperationInProgress => "operation_in_progress",
            Self::ResourceExhausted => "resource_exhausted",
            Self::InternalError => "internal_error",
        }
    }

    pub fn status_code(self) -> u16 {
        match self {
            Self::InvalidRequest => 400,
            Self::ChildNotFound | Self::PandoNotFound => 404,
            Self::OperationInProgress => 409,
            Self::ResourceExhausted => 429,
            Self::InternalError => 500,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LifecycleError {
    code: LifecycleErrorCode,
    detail: String,
}

impl LifecycleError {
    pub fn new(code: LifecycleErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    pub fn invalid_request(detail: impl Into<String>) -> Self {
        Self::new(LifecycleErrorCode::InvalidRequest, detail)
    }

    pub fn child_not_found(detail: impl Into<String>) -> Self {
        Self::new(LifecycleErrorCode::ChildNotFound, detail)
    }

    pub fn pando_not_found(detail: impl Into<String>) -> Self {
        Self::new(LifecycleErrorCode::PandoNotFound, detail)
    }

    pub fn operation_in_progress(detail: impl Into<String>) -> Self {
        Self::new(LifecycleErrorCode::OperationInProgress, detail)
    }

    pub fn resource_exhausted(detail: impl Into<String>) -> Self {
        Self::new(LifecycleErrorCode::ResourceExhausted, detail)
    }

    pub fn internal_error(detail: impl Into<String>) -> Self {
        Self::new(LifecycleErrorCode::InternalError, detail)
    }

    pub fn code(&self) -> LifecycleErrorCode {
        self.code
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.detail)
    }
}

impl std::error::Error for LifecycleError {}

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
    if let Some(typed) = error.downcast_ref::<LifecycleError>() {
        return lifecycle_error(
            typed.code().status_code(),
            typed.code().as_str(),
            typed.detail(),
        );
    }

    // Legacy string-prefix fallback for compatibility with call sites not yet migrated.
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
    runtime: &(impl LifecycleApi + ?Sized),
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

pub fn handle_lifecycle_refresh(
    request: &HttpRequest,
    runtime: &(impl LifecycleApi + ?Sized),
) -> HttpResponse {
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
    runtime: &(impl LifecycleApi + ?Sized),
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
    runtime: &(impl LifecycleApi + ?Sized),
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
