use patina_sdk::granted;
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChild};
use patina_sdk::register_knowledge_child;

#[derive(Default)]
struct LakeManagerChild;

impl KnowledgeChild for LakeManagerChild {
    fn name(&self) -> String {
        "lake-manager".into()
    }

    fn on_load(&mut self) -> Result<(), String> {
        granted::log().info("lake-manager loaded");
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
            "list" => Ok(serde_json::json!({
                "status": "stub",
                "message": "lake-manager scaffold installed; list action not implemented yet",
                "lakes": []
            })
            .to_string()),
            "create" => Ok(serde_json::json!({
                "status": "stub",
                "message": "lake-manager scaffold installed; create action not implemented yet"
            })
            .to_string()),
            other => Err(format!("lake-manager: unknown action '{}'", other)),
        }
    }
}

register_knowledge_child!(LakeManagerChild);
