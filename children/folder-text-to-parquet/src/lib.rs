use chrono::{DateTime, Utc};
use patina_sdk::granted;
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChild};
use patina_sdk::register_knowledge_child;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use uuid::Uuid;

#[derive(Default)]
struct FolderTextToParquetChild;

#[derive(Debug, Clone, Serialize)]
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
struct FileFoundEvent {
    source_path: String,
    source_hash: String,
    source_size_bytes: u64,
    discovered_at: String,
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

fn resolve_folder_path(payload: &Value) -> Result<PathBuf, String> {
    let Some(folder_path) = payload.get("folder_path").and_then(|v| v.as_str()) else {
        return Err("missing folder_path in action payload".to_string());
    };
    if folder_path.trim().is_empty() {
        return Err("folder_path cannot be empty".to_string());
    }
    Ok(PathBuf::from(folder_path))
}

fn emit_metric_counter(
    name: &str,
    delta: f64,
    labels: Vec<(String, String)>,
) -> Result<(), String> {
    patina_sdk::knowledge_child::patina::measure::measure::emit(
        &patina_sdk::knowledge_child::patina::measure::measure::Metric {
            name: name.to_string(),
            value: delta,
            labels,
        },
    )
}

fn emit_metric_gauge(name: &str, value: f64) -> Result<(), String> {
    patina_sdk::knowledge_child::patina::measure::measure::gauge(name, value)
}

fn emit_file_found(event: &FileFoundEvent) -> Result<u64, String> {
    let payload = serde_json::to_string(event).map_err(|e| e.to_string())?;
    let client = patina_sdk::knowledge_child::wasi::messaging::producer::connect("file.found")?;
    let message = patina_sdk::knowledge_child::wasi::messaging::types::Message {
        topic: "file.found".to_string(),
        content_type: Some("application/json".to_string()),
        data: payload.into_bytes(),
        metadata: vec![],
    };
    patina_sdk::knowledge_child::wasi::messaging::producer::send(&client, &message)
}

fn list_flat_entries(folder: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    let entries = fs::read_dir(folder)
        .map_err(|e| format!("failed to read folder '{}': {}", folder.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read folder entry: {}", e))?;
        paths.push(entry.path());
    }
    paths.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
    Ok(paths)
}

impl FolderTextToParquetChild {
    fn scan(&mut self, payload: &str) -> Result<String, String> {
        let payload_value = parse_payload(payload)?;
        let folder_path = resolve_folder_path(&payload_value)?;

        if !folder_path.exists() {
            return Err(format!("folder does not exist: {}", folder_path.display()));
        }
        if !folder_path.is_dir() {
            return Err(format!(
                "folder_path is not a directory: {}",
                folder_path.display()
            ));
        }

        let batch_id = format!("scan-{}", Utc::now().format("%Y%m%d-%H%M%S"));
        let folder_label = folder_path.to_string_lossy().to_string();

        let mut records = Vec::new();
        let mut skipped = Vec::new();
        let state = granted::state();
        let log = granted::log();

        for path in list_flat_entries(&folder_path)? {
            let path_str = path.to_string_lossy().to_string();

            let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
                skipped.push(SkippedFile {
                    path: path_str,
                    reason: "non-utf8-filename".to_string(),
                });
                continue;
            };

            if file_name.starts_with('.') {
                skipped.push(SkippedFile {
                    path: path.to_string_lossy().to_string(),
                    reason: "hidden".to_string(),
                });
                continue;
            }

            let file_type = fs::symlink_metadata(&path)
                .map_err(|e| format!("failed to stat '{}': {}", path.display(), e))?
                .file_type();
            if file_type.is_symlink() {
                skipped.push(SkippedFile {
                    path: path.to_string_lossy().to_string(),
                    reason: "symlink".to_string(),
                });
                continue;
            }
            if !file_type.is_file() {
                skipped.push(SkippedFile {
                    path: path.to_string_lossy().to_string(),
                    reason: "non-file".to_string(),
                });
                continue;
            }

            let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                skipped.push(SkippedFile {
                    path: path.to_string_lossy().to_string(),
                    reason: "missing-extension".to_string(),
                });
                continue;
            };
            let Some(content_type) = content_type_for_extension(ext) else {
                skipped.push(SkippedFile {
                    path: path.to_string_lossy().to_string(),
                    reason: "unsupported-extension".to_string(),
                });
                continue;
            };

            let bytes = fs::read(&path)
                .map_err(|e| format!("failed to read file '{}': {}", path.display(), e))?;
            if bytes.is_empty() {
                skipped.push(SkippedFile {
                    path: path.to_string_lossy().to_string(),
                    reason: "empty".to_string(),
                });
                continue;
            }

            let content = match String::from_utf8(bytes.clone()) {
                Ok(value) => value,
                Err(_) => {
                    skipped.push(SkippedFile {
                        path: path.to_string_lossy().to_string(),
                        reason: "non-utf8-content".to_string(),
                    });
                    continue;
                }
            };

            let metadata = fs::metadata(&path)
                .map_err(|e| format!("failed to read metadata '{}': {}", path.display(), e))?;
            let source_hash = sha256_hex(&bytes);
            let content_hash = sha256_hex(content.as_bytes());
            let source_modified_at = metadata
                .modified()
                .map(rfc3339_from_system_time)
                .unwrap_or_else(|_| Utc::now().to_rfc3339());
            let discovered_at = Utc::now().to_rfc3339();

            let record = Record {
                record_id: Uuid::new_v4().to_string(),
                source_path: path.to_string_lossy().to_string(),
                source_hash: source_hash.clone(),
                source_modified_at,
                source_size_bytes: bytes.len() as u64,
                content,
                content_hash,
                content_type: content_type.to_string(),
                encoding: "utf-8".to_string(),
                line_count: 0,
                ingested_at: Utc::now().to_rfc3339(),
                batch_id: batch_id.clone(),
                schema_version: 1,
            };

            let mut record = record;
            record.line_count = record.content.lines().count() as u64;

            let key = format!("record:{}", record.source_hash);
            let state_json = serde_json::to_string(&record).map_err(|e| e.to_string())?;

            let write_start = Instant::now();
            state.put(&key, &state_json)?;
            let write_latency_ms = write_start.elapsed().as_secs_f64() * 1000.0;

            emit_file_found(&FileFoundEvent {
                source_path: record.source_path.clone(),
                source_hash: record.source_hash.clone(),
                source_size_bytes: record.source_size_bytes,
                discovered_at,
            })?;

            emit_metric_counter(
                "files_discovered",
                1.0,
                vec![("source_folder".to_string(), folder_label.clone())],
            )?;
            emit_metric_counter("records_written", 1.0, vec![])?;
            emit_metric_gauge("write_latency_ms", write_latency_ms)?;

            records.push(record);
        }

        for skipped_file in &skipped {
            log.info(&format!(
                "folder-text-to-parquet skipped {} ({})",
                skipped_file.path, skipped_file.reason
            ));
        }

        let state_keys = state.list_prefix("record:");
        Ok(serde_json::json!({
            "status": "ok",
            "source_folder": folder_label,
            "processed_records": records.len(),
            "skipped_files": skipped,
            "state_keys": state_keys,
            "records": records,
        })
        .to_string())
    }
}

impl KnowledgeChild for FolderTextToParquetChild {
    fn name(&self) -> String {
        "folder-text-to-parquet".to_string()
    }

    fn on_load(&mut self) -> Result<(), String> {
        granted::log().info("folder-text-to-parquet loaded");
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
            "scan" => self.scan(payload),
            other => Err(format!(
                "folder-text-to-parquet: unknown action '{}'",
                other
            )),
        }
    }
}

register_knowledge_child!(FolderTextToParquetChild);
