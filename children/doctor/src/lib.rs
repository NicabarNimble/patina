use patina_sdk::granted;
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChild};
use patina_sdk::register_knowledge_child;

#[derive(Default)]
struct DoctorChild;

impl KnowledgeChild for DoctorChild {
    fn name(&self) -> String {
        "doctor".into()
    }

    fn on_load(&mut self) -> Result<(), String> {
        granted::log().info("doctor loaded");
        Ok(())
    }

    fn health(&self) -> ChildHealth {
        ChildHealth {
            status: HealthStatus::Healthy,
            reason: Some("stub".to_string()),
        }
    }

    fn handle(&mut self, action: &str, _payload: &str) -> Result<String, String> {
        match action {
            "run" => Ok(serde_json::json!({
                "status": "stub",
                "message": "doctor child scaffold installed; run action not implemented yet"
            })
            .to_string()),
            other => Err(format!("doctor: unknown action '{}'", other)),
        }
    }
}

register_knowledge_child!(DoctorChild);
