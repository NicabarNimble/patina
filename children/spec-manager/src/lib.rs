use patina_sdk::granted::{self, Bundle as GrantedBundle};
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChild};
use patina_sdk::register_knowledge_child;

#[derive(Debug, Clone)]
struct SpecManagerToys {
    log: granted::Log,
    state: granted::State,
}

impl GrantedBundle for SpecManagerToys {
    fn granted() -> Self {
        Self {
            log: granted::log(),
            state: granted::state(),
        }
    }
}

#[derive(Default)]
struct SpecManagerChild {
    toys: Option<SpecManagerToys>,
}

impl SpecManagerChild {
    fn toys(&self) -> &SpecManagerToys {
        self.toys
            .as_ref()
            .expect("spec-manager toys not initialized")
    }

    fn toys_mut(&mut self) -> &mut SpecManagerToys {
        self.toys
            .as_mut()
            .expect("spec-manager toys not initialized")
    }

    fn dispatch_count(&self) -> u64 {
        self.toys()
            .state
            .get("spec-manager.dispatch-count")
            .and_then(|value| serde_json::from_str::<u64>(&value).ok())
            .unwrap_or(0)
    }
}

impl KnowledgeChild for SpecManagerChild {
    fn name(&self) -> String {
        "spec-manager".into()
    }

    fn on_load(&mut self) -> Result<(), String> {
        self.toys = Some(SpecManagerToys::granted());
        self.toys().log.info("spec-manager loaded");
        Ok(())
    }

    fn health(&self) -> ChildHealth {
        ChildHealth {
            status: HealthStatus::Healthy,
            reason: Some(format!("dispatch-count={}", self.dispatch_count())),
        }
    }

    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
        match action {
            "health" => Ok(serde_json::json!({"status":"ok"}).to_string()),
            "dispatch" => {
                let count = self.dispatch_count() + 1;
                self.toys_mut().state.put(
                    "spec-manager.dispatch-count",
                    &serde_json::to_string(&count).map_err(|e| e.to_string())?,
                )?;
                self.toys_mut().state.put(
                    "spec-manager.last-dispatch",
                    &serde_json::json!({"payload": payload, "count": count}).to_string(),
                )?;
                Ok(serde_json::json!({
                    "status": "ok",
                    "message": "spec dispatch recorded",
                    "count": count,
                })
                .to_string())
            }
            other => Err(format!("spec-manager: unknown action '{}'", other)),
        }
    }
}

register_knowledge_child!(SpecManagerChild);
