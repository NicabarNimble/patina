use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::*;
use crate::view_buffer::{
    ComposeViewRequest, ConnectWindowRequest, DisconnectWindowRequest, KillBufferRequest,
    OpenBufferOutcome, OpenBufferRequest, OpenRequestShapeRequest, ViewShape,
};

#[derive(Serialize)]
struct ShapesResponse<T> {
    shapes: T,
}

#[derive(Serialize)]
struct ShapeResponse<T> {
    shape: T,
}

#[derive(Serialize)]
struct RequestsResponse<T> {
    requests: T,
}

#[derive(Serialize)]
struct RequestResponse<T> {
    request: T,
}

#[derive(Serialize)]
struct RequestDetailsResponse<T> {
    details: T,
}

#[derive(Serialize)]
struct RequestDetailResponse<T> {
    detail: T,
}

#[derive(Serialize)]
struct BuffersResponse<T> {
    buffers: T,
}

#[derive(Debug, Deserialize)]
struct DeactivateShapeRequest {
    shape_id: String,
}

#[derive(Serialize)]
struct WindowsResponse<T> {
    windows: T,
}

#[derive(Serialize)]
struct GapsResponse<T> {
    gaps: T,
}

pub fn handle_list_view_shapes(runtime: &(impl ViewBufferApi + ?Sized)) -> HttpResponse {
    match runtime.view_shapes_list() {
        Ok(shapes) => HttpResponse::json(200, &ShapesResponse { shapes }),
        Err(error) => json_error(500, &format!("list view shapes failed: {}", error)),
    }
}

pub fn handle_get_view_shape(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(shape_id) = request.path.strip_prefix("/api/view-shapes/") else {
        return json_error(400, "missing view shape id");
    };
    if shape_id.trim().is_empty() {
        return json_error(400, "missing view shape id");
    }

    match runtime.view_shape_get(shape_id) {
        Ok(Some(shape)) => HttpResponse::json(200, &ShapeResponse { shape }),
        Ok(None) => json_error(404, &format!("unknown view shape '{}'", shape_id)),
        Err(error) => json_error(500, &format!("get view shape failed: {}", error)),
    }
}

pub fn handle_upsert_view_shape(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let shape = match parse_shape_json(request) {
        Ok(shape) => shape,
        Err(error) => return json_error(400, &error),
    };

    match runtime.view_shape_upsert(shape) {
        Ok(shape) => HttpResponse::json(200, &ShapeResponse { shape }),
        Err(error) => json_error(500, &format!("upsert view shape failed: {}", error)),
    }
}

pub fn handle_deactivate_view_shape(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(deactivate_request) = parse_json::<DeactivateShapeRequest>(request) else {
        return json_error(400, "Invalid JSON");
    };
    if deactivate_request.shape_id.trim().is_empty() {
        return json_error(400, "missing view shape id");
    }

    match runtime.view_shape_deactivate(&deactivate_request.shape_id) {
        Ok(true) => HttpResponse::json(
            200,
            &serde_json::json!({"shape_id": deactivate_request.shape_id, "active": false}),
        ),
        Ok(false) => json_error(
            404,
            &format!("unknown view shape '{}'", deactivate_request.shape_id),
        ),
        Err(error) => json_error(500, &format!("deactivate view shape failed: {}", error)),
    }
}

pub fn handle_list_view_requests(runtime: &(impl ViewBufferApi + ?Sized)) -> HttpResponse {
    match runtime.view_requests_list() {
        Ok(requests) => HttpResponse::json(200, &RequestsResponse { requests }),
        Err(error) => json_error(500, &format!("list view requests failed: {}", error)),
    }
}

pub fn handle_get_view_request(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(request_id) = request.path.strip_prefix("/api/view-requests/") else {
        return json_error(400, "missing view request id");
    };
    if request_id.trim().is_empty() {
        return json_error(400, "missing view request id");
    }

    match runtime.view_request_get(request_id) {
        Ok(Some(view_request)) => HttpResponse::json(
            200,
            &RequestResponse {
                request: view_request,
            },
        ),
        Ok(None) => json_error(404, &format!("unknown view request '{}'", request_id)),
        Err(error) => json_error(500, &format!("get view request failed: {}", error)),
    }
}

pub fn handle_list_view_request_details(runtime: &(impl ViewBufferApi + ?Sized)) -> HttpResponse {
    // obligation: spec.mother-view-request-ux.mvru3-detail-api
    match runtime.view_request_details_list() {
        Ok(details) => HttpResponse::json(200, &RequestDetailsResponse { details }),
        Err(error) => json_error(500, &format!("list view request details failed: {}", error)),
    }
}

pub fn handle_get_view_request_detail(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    // obligation: spec.mother-view-request-ux.mvru3-detail-api
    let Some(request_id) = request.path.strip_prefix("/api/view-requests/") else {
        return json_error(400, "missing view request id");
    };
    let Some(request_id) = request_id.strip_suffix("/detail") else {
        return json_error(400, "missing view request detail suffix");
    };
    if request_id.trim().is_empty() {
        return json_error(400, "missing view request id");
    }

    match runtime.view_request_detail_get(request_id) {
        Ok(Some(detail)) => HttpResponse::json(200, &RequestDetailResponse { detail }),
        Ok(None) => json_error(404, &format!("unknown view request '{}'", request_id)),
        Err(error) => json_error(500, &format!("get view request detail failed: {}", error)),
    }
}

pub fn handle_compose_view_request(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(compose_request) = parse_json::<ComposeViewRequest>(request) else {
        return json_error(400, "Invalid JSON");
    };
    if compose_request.raw_request.trim().is_empty() {
        return json_error(400, "raw display request must not be empty");
    }

    match runtime.view_request_compose(compose_request) {
        Ok(composed) => HttpResponse::json(200, &composed),
        Err(error) => json_error(500, &format!("compose view request failed: {}", error)),
    }
}

pub fn handle_open_view_request_shape(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    // obligation: spec.mother-view-request-ux.mvru4-open-linked-shape-action
    let Some(open_request) = parse_json::<OpenRequestShapeRequest>(request) else {
        return json_error(400, "Invalid JSON");
    };
    if open_request.request_id.trim().is_empty() {
        return json_error(400, "missing view request id");
    }
    if open_request
        .shape_id
        .as_deref()
        .is_some_and(|shape_id| shape_id.trim().is_empty())
    {
        return json_error(400, "missing view shape id");
    }

    match runtime.view_request_open_shape(open_request) {
        Ok(Some(outcome)) => HttpResponse::json(200, &outcome),
        Ok(None) => json_error(404, "unknown view request"),
        Err(error) => json_error(400, &format!("open view request shape failed: {}", error)),
    }
}

pub fn handle_list_view_buffers(runtime: &(impl ViewBufferApi + ?Sized)) -> HttpResponse {
    match runtime.view_buffers_list() {
        Ok(buffers) => HttpResponse::json(200, &BuffersResponse { buffers }),
        Err(error) => json_error(500, &format!("list view buffers failed: {}", error)),
    }
}

pub fn handle_open_view_buffer(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(open_request) = parse_json::<OpenBufferRequest>(request) else {
        return json_error(400, "Invalid JSON");
    };

    match runtime.view_buffer_open(open_request) {
        Ok(OpenBufferOutcome::Opened(opened)) => HttpResponse::json(200, &opened),
        Ok(OpenBufferOutcome::ObservabilityGap(gap)) => HttpResponse::json(
            409,
            &serde_json::json!({
                "error": "observability_gap",
                "gap": gap,
            }),
        ),
        Err(error) => json_error(500, &format!("open view buffer failed: {}", error)),
    }
}

pub fn handle_connect_view_buffer_window(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(connect_request) = parse_json::<ConnectWindowRequest>(request) else {
        return json_error(400, "Invalid JSON");
    };

    match runtime.view_buffer_connect_window(connect_request) {
        Ok(window) => HttpResponse::json(200, &window),
        Err(error) => json_error(
            500,
            &format!("connect view buffer window failed: {}", error),
        ),
    }
}

pub fn handle_disconnect_view_buffer_window(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(disconnect_request) = parse_json::<DisconnectWindowRequest>(request) else {
        return json_error(400, "Invalid JSON");
    };

    match runtime.view_buffer_disconnect_window(disconnect_request) {
        Ok(window) => HttpResponse::json(200, &window),
        Err(error) => json_error(
            500,
            &format!("disconnect view buffer window failed: {}", error),
        ),
    }
}

pub fn handle_kill_view_buffer(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(kill_request) = parse_json::<KillBufferRequest>(request) else {
        return json_error(400, "Invalid JSON");
    };

    match runtime.view_buffer_kill(kill_request) {
        Ok(buffer) => HttpResponse::json(200, &buffer),
        Err(error) => json_error(500, &format!("kill view buffer failed: {}", error)),
    }
}

pub fn handle_list_view_buffer_gaps(runtime: &(impl ViewBufferApi + ?Sized)) -> HttpResponse {
    match runtime.view_buffer_gaps_list() {
        Ok(gaps) => HttpResponse::json(200, &GapsResponse { gaps }),
        Err(error) => json_error(500, &format!("list view buffer gaps failed: {}", error)),
    }
}

pub fn handle_list_view_buffer_windows(runtime: &(impl ViewBufferApi + ?Sized)) -> HttpResponse {
    match runtime.view_buffer_windows_list() {
        Ok(windows) => HttpResponse::json(200, &WindowsResponse { windows }),
        Err(error) => json_error(500, &format!("list view buffer windows failed: {}", error)),
    }
}

fn parse_json<T>(request: &HttpRequest) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    if request.body.is_empty() {
        return None;
    }
    serde_json::from_slice(&request.body).ok()
}

fn parse_shape_json(request: &HttpRequest) -> Result<ViewShape, String> {
    if request.body.is_empty() {
        return Err("Invalid JSON".to_string());
    }
    let value: serde_json::Value =
        serde_json::from_slice(&request.body).map_err(|_| "Invalid JSON".to_string())?;
    reject_unknown_shape_fields(&value)?;
    serde_json::from_value(value).map_err(|_| "Invalid view shape".to_string())
}

fn reject_unknown_shape_fields(value: &serde_json::Value) -> Result<(), String> {
    let Some(object) = value.as_object() else {
        return Err("Invalid view shape".to_string());
    };
    let allowed: BTreeSet<&str> = [
        "shape_id",
        "title",
        "source_ref",
        "scope",
        "version",
        "active",
        "major_mode",
        "minor_modes",
        "maturity",
        "payload_contract",
        "payload_version",
        "vision_id",
        "project_uid",
        "replaced_by",
        "requirements",
    ]
    .into_iter()
    .collect();
    for key in object.keys() {
        if !allowed.contains(key.as_str()) {
            return Err(format!("unsupported view shape field '{}'", key));
        }
    }

    if let Some(requirements) = object.get("requirements") {
        let Some(requirements) = requirements.as_array() else {
            return Err("Invalid view shape".to_string());
        };
        let allowed_requirement: BTreeSet<&str> =
            ["fact_path", "required", "purpose"].into_iter().collect();
        for requirement in requirements {
            let Some(requirement) = requirement.as_object() else {
                return Err("Invalid view shape".to_string());
            };
            for key in requirement.keys() {
                if !allowed_requirement.contains(key.as_str()) {
                    return Err(format!("unsupported view requirement field '{}'", key));
                }
            }
        }
    }

    Ok(())
}
