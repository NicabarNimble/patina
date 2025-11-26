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
    /// Recipe version (for future compatibility)
    pub version: u32,
    /// Embedding model to use (e.g., "e5-base-v2")
    pub embedding_model: String,
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

    /// Validate recipe configuration
    fn validate(&self) -> Result<()> {
        // Check version
        if self.version != 1 {
            anyhow::bail!("Unsupported recipe version: {}", self.version);
        }

        // Check at least one projection
        if self.projections.is_empty() {
            anyhow::bail!("Recipe must define at least one projection");
        }

        // Validate each projection
        for (name, config) in &self.projections {
            config
                .validate(name)
                .with_context(|| format!("Invalid projection config: {}", name))?;
        }

        Ok(())
    }

    /// Get projection by name
    pub fn get_projection(&self, name: &str) -> Option<&ProjectionConfig> {
        self.projections.get(name)
    }
}

impl ProjectionConfig {
    /// Validate projection configuration
    fn validate(&self, name: &str) -> Result<()> {
        // Check layers
        if self.layers.len() != 3 {
            anyhow::bail!(
                "Projection '{}': layers must have exactly 3 dimensions [input, hidden, output], got {}",
                name,
                self.layers.len()
            );
        }

        if self.layers.contains(&0) {
            anyhow::bail!("Projection '{}': layer dimensions must be > 0", name);
        }

        // Check epochs
        if self.epochs == 0 {
            anyhow::bail!("Projection '{}': epochs must be > 0", name);
        }

        // Check batch size
        if self.batch_size == 0 {
            anyhow::bail!("Projection '{}': batch_size must be > 0", name);
        }

        Ok(())
    }

    /// Get input dimension
    pub fn input_dim(&self) -> usize {
        self.layers[0]
    }

    /// Get hidden dimension
    pub fn hidden_dim(&self) -> usize {
        self.layers[1]
    }

    /// Get output dimension
    pub fn output_dim(&self) -> usize {
        self.layers[2]
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
        assert_eq!(recipe.embedding_model, "e5-base-v2");
        assert_eq!(recipe.projections.len(), 1);

        let semantic = recipe.get_projection("semantic").unwrap();
        assert_eq!(semantic.layers, vec![768, 1024, 256]);
        assert_eq!(semantic.epochs, 10);
        assert_eq!(semantic.batch_size, 32);
        assert_eq!(semantic.input_dim(), 768);
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
