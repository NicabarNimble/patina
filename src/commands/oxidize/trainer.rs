//! Simple 2-layer MLP trainer with triplet loss
//!
//! Phase 2: Training + safetensors export (MLX-compatible)

use anyhow::{Context, Result};
use fastrand::Rng;
use safetensors::SafeTensors;
use std::path::Path;

/// Cache of intermediate values from forward pass (for backprop)
struct ForwardCache {
    /// Input to the network (stored for W1 gradient)
    input: Vec<f32>,
    /// Pre-activation at hidden layer
    z1: Vec<f32>,
    /// Hidden layer activations (post-ReLU)
    h1: Vec<f32>,
    /// Output layer (pre-normalization)
    out: Vec<f32>,
}

/// Triplet of forward caches for anchor, positive, and negative samples
struct TripletCache {
    anchor: ForwardCache,
    positive: ForwardCache,
    negative: ForwardCache,
    /// Normalized outputs
    out_a_norm: Vec<f32>,
    out_p_norm: Vec<f32>,
    out_n_norm: Vec<f32>,
}

/// A simple 2-layer MLP: input → hidden (ReLU) → output
pub struct Projection {
    /// Weight matrix: hidden_dim × input_dim
    pub w1: Vec<Vec<f32>>,
    /// Bias vector: hidden_dim
    pub b1: Vec<f32>,
    /// Weight matrix: output_dim × hidden_dim
    pub w2: Vec<Vec<f32>>,
    /// Bias vector: output_dim
    pub b2: Vec<f32>,
}

impl Projection {
    /// Create new projection with deterministic Xavier initialization
    /// Phase 5c: fixed seed for reproducible projections across runs.
    pub fn new(input_dim: usize, hidden_dim: usize, output_dim: usize) -> Self {
        let mut rng = Rng::with_seed(42);

        // Xavier initialization: scale = sqrt(6 / (fan_in + fan_out))
        let scale1 = (6.0 / (input_dim + hidden_dim) as f32).sqrt();
        let scale2 = (6.0 / (hidden_dim + output_dim) as f32).sqrt();

        let w1 = (0..hidden_dim)
            .map(|_| {
                (0..input_dim)
                    .map(|_| (rng.f32() * 2.0 - 1.0) * scale1)
                    .collect()
            })
            .collect();

        let b1 = vec![0.0; hidden_dim];

        let w2 = (0..output_dim)
            .map(|_| {
                (0..hidden_dim)
                    .map(|_| (rng.f32() * 2.0 - 1.0) * scale2)
                    .collect()
            })
            .collect();

        let b2 = vec![0.0; output_dim];

        Self { w1, b1, w2, b2 }
    }

    /// Forward pass through the network
    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        // Layer 1: input → hidden (with ReLU)
        let hidden: Vec<f32> = self
            .w1
            .iter()
            .zip(self.b1.iter())
            .map(|(w_row, b)| {
                let z = dot(w_row, input) + b;
                z.max(0.0) // ReLU activation
            })
            .collect();

        // Layer 2: hidden → output (linear, will normalize later)
        self.w2
            .iter()
            .zip(self.b2.iter())
            .map(|(w_row, b)| dot(w_row, &hidden) + b)
            .collect()
    }

    /// Forward pass with intermediate values for backprop
    fn forward_with_cache(&self, input: &[f32]) -> ForwardCache {
        // Layer 1
        let z1: Vec<f32> = self
            .w1
            .iter()
            .zip(self.b1.iter())
            .map(|(w_row, b)| dot(w_row, input) + b)
            .collect();

        let h1: Vec<f32> = z1.iter().map(|&z| z.max(0.0)).collect();

        // Layer 2
        let out: Vec<f32> = self
            .w2
            .iter()
            .zip(self.b2.iter())
            .map(|(w_row, b)| dot(w_row, &h1) + b)
            .collect();

        ForwardCache {
            input: input.to_vec(),
            z1,
            h1,
            out,
        }
    }

    /// Train projection with triplet loss using gradient descent
    pub fn train(
        &mut self,
        anchors: &[Vec<f32>],
        positives: &[Vec<f32>],
        negatives: &[Vec<f32>],
        epochs: usize,
        learning_rate: f32,
    ) -> Result<Vec<f32>> {
        let mut losses = Vec::new();

        for epoch in 0..epochs {
            let mut epoch_loss = 0.0;

            for i in 0..(anchors.len()) {
                let anchor = &anchors[i];
                let positive = &positives[i];
                let negative = &negatives[i];

                // Forward pass
                let cache_a = self.forward_with_cache(anchor);
                let cache_p = self.forward_with_cache(positive);
                let cache_n = self.forward_with_cache(negative);

                // L2 normalize outputs
                let out_a_norm = l2_normalize(&cache_a.out);
                let out_p_norm = l2_normalize(&cache_p.out);
                let out_n_norm = l2_normalize(&cache_n.out);

                // Triplet loss with margin
                let margin = 0.2;
                let pos_dist = euclidean_distance(&out_a_norm, &out_p_norm);
                let neg_dist = euclidean_distance(&out_a_norm, &out_n_norm);
                let loss = (pos_dist - neg_dist + margin).max(0.0);

                epoch_loss += loss;

                // Simple gradient descent (skip if loss is zero)
                if loss > 0.0 {
                    let cache = TripletCache {
                        anchor: cache_a,
                        positive: cache_p,
                        negative: cache_n,
                        out_a_norm,
                        out_p_norm,
                        out_n_norm,
                    };
                    self.update_weights(&cache, learning_rate);
                }
            }

            let avg_loss = epoch_loss / anchors.len() as f32;
            losses.push(avg_loss);

            if epoch % 2 == 0 || epoch == epochs - 1 {
                println!("   Epoch {}/{}: loss = {:.4}", epoch + 1, epochs, avg_loss);
            }
        }

        Ok(losses)
    }

    /// Weight update with proper backpropagation through L2 normalization
    ///
    /// Phase 5d: replaces the broken gradient approximation that had:
    /// - Layer 1: constant decay (not a gradient)
    /// - Layer 2: missing normalization Jacobian, wrong hidden state mixing
    fn update_weights(&mut self, cache: &TripletCache, lr: f32) {
        let dim = cache.out_a_norm.len();

        // Step 1: dL/dy for each branch (gradient of euclidean distance)
        let pos_dist = euclidean_distance(&cache.out_a_norm, &cache.out_p_norm);
        let neg_dist = euclidean_distance(&cache.out_a_norm, &cache.out_n_norm);

        let mut dl_dy_a = vec![0.0f32; dim];
        let mut dl_dy_p = vec![0.0f32; dim];
        let mut dl_dy_n = vec![0.0f32; dim];

        for i in 0..dim {
            let pos_grad = if pos_dist > 1e-8 {
                (cache.out_a_norm[i] - cache.out_p_norm[i]) / pos_dist
            } else {
                0.0
            };
            let neg_grad = if neg_dist > 1e-8 {
                (cache.out_a_norm[i] - cache.out_n_norm[i]) / neg_dist
            } else {
                0.0
            };
            dl_dy_a[i] = pos_grad - neg_grad;
            dl_dy_p[i] = -pos_grad;
            dl_dy_n[i] = neg_grad;
        }

        // Step 2: Backprop through L2 normalization
        // y = out / ||out||, so dL/dout = (dL/dy - y * dot(dL/dy, y)) / ||out||
        let dl_dout_a = grad_l2_norm(&dl_dy_a, &cache.anchor.out, &cache.out_a_norm);
        let dl_dout_p = grad_l2_norm(&dl_dy_p, &cache.positive.out, &cache.out_p_norm);
        let dl_dout_n = grad_l2_norm(&dl_dy_n, &cache.negative.out, &cache.out_n_norm);

        // Step 3: Update W2 and b2
        // out = W2 * h1 + b2, so dL/dW2[i][j] = dL/dout[i] * h1[j]
        for i in 0..self.w2.len() {
            for j in 0..self.w2[i].len() {
                let grad = dl_dout_a[i] * cache.anchor.h1[j]
                    + dl_dout_p[i] * cache.positive.h1[j]
                    + dl_dout_n[i] * cache.negative.h1[j];
                self.w2[i][j] -= lr * grad;
            }
            self.b2[i] -= lr * (dl_dout_a[i] + dl_dout_p[i] + dl_dout_n[i]);
        }

        // Step 4: Backprop through linear layer to get dL/dh1
        let dl_dh1_a = backprop_linear(&dl_dout_a, &self.w2);
        let dl_dh1_p = backprop_linear(&dl_dout_p, &self.w2);
        let dl_dh1_n = backprop_linear(&dl_dout_n, &self.w2);

        // Step 5: Backprop through ReLU
        let dl_dz1_a = grad_relu(&dl_dh1_a, &cache.anchor.z1);
        let dl_dz1_p = grad_relu(&dl_dh1_p, &cache.positive.z1);
        let dl_dz1_n = grad_relu(&dl_dh1_n, &cache.negative.z1);

        // Step 6: Update W1 and b1
        // z1 = W1 * x + b1, so dL/dW1[i][j] = dL/dz1[i] * x[j]
        for i in 0..self.w1.len() {
            let grad_b = dl_dz1_a[i] + dl_dz1_p[i] + dl_dz1_n[i];
            for j in 0..self.w1[i].len() {
                let grad = dl_dz1_a[i] * cache.anchor.input[j]
                    + dl_dz1_p[i] * cache.positive.input[j]
                    + dl_dz1_n[i] * cache.negative.input[j];
                self.w1[i][j] -= lr * grad;
            }
            self.b1[i] -= lr * grad_b;
        }
    }

    /// Save projection weights to safetensors format (MLX-compatible)
    pub fn save_safetensors(&self, path: &Path) -> Result<()> {
        use safetensors::tensor::{Dtype, TensorView};

        let hidden_dim = self.w1.len();
        let input_dim = self.w1[0].len();
        let output_dim = self.w2.len();

        // Flatten matrices to 1D vectors
        let w1_flat: Vec<f32> = self.w1.iter().flat_map(|row| row.iter().copied()).collect();
        let w2_flat: Vec<f32> = self.w2.iter().flat_map(|row| row.iter().copied()).collect();

        // Convert Vec<f32> to &[u8] for safetensors
        let w1_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                w1_flat.as_ptr() as *const u8,
                w1_flat.len() * std::mem::size_of::<f32>(),
            )
        };
        let b1_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                self.b1.as_ptr() as *const u8,
                self.b1.len() * std::mem::size_of::<f32>(),
            )
        };
        let w2_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                w2_flat.as_ptr() as *const u8,
                w2_flat.len() * std::mem::size_of::<f32>(),
            )
        };
        let b2_bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                self.b2.as_ptr() as *const u8,
                self.b2.len() * std::mem::size_of::<f32>(),
            )
        };

        // Create TensorViews
        let tensors = vec![
            (
                "w1.weight",
                TensorView::new(Dtype::F32, vec![hidden_dim, input_dim], w1_bytes)?,
            ),
            (
                "w1.bias",
                TensorView::new(Dtype::F32, vec![hidden_dim], b1_bytes)?,
            ),
            (
                "w2.weight",
                TensorView::new(Dtype::F32, vec![output_dim, hidden_dim], w2_bytes)?,
            ),
            (
                "w2.bias",
                TensorView::new(Dtype::F32, vec![output_dim], b2_bytes)?,
            ),
        ];

        // Serialize and write (no metadata — HashMap serialization is non-deterministic,
        // which breaks safetensors checksum reproducibility. Dimensions are already
        // encoded in the tensor shapes.)
        let serialized =
            safetensors::tensor::serialize(tensors, None).context("Failed to serialize tensors")?;

        std::fs::write(path, serialized)
            .context(format!("Failed to write file: {}", path.display()))?;

        Ok(())
    }

    /// Load projection weights from safetensors format
    pub fn load_safetensors(path: &Path) -> Result<Self> {
        use std::fs;

        let buffer = fs::read(path).context(format!("Failed to read file: {}", path.display()))?;

        let tensors =
            SafeTensors::deserialize(&buffer).context("Failed to deserialize safetensors")?;

        // Load tensors and extract dimensions from shapes
        let w1_view = tensors.tensor("w1.weight")?;
        let b1_view = tensors.tensor("w1.bias")?;
        let w2_view = tensors.tensor("w2.weight")?;
        let b2_view = tensors.tensor("w2.bias")?;

        let shape_w1 = w1_view.shape(); // [hidden_dim, input_dim]
        let shape_w2 = w2_view.shape(); // [output_dim, hidden_dim]

        let hidden_dim = shape_w1[0];
        let input_dim = shape_w1[1];
        let _output_dim = shape_w2[0]; // Validated by shape consistency

        // Convert to Vec<f32>
        let w1_flat: Vec<f32> = w1_view
            .data()
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        let b1: Vec<f32> = b1_view
            .data()
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        let w2_flat: Vec<f32> = w2_view
            .data()
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        let b2: Vec<f32> = b2_view
            .data()
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        // Reshape w1: flat array → Vec<Vec<f32>>
        let w1: Vec<Vec<f32>> = w1_flat
            .chunks_exact(input_dim)
            .map(|chunk| chunk.to_vec())
            .collect();

        // Reshape w2: flat array → Vec<Vec<f32>>
        let w2: Vec<Vec<f32>> = w2_flat
            .chunks_exact(hidden_dim)
            .map(|chunk| chunk.to_vec())
            .collect();

        Ok(Self { w1, b1, w2, b2 })
    }
}

/// Gradient through L2 normalization
/// Given dL/dy where y = out / ||out||, compute dL/dout
fn grad_l2_norm(dl_dy: &[f32], out: &[f32], y_norm: &[f32]) -> Vec<f32> {
    let norm: f32 = out.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm < 1e-8 {
        return vec![0.0; dl_dy.len()];
    }
    let dot_dy_y: f32 = dl_dy.iter().zip(y_norm.iter()).map(|(a, b)| a * b).sum();
    dl_dy
        .iter()
        .zip(y_norm.iter())
        .map(|(dl_i, y_i)| (dl_i - y_i * dot_dy_y) / norm)
        .collect()
}

/// Backpropagate through linear layer: dL/dinput from dL/doutput
fn backprop_linear(dl_dout: &[f32], weights: &[Vec<f32>]) -> Vec<f32> {
    let input_dim = weights[0].len();
    let mut dl_dinput = vec![0.0; input_dim];
    for (i, w_row) in weights.iter().enumerate() {
        for (j, &w) in w_row.iter().enumerate() {
            dl_dinput[j] += dl_dout[i] * w;
        }
    }
    dl_dinput
}

/// Gradient through ReLU: pass through where z > 0, zero otherwise
fn grad_relu(dl_dh: &[f32], z: &[f32]) -> Vec<f32> {
    dl_dh
        .iter()
        .zip(z.iter())
        .map(|(g, &z_val)| if z_val > 0.0 { *g } else { 0.0 })
        .collect()
}

/// Dot product of two vectors
fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// L2 normalize a vector
fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        v.to_vec()
    } else {
        v.iter().map(|x| x / norm).collect()
    }
}

/// Euclidean distance between two vectors
fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projection_creation() {
        let proj = Projection::new(768, 1024, 256);
        assert_eq!(proj.w1.len(), 1024);
        assert_eq!(proj.w1[0].len(), 768);
        assert_eq!(proj.w2.len(), 256);
        assert_eq!(proj.w2[0].len(), 1024);
    }

    #[test]
    fn test_forward_pass() {
        let proj = Projection::new(10, 20, 5);
        let input = vec![1.0; 10];
        let output = proj.forward(&input);
        assert_eq!(output.len(), 5);
    }

    #[test]
    fn test_triplet_loss_decreases() {
        let mut proj = Projection::new(10, 20, 5);

        // Create simple training data
        let anchors = vec![vec![1.0; 10], vec![0.5; 10]];
        let positives = vec![vec![1.1; 10], vec![0.6; 10]]; // Similar to anchors
        let negatives = vec![vec![0.0; 10], vec![1.0; 10]]; // Different from anchors

        let losses = proj
            .train(&anchors, &positives, &negatives, 5, 0.01)
            .unwrap();

        // Loss should decrease (or at least not increase significantly)
        assert!(losses.len() == 5);
        assert!(losses[4] <= losses[0] * 1.1); // Allow 10% tolerance
    }

    #[test]
    fn test_l2_normalize() {
        let v = vec![3.0, 4.0];
        let normalized = l2_normalize(&v);
        assert!((normalized[0] - 0.6).abs() < 0.01);
        assert!((normalized[1] - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_save_load_safetensors() -> Result<()> {
        use std::path::PathBuf;
        use tempfile::tempdir;

        // Create projection
        let original = Projection::new(10, 20, 5);

        // Save to temp file
        let dir = tempdir()?;
        let path = dir.path().join("test.safetensors");
        original.save_safetensors(&path)?;

        // Load back
        let loaded = Projection::load_safetensors(&path)?;

        // Verify dimensions
        assert_eq!(loaded.w1.len(), 20);
        assert_eq!(loaded.w1[0].len(), 10);
        assert_eq!(loaded.b1.len(), 20);
        assert_eq!(loaded.w2.len(), 5);
        assert_eq!(loaded.w2[0].len(), 20);
        assert_eq!(loaded.b2.len(), 5);

        // Verify weights match
        for i in 0..original.w1.len() {
            for j in 0..original.w1[i].len() {
                assert!((original.w1[i][j] - loaded.w1[i][j]).abs() < 0.0001);
            }
        }

        for i in 0..original.b1.len() {
            assert!((original.b1[i] - loaded.b1[i]).abs() < 0.0001);
        }

        Ok(())
    }

    #[test]
    fn test_forward_pass_after_load() -> Result<()> {
        use tempfile::tempdir;

        // Create and save projection
        let original = Projection::new(10, 20, 5);
        let input = vec![1.0; 10];
        let original_output = original.forward(&input);

        let dir = tempdir()?;
        let path = dir.path().join("test.safetensors");
        original.save_safetensors(&path)?;

        // Load and test forward pass
        let loaded = Projection::load_safetensors(&path)?;
        let loaded_output = loaded.forward(&input);

        // Outputs should match
        assert_eq!(original_output.len(), loaded_output.len());
        for i in 0..original_output.len() {
            assert!((original_output[i] - loaded_output[i]).abs() < 0.0001);
        }

        Ok(())
    }
}
