use chrono::Utc;
use patina_sdk::granted;
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChild};
use patina_sdk::register_knowledge_child;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Default)]
struct LakehouseCatalogChild;

#[derive(Debug, Clone, Deserialize)]
struct FileWrittenEvent {
    file_path: String,
    record_count: u64,
    written_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct CatalogFileEntry {
    file_path: String,
    record_count: u64,
    written_at: String,
    registered_at: String,
    schema_version: u32,
}

#[derive(Debug, Clone, Serialize)]
struct CatalogSchema {
    version: u32,
    columns: Vec<String>,
    updated_at: String,
}

fn parse_payload(payload: &str) -> Result<Value, String> {
    if payload.trim().is_empty() {
        Ok(serde_json::json!({}))
    } else {
        serde_json::from_str(payload).map_err(|e| format!("invalid payload json: {}", e))
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

impl LakehouseCatalogChild {
    fn register_written(&mut self, payload: &str) -> Result<String, String> {
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
            "file.written",
            after_offset,
            limit,
        )?;

        let state = granted::state();
        let mut registered = 0_u64;
        let mut entries = Vec::new();
        let mut last_offset = None;

        for event in events {
            last_offset = Some(event.offset);
            let file_written: FileWrittenEvent = serde_json::from_str(&event.payload)
                .map_err(|e| format!("invalid file.written payload: {}", e))?;

            let file_key = format!(
                "catalog:file:{}",
                sha256_hex(file_written.file_path.as_bytes())
            );
            let entry = CatalogFileEntry {
                file_path: file_written.file_path,
                record_count: file_written.record_count,
                written_at: file_written.written_at,
                registered_at: Utc::now().to_rfc3339(),
                schema_version: 1,
            };

            state.put(
                &file_key,
                &serde_json::to_string(&entry).map_err(|e| e.to_string())?,
            )?;
            entries.push(entry);
            registered += 1;
        }

        if let Some(offset) = last_offset {
            patina_sdk::knowledge_child::patina::events_stream::events_stream::ack(
                "file.written",
                offset,
            )?;
        }

        let schema = CatalogSchema {
            version: 1,
            columns: vec![
                "record_id".to_string(),
                "source_path".to_string(),
                "source_hash".to_string(),
                "source_modified_at".to_string(),
                "source_size_bytes".to_string(),
                "content".to_string(),
                "content_hash".to_string(),
                "content_type".to_string(),
                "encoding".to_string(),
                "line_count".to_string(),
                "ingested_at".to_string(),
                "batch_id".to_string(),
                "schema_version".to_string(),
            ],
            updated_at: Utc::now().to_rfc3339(),
        };
        state.put(
            "catalog:schema:current",
            &serde_json::to_string(&schema).map_err(|e| e.to_string())?,
        )?;

        Ok(serde_json::json!({
            "status": "ok",
            "registered_files": registered,
            "entries": entries,
            "catalog_keys": state.list_prefix("catalog:file:"),
            "acked_through": last_offset,
        })
        .to_string())
    }
}

impl KnowledgeChild for LakehouseCatalogChild {
    fn name(&self) -> String {
        "lakehouse-catalog".to_string()
    }

    fn on_load(&mut self) -> Result<(), String> {
        granted::log().info("lakehouse-catalog loaded");
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
            "register-written" => self.register_written(payload),
            other => Err(format!("lakehouse-catalog: unknown action '{}'", other)),
        }
    }
}

register_knowledge_child!(LakehouseCatalogChild);
