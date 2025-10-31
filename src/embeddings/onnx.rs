//! ONNX Runtime embedder for all-MiniLM-L6-v2

use super::EmbeddingEngine;
use anyhow::{anyhow, bail, Context, Result};
use ndarray::Array2;
use ort::{inputs, session::Session, value::Value};
use std::path::Path;
use tokenizers::Tokenizer;

/// ONNX-based embedding generator
pub struct OnnxEmbedder {
    session: Session,
    tokenizer: Tokenizer,
    dimension: usize,
}

impl OnnxEmbedder {
    /// Create a new ONNX embedder
    ///
    /// Loads the ONNX model and tokenizer from resources/models/
    pub fn new() -> Result<Self> {
        // Load ONNX model
        let model_path = Path::new("resources/models/all-MiniLM-L6-v2.onnx");

        if !model_path.exists() {
            bail!(
                "ONNX model not found at: {}\n\n\
                Download it with:\n  \
                curl -L -o resources/models/all-MiniLM-L6-v2.onnx \\\n  \
                  https://huggingface.co/Xenova/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx",
                model_path.display()
            );
        }

        let session = Session::builder()
            .context("Failed to create ONNX session builder")?
            .commit_from_file(model_path)
            .context("Failed to load ONNX model")?;

        // Load tokenizer
        let tokenizer_path = Path::new("resources/models/tokenizer.json");

        if !tokenizer_path.exists() {
            bail!(
                "Tokenizer not found at: {}\n\n\
                Download it with:\n  \
                curl -L -o resources/models/tokenizer.json \\\n  \
                  https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json",
                tokenizer_path.display()
            );
        }

        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;

        Ok(Self {
            session,
            tokenizer,
            dimension: 384, // all-MiniLM-L6-v2 dimension
        })
    }

    /// Tokenize text into input_ids and attention_mask
    fn tokenize(&self, text: &str) -> Result<(Vec<i64>, Vec<i64>)> {
        let encoding = self
            .tokenizer
            .encode(text, true) // Add special tokens ([CLS], [SEP])
            .map_err(|e| anyhow!("Tokenization failed: {}", e))?;

        let input_ids = encoding.get_ids().iter().map(|&x| x as i64).collect();
        let attention_mask = encoding
            .get_attention_mask()
            .iter()
            .map(|&x| x as i64)
            .collect();

        Ok((input_ids, attention_mask))
    }

    /// Mean pooling - average token embeddings weighted by attention mask
    fn mean_pooling(&self, token_embeddings: &Array2<f32>, attention_mask: &[i64]) -> Vec<f32> {
        let mask_sum: f32 = attention_mask.iter().map(|&x| x as f32).sum();

        // Handle case where all attention masks are 0
        if mask_sum == 0.0 {
            return vec![0.0; self.dimension];
        }

        let mut pooled = vec![0.0; self.dimension];
        for (i, &mask) in attention_mask.iter().enumerate() {
            if mask == 1 && i < token_embeddings.nrows() {
                for j in 0..self.dimension {
                    pooled[j] += token_embeddings[[i, j]];
                }
            }
        }

        pooled.iter().map(|&x| x / mask_sum).collect()
    }

    /// L2 normalize a vector
    fn normalize(&self, vec: &[f32]) -> Vec<f32> {
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();

        // Handle zero norm case
        if norm == 0.0 {
            return vec.to_vec();
        }

        vec.iter().map(|x| x / norm).collect()
    }
}

impl EmbeddingEngine for OnnxEmbedder {
    fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        // Tokenize
        let (input_ids, attention_mask) = self.tokenize(text)?;

        // Prepare inputs as Array2
        let seq_len = input_ids.len();
        let input_ids_array = Array2::from_shape_vec((1, seq_len), input_ids.clone())
            .context("Failed to create input_ids array")?;

        let attention_mask_array =
            Array2::from_shape_vec((1, attention_mask.len()), attention_mask.clone())
                .context("Failed to create attention_mask array")?;

        // Token type IDs - all zeros for single-sentence embeddings
        let token_type_ids = vec![0i64; seq_len];
        let token_type_ids_array = Array2::from_shape_vec((1, seq_len), token_type_ids)
            .context("Failed to create token_type_ids array")?;

        // Run inference and extract data (need to finish with outputs before using self methods)
        let token_embeddings_2d = {
            let outputs = self
                .session
                .run(inputs![
                    "input_ids" => Value::from_array(input_ids_array)?,
                    "attention_mask" => Value::from_array(attention_mask_array)?,
                    "token_type_ids" => Value::from_array(token_type_ids_array)?
                ])
                .context("ONNX inference failed")?;

            // Extract token embeddings from last_hidden_state
            let (shape, data) = outputs["last_hidden_state"]
                .try_extract_tensor::<f32>()
                .context("Failed to extract last_hidden_state tensor")?;

            // Shape is [batch_size=1, seq_len, hidden_dim=384]
            let shape_dims = shape.as_ref();
            if shape_dims.len() != 3 {
                bail!("Expected 3D tensor, got shape: {:?}", shape_dims);
            }

            let seq_len = shape_dims[1] as usize;
            let hidden_dim = shape_dims[2] as usize;

            // Convert flat data to Array2 for the first batch item
            let batch_offset = seq_len * hidden_dim;
            Array2::from_shape_vec(
                (seq_len, hidden_dim),
                data[0..batch_offset].to_vec()
            )
            .context("Failed to reshape token embeddings")?
            // outputs is dropped here, releasing the mutable borrow
        };

        // Mean pooling
        let embedding = self.mean_pooling(&token_embeddings_2d, &attention_mask);

        // L2 normalize
        let normalized = self.normalize(&embedding);

        Ok(normalized)
    }

    fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // Simple sequential processing for now
        // TODO: Optimize with true batch processing
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        "all-MiniLM-L6-v2 (ONNX)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    #[ignore] // Only run when model files are available
    fn test_onnx_embedder_creation() {
        let embedder = OnnxEmbedder::new();
        assert!(embedder.is_ok(), "Failed to create embedder: {:?}", embedder.err());
    }

    #[test]
    #[ignore] // Only run when model files are available
    fn test_embed_basic() {
        let mut embedder = OnnxEmbedder::new().unwrap();
        let embedding = embedder.embed("This is a test").unwrap();

        assert_eq!(embedding.len(), 384);
        assert!(embedding.iter().any(|&x| x != 0.0), "Embedding is all zeros");

        // Check normalization (L2 norm should be ~1.0)
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert_relative_eq!(norm, 1.0, epsilon = 1e-5);
    }

    #[test]
    #[ignore] // Only run when model files are available
    fn test_semantic_similarity() {
        let mut embedder = OnnxEmbedder::new().unwrap();

        let e1 = embedder.embed("The cat sits on the mat").unwrap();
        let e2 = embedder.embed("A cat is sitting on a mat").unwrap();
        let e3 = embedder.embed("The weather is nice today").unwrap();

        let sim_12 = crate::embeddings::cosine_similarity(&e1, &e2);
        let sim_13 = crate::embeddings::cosine_similarity(&e1, &e3);

        // Similar sentences should have higher similarity
        assert!(
            sim_12 > sim_13,
            "Expected sim(cat,cat)={} > sim(cat,weather)={}",
            sim_12,
            sim_13
        );
        assert!(sim_12 > 0.7, "Expected high similarity for similar sentences");
    }
}
