use super::*;

pub fn handle_secrets_get(runtime: &(impl SecretsApi + ?Sized)) -> HttpResponse {
    match runtime.secrets_get() {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(_) => json_error(404, "No cached secrets"),
    }
}

pub fn handle_secrets_cache(
    request: &HttpRequest,
    runtime: &(impl SecretsApi + ?Sized),
) -> HttpResponse {
    if request.body.is_empty() {
        return json_error(400, "Missing request body");
    }

    let payload: serde_json::Value = match serde_json::from_slice(&request.body) {
        Ok(v) => v,
        Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
    };

    match runtime.secrets_cache(payload) {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(e) => json_error(500, &format!("Cache failed: {}", e)),
    }
}

pub fn handle_secrets_lock(runtime: &(impl SecretsApi + ?Sized)) -> HttpResponse {
    match runtime.secrets_lock() {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(e) => json_error(500, &format!("Lock failed: {}", e)),
    }
}
