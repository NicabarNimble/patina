use anyhow::Result;
use patina_protocol::{
    BuiltinChild, BuiltinChildAction, BuiltinChildRequest, BuiltinChildResult, DoctorRunRequest,
};

#[allow(dead_code)]
pub(crate) fn execute_value() -> Result<serde_json::Value> {
    patina::mother::doctor_runtime::execute_value()
}

pub fn execute_cli(json_output: bool) -> Result<()> {
    let protocol_request = BuiltinChildRequest::new(
        BuiltinChild::Doctor,
        BuiltinChildAction::DoctorRun(DoctorRunRequest),
    );
    let client = patina::mother::control_plane_client();
    let response = client.child_action_typed(&protocol_request).map_err(|e| {
        anyhow::anyhow!(
            "doctor child unavailable via Mother (start with `patina mother start`): {}",
            e
        )
    })?;
    let result = match response.result {
        BuiltinChildResult::DoctorRun(result) => result,
        other => anyhow::bail!("Unexpected typed response from doctor: {:?}", other),
    };

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result.data)?);
        if result.exit_code != 0 {
            std::process::exit(result.exit_code as i32);
        }
        return Ok(());
    }

    let status = result
        .data
        .get("health")
        .and_then(|h| h.get("status"))
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");
    println!("Doctor status: {}", status);

    if result.exit_code != 0 {
        std::process::exit(result.exit_code as i32);
    }
    Ok(())
}
