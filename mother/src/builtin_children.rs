use crate::http_daemon::{json_error, HttpResponse};
use crate::secrets_authority_api;
use patina_protocol::{
    BuiltinChild, BuiltinChildAction, BuiltinChildRequest, BuiltinChildResponse,
    BuiltinChildResult, DoctorRunResult,
};

pub struct BuiltinDispatchRuntime<'a> {
    pub spec_dispatch:
        &'a dyn Fn(patina_protocol::SpecDispatchRequest) -> anyhow::Result<serde_json::Value>,
    pub lake_dispatch:
        &'a dyn Fn(patina_protocol::LakeDispatchRequest) -> anyhow::Result<serde_json::Value>,
    pub doctor_run: &'a dyn Fn() -> anyhow::Result<DoctorRunResult>,
}

pub fn handle_builtin_child_request(
    child_name: &str,
    action: &str,
    body: &[u8],
    runtime: BuiltinDispatchRuntime<'_>,
    secrets_backend: &dyn secrets_authority_api::SecretsAuthorityBackend,
) -> Option<HttpResponse> {
    BuiltinChild::from_route(child_name)?;

    let request = match BuiltinChildRequest::from_http_parts(child_name, action, body) {
        Ok(request) => request,
        Err(message) if message.starts_with("Unsupported action") => return None,
        Err(message) => return Some(json_error(400, &message)),
    };

    let response = match request.action {
        BuiltinChildAction::Health => BuiltinChildResponse::new(
            request.child,
            BuiltinChildResult::Health {
                status: "healthy".to_string(),
            },
        ),
        BuiltinChildAction::SpecDispatch(dispatch) => match (runtime.spec_dispatch)(dispatch) {
            Ok(value) => BuiltinChildResponse::new(
                BuiltinChild::SpecManager,
                BuiltinChildResult::Dispatch { payload: value },
            ),
            Err(e) => return Some(json_error(400, &e.to_string())),
        },
        BuiltinChildAction::LakeDispatch(dispatch) => match (runtime.lake_dispatch)(dispatch) {
            Ok(value) => BuiltinChildResponse::new(
                BuiltinChild::LakeManager,
                BuiltinChildResult::Dispatch { payload: value },
            ),
            Err(e) => return Some(json_error(400, &e.to_string())),
        },
        BuiltinChildAction::DoctorRun(_) => match (runtime.doctor_run)() {
            Ok(result) => BuiltinChildResponse::new(
                BuiltinChild::Doctor,
                BuiltinChildResult::DoctorRun(result),
            ),
            Err(e) => return Some(json_error(400, &e.to_string())),
        },
        BuiltinChildAction::SecretsDispatch(dispatch) => {
            return Some(secrets_authority_api::dispatch(
                dispatch.operation.into_payload(),
                secrets_backend,
            ));
        }
    };

    Some(HttpResponse::json(200, &response.into_http_payload()))
}
