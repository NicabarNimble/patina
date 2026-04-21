wit_bindgen::generate!({
    path: "wit",
    world: "slate-manager",
    generate_all,
});

use patina_sdk::toys;

struct SlateManager;

fn extract_command_name(payload: &serde_json::Value) -> Option<String> {
    let command = payload.get("command")?.as_object()?;
    let key = command.keys().next()?.to_ascii_lowercase();
    Some(key)
}

fn extract_backend_mode(payload: &serde_json::Value) -> String {
    payload
        .get("backend_mode")
        .and_then(|value| value.as_str())
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "off".to_string())
}

impl exports::patina::slate::control::Guest for SlateManager {
    fn dispatch(command_json: String) -> Result<String, String> {
        toys::measure::counter("slate_dispatch_calls", 1.0)?;

        let envelope: serde_json::Value = serde_json::from_str(&command_json)
            .map_err(|error| format!("invalid command_json: {}", error))?;
        let command =
            extract_command_name(&envelope).ok_or_else(|| "missing command payload".to_string())?;
        let backend_mode = extract_backend_mode(&envelope);

        toys::measure::counter(&format!("slate_dispatch_command_{}", command), 1.0)?;
        toys::log::info(
            "slate-manager",
            &format!(
                "dispatch scaffold command={} backend_mode={} bytes={}",
                command,
                backend_mode,
                command_json.len()
            ),
        );

        let payload = serde_json::json!({
            "status": "scaffold",
            "message": "slate-manager full-WIT scaffold active; command parity wiring pending",
            "command": command,
            "backend_mode": backend_mode,
            "bytes": command_json.len(),
        });
        Ok(payload.to_string())
    }
}

export!(SlateManager);
