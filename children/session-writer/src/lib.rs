use patina_sdk::granted::{self, Bundle as GrantedBundle};
use patina_sdk::helpers::session as session_helpers;
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChild};
use patina_sdk::register_knowledge_child;

#[derive(Debug, Clone)]
struct SessionWriterToys {
    log: granted::Log,
    state: granted::State,
    session: granted::Session,
    peer: granted::Peer,
}

impl GrantedBundle for SessionWriterToys {
    fn granted() -> Self {
        Self {
            log: granted::log(),
            state: granted::state(),
            session: granted::session(),
            peer: granted::peer(),
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

impl KnowledgeChild for SessionWriterChild {
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
                session_helpers::checkpoint::<patina_sdk::knowledge_child::host::GuestHost>(
                    "close", payload, status,
                )?;
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
                session_helpers::handoff::<patina_sdk::knowledge_child::host::GuestHost>(
                    modified_files,
                    summary,
                )?;
                Ok(serde_json::json!({"status":"ok"}).to_string())
            }
            "data-ingested" => {
                let source_id = value
                    .get("source_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown-source");
                let written = value.get("written").and_then(|v| v.as_u64()).unwrap_or(0);
                let owner = value.get("owner").and_then(|v| v.as_str()).unwrap_or("");
                let repo = value.get("repo").and_then(|v| v.as_str()).unwrap_or("");
                let summary = if owner.is_empty() || repo.is_empty() {
                    format!("ducklake ingested {} rows from {}", written, source_id)
                } else {
                    format!(
                        "ducklake ingested {} rows from {} ({}/{})",
                        written, source_id, owner, repo
                    )
                };
                self.toys.session.write("activity-log", &summary)?;
                self.toys.state.put(
                    "last-data-ingested",
                    &serde_json::json!({
                        "source_id": source_id,
                        "owner": owner,
                        "repo": repo,
                        "written": written,
                    })
                    .to_string(),
                )?;
                Ok(serde_json::json!({"status":"ok"}).to_string())
            }
            other => Err(format!("session-writer: unknown action '{}'", other)),
        }
    }

    fn tick(&mut self) -> Vec<patina_sdk::substrate::TaskIntent> {
        let after_offset = self
            .toys
            .state
            .get("peer-offset:data-ingested")
            .and_then(|raw| serde_json::from_str::<u64>(&raw).ok());
        let events = match self.toys.peer.on_event("data-ingested", after_offset, 32) {
            Ok(events) => events,
            Err(error) => {
                self.toys
                    .log
                    .warn(&format!("session-writer peer poll failed: {}", error));
                return vec![];
            }
        };

        let mut last_offset = after_offset;
        for event in events {
            let _ = self.handle("data-ingested", &event.payload_json);
            last_offset = Some(event.offset);
        }

        if let Some(offset) = last_offset {
            let _ = self.toys.state.put(
                "peer-offset:data-ingested",
                &serde_json::json!(offset).to_string(),
            );
        }

        vec![]
    }
}

register_knowledge_child!(SessionWriterChild);
