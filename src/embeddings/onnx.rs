//! ONNX Runtime embedder for all-MiniLM-L6-v2

use super::EmbeddingEngine;
use anyhow::{anyhow, bail, Context, Result};
use ndarray::Array2;
use ort::{inputs, session::Session, value::Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use tokenizers::Tokenizer;

/// Default model name used for integrity verification
const DEFAULT_MODEL_NAME: &str = "all-MiniLM-L6-v2";

/// SHA-256 hash of the known-good default model (all-MiniLM-L6-v2 INT8 quantized)
const DEFAULT_MODEL_SHA256: &str =
    "afdb6f1a0e45b715d0bb9b11772f032c399babd23bfc31fed1c170afc848bdb1";

/// Execution mode for ONNX inference sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnnxMode {
    /// Auto-threaded, non-deterministic. For interactive single-query paths (scry, assay).
    FastQuery,
    /// Single-threaded, deterministic. For oxidize builds and rayon parallel workers.
    DeterministicBuild,
}

/// ONNX-based embedding generator
pub struct OnnxEmbedder {
    session: Session,
    tokenizer: Tokenizer,
    dimension: usize,
    model_name: String,
    query_prefix: Option<String>,
    passage_prefix: Option<String>,
}

impl OnnxEmbedder {
    /// Create a new ONNX embedder from default paths
    ///
    /// Uses INT8 quantized model (23MB, 3-4x faster than FP32, 98% accuracy).
    pub fn new() -> Result<Self> {
        let model_path = Path::new("resources/models/all-MiniLM-L6-v2-int8.onnx");
        let tokenizer_path = Path::new("resources/models/tokenizer.json");

        Self::new_from_paths(
            model_path,
            tokenizer_path,
            "all-MiniLM-L6-v2",
            384,
            None,
            None,
        )
    }

    /// Create a new ONNX embedder from custom paths
    ///
    /// Allows specifying different model/tokenizer locations (useful for testing)
    ///
    /// # Arguments
    /// * `model_path` - Path to ONNX model file
    /// * `tokenizer_path` - Path to tokenizer.json file
    /// * `model_name` - Human-readable model name (e.g., "bge-base-en-v1.5")
    /// * `dimension` - Embedding dimension (384 for small models, 768 for base models)
    /// * `query_prefix` - Optional prefix for query embeddings (for asymmetric models like BGE)
    /// * `passage_prefix` - Optional prefix for passage embeddings (for asymmetric models like E5)
    pub fn new_from_paths(
        model_path: &Path,
        tokenizer_path: &Path,
        model_name: &str,
        dimension: usize,
        query_prefix: Option<String>,
        passage_prefix: Option<String>,
    ) -> Result<Self> {
        Self::new_from_paths_with_mode(
            model_path,
            tokenizer_path,
            model_name,
            dimension,
            query_prefix,
            passage_prefix,
            OnnxMode::DeterministicBuild,
        )
    }

    /// Create a new ONNX embedder with explicit execution mode
    pub fn new_from_paths_with_mode(
        model_path: &Path,
        tokenizer_path: &Path,
        model_name: &str,
        dimension: usize,
        query_prefix: Option<String>,
        passage_prefix: Option<String>,
        mode: OnnxMode,
    ) -> Result<Self> {
        // Load ONNX model
        if !model_path.exists() {
            bail!(
                "ONNX model not found at: {}\n\n\
                Download it with:\n  \
                mkdir -p $(dirname {}) && \\\n  \
                curl -L -o {} \\\n  \
                https://huggingface.co/Xenova/all-MiniLM-L6-v2/resolve/main/onnx/model_quantized.onnx",
                model_path.display(),
                model_path.display(),
                model_path.display()
            );
        }

        // Verify integrity of the default model before loading
        if model_name == DEFAULT_MODEL_NAME {
            verify_model_integrity(model_path, DEFAULT_MODEL_SHA256)?;
        }

        let builder = Session::builder().context("Failed to create ONNX session builder")?;

        let session = match mode {
            OnnxMode::FastQuery => {
                // Auto-threaded: let ONNX use all cores for single-query latency
                builder
                    .commit_from_file(model_path)
                    .context("Failed to load ONNX model")?
            }
            OnnxMode::DeterministicBuild => {
                // Single-threaded + deterministic: stable artifacts, used with outer rayon parallelism
                builder
                    .with_intra_threads(1)
                    .context("Failed to set intra-op threads")?
                    .with_inter_threads(1)
                    .context("Failed to set inter-op threads")?
                    .with_deterministic_compute(true)
                    .context("Failed to enable deterministic compute")?
                    .commit_from_file(model_path)
                    .context("Failed to load ONNX model")?
            }
        };

        // Load tokenizer
        if !tokenizer_path.exists() {
            bail!(
                "Tokenizer not found at: {}\n\n\
                Download it with:\n  \
                curl -L -o {} \\\n  \
                  https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json",
                tokenizer_path.display(),
                tokenizer_path.display()
            );
        }

        let mut tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;

        // Enable truncation to 512 tokens (ONNX model limit for e5/bge/minilm)
        // This prevents "Attempting to broadcast an axis by a dimension other than 1"
        // errors when embedding large functions
        tokenizer
            .with_truncation(Some(tokenizers::TruncationParams {
                max_length: 512,
                ..Default::default()
            }))
            .map_err(|e| anyhow!("Failed to configure truncation: {}", e))?;

        Ok(Self {
            session,
            tokenizer,
            dimension,
            model_name: model_name.to_string(),
            query_prefix,
            passage_prefix,
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

    /// Batched ONNX inference: tokenize, pad, stack into [batch_size, max_seq_len], single run()
    fn embed_batch_inner(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let batch_size = texts.len();

        // Tokenize all texts
        let tokenized: Vec<(Vec<i64>, Vec<i64>)> = texts
            .iter()
            .map(|t| self.tokenize(t))
            .collect::<Result<Vec<_>>>()?;

        // Find max sequence length for padding
        let max_seq_len = tokenized
            .iter()
            .map(|(ids, _)| ids.len())
            .max()
            .unwrap_or(0);

        // Build padded tensors: [batch_size, max_seq_len]
        let mut all_input_ids = vec![0i64; batch_size * max_seq_len];
        let mut all_attention_mask = vec![0i64; batch_size * max_seq_len];
        let all_token_type_ids = vec![0i64; batch_size * max_seq_len];

        for (i, (ids, mask)) in tokenized.iter().enumerate() {
            let offset = i * max_seq_len;
            for (j, &id) in ids.iter().enumerate() {
                all_input_ids[offset + j] = id;
            }
            for (j, &m) in mask.iter().enumerate() {
                all_attention_mask[offset + j] = m;
            }
        }

        let input_ids_array = Array2::from_shape_vec((batch_size, max_seq_len), all_input_ids)
            .context("Failed to create batched input_ids array")?;
        let attention_mask_array =
            Array2::from_shape_vec((batch_size, max_seq_len), all_attention_mask.clone())
                .context("Failed to create batched attention_mask array")?;
        let token_type_ids_array =
            Array2::from_shape_vec((batch_size, max_seq_len), all_token_type_ids)
                .context("Failed to create batched token_type_ids array")?;

        // Single ONNX inference call for the whole batch
        let (shape_dims, flat_data) = {
            let outputs = self
                .session
                .run(inputs![
                    "input_ids" => Value::from_array(input_ids_array)?,
                    "attention_mask" => Value::from_array(attention_mask_array)?,
                    "token_type_ids" => Value::from_array(token_type_ids_array)?
                ])
                .context("Batched ONNX inference failed")?;

            let (shape, data) = outputs["last_hidden_state"]
                .try_extract_tensor::<f32>()
                .context("Failed to extract batched last_hidden_state tensor")?;

            let dims: Vec<usize> = shape.as_ref().iter().map(|&d| d as usize).collect();
            if dims.len() != 3 {
                bail!("Expected 3D tensor, got shape: {:?}", dims);
            }
            (dims, data.to_vec())
        };

        // shape_dims = [batch_size, seq_len, hidden_dim]
        let seq_len = shape_dims[1];
        let hidden_dim = shape_dims[2];

        // Extract per-item embeddings, mean-pool, normalize
        let mut results = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let item_offset = i * seq_len * hidden_dim;
            let item_data = &flat_data[item_offset..item_offset + seq_len * hidden_dim];
            let token_embeddings =
                Array2::from_shape_vec((seq_len, hidden_dim), item_data.to_vec())
                    .context("Failed to reshape batch item embeddings")?;

            // Get this item's attention mask for mean pooling
            let item_mask_start = i * max_seq_len;
            let item_mask = &all_attention_mask[item_mask_start..item_mask_start + max_seq_len];

            let embedding = self.mean_pooling(&token_embeddings, item_mask);
            let normalized = self.normalize(&embedding);
            results.push(normalized);
        }

        Ok(results)
    }
}

/// Verify ONNX model file integrity via SHA-256
fn verify_model_integrity(model_path: &Path, expected_hash: &str) -> Result<()> {
    let bytes = std::fs::read(model_path)
        .with_context(|| format!("Failed to read model for integrity check: {:?}", model_path))?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if hash != expected_hash {
        bail!(
            "ONNX model integrity check failed: {:?}\n  Expected: {}\n  Got:      {}",
            model_path,
            expected_hash,
            hash
        );
    }
    Ok(())
}

impl EmbeddingEngine for OnnxEmbedder {
    fn embed_query(&mut self, text: &str) -> Result<Vec<f32>> {
        let input = if let Some(prefix) = &self.query_prefix {
            format!("{}{}", prefix, text)
        } else {
            text.to_string()
        };
        self.embed(&input)
    }

    fn embed_passage(&mut self, text: &str) -> Result<Vec<f32>> {
        let input = if let Some(prefix) = &self.passage_prefix {
            format!("{}{}", prefix, text)
        } else {
            text.to_string()
        };
        self.embed(&input)
    }

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
            Array2::from_shape_vec((seq_len, hidden_dim), data[0..batch_offset].to_vec())
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
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        if texts.len() == 1 {
            return Ok(vec![self.embed(&texts[0])?]);
        }
        self.embed_batch_inner(texts)
    }

    fn embed_passage_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if let Some(prefix) = &self.passage_prefix.clone() {
            let prefixed: Vec<String> = texts.iter().map(|t| format!("{}{}", prefix, t)).collect();
            self.embed_batch_inner(&prefixed)
        } else {
            self.embed_batch_inner(texts)
        }
    }

    fn embed_query_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if let Some(prefix) = &self.query_prefix.clone() {
            let prefixed: Vec<String> = texts.iter().map(|t| format!("{}{}", prefix, t)).collect();
            self.embed_batch_inner(&prefixed)
        } else {
            self.embed_batch_inner(texts)
        }
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::path::Path;

    fn get_test_embedder() -> Option<OnnxEmbedder> {
        // Use all-minilm baseline model for consistent unit tests (384 dims)
        let model_path = Path::new("resources/models/all-minilm-l6-v2/model_quantized.onnx");
        let tokenizer_path = Path::new("resources/models/all-minilm-l6-v2/tokenizer.json");

        if !model_path.exists() || !tokenizer_path.exists() {
            eprintln!(
                "Skipping ONNX test: model fixtures missing. Run ./scripts/download-model.sh all-minilm-l6-v2"
            );
            return None;
        }

        Some(
            OnnxEmbedder::new_from_paths(
                model_path,
                tokenizer_path,
                "all-MiniLM-L6-v2",
                384,
                None,
                None,
            )
            .expect("Test model should load"),
        )
    }

    #[test]
    fn test_onnx_embedder_creation() {
        let Some(_embedder) = get_test_embedder() else {
            return;
        };
        // If we get here, creation succeeded
    }

    #[test]
    fn test_embed_basic() {
        let Some(mut embedder) = get_test_embedder() else {
            return;
        };
        let embedding = embedder.embed("This is a test").unwrap();

        assert_eq!(embedding.len(), 384);
        assert!(
            embedding.iter().any(|&x| x != 0.0),
            "Embedding is all zeros"
        );

        // Check normalization (L2 norm should be ~1.0)
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert_relative_eq!(norm, 1.0, epsilon = 1e-5);
    }

    #[test]
    fn test_semantic_similarity() {
        let Some(mut embedder) = get_test_embedder() else {
            return;
        };

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
        assert!(
            sim_12 > 0.7,
            "Expected high similarity for similar sentences"
        );
    }
}
