use super::*;

pub fn handle_pando_registry_init(
    request: &HttpRequest,
    runtime: &(impl PandoRegistryApi + ?Sized),
) -> HttpResponse {
    if request.body.is_empty() {
        return json_error(400, "Missing request body");
    }

    let init: patina_protocol::PandoRegistryInit = match serde_json::from_slice(&request.body) {
        Ok(v) => v,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
    };

    match runtime.pando_registry_init(init) {
        Ok(state) => HttpResponse::json(200, &state),
        Err(e) => json_error(400, &e.to_string()),
    }
}

pub fn handle_pando_list(runtime: &(impl PandoRegistryApi + ?Sized)) -> HttpResponse {
    match runtime.pando_list() {
        Ok(state) => HttpResponse::json(200, &state),
        Err(e) => json_error(500, &e.to_string()),
    }
}
