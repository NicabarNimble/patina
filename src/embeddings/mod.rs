//! Embeddings module - Generate semantic embeddings for text
//!
//! Provides trait-based abstraction for embedding generation with ONNX backend.

mod onnx;
mod similarity;

pub use onnx::OnnxEmbedder;
pub use similarity::{cosine_similarity, euclidean_distance};

use anyhow::Result;

/// Trait for embedding generation engines
pub trait EmbeddingEngine {
    /// Generate embedding for a single text
    fn embed(&mut self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts (batch processing)
    fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Get embedding dimension (e.g., 384 for all-MiniLM-L6-v2)
    fn dimension(&self) -> usize;

    /// Get model name
    fn model_name(&self) -> &str;
}

/// Factory function to create the default embedder (ONNX)
pub fn create_embedder() -> Result<Box<dyn EmbeddingEngine>> {
    Ok(Box::new(OnnxEmbedder::new()?))
}
