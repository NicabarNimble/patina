//! Unit tests for semantic search API (no sqlite-vec required)

use patina::query::semantic_search::{distance_to_similarity, vec_f32_to_bytes};

#[test]
fn test_distance_to_similarity_cosine() {
    // Identical vectors (cosine distance = 0)
    let sim = distance_to_similarity(0.0);
    assert!((sim - 1.0).abs() < 0.001, "Distance 0 should give similarity 1.0");

    // Orthogonal vectors (cosine distance = 1)
    let sim = distance_to_similarity(1.0);
    assert!((sim - 0.0).abs() < 0.001, "Distance 1 should give similarity 0.0");

    // Opposite vectors (cosine distance = 2)
    let sim = distance_to_similarity(2.0);
    assert!((sim - (-1.0)).abs() < 0.001, "Distance 2 should give similarity -1.0");

    // Verify monotonic decrease
    assert!(distance_to_similarity(0.0) > distance_to_similarity(0.5));
    assert!(distance_to_similarity(0.5) > distance_to_similarity(1.0));
    assert!(distance_to_similarity(1.0) > distance_to_similarity(2.0));
}

#[test]
fn test_vec_f32_to_bytes_conversion() {
    let vec = vec![1.0, 2.5, -3.14159];
    let bytes = vec_f32_to_bytes(&vec);

    // Should be 12 bytes (3 floats × 4 bytes each)
    assert_eq!(bytes.len(), 12);

    // Verify round-trip conversion
    let mut reconstructed = Vec::new();
    for chunk in bytes.chunks(4) {
        let bytes_array: [u8; 4] = chunk.try_into().unwrap();
        reconstructed.push(f32::from_le_bytes(bytes_array));
    }

    // Check all values match
    for (original, reconstructed) in vec.iter().zip(reconstructed.iter()) {
        assert!((original - reconstructed).abs() < 1e-6, "Values should match after round-trip");
    }
}

#[test]
fn test_vec_f32_to_bytes_empty() {
    let vec: Vec<f32> = vec![];
    let bytes = vec_f32_to_bytes(&vec);
    assert_eq!(bytes.len(), 0, "Empty vector should produce empty bytes");
}

#[test]
fn test_vec_f32_to_bytes_large_vector() {
    // Test with 384-dimensional vector (same as all-MiniLM-L6-v2)
    let vec: Vec<f32> = (0..384).map(|i| i as f32 * 0.01).collect();
    let bytes = vec_f32_to_bytes(&vec);

    // Should be 1536 bytes (384 floats × 4 bytes each)
    assert_eq!(bytes.len(), 1536);

    // Verify a few sample values
    let val_0 = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    assert!((val_0 - 0.0).abs() < 1e-6);

    let val_100_offset = 100 * 4;
    let val_100 = f32::from_le_bytes([
        bytes[val_100_offset],
        bytes[val_100_offset + 1],
        bytes[val_100_offset + 2],
        bytes[val_100_offset + 3],
    ]);
    assert!((val_100 - 1.0).abs() < 1e-6);
}
