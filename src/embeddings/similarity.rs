//! Similarity and distance metrics for embeddings

/// Compute cosine similarity between two embedding vectors
///
/// Returns a value between -1.0 and 1.0, where:
/// - 1.0 = identical vectors
/// - 0.0 = orthogonal vectors
/// - -1.0 = opposite vectors
///
/// # Panics
/// Panics if vectors have different dimensions
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(
        a.len(),
        b.len(),
        "Vectors must have same dimension: {} vs {}",
        a.len(),
        b.len()
    );

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    // Handle zero magnitude case
    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

/// Compute Euclidean distance between two embedding vectors
///
/// Returns the L2 distance (always >= 0.0)
///
/// # Panics
/// Panics if vectors have different dimensions
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(
        a.len(),
        b.len(),
        "Vectors must have same dimension: {} vs {}",
        a.len(),
        b.len()
    );

    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_relative_eq!(cosine_similarity(&a, &b), 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert_relative_eq!(cosine_similarity(&a, &b), 0.0, epsilon = 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        assert_relative_eq!(cosine_similarity(&a, &b), -1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_euclidean_distance_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_relative_eq!(euclidean_distance(&a, &b), 0.0, epsilon = 1e-6);
    }

    #[test]
    fn test_euclidean_distance_unit() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert_relative_eq!(euclidean_distance(&a, &b), 1.0, epsilon = 1e-6);
    }

    #[test]
    #[should_panic(expected = "Vectors must have same dimension")]
    fn test_cosine_similarity_different_dimensions() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        cosine_similarity(&a, &b);
    }
}
