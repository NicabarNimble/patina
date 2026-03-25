//! Models child plugin — embedding model path resolution for Mother daemon.
//!
//! Phase 1 scope: pure computation only (no WASI filesystem access).
//! Returns expected paths and "unknown" status where filesystem
//! verification would be needed.
//!
//! Actions:
//! - "resolve_model" → returns expected model directory path
//! - "model_status"  → returns tri-state status (cached/local/unknown)

use patina_sdk::{register_mother_child, ChildHealth, HealthStatus, MotherChild};

#[derive(Default)]
struct ModelsChild;

impl MotherChild for ModelsChild {
    fn name(&self) -> String {
        "models".into()
    }

    fn health(&self) -> ChildHealth {
        // Phase 1: can't verify model files without WASI filesystem
        ChildHealth {
            status: HealthStatus::Healthy,
            reason: None,
        }
    }

    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
        match action {
            "resolve_model" => self.resolve_model(payload),
            "model_status" => self.model_status(payload),
            _ => Err(format!("models: unknown action '{}'", action)),
        }
    }
}

impl ModelsChild {
    /// Resolve expected model directory path.
    ///
    /// Phase 1: pure computation — returns the path pattern where the model
    /// would live. Cannot verify existence without WASI filesystem access.
    fn resolve_model(&self, payload: &str) -> Result<String, String> {
        let name = Self::extract_name(payload)?;

        // Standard model cache path: ~/.patina/cache/models/{name}/
        // Phase 1: return the relative pattern, host resolves to absolute
        let response = serde_json::json!({
            "path": format!(".patina/cache/models/{}", name),
            "verified": false,
            "note": "Phase 1: path computed, not verified (no WASI filesystem access)"
        });
        serde_json::to_string(&response).map_err(|e| format!("json error: {}", e))
    }

    /// Get model status (tri-state: cached/local/unknown).
    ///
    /// Phase 1: all states are "unknown" — filesystem checks require WASI.
    fn model_status(&self, payload: &str) -> Result<String, String> {
        let name = Self::extract_name(payload)?;

        let response = serde_json::json!({
            "name": name,
            "in_cache": "unknown",
            "in_local": "unknown",
            "provenance": null,
            "note": "Phase 1: status unknown (no WASI filesystem access)"
        });
        serde_json::to_string(&response).map_err(|e| format!("json error: {}", e))
    }

    /// Extract "name" field from JSON payload.
    fn extract_name(payload: &str) -> Result<String, String> {
        let value: serde_json::Value =
            serde_json::from_str(payload).map_err(|e| format!("invalid JSON: {}", e))?;

        value
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "missing 'name' field".to_string())
    }
}

register_mother_child!(ModelsChild);
