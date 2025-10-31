//! Test INT8 default model selection
//!
//! Run with: cargo test --test model_selection -- --nocapture

use patina::embeddings::create_embedder;

#[test]
fn test_default_uses_int8() {
    // Clear environment variable to test default
    std::env::remove_var("PATINA_MODEL");

    let mut embedder = create_embedder().expect("Should load default INT8 model");

    // Verify it works
    let embedding = embedder.embed("test").expect("Should generate embedding");
    assert_eq!(embedding.len(), 384);

    println!("✅ Default model loaded: {}", embedder.model_name());
}

#[test]
fn test_fp32_override() {
    // Set environment variable to use FP32
    std::env::set_var("PATINA_MODEL", "fp32");

    let mut embedder = create_embedder().expect("Should load FP32 model when requested");

    // Verify it works
    let embedding = embedder.embed("test").expect("Should generate embedding");
    assert_eq!(embedding.len(), 384);

    println!("✅ FP32 override works: {}", embedder.model_name());

    // Clean up
    std::env::remove_var("PATINA_MODEL");
}

#[test]
fn test_int8_and_fp32_produce_similar_results() {
    let text = "prefer Rust for systems programming";

    // INT8 model
    std::env::remove_var("PATINA_MODEL");
    let mut int8_embedder = create_embedder().expect("Should load INT8 model");
    let int8_embedding = int8_embedder.embed(text).expect("Should generate embedding");

    // FP32 model
    std::env::set_var("PATINA_MODEL", "fp32");
    let mut fp32_embedder = create_embedder().expect("Should load FP32 model");
    let fp32_embedding = fp32_embedder.embed(text).expect("Should generate embedding");

    // Calculate difference
    let diff: f32 = int8_embedding.iter()
        .zip(fp32_embedding.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>() / int8_embedding.len() as f32;

    println!("Average element difference: {:.6}", diff);

    // Difference should be small
    assert!(diff < 0.01, "INT8 and FP32 embeddings should be similar (diff={:.6})", diff);

    // Clean up
    std::env::remove_var("PATINA_MODEL");
}
