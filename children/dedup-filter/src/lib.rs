use chrono::Utc;
use patina_sdk::granted;
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChild};
use patina_sdk::register_child;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default)]
struct DedupFilterChild;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
    record_id: String,
    source_path: String,
    source_hash: String,
    source_modified_at: String,
    source_size_bytes: u64,
    content: String,
    content_hash: String,
    content_type: String,
    encoding: String,
    line_count: u64,
    ingested_at: String,
    batch_id: String,
    schema_version: u32,
}

#[derive(Debug, Clone, Serialize)]
struct DuplicateRecord {
    source_path: String,
    content_hash: String,
    reason: String,
}

fn parse_payload(payload: &str) -> Result<Value, String> {
    if payload.trim().is_empty() {
        Ok(serde_json::json!({}))
    } else {
        serde_json::from_str(payload).map_err(|e| format!("invalid payload json: {}", e))
    }
}

fn emit(topic: &str, payload_json: String) -> Result<u64, String> {
    let messaging = granted::messaging();
    let client = messaging.connect(topic)?;
    let message = patina_sdk::toys::MessagingMessage {
        topic: topic.to_string(),
        content_type: Some("application/json".to_string()),
        data: payload_json.into_bytes(),
        metadata: vec![],
    };
    messaging.send(&client, &message)
}

fn emit_metric_counter(name: &str, delta: f64) -> Result<(), String> {
    patina_sdk::knowledge_child::patina::measure::measure::counter(name, delta)
}

fn emit_metric_gauge(name: &str, value: f64) -> Result<(), String> {
    patina_sdk::knowledge_child::patina::measure::measure::gauge(name, value)
}

impl DedupFilterChild {
    fn filter_dedup(&mut self, payload: &str) -> Result<String, String> {
        let payload_value = parse_payload(payload)?;
        let limit = payload_value
            .get("limit")
            .and_then(|value| value.as_u64())
            .unwrap_or(128)
            .min(u32::MAX as u64) as u32;
        let after_offset = payload_value
            .get("after_offset")
            .and_then(|value| value.as_u64());

        let events = patina_sdk::knowledge_child::patina::events_stream::events_stream::subscribe(
            "record.validated",
            after_offset,
            limit,
        )?;

        let state = granted::state();
        let log = granted::log();
        let mut ready_records = 0_u64;
        let mut duplicate_records = 0_u64;
        let mut duplicates = Vec::new();
        let mut last_offset = None;
        let mut duplicate_output_violations = 0_u64;
        let mut ready_hashes = std::collections::HashSet::new();

        for event in events {
            last_offset = Some(event.offset);
            let record: Record = serde_json::from_str(&event.payload)
                .map_err(|e| format!("invalid record.validated payload: {}", e))?;
            emit_metric_counter("records_seen", 1.0)?;

            let dedup_key = format!("dedup:{}", record.content_hash);
            if state.get(&dedup_key).is_some() {
                let duplicate = DuplicateRecord {
                    source_path: record.source_path,
                    content_hash: record.content_hash,
                    reason: "duplicate-content-hash".to_string(),
                };
                emit(
                    "record.duplicate",
                    serde_json::to_string(&duplicate).map_err(|e| e.to_string())?,
                )?;
                duplicates.push(duplicate);
                duplicate_records += 1;
                emit_metric_counter("duplicate_records", 1.0)?;
                continue;
            }

            state.put(&dedup_key, &Utc::now().to_rfc3339())?;
            emit(
                "record.ready",
                serde_json::to_string(&record).map_err(|e| e.to_string())?,
            )?;
            if !ready_hashes.insert(record.content_hash.clone()) {
                duplicate_output_violations += 1;
            }
            ready_records += 1;
            emit_metric_counter("ready_records", 1.0)?;
        }

        let duplicate_output_rate_pct = if ready_records == 0 {
            0.0
        } else {
            (duplicate_output_violations as f64 / ready_records as f64) * 100.0
        };
        emit_metric_gauge("duplicate_output_rate_pct", duplicate_output_rate_pct)?;

        if let Some(offset) = last_offset {
            patina_sdk::knowledge_child::patina::events_stream::events_stream::ack(
                "record.validated",
                offset,
            )?;
        }

        for duplicate in &duplicates {
            log.info(&format!(
                "dedup-filter duplicate {} ({})",
                duplicate.source_path, duplicate.reason
            ));
        }

        Ok(serde_json::json!({
            "status": "ok",
            "ready_records": ready_records,
            "duplicate_records": duplicate_records,
            "duplicate_output_rate_pct": duplicate_output_rate_pct,
            "duplicates": duplicates,
            "dedup_keys": state.list_prefix("dedup:"),
            "acked_through": last_offset,
        })
        .to_string())
    }
}

impl KnowledgeChild for DedupFilterChild {
    fn name(&self) -> String {
        "dedup-filter".to_string()
    }

    fn on_load(&mut self) -> Result<(), String> {
        granted::log().info("dedup-filter loaded");
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
            "filter-dedup" => self.filter_dedup(payload),
            other => Err(format!("dedup-filter: unknown action '{}'", other)),
        }
    }
}

register_child!(DedupFilterChild);
