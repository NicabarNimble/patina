//! Embeddings module - Generate semantic embeddings for text
//!
//! Provides trait-based abstraction for embedding generation with ONNX backend.
//! Supports multiple embedding models via configuration.

pub mod models;
pub mod offsets;
mod onnx;
mod similarity;

pub use models::{Config, ModelDefinition, ModelRegistry};
pub use onnx::{OnnxEmbedder, OnnxMode};
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

    /// Generate embeddings for multiple passage texts (with model-specific prefix)
    fn embed_passage_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed_passage(t)).collect()
    }

    /// Generate embeddings for multiple query texts (with model-specific prefix)
    fn embed_query_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed_query(t)).collect()
    }

    /// Get embedding dimension (e.g., 384 for all-MiniLM-L6-v2)
    fn dimension(&self) -> usize;

    /// Get model name
    fn model_name(&self) -> &str;
}

/// Factory function to create embedder from configuration
///
/// Uses FastQuery mode: auto-threaded for interactive latency (scry, assay).
/// Reads model selection from .patina/config.toml and creates appropriate embedder.
/// Falls back to default (all-MiniLM-L6-v2) if config doesn't exist.
pub fn create_embedder() -> Result<Box<dyn EmbeddingEngine>> {
    create_embedder_with_mode(OnnxMode::FastQuery)
}

/// Create embedder for deterministic builds (oxidize)
///
/// Uses DeterministicBuild mode: single-threaded, deterministic compute.
/// Produces stable artifacts across runs. Use with rayon outer parallelism.
pub fn create_embedder_deterministic() -> Result<Box<dyn EmbeddingEngine>> {
    create_embedder_with_mode(OnnxMode::DeterministicBuild)
}

/// Create embedder with explicit execution mode
fn create_embedder_with_mode(mode: OnnxMode) -> Result<Box<dyn EmbeddingEngine>> {
    let config = Config::load()?;
    let model_def = config.get_model_definition()?;
    create_onnx_embedder(&model_def, mode)
}

/// Create ONNX embedder from model definition
fn create_onnx_embedder(
    model_def: &ModelDefinition,
    mode: OnnxMode,
) -> Result<Box<dyn EmbeddingEngine>> {
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

    Ok(Box::new(OnnxEmbedder::new_from_paths_with_mode(
        final_model_path,
        &tokenizer_path,
        &model_def.name,
        model_def.dimensions,
        model_def.query_prefix.clone(),
        model_def.passage_prefix.clone(),
        mode,
    )?))
}

/// Get current model name from configuration
pub fn get_current_model_name() -> Result<String> {
    let config = Config::load()?;
    Ok(config.embeddings.model)
}
