//! Test to compare INT8 quantized model quality vs FP32 production model
//!
//! Run with: cargo test --test quantization_quality -- --nocapture --ignored

use patina::embeddings::{cosine_similarity, EmbeddingEngine, OnnxEmbedder};
use std::path::Path;

#[test]
#[ignore] // Manual test - requires both models
fn compare_int8_vs_fp32_quality() {
    println!("\n🔬 Comparing FP32 vs INT8 quantized models\n");

    // Load both models
    let fp32_path = Path::new("resources/models/all-MiniLM-L6-v2.onnx");
    let int8_path = Path::new("target/test-models/all-MiniLM-L6-v2-int8.onnx");
    let tokenizer_fp32 = Path::new("resources/models/tokenizer.json");
    let tokenizer_int8 = Path::new("target/test-models/tokenizer.json");

    if !fp32_path.exists() {
        println!("⚠️  FP32 model not found. Only testing INT8.");
        println!("   Download with: curl -L -o resources/models/all-MiniLM-L6-v2.onnx https://huggingface.co/Xenova/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx");
        return;
    }

    if !int8_path.exists() {
        println!("❌ INT8 model not found. Run: ./scripts/download-test-models.sh");
        return;
    }

    let mut fp32_embedder = OnnxEmbedder::new_from_paths(
        fp32_path,
        tokenizer_fp32,
        "all-MiniLM-L6-v2-FP32",
        384,
        None,
        None,
    )
    .expect("FP32 model should load");
    let mut int8_embedder = OnnxEmbedder::new_from_paths(
        int8_path,
        tokenizer_int8,
        "all-MiniLM-L6-v2-INT8",
        384,
        None,
        None,
    )
    .expect("INT8 model should load");

    // Test queries from actual use case
    let queries = vec![
        "prefer Rust for CLI tools",
        "avoid global mutable state",
        "type safe programming",
        "security best practices",
        "character-driven narratives",
    ];

    let beliefs = vec![
        (
            "prefers_rust_for_cli_tools",
            "I prefer using Rust for command-line tools",
        ),
        ("values_type_safety", "Type safety is important to me"),
        ("avoid_global_state", "I avoid using global mutable state"),
        (
            "prefers_composition",
            "I prefer composition over inheritance",
        ),
        (
            "security_first",
            "Always review generated code for security issues",
        ),
    ];

    println!("📊 Belief statements:");
    for (name, text) in &beliefs {
        println!("   - {}: \"{}\"", name, text);
    }
    println!();

    // Generate belief embeddings with both models
    let fp32_beliefs: Vec<_> = beliefs
        .iter()
        .map(|(name, text)| (*name, fp32_embedder.embed(text).unwrap()))
        .collect();

    let int8_beliefs: Vec<_> = beliefs
        .iter()
        .map(|(name, text)| (*name, int8_embedder.embed(text).unwrap()))
        .collect();

    let mut ranking_matches = 0;
    let mut total_queries = 0;
    let mut max_score_diff = 0.0f32;

    // Test each query
    for query in &queries {
        println!("🔍 Query: \"{}\"", query);

        let fp32_q = fp32_embedder.embed(query).unwrap();
        let int8_q = int8_embedder.embed(query).unwrap();

        // Rank beliefs by similarity
        let mut fp32_results: Vec<_> = fp32_beliefs
            .iter()
            .map(|(name, emb)| (*name, cosine_similarity(&fp32_q, emb)))
            .collect();
        fp32_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let mut int8_results: Vec<_> = int8_beliefs
            .iter()
            .map(|(name, emb)| (*name, cosine_similarity(&int8_q, emb)))
            .collect();
        int8_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        println!("   FP32 ranking:");
        for (i, (name, score)) in fp32_results.iter().take(3).enumerate() {
            println!("      {}. {} ({:.4})", i + 1, name, score);
        }

        println!("   INT8 ranking:");
        for (i, (name, score)) in int8_results.iter().take(3).enumerate() {
            println!("      {}. {} ({:.4})", i + 1, name, score);
        }

        // Check if top-3 ranking is preserved
        let fp32_top3: Vec<_> = fp32_results.iter().take(3).map(|x| x.0).collect();
        let int8_top3: Vec<_> = int8_results.iter().take(3).map(|x| x.0).collect();

        if fp32_top3 == int8_top3 {
            println!("   ✅ Top-3 ranking preserved!");
            ranking_matches += 1;
        } else {
            println!("   ⚠️  Top-3 ranking differs");
        }

        // Calculate max score difference for this query
        for i in 0..beliefs.len() {
            let diff = (fp32_results[i].1 - int8_results[i].1).abs();
            max_score_diff = max_score_diff.max(diff);
        }

        println!();
        total_queries += 1;
    }

    println!("\n📈 Summary:");
    println!(
        "   Top-3 ranking preservation: {}/{} queries ({:.0}%)",
        ranking_matches,
        total_queries,
        (ranking_matches as f32 / total_queries as f32) * 100.0
    );
    println!("   Max similarity score difference: {:.4}", max_score_diff);
    println!();

    if ranking_matches == total_queries {
        println!("✅ INT8 quantized model preserves ranking perfectly for this use case!");
    } else if ranking_matches as f32 / total_queries as f32 > 0.8 {
        println!("✅ INT8 quantized model is good enough (>80% ranking match)");
    } else {
        println!("⚠️  INT8 may not be sufficient - consider using FP32 for production");
    }

    // Assert that at least 80% of rankings are preserved
    assert!(
        ranking_matches as f32 / total_queries as f32 >= 0.8,
        "INT8 model should preserve at least 80% of top-3 rankings"
    );
}
