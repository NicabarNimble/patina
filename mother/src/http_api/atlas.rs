use super::*;

pub fn handle_atlas_dashboard(runtime: &(impl AtlasApi + ?Sized)) -> HttpResponse {
    match runtime.atlas_dashboard_html() {
        Ok(html) => HttpResponse {
            status: 200,
            headers: vec![(
                "Content-Type".to_string(),
                "text/html; charset=utf-8".to_string(),
            )],
            body: html.into_bytes(),
        },
        Err(error) => json_error(500, &format!("atlas dashboard render failed: {}", error)),
    }
}

pub fn handle_atlas_snapshot(runtime: &(impl AtlasApi + ?Sized)) -> HttpResponse {
    match runtime.atlas_snapshot() {
        Ok(snapshot) => HttpResponse::json(200, &snapshot),
        Err(error) => json_error(500, &format!("atlas snapshot failed: {}", error)),
    }
}

pub fn handle_bridge_translate(
    request: &HttpRequest,
    runtime: &(impl AtlasApi + ?Sized),
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
