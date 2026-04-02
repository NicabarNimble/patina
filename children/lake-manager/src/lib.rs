use patina_sdk::granted::{self, Bundle as GrantedBundle};
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChild};
use patina_sdk::register_knowledge_child;

#[derive(Debug, Clone)]
struct LakeManagerToys {
    log: granted::Log,
    state: granted::State,
}

impl GrantedBundle for LakeManagerToys {
    fn granted() -> Self {
        Self {
            log: granted::log(),
            state: granted::state(),
        }
    }
}

#[derive(Default)]
struct LakeManagerChild {
    toys: Option<LakeManagerToys>,
}

impl LakeManagerChild {
    fn toys(&self) -> &LakeManagerToys {
        self.toys
            .as_ref()
            .expect("lake-manager toys not initialized")
    }

    fn toys_mut(&mut self) -> &mut LakeManagerToys {
        self.toys
            .as_mut()
            .expect("lake-manager toys not initialized")
    }
}

impl KnowledgeChild for LakeManagerChild {
    fn name(&self) -> String {
        "lake-manager".into()
    }

    fn on_load(&mut self) -> Result<(), String> {
        self.toys = Some(LakeManagerToys::granted());
        self.toys().log.info("lake-manager loaded");
        Ok(())
    }

    fn health(&self) -> ChildHealth {
        ChildHealth {
            status: HealthStatus::Healthy,
            reason: Some("state-backed stub".to_string()),
        }
    }

    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
        match action {
            "list" => {
                let lakes = self.toys().state.list_prefix("lake:");
                Ok(serde_json::json!({
                    "status": "ok",
                    "lakes": lakes,
                })
                .to_string())
            }
            "create" => {
                let body: serde_json::Value = if payload.trim().is_empty() {
                    serde_json::json!({})
                } else {
                    serde_json::from_str(payload)
                        .map_err(|e| format!("lake-manager: invalid payload: {}", e))?
                };
                let name = body
                    .get("name")
                    .and_then(|value| value.as_str())
                    .unwrap_or("default");
                self.toys_mut().state.put(
                    &format!("lake:{}", name),
                    &serde_json::json!({"name": name}).to_string(),
                )?;
                Ok(serde_json::json!({"status": "ok", "name": name}).to_string())
            }
            other => Err(format!("lake-manager: unknown action '{}'", other)),
        }
    }
}

register_knowledge_child!(LakeManagerChild);
