use chrono::{DateTime, Utc};
use patina_sdk::granted;
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChild};
use patina_sdk::register_child;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Default)]
struct ContentExtractorChild;

#[derive(Debug, Clone, Deserialize)]
struct FileFoundPayload {
    source_path: String,
}

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
struct SkippedFile {
    path: String,
    reason: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn content_type_for_extension(ext: &str) -> Option<&'static str> {
    match ext {
        "txt" => Some("text/plain"),
        "md" => Some("text/markdown"),
        _ => None,
    }
}

fn rfc3339_from_system_time(value: std::time::SystemTime) -> String {
    DateTime::<Utc>::from(value).to_rfc3339()
}

fn parse_payload(payload: &str) -> Result<Value, String> {
    if payload.trim().is_empty() {
        Ok(serde_json::json!({}))
    } else {
        serde_json::from_str(payload).map_err(|e| format!("invalid payload json: {}", e))
    }
}

fn emit_record_extracted(record: &Record) -> Result<u64, String> {
    let payload = serde_json::to_string(record).map_err(|e| e.to_string())?;
    let client =
        patina_sdk::knowledge_child::wasi::messaging::producer::connect("record.extracted")?;
    let message = patina_sdk::knowledge_child::wasi::messaging::types::Message {
        topic: "record.extracted".to_string(),
        content_type: Some("application/json".to_string()),
        data: payload.into_bytes(),
        metadata: vec![],
    };
    patina_sdk::knowledge_child::wasi::messaging::producer::send(&client, &message)
}

fn build_record_from_path(path: &Path, batch_id: &str) -> Result<Record, String> {
    let bytes =
        fs::read(path).map_err(|e| format!("failed to read file '{}': {}", path.display(), e))?;
    let content = String::from_utf8(bytes.clone())
        .map_err(|_| format!("file '{}' is not valid utf-8", path.display()))?;
    let metadata = fs::metadata(path)
        .map_err(|e| format!("failed to read metadata '{}': {}", path.display(), e))?;
    let source_hash = sha256_hex(&bytes);
    let content_hash = sha256_hex(content.as_bytes());
    let source_modified_at = metadata
        .modified()
        .map(rfc3339_from_system_time)
        .unwrap_or_else(|_| Utc::now().to_rfc3339());

    Ok(Record {
        record_id: Uuid::new_v4().to_string(),
        source_path: path.to_string_lossy().to_string(),
        source_hash,
        source_modified_at,
        source_size_bytes: bytes.len() as u64,
        content_hash,
        content_type: content_type_for_extension(
            path.extension().and_then(|s| s.to_str()).unwrap_or(""),
        )
        .unwrap_or("text/plain")
        .to_string(),
        encoding: "utf-8".to_string(),
        line_count: content.lines().count() as u64,
        ingested_at: Utc::now().to_rfc3339(),
        batch_id: batch_id.to_string(),
        schema_version: 1,
        content,
    })
}

impl ContentExtractorChild {
    fn extract_found(&mut self, payload: &str) -> Result<String, String> {
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
            "file.found",
            after_offset,
            limit,
        )?;

        let batch_id = format!("scan-{}", Utc::now().format("%Y%m%d-%H%M%S"));
        let mut records = Vec::new();
        let mut skipped = Vec::new();
        let mut last_offset = None;
        let log = granted::log();

        for event in events {
            last_offset = Some(event.offset);
            let payload: FileFoundPayload = serde_json::from_str(&event.payload)
                .map_err(|e| format!("invalid file.found payload: {}", e))?;
            let path = PathBuf::from(&payload.source_path);

            match build_record_from_path(&path, &batch_id) {
                Ok(record) => {
                    emit_record_extracted(&record)?;
                    records.push(record);
                }
                Err(error) => {
                    skipped.push(SkippedFile {
                        path: path.to_string_lossy().to_string(),
                        reason: error,
                    });
                }
            }
        }

        if let Some(offset) = last_offset {
            patina_sdk::knowledge_child::patina::events_stream::events_stream::ack(
                "file.found",
                offset,
            )?;
        }

        for skipped_file in &skipped {
            log.info(&format!(
                "content-extractor skipped {} ({})",
                skipped_file.path, skipped_file.reason
            ));
        }

        Ok(serde_json::json!({
            "status": "ok",
            "processed_records": records.len(),
            "skipped_files": skipped,
            "records": records,
            "acked_through": last_offset,
        })
        .to_string())
    }
}

impl KnowledgeChild for ContentExtractorChild {
    fn name(&self) -> String {
        "content-extractor".to_string()
    }

    fn on_load(&mut self) -> Result<(), String> {
        granted::log().info("content-extractor loaded");
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
            "extract-found" => self.extract_found(payload),
            other => Err(format!("content-extractor: unknown action '{}'", other)),
        }
    }
}

register_child!(ContentExtractorChild);
