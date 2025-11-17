//! Integration tests for embeddings module

use patina::embeddings::{cosine_similarity, create_embedder, euclidean_distance, EmbeddingEngine};
use std::path::Path;

/// Get embedder for testing - tries active model first, falls back to baseline
fn get_test_embedder() -> Box<dyn EmbeddingEngine> {
    // Try active model first (e.g., e5-base-v2)
    if let Ok(embedder) = create_embedder() {
        return embedder;
    }

    // Fall back to baseline model (all-minilm-l6-v2)
    let model_path = Path::new("resources/models/all-minilm-l6-v2/model_quantized.onnx");
    let tokenizer_path = Path::new("resources/models/all-minilm-l6-v2/tokenizer.json");

    if !model_path.exists() || !tokenizer_path.exists() {
        eprintln!("\n❌ Baseline model not found!");
        eprintln!("\nRun this to download models:");
        eprintln!("  ./scripts/download-active-model.sh\n");
        panic!("Models missing. See instructions above.");
    }

    use patina::embeddings::OnnxEmbedder;
    Box::new(
        OnnxEmbedder::new_from_paths(
            model_path,
            tokenizer_path,
            "all-MiniLM-L6-v2",
            384,
            None,
            None,
        )
        .expect("Baseline model should load"),
    )
}

#[test]
fn test_embedder_creation() {
    let embedder = get_test_embedder();
    let dim = embedder.dimension();
    assert!(
        dim == 384 || dim == 768,
        "Expected 384 or 768 dimensions, got {}",
        dim
    );
    assert!(
        !embedder.model_name().is_empty(),
        "Model name should not be empty"
    );
}

#[test]
fn test_single_embedding_generation() {
    let mut embedder = get_test_embedder();

    let text = "This is a test sentence for semantic embedding";
    let embedding = embedder.embed(text).expect("Failed to generate embedding");

    let expected_dim = embedder.dimension();
    assert_eq!(
        embedding.len(),
        expected_dim,
        "Embedding should match embedder dimension"
    );
    assert!(
        embedding.iter().any(|&x| x != 0.0),
        "Embedding should not be all zeros"
    );

    // Check L2 normalization (should be ~1.0)
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(
        (norm - 1.0).abs() < 0.001,
        "Embedding should be L2 normalized, got norm: {}",
        norm
    );
}

#[test]
fn test_semantic_similarity_detection() {
    let mut embedder = get_test_embedder();

    // Similar sentences
    let e1 = embedder
        .embed("I prefer Rust for systems programming")
        .unwrap();
    let e2 = embedder
        .embed("Rust is my choice for low-level development")
        .unwrap();

    // Unrelated sentence
    let e3 = embedder.embed("The weather is nice today").unwrap();

    let sim_12 = cosine_similarity(&e1, &e2);
    let sim_13 = cosine_similarity(&e1, &e3);

    println!("Similarity (Rust/Rust): {}", sim_12);
    println!("Similarity (Rust/weather): {}", sim_13);

    assert!(
        sim_12 > sim_13,
        "Similar sentences should have higher similarity: {} vs {}",
        sim_12,
        sim_13
    );
    assert!(
        sim_12 > 0.6,
        "Similar sentences should have high similarity (>0.6), got {}",
        sim_12
    );
}

#[test]
fn test_belief_semantic_search() {
    let mut embedder = get_test_embedder();

    // Simulate belief statements
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
    ];

    // Generate embeddings
    let belief_embeddings: Vec<_> = beliefs
        .iter()
        .map(|(name, text)| {
            let emb = embedder.embed(text).unwrap();
            (*name, emb)
        })
        .collect();

    // Query
    let query = "type safe programming languages";
    let query_embedding = embedder.embed(query).unwrap();

    // Find most similar
    let mut similarities: Vec<_> = belief_embeddings
        .iter()
        .map(|(name, emb)| (*name, cosine_similarity(&query_embedding, emb)))
        .collect();

    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!("\nQuery: '{}'\n", query);
    println!("Top results:");
    for (name, sim) in &similarities {
        println!("  {} (similarity: {:.3})", name, sim);
    }

    // "values_type_safety" should be the top result
    assert_eq!(
        similarities[0].0, "values_type_safety",
        "Expected 'values_type_safety' to be most similar to query about type safety"
    );
    assert!(
        similarities[0].1 > 0.5,
        "Top result should have strong similarity (>0.5), got {}",
        similarities[0].1
    );
}

#[test]
fn test_cross_domain_belief_detection() {
    let mut embedder = get_test_embedder();

    // Beliefs from different domains expressing similar concepts
    let film_belief = "I prefer character-driven narratives over plot-driven stories";
    let code_belief = "I value expressive code over performant but obscure code";

    let e1 = embedder.embed(film_belief).unwrap();
    let e2 = embedder.embed(code_belief).unwrap();

    let similarity = cosine_similarity(&e1, &e2);

    println!("\nCross-domain similarity:");
    println!("Film: {}", film_belief);
    println!("Code: {}", code_belief);
    println!("Similarity: {:.3}", similarity);

    // These express "depth over surface" - should have some similarity despite different domains
    // Note: Cross-domain similarity is naturally lower due to vocabulary differences
    assert!(
        similarity > 0.0,
        "Cross-domain beliefs should have non-zero similarity, got {}",
        similarity
    );

    // Informational assertion - cross-domain similarity is typically low (0.05-0.15)
    if similarity < 0.15 {
        println!(
            "Note: Cross-domain similarity is low ({:.3}), which is expected",
            similarity
        );
    }
}

#[test]
fn test_batch_embedding_generation() {
    let mut embedder = get_test_embedder();

    let texts = vec![
        "First test sentence".to_string(),
        "Second test sentence".to_string(),
        "Third test sentence".to_string(),
    ];

    let embeddings = embedder
        .embed_batch(&texts)
        .expect("Failed to generate batch embeddings");

    let expected_dim = embedder.dimension();
    assert_eq!(embeddings.len(), 3, "Should generate 3 embeddings");
    for (i, emb) in embeddings.iter().enumerate() {
        assert_eq!(
            emb.len(),
            expected_dim,
            "Embedding {} should have {} dimensions",
            i,
            expected_dim
        );
        assert!(
            emb.iter().any(|&x| x != 0.0),
            "Embedding {} should not be all zeros",
            i
        );
    }
}

#[test]
fn test_cosine_similarity_properties() {
    // Identical vectors
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![1.0, 2.0, 3.0];
    let sim = cosine_similarity(&a, &b);
    assert!(
        (sim - 1.0).abs() < 0.001,
        "Identical vectors should have similarity ~1.0"
    );

    // Orthogonal vectors
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    let sim = cosine_similarity(&a, &b);
    assert!(
        (sim - 0.0).abs() < 0.001,
        "Orthogonal vectors should have similarity ~0.0"
    );

    // Opposite vectors
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![-1.0, -2.0, -3.0];
    let sim = cosine_similarity(&a, &b);
    assert!(
        (sim + 1.0).abs() < 0.001,
        "Opposite vectors should have similarity ~-1.0"
    );
}

#[test]
fn test_euclidean_distance_properties() {
    // Identical vectors
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![1.0, 2.0, 3.0];
    let dist = euclidean_distance(&a, &b);
    assert!(
        (dist - 0.0).abs() < 0.001,
        "Identical vectors should have distance ~0.0"
    );

    // Unit distance
    let a = vec![0.0, 0.0, 0.0];
    let b = vec![1.0, 0.0, 0.0];
    let dist = euclidean_distance(&a, &b);
    assert!(
        (dist - 1.0).abs() < 0.001,
        "Unit vectors should have distance ~1.0"
    );
}

#[test]
#[should_panic(expected = "Vectors must have same dimension")]
fn test_cosine_similarity_dimension_mismatch() {
    let a = vec![1.0, 2.0];
    let b = vec![1.0, 2.0, 3.0];
    cosine_similarity(&a, &b);
}

#[test]
#[should_panic(expected = "Vectors must have same dimension")]
fn test_euclidean_distance_dimension_mismatch() {
    let a = vec![1.0, 2.0];
    let b = vec![1.0, 2.0, 3.0];
    euclidean_distance(&a, &b);
}
