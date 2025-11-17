//! Test INT8 model loading
//!
//! Run with: cargo test --test model_selection -- --nocapture

use patina::embeddings::create_embedder;

#[test]
fn test_int8_model_loads() {
    let mut embedder = create_embedder().expect("Should load model from config");

    // Verify it works
    let embedding = embedder.embed("test").expect("Should generate embedding");
    let expected_dim = embedder.dimension();
    assert_eq!(
        embedding.len(),
        expected_dim,
        "Embedding should match model dimension"
    );

    println!(
        "✅ Model loaded: {} ({} dimensions)",
        embedder.model_name(),
        expected_dim
    );
}
