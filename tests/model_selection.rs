//! Test INT8 model loading
//!
//! Run with: cargo test --test model_selection -- --nocapture

use patina::embeddings::create_embedder;

#[test]
fn test_int8_model_loads() {
    let mut embedder = create_embedder().expect("Should load INT8 model");

    // Verify it works
    let embedding = embedder.embed("test").expect("Should generate embedding");
    assert_eq!(embedding.len(), 384);

    println!("✅ Model loaded: {}", embedder.model_name());
}
