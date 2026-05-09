use super::*;

pub fn handle_bridge_translate(
    request: &HttpRequest,
    runtime: &(impl BridgeApi + ?Sized),
) -> HttpResponse {
    if request.body.is_empty() {
        return json_error(400, "Missing request body");
    }

    let bridge_request: crate::bridge::BridgeRequest = match serde_json::from_slice(&request.body) {
        Ok(value) => value,
        Err(error) => return json_error(400, &format!("Invalid JSON: {}", error)),
    };

    match runtime.bridge_translate(bridge_request) {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(error) => json_error(500, &format!("bridge translate failed: {}", error)),
    }
}
