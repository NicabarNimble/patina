use anyhow::Result;
use patina_protocol::{BuiltinChild, BuiltinChildAction, BuiltinChildRequest, DoctorRunRequest};

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

    if json_output {
        if let Some(data) = response.get("data") {
            println!("{}", serde_json::to_string_pretty(data)?);
        } else {
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        if let Some(code) = response.get("exit_code").and_then(|v| v.as_i64()) {
            if code != 0 {
                std::process::exit(code as i32);
            }
        }
        return Ok(());
    }

    if let Some(text) = response.get("text").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            println!("{}", text);
        }
    }
    if let Some(data) = response.get("data") {
        let status = data
            .get("health")
            .and_then(|h| h.get("status"))
            .and_then(|s| s.as_str())
            .unwrap_or("unknown");
        println!("Doctor status: {}", status);
    }
    if let Some(code) = response.get("exit_code").and_then(|v| v.as_i64()) {
        if code != 0 {
            std::process::exit(code as i32);
        }
    }
    Ok(())
}
