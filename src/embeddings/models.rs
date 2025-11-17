//! Model registry and configuration for embedding models
//!
//! Allows easy switching between different embedding models via configuration.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// Model definition from registry
#[derive(Debug, Deserialize, Clone)]
pub struct ModelDefinition {
    pub name: String,
    pub description: String,
    pub path: String,
    pub dimensions: usize,
    pub metric: String,
    pub source: String,
    pub use_case: String,
    pub performance: String,
    #[serde(default)]
    pub instructions: Option<String>,

    /// Query prefix for asymmetric models (e.g., "Represent this sentence for searching relevant passages: " for BGE)
    #[serde(default)]
    pub query_prefix: Option<String>,

    /// Passage prefix for asymmetric models (e.g., "passage: " for E5)
    #[serde(default)]
    pub passage_prefix: Option<String>,
}

/// Model registry (from resources/models/registry.toml)
#[derive(Debug, Deserialize)]
pub struct ModelRegistry {
    pub models: HashMap<String, ModelDefinition>,
    pub default: DefaultConfig,
}

/// Default configuration
#[derive(Debug, Deserialize)]
pub struct DefaultConfig {
    pub model: String,
    pub benchmark_queries: Vec<String>,
}

/// User configuration (from .patina/config.toml)
#[derive(Debug, Deserialize)]
pub struct Config {
    pub embeddings: EmbeddingsConfig,
}

#[derive(Debug, Deserialize)]
pub struct EmbeddingsConfig {
    pub model: String,
}

impl ModelRegistry {
    /// Load model registry from resources/models/registry.toml
    pub fn load() -> Result<Self> {
        let registry_path = PathBuf::from("resources/models/registry.toml");
        let content = std::fs::read_to_string(&registry_path)
            .with_context(|| format!("Failed to read model registry: {:?}", registry_path))?;

        toml::from_str(&content).context("Failed to parse model registry TOML")
    }

    /// Get model definition by name
    pub fn get_model(&self, name: &str) -> Result<&ModelDefinition> {
        self.models
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Model '{}' not found in registry", name))
    }

    /// List available models
    pub fn list_models(&self) -> Vec<&String> {
        self.models.keys().collect()
    }
}

impl Config {
    /// Load user configuration from .patina/config.toml
    pub fn load() -> Result<Self> {
        let config_path = PathBuf::from(".patina/config.toml");

        // Create default config if doesn't exist
        if !config_path.exists() {
            return Self::create_default();
        }

        let content = std::fs::read_to_string(&config_path)
            .context("Failed to read config file")?;

        toml::from_str(&content).context("Failed to parse config TOML")
    }

    /// Create default configuration
    fn create_default() -> Result<Self> {
        std::fs::create_dir_all(".patina")?;

        let default_config = r#"# Patina User Configuration
[embeddings]
model = "all-minilm-l6-v2"
"#;

        std::fs::write(".patina/config.toml", default_config)?;

        Ok(Config {
            embeddings: EmbeddingsConfig {
                model: "all-minilm-l6-v2".to_string(),
            },
        })
    }

    /// Get current model definition from registry
    pub fn get_model_definition(&self) -> Result<ModelDefinition> {
        let registry = ModelRegistry::load()?;
        let model = registry.get_model(&self.embeddings.model)?;
        Ok(model.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_registry() {
        let registry = ModelRegistry::load().expect("Failed to load registry");
        assert!(registry.models.contains_key("all-minilm-l6-v2"));
        assert!(registry.models.contains_key("bge-base-en-v1-5"));
    }

    #[test]
    fn test_get_model() {
        let registry = ModelRegistry::load().unwrap();
        let model = registry.get_model("all-minilm-l6-v2").unwrap();
        assert_eq!(model.dimensions, 384);
        assert_eq!(model.metric, "cosine");
    }

    #[test]
    fn test_list_models() {
        let registry = ModelRegistry::load().unwrap();
        let models = registry.list_models();
        assert!(!models.is_empty());
    }
}
