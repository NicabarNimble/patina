wit_bindgen::generate!({
    path: "wit",
    world: "file-system-monitor",
    generate_all,
});

use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

struct FileSystemMonitor;

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn content_type_supported(ext: &str) -> bool {
    matches!(ext, "txt" | "md")
}

fn list_flat_entries(folder: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    let entries = std::fs::read_dir(folder)
        .map_err(|e| format!("failed to read folder '{}': {}", folder.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read folder entry: {}", e))?;
        paths.push(entry.path());
    }
    paths.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
    Ok(paths)
}

impl exports::patina::records::source::Guest for FileSystemMonitor {
    fn scan(folder: String) -> Result<Vec<patina::records::types::FileFound>, String> {
        let folder_path = PathBuf::from(&folder);
        if !folder_path.exists() {
            return Err(format!("folder does not exist: {}", folder_path.display()));
        }
        if !folder_path.is_dir() {
            return Err(format!(
                "folder_path is not a directory: {}",
                folder_path.display()
            ));
        }

        let mut discovered = Vec::new();

        for path in list_flat_entries(&folder_path)? {
            let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if file_name.starts_with('.') {
                continue;
            }

            let file_type = std::fs::symlink_metadata(&path)
                .map_err(|e| format!("failed to stat '{}': {}", path.display(), e))?
                .file_type();
            if file_type.is_symlink() || !file_type.is_file() {
                continue;
            }

            let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                continue;
            };
            if !content_type_supported(ext) {
                continue;
            }

            let bytes = std::fs::read(&path)
                .map_err(|e| format!("failed to read file '{}': {}", path.display(), e))?;
            if bytes.is_empty() || String::from_utf8(bytes.clone()).is_err() {
                continue;
            }

            discovered.push(patina::records::types::FileFound {
                source_path: path.to_string_lossy().to_string(),
                source_hash: sha256_hex(&bytes),
                source_size_bytes: bytes.len() as u64,
                discovered_at: Utc::now().to_rfc3339(),
            });
        }

        patina::measure::measure::counter("files_discovered", discovered.len() as f64)?;
        wasi::logging::logging::log(
            wasi::logging::logging::Level::Info,
            "file-system-monitor",
            &format!("discovered {} files", discovered.len()),
        );

        Ok(discovered)
    }
}

export!(FileSystemMonitor);
