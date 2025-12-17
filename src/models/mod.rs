//! Model management for Patina
//!
//! Resolves model files from the mothership cache (`~/.patina/cache/models/`).
//! Tracks provenance in `~/.patina/models.lock`.
//!
//! # Design
//!
//! Base models are infrastructure shared across projects. The registry defines
//! what models exist, the lock file tracks what's downloaded and where it came from.
//!
//! ```text
//! registry.toml (in binary)  →  What models exist
//!      ↓
//! models.lock (mothership)   →  What's downloaded + provenance
//!      ↓
//! config.toml (project)      →  What model this project uses
//!      ↓
//! this module                →  Resolves path to actual files
//! ```
//!
//! # Example
//!
//! ```ignore
//! use patina::models::{ModelLock, LockedModel};
//!
//! // Load existing lock file (or empty if doesn't exist)
//! let lock = ModelLock::load()?;
//!
//! // Check if model is available
//! if let Some(model) = lock.get("e5-base-v2") {
//!     println!("Downloaded: {}", model.downloaded);
//! }
//!
//! // Record a new download
//! lock.insert("e5-base-v2", LockedModel { ... });
//! lock.save()?;
//! ```

mod download;
mod internal;

pub use download::{download_and_verify, download_file, sha256_file};
pub use internal::{LockedModel, ModelLock};

use crate::paths;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Check if a model is available in the mothership cache.
///
/// Returns the path to the model directory if tokenizer.json exists and
/// either model.onnx or model_quantized.onnx exists.
pub fn cached_model_path(name: &str) -> Option<PathBuf> {
    let model_dir = paths::models::model_dir(name);
    let tokenizer = paths::models::model_tokenizer(name);

    if !tokenizer.exists() {
        return None;
    }

    // Check for model.onnx or model_quantized.onnx
    let onnx = model_dir.join("model.onnx");
    let onnx_quantized = model_dir.join("model_quantized.onnx");

    if onnx.exists() || onnx_quantized.exists() {
        Some(model_dir)
    } else {
        None
    }
}

/// Check if a model is tracked in the lock file.
///
/// A model can be tracked (recorded in lock) but not present (files deleted).
pub fn is_tracked(name: &str) -> Result<bool> {
    let lock = ModelLock::load()?;
    Ok(lock.get(name).is_some())
}

/// Check if a directory contains valid model files.
///
/// Valid = tokenizer.json + (model.onnx OR model_quantized.onnx)
fn has_valid_model_files(dir: &Path) -> bool {
    let tokenizer = dir.join("tokenizer.json");
    if !tokenizer.exists() {
        return false;
    }

    let onnx = dir.join("model.onnx");
    let onnx_quantized = dir.join("model_quantized.onnx");
    onnx.exists() || onnx_quantized.exists()
}

/// Get the resolved path for a model, checking cache first then local.
///
/// Resolution order:
/// 1. Mothership cache (`~/.patina/cache/models/{name}/`)
/// 2. Local project path (`resources/models/{name}/`)
///
/// Returns the first path where valid model files exist.
pub fn resolve_model_path(name: &str) -> Result<PathBuf> {
    // Try mothership cache first
    if let Some(path) = cached_model_path(name) {
        return Ok(path);
    }

    // Fall back to local (legacy) path
    let local_path = PathBuf::from(format!("resources/models/{}", name));
    if has_valid_model_files(&local_path) {
        return Ok(local_path);
    }

    anyhow::bail!(
        "Model '{}' not found. Run `patina model add {}` to download it.",
        name,
        name
    )
}

/// Get model status: where it's available and provenance info.
#[derive(Debug)]
pub struct ModelStatus {
    pub name: String,
    pub in_cache: bool,
    pub in_local: bool,
    pub provenance: Option<LockedModel>,
}

/// Check the status of a model.
pub fn model_status(name: &str) -> Result<ModelStatus> {
    let lock = ModelLock::load()?;
    let provenance = lock.get(name).cloned();

    let cache_dir = paths::models::model_dir(name);
    let in_cache = has_valid_model_files(&cache_dir);

    let local_path = PathBuf::from(format!("resources/models/{}", name));
    let in_local = has_valid_model_files(&local_path);

    Ok(ModelStatus {
        name: name.to_string(),
        in_cache,
        in_local,
        provenance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_model_path_missing() {
        // Model doesn't exist in cache
        let path = cached_model_path("nonexistent-model-xyz");
        assert!(path.is_none());
    }

    #[test]
    fn test_resolve_model_path_local_fallback() {
        // Should find local model if it exists
        let result = resolve_model_path("all-minilm-l6-v2");
        // This will succeed if local model exists, fail otherwise
        // Either outcome is valid for this test
        match result {
            Ok(path) => {
                assert!(path.to_string_lossy().contains("all-minilm-l6-v2"));
            }
            Err(e) => {
                assert!(e.to_string().contains("not found"));
            }
        }
    }
}
