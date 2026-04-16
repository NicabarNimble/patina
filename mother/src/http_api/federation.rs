use serde::Deserialize;

use super::*;

#[derive(Deserialize, Default)]
struct FederationNoopRequest {}

pub fn handle_federation_status(
    request: &HttpRequest,
    runtime: &(impl FederationApi + ?Sized),
) -> HttpResponse {
    if !request.body.is_empty()
        && serde_json::from_slice::<FederationNoopRequest>(&request.body).is_err()
    {
        return json_error(400, "Invalid JSON");
    }
    match runtime.federation_status() {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(error) => json_error(500, &error.to_string()),
    }
}

pub fn handle_federation_refresh(
    request: &HttpRequest,
    runtime: &(impl FederationApi + ?Sized),
) -> HttpResponse {
    if !request.body.is_empty()
        && serde_json::from_slice::<FederationNoopRequest>(&request.body).is_err()
    {
        return json_error(400, "Invalid JSON");
    }
    match runtime.federation_refresh() {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(error) => json_error(500, &error.to_string()),
    }
}

pub fn handle_federation_query(
    request: &HttpRequest,
    runtime: &(impl FederationApi + ?Sized),
) -> HttpResponse {
    if request.body.is_empty() {
        return json_error(400, "Missing request body");
    }

    let payload: crate::protocol::FederationQueryPayload =
        match serde_json::from_slice(&request.body) {
            Ok(v) => v,
            Err(e) => return json_error(400, &format!("Invalid JSON: {}", e)),
        };
    match runtime.federation_query(payload) {
        Ok(payload) => HttpResponse::json(200, &payload),
        Err(error) => json_error(500, &error.to_string()),
    }
}
