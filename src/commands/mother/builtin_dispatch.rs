use mother_crate::builtin_children::{self, BuiltinDispatchRuntime};
use mother_crate::http_daemon::HttpResponse;
use patina_protocol::{DoctorRunResult, LakeDispatchRequest, SpecDispatchRequest};

pub(super) fn handle_builtin_child_request(
    child_name: &str,
    action: &str,
    body: &[u8],
) -> Option<HttpResponse> {
    builtin_children::handle_builtin_child_request(
        child_name,
        action,
        body,
        BuiltinDispatchRuntime {
            spec_dispatch: &spec_dispatch,
            lake_dispatch: &lake_dispatch,
            doctor_run: &doctor_run,
        },
        &mother_crate::secrets_authority_backend::MotherSecretsAuthorityBackend,
    )
}

fn spec_dispatch(request: SpecDispatchRequest) -> anyhow::Result<serde_json::Value> {
    let command: patina::spec::SpecCommands = serde_json::from_value(request.command)
        .map_err(|e| anyhow::anyhow!("Invalid spec-manager command payload: {}", e))?;
    patina::spec::execute_command_value(command)
}

fn lake_dispatch(request: LakeDispatchRequest) -> anyhow::Result<serde_json::Value> {
    let command: patina::lake::LakeCommand = serde_json::from_value(request.command)
        .map_err(|e| anyhow::anyhow!("Invalid lake-manager command payload: {}", e))?;
    patina::lake::execute_value(command)
}

fn doctor_run() -> anyhow::Result<DoctorRunResult> {
    let value = patina::mother::doctor_runtime::execute_value()?;
    let exit_code = value.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0);
    Ok(DoctorRunResult {
        data: value,
        exit_code,
    })
}
