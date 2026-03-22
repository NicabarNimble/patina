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
        let value: serde_json::Value = if payload.trim().is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(payload)
                .map_err(|e| format!("session-writer: invalid payload json: {}", e))?
        };

        match action {
            "init-session" => {
                let session_id = self.toys.session.get_session_id();
                let previous = self.toys.session.get_previous_session();
                let previous_runtime = self.toys.session.get_previous_session_runtime_id();
                let previous_handoff = self.toys.session.get_previous_session_handoff();
                self.toys
                    .state
                    .put("session-id", &serde_json::json!(session_id).to_string())?;
                if let Some(previous) = previous {
                    self.toys
                        .state
                        .put("previous-session", &serde_json::json!(previous).to_string())?;
                }
                if let Some(runtime_id) = previous_runtime {
                    self.toys.session.set_parent_session(&runtime_id)?;
                    self.toys.state.put(
                        "parent-session-runtime",
                        &serde_json::json!(runtime_id).to_string(),
                    )?;
                }
                if let Some(handoff) = previous_handoff {
                    self.toys.session.write("parent-handoff", &handoff)?;
                }
                Ok(serde_json::json!({"status":"initialized"}).to_string())
            }
            "note" => {
                let content = value
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or(payload);
                self.toys.session.write("note", content)?;
                Ok(serde_json::json!({"status":"ok"}).to_string())
            }
            "update" => {
                let content = value
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or(payload);
                self.toys.session.write("update", content)?;
                Ok(serde_json::json!({"status":"ok"}).to_string())
            }
            "spec-link" => {
                let content = value
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or(payload);
                self.toys.session.write("spec-link", content)?;
                Ok(serde_json::json!({"status":"ok"}).to_string())
            }
            "close" => {
                let status = value
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("closed");
                self.toys.session.write("close", payload)?;
                self.toys.session.set_status(status)?;
                if let Some(tag) = value.get("tag").and_then(|v| v.as_str()) {
                    self.toys.session.create_tag(tag)?;
                }
                Ok(serde_json::json!({"status":"ok"}).to_string())
            }
            "crash-handoff" => {
                let modified_files = value
                    .get("modified_files")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let summary = value
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or(payload);
                self.toys.session.write("crash-handoff", payload)?;
                self.toys.session.write_handoff(modified_files, summary)?;
                Ok(serde_json::json!({"status":"ok"}).to_string())
            }
            other => Err(format!("session-writer: unknown action '{}'", other)),
        }
    }
}

register_knowledge_child!(SessionWriterChild);
