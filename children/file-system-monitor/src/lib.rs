use chrono::Utc;
use patina_sdk::granted;
use patina_sdk::knowledge_child::{ChildHealth, HealthStatus, KnowledgeChild};
use patina_sdk::register_child;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct FileSystemMonitorChild;

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

fn content_type_for_extension(ext: &str) -> Option<&'static str> {
    match ext {
        "txt" => Some("text/plain"),
        "md" => Some("text/markdown"),
        _ => None,
    }
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

impl FileSystemMonitorChild {
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

        let folder_label = folder_path.to_string_lossy().to_string();
        let mut discovered = Vec::new();
        let mut skipped = Vec::new();
        let log = granted::log();

        for path in list_flat_entries(&folder_path)? {
            let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
                skipped.push(SkippedFile {
                    path: path.to_string_lossy().to_string(),
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
            if content_type_for_extension(ext).is_none() {
                skipped.push(SkippedFile {
                    path: path.to_string_lossy().to_string(),
                    reason: "unsupported-extension".to_string(),
                });
                continue;
            }

            let bytes = fs::read(&path)
                .map_err(|e| format!("failed to read file '{}': {}", path.display(), e))?;
            if bytes.is_empty() {
                skipped.push(SkippedFile {
                    path: path.to_string_lossy().to_string(),
                    reason: "empty".to_string(),
                });
                continue;
            }
            if String::from_utf8(bytes.clone()).is_err() {
                skipped.push(SkippedFile {
                    path: path.to_string_lossy().to_string(),
                    reason: "non-utf8-content".to_string(),
                });
                continue;
            }

            let event = FileFoundEvent {
                source_path: path.to_string_lossy().to_string(),
                source_hash: sha256_hex(&bytes),
                source_size_bytes: bytes.len() as u64,
                discovered_at: Utc::now().to_rfc3339(),
            };

            emit_file_found(&event)?;
            emit_metric_counter(
                "files_discovered",
                1.0,
                vec![("source_folder".to_string(), folder_label.clone())],
            )?;
            discovered.push(event);
        }

        for skipped_file in &skipped {
            log.info(&format!(
                "file-system-monitor skipped {} ({})",
                skipped_file.path, skipped_file.reason
            ));
        }

        Ok(serde_json::json!({
            "status": "ok",
            "source_folder": folder_label,
            "discovered": discovered,
            "processed_files": discovered.len(),
            "skipped_files": skipped,
        })
        .to_string())
    }
}

impl KnowledgeChild for FileSystemMonitorChild {
    fn name(&self) -> String {
        "file-system-monitor".to_string()
    }

    fn on_load(&mut self) -> Result<(), String> {
        granted::log().info("file-system-monitor loaded");
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
            other => Err(format!("file-system-monitor: unknown action '{}'", other)),
        }
    }
}

register_child!(FileSystemMonitorChild);
