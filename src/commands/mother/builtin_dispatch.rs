use mother_crate::builtin_children::{self, BuiltinChildExecutor};
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
        &CliBuiltinExecutor,
        &mother_crate::secrets_authority_backend::MotherSecretsAuthorityBackend,
    )
}

struct CliBuiltinExecutor;

impl BuiltinChildExecutor for CliBuiltinExecutor {
    fn spec_dispatch(&self, request: SpecDispatchRequest) -> anyhow::Result<serde_json::Value> {
        let command: patina::mother::spec_runtime::SpecCommands =
            serde_json::from_value(request.command)
                .map_err(|e| anyhow::anyhow!("Invalid spec-manager command payload: {}", e))?;
        patina::mother::spec_runtime::execute_value(command)
    }

    fn lake_dispatch(&self, request: LakeDispatchRequest) -> anyhow::Result<serde_json::Value> {
        let command: patina::mother::lake_runtime::LakeCommand =
            serde_json::from_value(request.command)
                .map_err(|e| anyhow::anyhow!("Invalid lake-manager command payload: {}", e))?;
        patina::mother::lake_runtime::execute_value(command)
    }

    fn doctor_run(&self) -> anyhow::Result<DoctorRunResult> {
        let value = patina::mother::doctor_runtime::execute_value()?;
        let exit_code = value.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0);
        Ok(DoctorRunResult {
            data: value,
            exit_code,
        })
    }
}
