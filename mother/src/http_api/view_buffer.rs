use serde::Serialize;

use super::*;
use crate::view_buffer::{
    ConnectWindowRequest, DisconnectWindowRequest, KillBufferRequest, OpenBufferOutcome,
    OpenBufferRequest,
};

#[derive(Serialize)]
struct BuffersResponse<T> {
    buffers: T,
}

#[derive(Serialize)]
struct WindowsResponse<T> {
    windows: T,
}

#[derive(Serialize)]
struct GapsResponse<T> {
    gaps: T,
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
