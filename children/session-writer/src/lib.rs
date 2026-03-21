use patina_sdk::granted::{self, Bundle as GrantedBundle};
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChildPlugin};
use patina_sdk::register_knowledge_child;

#[derive(Debug, Clone)]
struct SessionWriterToys {
    log: granted::Log,
    state: granted::State,
    session: granted::Session,
}

impl GrantedBundle for SessionWriterToys {
    fn granted() -> Self {
        Self {
            log: granted::log(),
            state: granted::state(),
            session: granted::session(),
        }
    }
}

struct SessionWriterChild {
    toys: SessionWriterToys,
}

impl Default for SessionWriterChild {
    fn default() -> Self {
        Self {
            toys: SessionWriterToys::granted(),
        }
    }
}

impl KnowledgeChildPlugin for SessionWriterChild {
    fn name(&self) -> String {
        "session-writer".into()
    }

    fn on_load(&mut self) -> Result<(), String> {
        self.toys.log.info("session-writer loaded");
        Ok(())
    }

    fn health(&self) -> ChildHealth {
        let session_id = self
            .toys
            .state
            .get("session-id")
            .unwrap_or_else(|| "unknown".to_string());
        ChildHealth {
            status: HealthStatus::Healthy,
            reason: Some(format!("session-id={}", session_id)),
        }
    }

    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
        match action {
            "init-session" => {
                let session_id = self
                    .toys
                    .session
                    .write("runtime", payload)
                    .map(|_| serde_json::json!({"status":"initialized"}).to_string())?;
                Ok(session_id)
            }
            other => Err(format!("session-writer: unknown action '{}'", other)),
        }
    }
}

register_knowledge_child!(SessionWriterChild);
