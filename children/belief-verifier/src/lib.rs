use patina_sdk::granted::{self, Bundle as GrantedBundle};
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChild};
use patina_sdk::register_child;
use patina_sdk::substrate::{self, PendingEvent, TaskIntent, TaskIntentKind};

#[derive(Debug, Clone)]
struct BeliefVerifierToys {
    log: granted::Log,
    measure: granted::Measure,
    state: granted::State,
    events: granted::Events,
    belief: granted::Belief,
}

impl GrantedBundle for BeliefVerifierToys {
    fn granted() -> Self {
        Self {
            log: granted::log(),
            measure: granted::measure(),
            state: granted::state(),
            events: granted::events(),
            belief: granted::belief(),
        }
    }
}

struct BeliefVerifierChild {
    toys: BeliefVerifierToys,
}

impl Default for BeliefVerifierChild {
    fn default() -> Self {
        Self {
            toys: BeliefVerifierToys::granted(),
        }
    }
}

impl BeliefVerifierChild {
    fn verification_count(&self) -> u64 {
        self.toys
            .state
            .get("verification-count")
            .and_then(|value| serde_json::from_str::<u64>(&value).ok())
            .unwrap_or(0)
    }

    fn increment_verification_count(&self) -> Result<(), String> {
        let next = self.verification_count() + 1;
        self.toys.state.put(
            "verification-count",
            &serde_json::to_string(&next).map_err(|e| e.to_string())?,
        )
    }

    fn verify_belief(&mut self, payload: &str) -> Result<String, String> {
        let value: serde_json::Value =
            serde_json::from_str(payload).map_err(|e| format!("invalid verify payload: {}", e))?;
        let belief_id = value
            .get("belief_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'belief_id'".to_string())?;
        let evidence_ids = value
            .get("evidence_ids")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let belief_json = self.toys.belief.query(
            "by-id",
            &serde_json::json!({"belief_id": belief_id}).to_string(),
        )?;
        let belief: serde_json::Value = serde_json::from_str(&belief_json)
            .map_err(|e| format!("invalid belief response: {}", e))?;
        let statement = belief
            .get("belief")
            .and_then(|b| b.get("statement"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let status = if statement.to_ascii_lowercase().contains("not ") {
            "failed"
        } else {
            "passed"
        };
        self.toys.belief.mutate(
            "record-verification",
            &serde_json::json!({
                "belief_id": belief_id,
                "status": status,
                "evidence_ids": evidence_ids,
                "notes": format!("verified by belief-verifier from statement length {}", statement.len()),
            })
            .to_string(),
        )?;
        for evidence_id in evidence_ids.iter().filter_map(|v| v.as_str()) {
            self.toys.belief.mutate(
                "attach-evidence",
                &serde_json::json!({
                    "belief_id": belief_id,
                    "evidence_id": evidence_id,
                })
                .to_string(),
            )?;
        }
        self.increment_verification_count()?;
        self.toys.measure.record(
            "believe",
            "belief-verifier",
            "verify",
            &serde_json::json!({"count": 1}).to_string(),
        )?;
        Ok(serde_json::json!({"belief_id": belief_id, "status": status}).to_string())
    }
}

impl KnowledgeChild for BeliefVerifierChild {
    fn name(&self) -> String {
        "belief-verifier".into()
    }

    fn on_load(&mut self) -> Result<(), String> {
        self.toys.log.info("belief-verifier loaded");
        Ok(())
    }

    fn health(&self) -> ChildHealth {
        ChildHealth {
            status: HealthStatus::Healthy,
            reason: Some(format!(
                "{} verifications recorded",
                self.verification_count()
            )),
        }
    }

    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String> {
        match action {
            "verify-belief" => self.verify_belief(payload),
            "status" => Ok(
                serde_json::json!({"verification_count": self.verification_count()}).to_string(),
            ),
            other => Err(format!("belief-verifier: unknown action '{}'", other)),
        }
    }

    fn drain(&mut self, limit: u32) -> Result<Vec<PendingEvent>, String> {
        let checkpoint_bucket = self
            .toys
            .state
            .open("checkpoints")
            .map_err(|e| e.to_string())?;
        let checkpoint = checkpoint_bucket
            .get_string("belief.changed")
            .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
            .and_then(|value| value.get("offset").and_then(|v| v.as_u64()));
        let events = self.toys.events.pull("belief.changed", checkpoint, limit)?;
        let mut last_offset = checkpoint.unwrap_or(0);
        for event in &events {
            let payload_value: serde_json::Value =
                serde_json::from_str(&event.payload_json).unwrap_or(serde_json::Value::Null);
            let queued_belief_id = payload_value
                .get("belief_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    format!(
                        "belief.changed event at offset {} missing belief_id",
                        event.offset
                    )
                })?;
            substrate::enqueue(&TaskIntent {
                kind: TaskIntentKind::VerifyBelief,
                payload_json: serde_json::json!({
                    "belief_id": queued_belief_id,
                    "evidence_ids": payload_value.get("evidence_ids").cloned().unwrap_or(serde_json::Value::Array(vec![])),
                })
                .to_string(),
                dedupe_key: Some(format!("belief:{}:{}", queued_belief_id, event.offset)),
            })?;
            last_offset = event.offset;
        }
        if last_offset > checkpoint.unwrap_or(0) {
            self.toys
                .events
                .ack_through("belief.changed", last_offset)?;
            checkpoint_bucket.set_string(
                "belief.changed",
                &serde_json::json!({"offset": last_offset}).to_string(),
            )?;
        }
        Ok(events)
    }

    fn tick(&mut self) -> Vec<TaskIntent> {
        vec![]
    }
}

register_child!(BeliefVerifierChild);
