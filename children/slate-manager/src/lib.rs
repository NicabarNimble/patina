wit_bindgen::generate!({
    path: "wit",
    world: "slate-manager",
    generate_all,
});

use patina_sdk::toys;

struct SlateManager;

impl exports::patina::slate::control::Guest for SlateManager {
    fn dispatch(command_json: String) -> Result<String, String> {
        toys::measure::counter("slate_dispatch_calls", 1.0)?;
        toys::log::info(
            "slate-manager",
            &format!(
                "dispatch observe-only scaffold bytes={}",
                command_json.len()
            ),
        );

        let payload = serde_json::json!({
            "status": "scaffold",
            "message": "slate-manager full-WIT child scaffold active; command parity wiring pending",
            "bytes": command_json.len(),
        });
        Ok(payload.to_string())
    }
}

export!(SlateManager);
