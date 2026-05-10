use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::*;
use crate::view_buffer::{
    ComposeViewRequest, ConnectWindowRequest, DisconnectWindowRequest, DisplayPattern,
    KillBufferRequest, LinkObservabilityGapRequest, MatureViewArtifactRequest, OpenBufferOutcome,
    OpenBufferRequest, OpenRequestShapeRequest, ResolveObservabilityGapRequest,
    ReviseViewShapeRequest, ViewDerivation, ViewShape,
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
struct ShapeRevisionsResponse<T> {
    revisions: T,
}

#[derive(Serialize)]
struct ShapeRevisionResponse<T> {
    revision: T,
}

#[derive(Serialize)]
struct DerivationsResponse<T> {
    derivations: T,
}

#[derive(Serialize)]
struct DerivationResponse<T> {
    derivation: T,
}

#[derive(Serialize)]
struct PatternsResponse<T> {
    patterns: T,
}

#[derive(Serialize)]
struct PatternResponse<T> {
    pattern: T,
}

#[derive(Serialize)]
struct MaturationEventsResponse<T> {
    events: T,
}

#[derive(Serialize)]
struct MaturationEventResponse<T> {
    event: T,
}

#[derive(Serialize)]
struct ObservabilityImprovementsResponse<T> {
    artifacts: T,
}

#[derive(Serialize)]
struct ObservabilityImprovementResponse<T> {
    artifact: T,
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

#[derive(Serialize)]
struct BufferPayloadResponse<T> {
    opened: T,
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

pub fn handle_list_view_shape_revisions(runtime: &(impl ViewBufferApi + ?Sized)) -> HttpResponse {
    match runtime.view_shape_revisions_list() {
        Ok(revisions) => HttpResponse::json(200, &ShapeRevisionsResponse { revisions }),
        Err(error) => json_error(500, &format!("list view shape revisions failed: {}", error)),
    }
}

pub fn handle_get_view_shape_revision(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(revision_id) = request.path.strip_prefix("/api/view-shape-revisions/") else {
        return json_error(400, "missing view shape revision id");
    };
    if revision_id.trim().is_empty() {
        return json_error(400, "missing view shape revision id");
    }

    match runtime.view_shape_revision_get(revision_id) {
        Ok(Some(revision)) => HttpResponse::json(200, &ShapeRevisionResponse { revision }),
        Ok(None) => json_error(
            404,
            &format!("unknown view shape revision '{}'", revision_id),
        ),
        Err(error) => json_error(500, &format!("get view shape revision failed: {}", error)),
    }
}

pub fn handle_revise_view_shape(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    // obligation: spec.mother-view-buffer-revision.mvbr6-api
    let Some(revise_request) = parse_json::<ReviseViewShapeRequest>(request) else {
        return json_error(400, "Invalid JSON");
    };
    if revise_request.shape_id.trim().is_empty() {
        return json_error(400, "missing view shape id");
    }
    if revise_request.reason.trim().is_empty() {
        return json_error(400, "missing view shape revision reason");
    }

    match runtime.view_shape_revise(revise_request) {
        Ok(outcome) => HttpResponse::json(200, &outcome),
        Err(error) => json_error(400, &format!("revise view shape failed: {}", error)),
    }
}

pub fn handle_list_view_derivations(runtime: &(impl ViewBufferApi + ?Sized)) -> HttpResponse {
    // obligation: spec.mother-view-maturation.mvmat6-api
    match runtime.view_derivations_list() {
        Ok(derivations) => HttpResponse::json(200, &DerivationsResponse { derivations }),
        Err(error) => json_error(500, &format!("list view derivations failed: {}", error)),
    }
}

pub fn handle_get_view_derivation(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(derivation_id) = request.path.strip_prefix("/api/view-derivations/") else {
        return json_error(400, "missing view derivation id");
    };
    if derivation_id.trim().is_empty() {
        return json_error(400, "missing view derivation id");
    }

    match runtime.view_derivation_get(derivation_id) {
        Ok(Some(derivation)) => HttpResponse::json(200, &DerivationResponse { derivation }),
        Ok(None) => json_error(404, &format!("unknown view derivation '{}'", derivation_id)),
        Err(error) => json_error(500, &format!("get view derivation failed: {}", error)),
    }
}

pub fn handle_upsert_view_derivation(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(derivation) = parse_json::<ViewDerivation>(request) else {
        return json_error(400, "Invalid JSON");
    };
    if derivation.derivation_id.trim().is_empty() {
        return json_error(400, "missing view derivation id");
    }
    if derivation.shape_id.trim().is_empty() {
        return json_error(400, "missing view shape id");
    }
    if derivation.label.trim().is_empty() {
        return json_error(400, "missing view derivation label");
    }
    if derivation.expression_ref.trim().is_empty() {
        return json_error(400, "missing view derivation expression ref");
    }

    match runtime.view_derivation_upsert(derivation) {
        Ok(derivation) => HttpResponse::json(200, &DerivationResponse { derivation }),
        Err(error) => json_error(400, &format!("upsert view derivation failed: {}", error)),
    }
}

pub fn handle_list_view_patterns(runtime: &(impl ViewBufferApi + ?Sized)) -> HttpResponse {
    // obligation: spec.mother-view-maturation.mvmat6-api
    match runtime.view_patterns_list() {
        Ok(patterns) => HttpResponse::json(200, &PatternsResponse { patterns }),
        Err(error) => json_error(500, &format!("list view patterns failed: {}", error)),
    }
}

pub fn handle_get_view_pattern(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(pattern_id) = request.path.strip_prefix("/api/view-patterns/") else {
        return json_error(400, "missing display pattern id");
    };
    if pattern_id.trim().is_empty() {
        return json_error(400, "missing display pattern id");
    }

    match runtime.view_pattern_get(pattern_id) {
        Ok(Some(pattern)) => HttpResponse::json(200, &PatternResponse { pattern }),
        Ok(None) => json_error(404, &format!("unknown display pattern '{}'", pattern_id)),
        Err(error) => json_error(500, &format!("get view pattern failed: {}", error)),
    }
}

pub fn handle_upsert_view_pattern(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(pattern) = parse_json::<DisplayPattern>(request) else {
        return json_error(400, "Invalid JSON");
    };
    if pattern.pattern_id.trim().is_empty() {
        return json_error(400, "missing display pattern id");
    }
    if pattern.shape_id.trim().is_empty() {
        return json_error(400, "missing view shape id");
    }

    match runtime.view_pattern_upsert(pattern) {
        Ok(pattern) => HttpResponse::json(200, &PatternResponse { pattern }),
        Err(error) => json_error(400, &format!("upsert view pattern failed: {}", error)),
    }
}

pub fn handle_list_view_maturation_events(runtime: &(impl ViewBufferApi + ?Sized)) -> HttpResponse {
    // obligation: spec.mother-view-maturation.mvmat6-api
    match runtime.view_maturation_events_list() {
        Ok(events) => HttpResponse::json(200, &MaturationEventsResponse { events }),
        Err(error) => json_error(
            500,
            &format!("list view maturation events failed: {}", error),
        ),
    }
}

pub fn handle_get_view_maturation_event(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(maturation_id) = request.path.strip_prefix("/api/view-maturation-events/") else {
        return json_error(400, "missing view maturation id");
    };
    if maturation_id.trim().is_empty() {
        return json_error(400, "missing view maturation id");
    }

    match runtime.view_maturation_event_get(maturation_id) {
        Ok(Some(event)) => HttpResponse::json(200, &MaturationEventResponse { event }),
        Ok(None) => json_error(404, &format!("unknown view maturation '{}'", maturation_id)),
        Err(error) => json_error(500, &format!("get view maturation event failed: {}", error)),
    }
}

pub fn handle_record_view_maturation(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    // obligation: spec.mother-view-maturation.mvmat6-api
    let Some(maturation_request) = parse_json::<MatureViewArtifactRequest>(request) else {
        return json_error(400, "Invalid JSON");
    };

    match runtime.view_maturation_record(maturation_request) {
        Ok(outcome) => HttpResponse::json(200, &outcome),
        Err(error) => json_error(400, &format!("record view maturation failed: {}", error)),
    }
}

pub fn handle_list_view_observability_improvements(
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    // obligation: spec.mother-view-maturation.mvmat6-api
    match runtime.view_observability_improvements_list() {
        Ok(artifacts) => HttpResponse::json(200, &ObservabilityImprovementsResponse { artifacts }),
        Err(error) => json_error(
            500,
            &format!("list view observability improvements failed: {}", error),
        ),
    }
}

pub fn handle_get_view_observability_improvement(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(artifact_id) = request
        .path
        .strip_prefix("/api/view-observability-improvements/")
    else {
        return json_error(400, "missing observability improvement artifact id");
    };
    if artifact_id.trim().is_empty() {
        return json_error(400, "missing observability improvement artifact id");
    }

    match runtime.view_observability_improvement_get(artifact_id) {
        Ok(Some(artifact)) => {
            HttpResponse::json(200, &ObservabilityImprovementResponse { artifact })
        }
        Ok(None) => json_error(
            404,
            &format!(
                "unknown observability improvement artifact '{}'",
                artifact_id
            ),
        ),
        Err(error) => json_error(
            500,
            &format!("get view observability improvement failed: {}", error),
        ),
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

pub fn handle_get_view_buffer_payload(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    let Some(buffer_id) = request.path.strip_prefix("/api/view-buffers/") else {
        return json_error(404, "view buffer payload not found");
    };
    let Some(buffer_id) = buffer_id.strip_suffix("/payload") else {
        return json_error(404, "view buffer payload not found");
    };
    if buffer_id.is_empty() {
        return json_error(404, "view buffer payload not found");
    }

    match runtime.view_buffer_payload_get(buffer_id) {
        Ok(opened) => HttpResponse::json(200, &BufferPayloadResponse { opened }),
        Err(error) => json_error(400, &format!("get view buffer payload failed: {}", error)),
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

pub fn handle_get_view_buffer_gap(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    // obligation: spec.mother-view-observability-workflow.mvow5-api
    let Some(gap_id) = request.path.strip_prefix("/api/view-buffers/gaps/") else {
        return json_error(400, "missing view observability gap id");
    };
    if gap_id.trim().is_empty() {
        return json_error(400, "missing view observability gap id");
    }

    match runtime.view_buffer_gap_get(gap_id) {
        Ok(Some(gap)) => HttpResponse::json(200, &serde_json::json!({"gap": gap})),
        Ok(None) => json_error(404, &format!("unknown view observability gap '{}'", gap_id)),
        Err(error) => json_error(500, &format!("get view buffer gap failed: {}", error)),
    }
}

pub fn handle_link_view_buffer_gap_work_item(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    // obligation: spec.mother-view-observability-workflow.mvow5-api
    let Some(link_request) = parse_json::<LinkObservabilityGapRequest>(request) else {
        return json_error(400, "Invalid JSON");
    };
    if link_request.gap_id.trim().is_empty() {
        return json_error(400, "missing view observability gap id");
    }
    if link_request.work_item_id.trim().is_empty() {
        return json_error(400, "missing work item id");
    }

    match runtime.view_buffer_gap_link_work_item(link_request) {
        Ok(gap) => HttpResponse::json(200, &serde_json::json!({"gap": gap})),
        Err(error) => json_error(400, &format!("link view buffer gap failed: {}", error)),
    }
}

pub fn handle_resolve_view_buffer_gap(
    request: &HttpRequest,
    runtime: &(impl ViewBufferApi + ?Sized),
) -> HttpResponse {
    // obligation: spec.mother-view-observability-workflow.mvow5-api
    let Some(resolve_request) = parse_json::<ResolveObservabilityGapRequest>(request) else {
        return json_error(400, "Invalid JSON");
    };
    if resolve_request.gap_id.trim().is_empty() {
        return json_error(400, "missing view observability gap id");
    }

    match runtime.view_buffer_gap_resolve(resolve_request) {
        Ok(gap) => HttpResponse::json(200, &serde_json::json!({"gap": gap})),
        Err(error) => json_error(400, &format!("resolve view buffer gap failed: {}", error)),
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
