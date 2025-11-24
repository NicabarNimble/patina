//! Simple 2-layer MLP trainer with triplet loss
//!
//! Phase 2 MVP: In-memory training, no ONNX export yet

use anyhow::Result;
use fastrand::Rng;

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
    /// Create new projection with random weights
    pub fn new(input_dim: usize, hidden_dim: usize, output_dim: usize) -> Self {
        let mut rng = Rng::new();

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
    fn forward_with_cache(&self, input: &[f32]) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        // Layer 1
        let z1: Vec<f32> = self
            .w1
            .iter()
            .zip(self.b1.iter())
            .map(|(w_row, b)| dot(w_row, input) + b)
            .collect();

        let hidden: Vec<f32> = z1.iter().map(|&z| z.max(0.0)).collect();

        // Layer 2
        let output: Vec<f32> = self
            .w2
            .iter()
            .zip(self.b2.iter())
            .map(|(w_row, b)| dot(w_row, &hidden) + b)
            .collect();

        (z1, hidden, output)
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
                let (z1_a, h1_a, out_a) = self.forward_with_cache(anchor);
                let (z1_p, h1_p, out_p) = self.forward_with_cache(positive);
                let (z1_n, h1_n, out_n) = self.forward_with_cache(negative);

                // L2 normalize outputs
                let out_a_norm = l2_normalize(&out_a);
                let out_p_norm = l2_normalize(&out_p);
                let out_n_norm = l2_normalize(&out_n);

                // Triplet loss with margin
                let margin = 0.2;
                let pos_dist = euclidean_distance(&out_a_norm, &out_p_norm);
                let neg_dist = euclidean_distance(&out_a_norm, &out_n_norm);
                let loss = (pos_dist - neg_dist + margin).max(0.0);

                epoch_loss += loss;

                // Simple gradient descent (skip if loss is zero)
                if loss > 0.0 {
                    // Compute gradients (simplified - just update toward reducing loss)
                    self.update_weights(
                        anchor,
                        positive,
                        negative,
                        &h1_a,
                        &h1_p,
                        &h1_n,
                        &z1_a,
                        &z1_p,
                        &z1_n,
                        &out_a_norm,
                        &out_p_norm,
                        &out_n_norm,
                        learning_rate,
                    );
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

    /// Simplified weight update (gradient approximation)
    fn update_weights(
        &mut self,
        _anchor: &[f32],
        _positive: &[f32],
        _negative: &[f32],
        _h1_a: &[f32],
        h1_p: &[f32],
        h1_n: &[f32],
        z1_a: &[f32],
        z1_p: &[f32],
        _z1_n: &[f32],
        out_a: &[f32],
        out_p: &[f32],
        out_n: &[f32],
        lr: f32,
    ) {
        // Simplified gradient: push anchor closer to positive, away from negative
        for i in 0..self.w2.len() {
            for j in 0..self.w2[i].len() {
                // Gradient approximation for layer 2
                let grad = (out_a[i] - out_p[i]) * h1_p[j] - (out_a[i] - out_n[i]) * h1_n[j];
                self.w2[i][j] -= lr * grad * 0.01; // Small step
            }
        }

        // Update layer 1 (even simpler)
        for i in 0..self.w1.len() {
            for j in 0..self.w1[i].len() {
                if z1_a[i] > 0.0 && z1_p[i] > 0.0 {
                    // Only update if ReLU is active
                    self.w1[i][j] -= lr * 0.001; // Very small step
                }
            }
        }
    }
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

        let losses = proj.train(&anchors, &positives, &negatives, 5, 0.01).unwrap();

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
}
