//! Embeddings module - Generate semantic embeddings for text
//!
//! Provides trait-based abstraction for embedding generation with ONNX backend.
//! Supports multiple embedding models via configuration.

mod database;
pub mod models;
mod onnx;
mod similarity;

pub use database::{EmbeddingMetadata, EmbeddingsDatabase};
pub use models::{Config, ModelDefinition, ModelRegistry};
pub use onnx::OnnxEmbedder;
pub use similarity::{cosine_similarity, euclidean_distance};

use anyhow::Result;

/// Trait for embedding generation engines
///
/// Requires Send for use in cached oracles (parallel query execution via rayon)
pub trait EmbeddingEngine: Send {
    /// Generate embedding for a single text
    fn embed(&mut self, text: &str) -> Result<Vec<f32>>;

    /// Generate embedding for a query text (with model-specific prefix if needed)
    ///
    /// For asymmetric models (e.g., BGE, E5), this applies query-specific formatting.
    /// For symmetric models (e.g., all-MiniLM), this is identical to embed().
    ///
    /// Default implementation calls embed() for backwards compatibility.
    fn embed_query(&mut self, text: &str) -> Result<Vec<f32>> {
        self.embed(text)
    }

    /// Generate embedding for a passage text (with model-specific prefix if needed)
    ///
    /// For asymmetric models (e.g., BGE, E5), this applies passage-specific formatting.
    /// For symmetric models (e.g., all-MiniLM), this is identical to embed().
    ///
    /// Default implementation calls embed() for backwards compatibility.
    fn embed_passage(&mut self, text: &str) -> Result<Vec<f32>> {
        self.embed(text)
    }

    /// Generate embeddings for multiple texts (batch processing)
    fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Get embedding dimension (e.g., 384 for all-MiniLM-L6-v2)
    fn dimension(&self) -> usize;

    /// Get model name
    fn model_name(&self) -> &str;
}

/// Factory function to create embedder from configuration
///
/// Reads model selection from .patina/config.toml and creates appropriate embedder.
/// Falls back to default (all-MiniLM-L6-v2) if config doesn't exist.
pub fn create_embedder() -> Result<Box<dyn EmbeddingEngine>> {
    create_embedder_from_config()
}

/// Create embedder from configuration file
fn create_embedder_from_config() -> Result<Box<dyn EmbeddingEngine>> {
    // Load user config (creates default if doesn't exist)
    let config = Config::load()?;

    // Get model definition from registry
    let model_def = config.get_model_definition()?;

    // Create ONNX embedder with model-specific paths
    create_onnx_embedder(&model_def)
}

/// Create ONNX embedder from model definition
fn create_onnx_embedder(model_def: &ModelDefinition) -> Result<Box<dyn EmbeddingEngine>> {
    // Resolve model path: checks mother cache first, then local
    let model_dir = crate::models::resolve_model_path(&model_def.name)?;

    // Construct paths based on model directory
    let model_path = model_dir.join("model.onnx");
    let tokenizer_path = model_dir.join("tokenizer.json");

    // Try quantized model first (if exists)
    let model_path_quantized = model_dir.join("model_quantized.onnx");
    let final_model_path = if model_path_quantized.exists() {
        &model_path_quantized
    } else {
        &model_path
    };

    Ok(Box::new(OnnxEmbedder::new_from_paths(
        final_model_path,
        &tokenizer_path,
        &model_def.name,
        model_def.dimensions,
        model_def.query_prefix.clone(),
        model_def.passage_prefix.clone(),
    )?))
}

/// Get current model name from configuration
pub fn get_current_model_name() -> Result<String> {
    let config = Config::load()?;
    Ok(config.embeddings.model)
}
