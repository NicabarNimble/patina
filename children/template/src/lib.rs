use patina_sdk::granted;
use patina_sdk::child::{ChildHealth, HealthStatus, Child};
use patina_sdk::register_child;

#[derive(Default)]
struct Child;

impl Child for Child {
    fn name(&self) -> String {
        "{{ child_name }}".into()
    }

    fn on_load(&mut self) -> Result<(), String> {
        granted::log().info("{{ child_name }} loaded");
        Ok(())
    }

    fn health(&self) -> ChildHealth {
        ChildHealth {
            status: HealthStatus::Healthy,
            reason: None,
        }
    }

    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
        match action {
            "ping" => Ok(serde_json::json!({"pong": payload}).to_string()),
            other => Err(format!("{{ child_name }}: unknown action '{}'", other)),
        }
    }
}

register_child!(Child);
