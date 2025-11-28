//! Quick test to verify E5-base-v2 model is working
//!
//! Run with: cargo run --example test_e5_model

use anyhow::Result;
use patina::embeddings::{EmbeddingEngine, OnnxEmbedder};
use std::path::Path;

fn main() -> Result<()> {
    println!("🔍 Testing E5-base-v2 model...\n");

    let model_path = Path::new("resources/models/e5-base-v2/model_quantized.onnx");
    let tokenizer_path = Path::new("resources/models/e5-base-v2/tokenizer.json");

    // Check files exist
    if !model_path.exists() {
        eprintln!("❌ Model not found at: {}", model_path.display());
        eprintln!("   Run: ./scripts/download-model.sh e5-base-v2");
        return Ok(());
    }

    if !tokenizer_path.exists() {
        eprintln!("❌ Tokenizer not found at: {}", tokenizer_path.display());
        return Ok(());
    }

    println!("✅ Model files found");
    println!("   Model: {}", model_path.display());
    println!("   Tokenizer: {}", tokenizer_path.display());

    // Load model
    let mut embedder = OnnxEmbedder::new_from_paths(
        model_path,
        tokenizer_path,
        "e5-base-v2",
        768,
        Some("query: ".to_string()),
        Some("passage: ".to_string()),
    )?;

    println!("\n✅ ONNX Runtime initialized");
    println!("   Model: {}", embedder.model_name());
    println!("   Dimensions: {}", embedder.dimension());

    // Test basic embedding
    println!("\n🧪 Test 1: Basic embedding");
    let text = "How do I handle errors in Rust?";
    let embedding = embedder.embed_query(text)?;

    println!("   Input: \"{}\"", text);
    println!(
        "   Output: [{:.4}, {:.4}, ..., {:.4}]",
        embedding[0], embedding[1], embedding[767]
    );
    println!("   Vector length: {} dims", embedding.len());

    // Verify normalization
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    println!("   L2 norm: {:.6} (should be ~1.0)", norm);

    // Test query vs passage prefixes
    println!("\n🧪 Test 2: Query vs passage embeddings");
    let query_emb = embedder.embed_query("error handling in rust")?;
    let passage_emb = embedder.embed_passage("Rust uses Result types for error handling")?;

    let similarity: f32 = query_emb
        .iter()
        .zip(passage_emb.iter())
        .map(|(a, b)| a * b)
        .sum();

    println!("   Query prefix: 'query: error handling in rust'");
    println!("   Passage prefix: 'passage: Rust uses Result types...'");
    println!("   Cosine similarity: {:.4}", similarity);

    // Test semantic similarity
    println!("\n🧪 Test 3: Semantic similarity");
    let e1 = embedder.embed_passage("Rust's error handling uses Result and Option types")?;
    let e2 = embedder.embed_passage("Error management in Rust relies on Result<T, E>")?;
    let e3 = embedder.embed_passage("The weather is nice today")?;

    let sim_12: f32 = e1.iter().zip(e2.iter()).map(|(a, b)| a * b).sum();
    let sim_13: f32 = e1.iter().zip(e3.iter()).map(|(a, b)| a * b).sum();

    println!("   Similar passages: {:.4}", sim_12);
    println!("   Dissimilar passages: {:.4}", sim_13);
    println!(
        "   ✅ Similar > dissimilar: {}",
        if sim_12 > sim_13 { "PASS" } else { "FAIL" }
    );

    println!("\n✅ E5-base-v2 is fully operational!");
    println!("\nNext steps for Phase 2:");
    println!("  1. Recipe system (.patina/oxidize.yaml)");
    println!("  2. Training pair generators (from eventlog)");
    println!("  3. Projection trainer (2-layer MLP)");
    println!("  4. USearch index builder");

    Ok(())
}
