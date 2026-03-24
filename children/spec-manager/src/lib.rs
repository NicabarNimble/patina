use patina_sdk::granted;
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChild};
use patina_sdk::register_knowledge_child;

#[derive(Default)]
struct SpecManagerChild;

impl KnowledgeChild for SpecManagerChild {
    fn name(&self) -> String {
        "spec-manager".into()
    }

    fn on_load(&mut self) -> Result<(), String> {
        granted::log().info("spec-manager loaded");
        Ok(())
    }

    fn health(&self) -> ChildHealth {
        ChildHealth {
            status: HealthStatus::Healthy,
            reason: Some("stub".to_string()),
        }
    }

    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
        match action {
            "health" => Ok(serde_json::json!({"status":"ok"}).to_string()),
            "dispatch" => Ok(serde_json::json!({
                "status": "stub",
                "message": "spec-manager scaffold installed; command dispatch not implemented yet",
                "payload": payload
            })
            .to_string()),
            other => Err(format!("spec-manager: unknown action '{}'", other)),
        }
    }
}

register_knowledge_child!(SpecManagerChild);
