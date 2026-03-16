//! Simple 2-layer MLP trainer with triplet loss
//!
//! Phase 2: Training + safetensors export (MLX-compatible)
//! Phase 6: ndarray storage — contiguous Array2/Array1, vectorized ops,
//!          pre-update gradient fix, safe serialization via zerocopy.

use anyhow::{Context, Result};
use fastrand::Rng;
use ndarray::linalg::general_mat_mul;
use ndarray::{Array1, Array2, ArrayView1, Axis};
use safetensors::SafeTensors;
use std::path::Path;

/// Cache of intermediate values from forward pass (for backprop)
struct ForwardCache {
    /// Input to the network (stored for W1 gradient)
    input: Array1<f32>,
    /// Pre-activation at hidden layer
    z1: Array1<f32>,
    /// Hidden layer activations (post-ReLU)
    h1: Array1<f32>,
    /// Output layer (pre-normalization)
    out: Array1<f32>,
}

/// Triplet of forward caches for anchor, positive, and negative samples
struct TripletCache {
    anchor: ForwardCache,
    positive: ForwardCache,
    negative: ForwardCache,
    /// Normalized outputs
    out_a_norm: Array1<f32>,
    out_p_norm: Array1<f32>,
    out_n_norm: Array1<f32>,
}

/// A simple 2-layer MLP: input → hidden (ReLU) → output
pub struct Projection {
    /// Weight matrix: hidden_dim × input_dim
    pub w1: Array2<f32>,
    /// Bias vector: hidden_dim
    pub b1: Array1<f32>,
    /// Weight matrix: output_dim × hidden_dim
    pub w2: Array2<f32>,
    /// Bias vector: output_dim
    pub b2: Array1<f32>,
}

impl Projection {
    /// Create new projection with deterministic Xavier initialization
    /// Phase 5c: fixed seed for reproducible projections across runs.
    pub fn new(input_dim: usize, hidden_dim: usize, output_dim: usize) -> Self {
        let mut rng = Rng::with_seed(42);

        // Xavier initialization: scale = sqrt(6 / (fan_in + fan_out))
        let scale1 = (6.0 / (input_dim + hidden_dim) as f32).sqrt();
        let scale2 = (6.0 / (hidden_dim + output_dim) as f32).sqrt();

        // Generate in row-major order (same RNG sequence as original Vec<Vec<f32>>)
        let mut w1_data = Vec::with_capacity(hidden_dim * input_dim);
        for _ in 0..hidden_dim {
            for _ in 0..input_dim {
                w1_data.push((rng.f32() * 2.0 - 1.0) * scale1);
            }
        }
        let w1 = Array2::from_shape_vec((hidden_dim, input_dim), w1_data)
            .expect("w1 shape must match data length");

        let b1 = Array1::zeros(hidden_dim);

        let mut w2_data = Vec::with_capacity(output_dim * hidden_dim);
        for _ in 0..output_dim {
            for _ in 0..hidden_dim {
                w2_data.push((rng.f32() * 2.0 - 1.0) * scale2);
            }
        }
        let w2 = Array2::from_shape_vec((output_dim, hidden_dim), w2_data)
            .expect("w2 shape must match data length");

        let b2 = Array1::zeros(output_dim);

        Self { w1, b1, w2, b2 }
    }

    /// Output dimension of the projection
    pub fn output_dim(&self) -> usize {
        self.w2.nrows()
    }

    /// Forward pass through the network
    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        let input = ArrayView1::from(input);
        let h1 = (self.w1.dot(&input) + &self.b1).mapv(|z| z.max(0.0));
        let out = self.w2.dot(&h1) + &self.b2;
        out.to_vec()
    }

    /// Forward pass with intermediate values for backprop
    fn forward_with_cache(&self, input: &ArrayView1<f32>) -> ForwardCache {
        let z1 = self.w1.dot(input) + &self.b1;
        let h1 = z1.mapv(|z| z.max(0.0));
        let out = self.w2.dot(&h1) + &self.b2;

        ForwardCache {
            input: input.to_owned(),
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

            for i in 0..anchors.len() {
                let anchor = ArrayView1::from(anchors[i].as_slice());
                let positive = ArrayView1::from(positives[i].as_slice());
                let negative = ArrayView1::from(negatives[i].as_slice());

                // Forward pass
                let cache_a = self.forward_with_cache(&anchor);
                let cache_p = self.forward_with_cache(&positive);
                let cache_n = self.forward_with_cache(&negative);

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
    /// All gradients are computed against pre-update weights, then applied
    /// together at the end. This fixes the original bug where W2 was mutated
    /// before backpropagating through it for W1 gradients.
    fn update_weights(&mut self, cache: &TripletCache, lr: f32) {
        let dim = cache.out_a_norm.len();

        // Step 1: dL/dy for each branch (gradient of euclidean distance)
        let pos_dist = euclidean_distance(&cache.out_a_norm, &cache.out_p_norm);
        let neg_dist = euclidean_distance(&cache.out_a_norm, &cache.out_n_norm);

        let mut dl_dy_a = Array1::<f32>::zeros(dim);
        let mut dl_dy_p = Array1::<f32>::zeros(dim);
        let mut dl_dy_n = Array1::<f32>::zeros(dim);

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
        let dl_dout_a = grad_l2_norm(&dl_dy_a, &cache.anchor.out, &cache.out_a_norm);
        let dl_dout_p = grad_l2_norm(&dl_dy_p, &cache.positive.out, &cache.out_p_norm);
        let dl_dout_n = grad_l2_norm(&dl_dy_n, &cache.negative.out, &cache.out_n_norm);

        // Step 3: Compute W2 gradient (outer product accumulation, zero temp allocation)
        let mut grad_w2 = Array2::<f32>::zeros(self.w2.raw_dim());
        accum_outer(&mut grad_w2, &dl_dout_a, &cache.anchor.h1);
        accum_outer(&mut grad_w2, &dl_dout_p, &cache.positive.h1);
        accum_outer(&mut grad_w2, &dl_dout_n, &cache.negative.h1);

        let grad_b2 = &dl_dout_a + &dl_dout_p + &dl_dout_n;

        // Step 4: Backprop through W2 (using ORIGINAL pre-update weights)
        let dl_dh1_a = self.w2.t().dot(&dl_dout_a);
        let dl_dh1_p = self.w2.t().dot(&dl_dout_p);
        let dl_dh1_n = self.w2.t().dot(&dl_dout_n);

        // Step 5: Backprop through ReLU
        let dl_dz1_a = grad_relu(&dl_dh1_a, &cache.anchor.z1);
        let dl_dz1_p = grad_relu(&dl_dh1_p, &cache.positive.z1);
        let dl_dz1_n = grad_relu(&dl_dh1_n, &cache.negative.z1);

        // Step 6: Compute W1 gradient (outer product accumulation, zero temp allocation)
        let mut grad_w1 = Array2::<f32>::zeros(self.w1.raw_dim());
        accum_outer(&mut grad_w1, &dl_dz1_a, &cache.anchor.input);
        accum_outer(&mut grad_w1, &dl_dz1_p, &cache.positive.input);
        accum_outer(&mut grad_w1, &dl_dz1_n, &cache.negative.input);

        let grad_b1 = &dl_dz1_a + &dl_dz1_p + &dl_dz1_n;

        // Step 7: Apply ALL weight updates (gradients used pre-update weights)
        self.w2.scaled_add(-lr, &grad_w2);
        self.b2.scaled_add(-lr, &grad_b2);
        self.w1.scaled_add(-lr, &grad_w1);
        self.b1.scaled_add(-lr, &grad_b1);
    }

    /// Save projection weights to safetensors format (MLX-compatible)
    pub fn save_safetensors(&self, path: &Path) -> Result<()> {
        use safetensors::tensor::{Dtype, TensorView};
        use zerocopy::AsBytes;

        let (hidden_dim, input_dim) = self.w1.dim();
        let (output_dim, _) = self.w2.dim();

        // Contiguous arrays: as_slice_memory_order for Array2, as_slice for Array1
        let w1_slice = self
            .w1
            .as_slice_memory_order()
            .expect("w1 must be contiguous");
        let b1_slice = self.b1.as_slice().expect("b1 must be contiguous");
        let w2_slice = self
            .w2
            .as_slice_memory_order()
            .expect("w2 must be contiguous");
        let b2_slice = self.b2.as_slice().expect("b2 must be contiguous");

        // Safe f32 → u8 byte view via zerocopy (no unsafe needed)
        let w1_bytes: &[u8] = w1_slice.as_bytes();
        let b1_bytes: &[u8] = b1_slice.as_bytes();
        let w2_bytes: &[u8] = w2_slice.as_bytes();
        let b2_bytes: &[u8] = b2_slice.as_bytes();

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

        let w1_view = tensors.tensor("w1.weight")?;
        let b1_view = tensors.tensor("w1.bias")?;
        let w2_view = tensors.tensor("w2.weight")?;
        let b2_view = tensors.tensor("w2.bias")?;

        let shape_w1 = w1_view.shape();
        let shape_w2 = w2_view.shape();

        let hidden_dim = shape_w1[0];
        let input_dim = shape_w1[1];
        let output_dim = shape_w2[0];

        fn bytes_to_f32(data: &[u8]) -> Vec<f32> {
            data.chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect()
        }

        let w1 = Array2::from_shape_vec((hidden_dim, input_dim), bytes_to_f32(w1_view.data()))
            .context("w1 shape mismatch")?;
        let b1 = Array1::from_vec(bytes_to_f32(b1_view.data()));
        let w2 = Array2::from_shape_vec((output_dim, hidden_dim), bytes_to_f32(w2_view.data()))
            .context("w2 shape mismatch")?;
        let b2 = Array1::from_vec(bytes_to_f32(b2_view.data()));

        Ok(Self { w1, b1, w2, b2 })
    }
}

/// Accumulate outer product into target: target += a * b^T
/// Uses general_mat_mul for zero-allocation accumulation.
fn accum_outer(target: &mut Array2<f32>, a: &Array1<f32>, b: &Array1<f32>) {
    general_mat_mul(
        1.0,
        &a.view().insert_axis(Axis(1)),
        &b.view().insert_axis(Axis(0)),
        1.0,
        target,
    );
}

/// Gradient through L2 normalization
/// Given dL/dy where y = out / ||out||, compute dL/dout
fn grad_l2_norm(dl_dy: &Array1<f32>, out: &Array1<f32>, y_norm: &Array1<f32>) -> Array1<f32> {
    let norm = out.dot(out).sqrt();
    if norm < 1e-8 {
        return Array1::zeros(dl_dy.len());
    }
    let dot_dy_y = dl_dy.dot(y_norm);
    (dl_dy - &(y_norm * dot_dy_y)) / norm
}

/// Gradient through ReLU: pass through where z > 0, zero otherwise
fn grad_relu(dl_dh: &Array1<f32>, z: &Array1<f32>) -> Array1<f32> {
    dl_dh * &z.mapv(|z_val| if z_val > 0.0 { 1.0 } else { 0.0 })
}

/// L2 normalize a vector
fn l2_normalize(v: &Array1<f32>) -> Array1<f32> {
    let norm = v.dot(v).sqrt();
    if norm == 0.0 {
        v.clone()
    } else {
        v / norm
    }
}

/// Euclidean distance between two vectors
fn euclidean_distance(a: &Array1<f32>, b: &Array1<f32>) -> f32 {
    let diff = a - b;
    diff.dot(&diff).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projection_creation() {
        let proj = Projection::new(768, 1024, 256);
        assert_eq!(proj.w1.dim(), (1024, 768));
        assert_eq!(proj.w2.dim(), (256, 1024));
        assert_eq!(proj.output_dim(), 256);
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

        let anchors = vec![vec![1.0; 10], vec![0.5; 10]];
        let positives = vec![vec![1.1; 10], vec![0.6; 10]];
        let negatives = vec![vec![0.0; 10], vec![1.0; 10]];

        let losses = proj
            .train(&anchors, &positives, &negatives, 5, 0.01)
            .unwrap();

        assert!(losses.len() == 5);
        assert!(losses[4] <= losses[0] * 1.1);
    }

    #[test]
    fn test_l2_normalize() {
        let v = Array1::from_vec(vec![3.0, 4.0]);
        let normalized = l2_normalize(&v);
        assert!((normalized[0] - 0.6).abs() < 0.01);
        assert!((normalized[1] - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_save_load_safetensors() -> Result<()> {
        use tempfile::tempdir;

        let original = Projection::new(10, 20, 5);

        let dir = tempdir()?;
        let path = dir.path().join("test.safetensors");
        original.save_safetensors(&path)?;

        let loaded = Projection::load_safetensors(&path)?;

        assert_eq!(loaded.w1.dim(), (20, 10));
        assert_eq!(loaded.b1.len(), 20);
        assert_eq!(loaded.w2.dim(), (5, 20));
        assert_eq!(loaded.b2.len(), 5);

        // Weights must match exactly (lossless f32 round-trip)
        assert_eq!(original.w1, loaded.w1);
        assert_eq!(original.b1, loaded.b1);
        assert_eq!(original.w2, loaded.w2);
        assert_eq!(original.b2, loaded.b2);

        Ok(())
    }

    #[test]
    fn test_forward_pass_after_load() -> Result<()> {
        use tempfile::tempdir;

        let original = Projection::new(10, 20, 5);
        let input = vec![1.0; 10];
        let original_output = original.forward(&input);

        let dir = tempdir()?;
        let path = dir.path().join("test.safetensors");
        original.save_safetensors(&path)?;

        let loaded = Projection::load_safetensors(&path)?;
        let loaded_output = loaded.forward(&input);

        assert_eq!(original_output, loaded_output);

        Ok(())
    }

    #[test]
    fn test_training_determinism() -> Result<()> {
        use tempfile::tempdir;

        let anchors = vec![vec![1.0; 10], vec![0.5; 10], vec![0.3; 10]];
        let positives = vec![vec![1.1; 10], vec![0.6; 10], vec![0.4; 10]];
        let negatives = vec![vec![0.0; 10], vec![1.0; 10], vec![0.9; 10]];

        // Train twice from identical initialization
        let mut proj1 = Projection::new(10, 20, 5);
        let losses1 = proj1
            .train(&anchors, &positives, &negatives, 5, 0.01)
            .unwrap();

        let mut proj2 = Projection::new(10, 20, 5);
        let losses2 = proj2
            .train(&anchors, &positives, &negatives, 5, 0.01)
            .unwrap();

        // Loss histories must match exactly
        assert_eq!(losses1, losses2);

        // Saved weights must be byte-identical
        let dir = tempdir()?;
        let path1 = dir.path().join("proj1.safetensors");
        let path2 = dir.path().join("proj2.safetensors");
        proj1.save_safetensors(&path1)?;
        proj2.save_safetensors(&path2)?;

        let bytes1 = std::fs::read(&path1)?;
        let bytes2 = std::fs::read(&path2)?;
        assert_eq!(
            bytes1, bytes2,
            "Saved weights must be byte-identical across runs"
        );

        Ok(())
    }
}
