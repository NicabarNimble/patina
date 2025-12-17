//! Recipe parser for oxidize.yaml
//!
//! Minimal MVP: Only semantic projection with simple configuration

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Oxidize recipe - defines how to build embeddings and projections
#[derive(Debug, Deserialize, Clone)]
pub struct OxidizeRecipe {
    /// Recipe version (1 or 2)
    pub version: u32,
    /// Embedding model to use (optional in v2, read from config.toml)
    pub embedding_model: Option<String>,
    /// Projection configurations
    pub projections: HashMap<String, ProjectionConfig>,
}

/// Configuration for a single projection
#[derive(Debug, Deserialize, Clone)]
pub struct ProjectionConfig {
    /// Layer dimensions: [input, hidden, output]
    /// For e5-base-v2: [768, 1024, 256]
    pub layers: Vec<usize>,
    /// Training epochs
    pub epochs: usize,
    /// Batch size for training
    pub batch_size: usize,
}

impl OxidizeRecipe {
    /// Load recipe from .patina/oxidize.yaml
    pub fn load() -> Result<Self> {
        Self::load_from_path(".patina/oxidize.yaml")
    }

    /// Load recipe from custom path
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read recipe: {}", path.display()))?;

        let recipe: OxidizeRecipe = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse recipe YAML: {}", path.display()))?;

        recipe.validate()?;

        Ok(recipe)
    }

    /// Get the effective model name (from recipe or config.toml)
    pub fn get_model_name(&self) -> Result<String> {
        use patina::embeddings::models::Config;

        if let Some(ref model) = self.embedding_model {
            return Ok(model.clone());
        }

        // Fall back to config.toml
        let config =
            Config::load().context("No embedding_model in recipe and config.toml not found")?;
        Ok(config.embeddings.model)
    }

    /// Get input dimensions from registry for the model
    pub fn get_input_dim(&self) -> Result<usize> {
        use patina::embeddings::models::ModelRegistry;

        let model_name = self.get_model_name()?;
        let registry = ModelRegistry::load()?;
        let model_def = registry.get_model(&model_name)?;
        Ok(model_def.dimensions)
    }

    /// Validate recipe configuration
    fn validate(&self) -> Result<()> {
        // Check version
        if self.version != 1 && self.version != 2 {
            anyhow::bail!(
                "Unsupported recipe version: {} (expected 1 or 2)",
                self.version
            );
        }

        // v1 requires embedding_model
        if self.version == 1 && self.embedding_model.is_none() {
            anyhow::bail!("Recipe v1 requires 'embedding_model' field");
        }

        // Check at least one projection
        if self.projections.is_empty() {
            anyhow::bail!("Recipe must define at least one projection");
        }

        // Validate each projection
        for (name, config) in &self.projections {
            config
                .validate(name, self.version)
                .with_context(|| format!("Invalid projection config: {}", name))?;
        }

        Ok(())
    }
}

impl ProjectionConfig {
    /// Validate projection configuration
    fn validate(&self, name: &str, version: u32) -> Result<()> {
        // Check layers based on version
        // v2 uses [hidden, output], v1 uses [input, hidden, output]
        let expected = if version == 1 {
            "[input, hidden, output]"
        } else {
            "[hidden, output]"
        };

        // v1: exactly 3, v2: 2 or 3 (3 for backwards compat)
        if version == 1 && self.layers.len() != 3 {
            anyhow::bail!(
                "Projection '{}': v1 layers must have 3 dimensions {}, got {}",
                name,
                expected,
                self.layers.len()
            );
        }

        if version == 2 && self.layers.len() != 2 && self.layers.len() != 3 {
            anyhow::bail!(
                "Projection '{}': v2 layers must have 2 or 3 dimensions, got {}",
                name,
                self.layers.len()
            );
        }

        if self.layers.contains(&0) {
            anyhow::bail!("Projection '{}': layer dimensions must be > 0", name);
        }

        if self.epochs == 0 {
            anyhow::bail!("Projection '{}': epochs must be > 0", name);
        }

        if self.batch_size == 0 {
            anyhow::bail!("Projection '{}': batch_size must be > 0", name);
        }

        Ok(())
    }

    /// Get input dimension (from layers or derive from registry)
    pub fn input_dim(&self, recipe: &OxidizeRecipe) -> Result<usize> {
        if self.layers.len() == 3 {
            // Full [input, hidden, output] - use as-is
            Ok(self.layers[0])
        } else {
            // v2 format [hidden, output] - derive from registry
            recipe.get_input_dim()
        }
    }

    /// Get hidden dimension
    pub fn hidden_dim(&self) -> usize {
        if self.layers.len() == 3 {
            self.layers[1]
        } else {
            self.layers[0] // v2: [hidden, output]
        }
    }

    /// Get output dimension
    pub fn output_dim(&self) -> usize {
        if self.layers.len() == 3 {
            self.layers[2]
        } else {
            self.layers[1] // v2: [hidden, output]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_valid_recipe() {
        let yaml = r#"
version: 1
embedding_model: e5-base-v2

projections:
  semantic:
    layers: [768, 1024, 256]
    epochs: 10
    batch_size: 32
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let recipe = OxidizeRecipe::load_from_path(temp_file.path()).unwrap();

        assert_eq!(recipe.version, 1);
        assert_eq!(recipe.embedding_model.as_deref(), Some("e5-base-v2"));
        assert_eq!(recipe.projections.len(), 1);

        let semantic = recipe.projections.get("semantic").unwrap();
        assert_eq!(semantic.layers, vec![768, 1024, 256]);
        assert_eq!(semantic.epochs, 10);
        assert_eq!(semantic.batch_size, 32);
        // v1 with 3 layers: input_dim comes from layers[0]
        assert_eq!(semantic.input_dim(&recipe).unwrap(), 768);
        assert_eq!(semantic.hidden_dim(), 1024);
        assert_eq!(semantic.output_dim(), 256);
    }

    #[test]
    fn test_invalid_version() {
        let yaml = r#"
version: 99
embedding_model: e5-base-v2
projections:
  semantic:
    layers: [768, 1024, 256]
    epochs: 10
    batch_size: 32
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = OxidizeRecipe::load_from_path(temp_file.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported recipe version"));
    }

    #[test]
    fn test_invalid_layers() {
        let yaml = r#"
version: 1
embedding_model: e5-base-v2
projections:
  semantic:
    layers: [768, 256]
    epochs: 10
    batch_size: 32
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = OxidizeRecipe::load_from_path(temp_file.path());
        assert!(result.is_err(), "Should reject invalid layer count");
    }

    #[test]
    fn test_zero_epochs() {
        let yaml = r#"
version: 1
embedding_model: e5-base-v2
projections:
  semantic:
    layers: [768, 1024, 256]
    epochs: 0
    batch_size: 32
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = OxidizeRecipe::load_from_path(temp_file.path());
        assert!(result.is_err(), "Should reject zero epochs");
    }
}
