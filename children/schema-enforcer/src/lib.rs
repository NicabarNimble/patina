use patina_sdk::child::{Child, ChildHealth, HealthStatus};
use patina_sdk::granted;
use patina_sdk::register_child;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default)]
struct SchemaEnforcerChild;

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
struct RejectedRecord {
    reason: String,
    record: Record,
}

fn parse_payload(payload: &str) -> Result<Value, String> {
    if payload.trim().is_empty() {
        Ok(serde_json::json!({}))
    } else {
        serde_json::from_str(payload).map_err(|e| format!("invalid payload json: {}", e))
    }
}

fn validate_record(record: &Record) -> Result<(), String> {
    if record.record_id.trim().is_empty() {
        return Err("missing record_id".to_string());
    }
    if record.source_path.trim().is_empty() {
        return Err("missing source_path".to_string());
    }
    if record.source_hash.trim().is_empty() {
        return Err("missing source_hash".to_string());
    }
    if record.source_modified_at.trim().is_empty() {
        return Err("missing source_modified_at".to_string());
    }
    if record.content.trim().is_empty() {
        return Err("missing content".to_string());
    }
    if record.content_hash.trim().is_empty() {
        return Err("missing content_hash".to_string());
    }
    if record.content_type.trim().is_empty() {
        return Err("missing content_type".to_string());
    }
    if record.encoding.trim().is_empty() {
        return Err("missing encoding".to_string());
    }
    if record.ingested_at.trim().is_empty() {
        return Err("missing ingested_at".to_string());
    }
    if record.batch_id.trim().is_empty() {
        return Err("missing batch_id".to_string());
    }
    if record.schema_version == 0 {
        return Err("schema_version must be >= 1".to_string());
    }
    Ok(())
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
    patina_sdk::child::patina::measure::measure::counter(name, delta)
}

fn emit_metric_gauge(name: &str, value: f64) -> Result<(), String> {
    patina_sdk::child::patina::measure::measure::gauge(name, value)
}

fn provenance_complete(record: &Record) -> bool {
    !record.source_path.trim().is_empty()
        && !record.source_hash.trim().is_empty()
        && !record.source_modified_at.trim().is_empty()
        && record.source_size_bytes > 0
        && !record.ingested_at.trim().is_empty()
        && !record.batch_id.trim().is_empty()
        && record.schema_version >= 1
}

impl SchemaEnforcerChild {
    fn enforce_schema(&mut self, payload: &str) -> Result<String, String> {
        let payload_value = parse_payload(payload)?;
        let limit = payload_value
            .get("limit")
            .and_then(|value| value.as_u64())
            .unwrap_or(128)
            .min(u32::MAX as u64) as u32;
        let after_offset = payload_value
            .get("after_offset")
            .and_then(|value| value.as_u64());

        let events = patina_sdk::child::patina::events_stream::events_stream::subscribe(
            "record.extracted",
            after_offset,
            limit,
        )?;

        let mut validated_records = 0_u64;
        let mut rejected_records = 0_u64;
        let mut rejected = Vec::new();
        let mut last_offset = None;
        let mut provenance_complete_records = 0_u64;
        let mut provenance_incomplete_records = 0_u64;
        let log = granted::log();

        for event in events {
            last_offset = Some(event.offset);
            let record: Record = serde_json::from_str(&event.payload)
                .map_err(|e| format!("invalid record.extracted payload: {}", e))?;

            if provenance_complete(&record) {
                provenance_complete_records += 1;
                emit_metric_counter("provenance_complete_records", 1.0)?;
            } else {
                provenance_incomplete_records += 1;
                emit_metric_counter("provenance_incomplete_records", 1.0)?;
            }

            match validate_record(&record) {
                Ok(()) => {
                    emit(
                        "record.validated",
                        serde_json::to_string(&record).map_err(|e| e.to_string())?,
                    )?;
                    validated_records += 1;
                }
                Err(reason) => {
                    let rejected_record = RejectedRecord { reason, record };
                    emit(
                        "record.rejected",
                        serde_json::to_string(&rejected_record).map_err(|e| e.to_string())?,
                    )?;
                    rejected.push(rejected_record);
                    rejected_records += 1;
                }
            }
        }

        let provenance_total = provenance_complete_records + provenance_incomplete_records;
        let provenance_completeness_pct = if provenance_total == 0 {
            100.0
        } else {
            (provenance_complete_records as f64 / provenance_total as f64) * 100.0
        };
        emit_metric_gauge("provenance_completeness_pct", provenance_completeness_pct)?;

        if let Some(offset) = last_offset {
            patina_sdk::child::patina::events_stream::events_stream::ack(
                "record.extracted",
                offset,
            )?;
        }

        for item in &rejected {
            log.info(&format!(
                "schema-enforcer rejected {} ({})",
                item.record.source_path, item.reason
            ));
        }

        Ok(serde_json::json!({
            "status": "ok",
            "validated_records": validated_records,
            "rejected_records": rejected_records,
            "provenance_complete_records": provenance_complete_records,
            "provenance_incomplete_records": provenance_incomplete_records,
            "provenance_completeness_pct": provenance_completeness_pct,
            "rejected": rejected,
            "acked_through": last_offset,
        })
        .to_string())
    }
}

impl Child for SchemaEnforcerChild {
    fn name(&self) -> String {
        "schema-enforcer".to_string()
    }

    fn on_load(&mut self) -> Result<(), String> {
        granted::log().info("schema-enforcer loaded");
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
            "enforce-schema" => self.enforce_schema(payload),
            other => Err(format!("schema-enforcer: unknown action '{}'", other)),
        }
    }
}

register_child!(SchemaEnforcerChild);
