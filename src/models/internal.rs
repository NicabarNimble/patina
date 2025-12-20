//! Internal implementation for model management.
//!
//! Lock file format and parsing. Not exposed in public API.

use crate::paths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// A locked model entry with provenance information.
///
/// Records when and where a model was downloaded from, plus integrity checksums.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedModel {
    /// When this model was downloaded (ISO 8601)
    pub downloaded: String,

    /// Source URL for the ONNX model file
    pub source_model: String,

    /// Source URL for the tokenizer file
    pub source_tokenizer: String,

    /// SHA256 checksum of the model file
    pub sha256_model: String,

    /// SHA256 checksum of the tokenizer file
    pub sha256_tokenizer: String,

    /// Total size in bytes
    pub size_bytes: u64,

    /// Embedding dimensions (from registry)
    pub dimensions: usize,
}

/// The models.lock file - tracks all downloaded models with provenance.
///
/// Format: TOML with each model as a section.
///
/// ```toml
/// # Patina Model Lock File
/// # Auto-generated - do not edit manually
///
/// [e5-base-v2]
/// downloaded = "2025-12-16T19:30:00Z"
/// source_model = "https://huggingface.co/..."
/// source_tokenizer = "https://huggingface.co/..."
/// sha256_model = "abc123..."
/// sha256_tokenizer = "def456..."
/// size_bytes = 110000000
/// dimensions = 768
/// ```
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ModelLock {
    #[serde(flatten)]
    models: HashMap<String, LockedModel>,
}

impl ModelLock {
    /// Load the lock file, or return empty if it doesn't exist.
    pub fn load() -> Result<Self> {
        let path = paths::models::lock_path();

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read lock file: {:?}", path))?;

        toml::from_str(&content).with_context(|| "Failed to parse models.lock")
    }

    /// Save the lock file.
    pub fn save(&self) -> Result<()> {
        let path = paths::models::lock_path();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let header = "# Patina Model Lock File\n\
                      # Auto-generated - do not edit manually\n\
                      # Re-download with: patina model add <name>\n\n";

        let content = toml::to_string_pretty(&self)?;
        let full_content = format!("{}{}", header, content);

        fs::write(&path, full_content)
            .with_context(|| format!("Failed to write lock file: {:?}", path))?;

        Ok(())
    }

    /// Get a locked model by name.
    pub fn get(&self, name: &str) -> Option<&LockedModel> {
        self.models.get(name)
    }

    /// Insert or update a locked model.
    pub fn insert(&mut self, name: &str, model: LockedModel) {
        self.models.insert(name.to_string(), model);
    }

    /// Remove a model from the lock file.
    pub fn remove(&mut self, name: &str) -> Option<LockedModel> {
        self.models.remove(name)
    }

    /// List all locked model names.
    pub fn list(&self) -> Vec<&str> {
        self.models.keys().map(|s| s.as_str()).collect()
    }

    /// Check if any models are tracked.
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// Number of tracked models.
    pub fn len(&self) -> usize {
        self.models.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locked_model_serde() {
        let model = LockedModel {
            downloaded: "2025-12-16T19:30:00Z".to_string(),
            source_model: "https://example.com/model.onnx".to_string(),
            source_tokenizer: "https://example.com/tokenizer.json".to_string(),
            sha256_model: "abc123".to_string(),
            sha256_tokenizer: "def456".to_string(),
            size_bytes: 100_000_000,
            dimensions: 768,
        };

        let toml_str = toml::to_string(&model).unwrap();
        assert!(toml_str.contains("downloaded"));
        assert!(toml_str.contains("sha256_model"));

        let parsed: LockedModel = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.dimensions, 768);
    }

    #[test]
    fn test_model_lock_operations() {
        let mut lock = ModelLock::default();
        assert!(lock.is_empty());

        let model = LockedModel {
            downloaded: "2025-12-16T19:30:00Z".to_string(),
            source_model: "https://example.com/model.onnx".to_string(),
            source_tokenizer: "https://example.com/tokenizer.json".to_string(),
            sha256_model: "abc123".to_string(),
            sha256_tokenizer: "def456".to_string(),
            size_bytes: 100_000_000,
            dimensions: 768,
        };

        lock.insert("test-model", model);
        assert_eq!(lock.len(), 1);
        assert!(lock.get("test-model").is_some());
        assert!(lock.get("other-model").is_none());

        let names = lock.list();
        assert!(names.contains(&"test-model"));

        lock.remove("test-model");
        assert!(lock.is_empty());
    }

    #[test]
    fn test_model_lock_roundtrip() {
        let mut lock = ModelLock::default();
        lock.insert(
            "e5-base-v2",
            LockedModel {
                downloaded: "2025-12-16T19:30:00Z".to_string(),
                source_model: "https://huggingface.co/model.onnx".to_string(),
                source_tokenizer: "https://huggingface.co/tokenizer.json".to_string(),
                sha256_model: "abc123def456".to_string(),
                sha256_tokenizer: "789ghi012jkl".to_string(),
                size_bytes: 110_000_000,
                dimensions: 768,
            },
        );

        // Serialize
        let toml_str = toml::to_string_pretty(&lock).unwrap();
        assert!(toml_str.contains("[e5-base-v2]"));
        assert!(toml_str.contains("dimensions = 768"));

        // Deserialize
        let parsed: ModelLock = toml::from_str(&toml_str).unwrap();
        let model = parsed.get("e5-base-v2").unwrap();
        assert_eq!(model.dimensions, 768);
        assert_eq!(model.sha256_model, "abc123def456");
    }
}
