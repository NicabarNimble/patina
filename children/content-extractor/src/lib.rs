wit_bindgen::generate!({
    path: "wit",
    world: "content-extractor",
    generate_all,
});

use chrono::{DateTime, Utc};
use patina_sdk::toys;
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

struct ContentExtractor;

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn content_type_for_extension(ext: &str) -> &'static str {
    match ext {
        "txt" => "text/plain",
        "md" => "text/markdown",
        _ => "text/plain",
    }
}

fn rfc3339_from_system_time(value: std::time::SystemTime) -> String {
    DateTime::<Utc>::from(value).to_rfc3339()
}

fn build_record(
    file: patina::records::types::FileFound,
) -> Result<patina::records::types::RecordEnvelope, String> {
    let source_path = file.source_path;
    let path = Path::new(&source_path);
    let bytes = std::fs::read(path)
        .map_err(|e| format!("failed to read file '{}': {}", path.display(), e))?;
    let content = String::from_utf8(bytes.clone())
        .map_err(|_| format!("file '{}' is not valid utf-8", path.display()))?;
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("failed to read metadata '{}': {}", path.display(), e))?;
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let source_modified_at = metadata
        .modified()
        .map(rfc3339_from_system_time)
        .unwrap_or_else(|_| Utc::now().to_rfc3339());

    Ok(patina::records::types::RecordEnvelope {
        record_id: Uuid::new_v4().to_string(),
        source_path,
        source_hash: if file.source_hash.is_empty() {
            sha256_hex(&bytes)
        } else {
            file.source_hash
        },
        source_modified_at,
        source_size_bytes: if file.source_size_bytes == 0 {
            bytes.len() as u64
        } else {
            file.source_size_bytes
        },
        content: content.clone(),
        content_hash: sha256_hex(content.as_bytes()),
        content_type: content_type_for_extension(&extension).to_string(),
        encoding: "utf-8".to_string(),
        line_count: content.lines().count() as u64,
        ingested_at: Utc::now().to_rfc3339(),
        batch_id: format!("scan-{}", Utc::now().format("%Y%m%d-%H%M%S")),
        schema_version: 1,
    })
}

impl exports::patina::records::extract::Guest for ContentExtractor {
    fn extract(
        files: Vec<patina::records::types::FileFound>,
    ) -> Result<Vec<patina::records::types::RecordEnvelope>, String> {
        let mut records = Vec::new();

        for file in files {
            match build_record(file) {
                Ok(record) => records.push(record),
                Err(error) => {
                    toys::log::warn("content-extractor", &error);
                }
            }
        }

        Ok(records)
    }
}

export!(ContentExtractor);
