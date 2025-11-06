//! Integration tests for neuro-symbolic reasoning
//!
//! Tests the full workflow: semantic search (neural) → ReasoningEngine (symbolic)

use patina::embeddings::{create_embedder, EmbeddingEngine, OnnxEmbedder};
use patina::query::SemanticSearch;
use patina::reasoning::{ReasoningEngine, ScoredObservation};
use patina::storage::types::ObservationMetadata;
use std::path::Path;
use tempfile::TempDir;

/// Get embedder for testing - tries production model first, falls back to test model
fn get_test_embedder() -> Box<dyn EmbeddingEngine> {
    // Try production model first (for local dev with full model)
    if let Ok(embedder) = create_embedder() {
        return embedder;
    }

    // Fall back to quantized test model
    let test_model = Path::new("target/test-models/all-MiniLM-L6-v2-int8.onnx");
    let test_tokenizer = Path::new("target/test-models/tokenizer.json");

    if !test_model.exists() || !test_tokenizer.exists() {
        eprintln!("\n❌ Test models not found!");
        eprintln!("\nRun this to download test models:");
        eprintln!("  ./scripts/download-test-models.sh\n");
        panic!("Test models missing. See instructions above.");
    }

    Box::new(
        OnnxEmbedder::new_from_paths(test_model, test_tokenizer).expect("Test model should load"),
    )
}

#[test]
fn test_neuro_symbolic_belief_validation_strong_evidence() {
    // Setup: Create temporary storage and search engine
    let embedder = get_test_embedder();
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut search =
        SemanticSearch::new(temp_dir.path(), embedder).expect("Failed to create search engine");

    // Add observations about Rust preference (strong evidence)
    // Using text that matches closely to query "I prefer Rust for systems programming"
    let observations = vec![
        ("I prefer Rust for systems programming", "pattern", 0.85, "session"),
        ("Rust is my preferred language for low-level systems work", "pattern", 0.85, "session"),
        ("I choose Rust for systems-level programming tasks", "decision", 0.85, "session"),
        ("Prefer using Rust when building systems software", "pattern", 0.85, "session"),
        ("Rust for systems programming is my go-to choice", "decision", 0.70, "commit"),
        ("Systems programming: Rust is what I use", "pattern", 0.70, "commit"),
    ];

    for (content, obs_type, reliability, source_type) in observations {
        let metadata = ObservationMetadata {
            created_at: Some(chrono::Utc::now()),
            updated_at: None,
            source: None,
            source_type: Some(source_type.to_string()),
            reliability: Some(reliability),
        };
        search
            .add_observation_with_metadata(content, obs_type, metadata)
            .expect("Failed to add observation");
    }

    // Neural layer: Semantic search for belief evidence
    let query = "I prefer Rust for systems programming";
    let mut query_embedder = get_test_embedder();
    let query_embedding = query_embedder.embed(query).expect("Failed to embed query");
    let search_results = search
        .observation_storage()
        .search_with_scores(&query_embedding, 10)
        .expect("Search should succeed");

    println!("\n=== Neural Layer: Semantic Search Results ===");
    for (obs, sim) in &search_results {
        println!(
            "  [{:.2}] {} (rel: {})",
            sim,
            obs.content,
            obs.metadata.reliability.unwrap_or(0.70)
        );
    }

    // Convert to ScoredObservation for Prolog
    let scored_obs: Vec<ScoredObservation> = search_results
        .into_iter()
        .map(|(obs, similarity)| ScoredObservation {
            id: obs.id.to_string(),
            observation_type: obs.observation_type,
            content: obs.content,
            similarity,
            reliability: obs.metadata.reliability.unwrap_or(0.70),
            source_type: obs.metadata.source_type.unwrap_or_else(|| "unknown".to_string()),
        })
        .collect();

    // Symbolic layer: Prolog validation
    let mut engine = ReasoningEngine::new().expect("Failed to create reasoning engine");
    engine
        .load_observations(&scored_obs)
        .expect("Failed to load observations");
    let validation = engine
        .validate_belief()
        .expect("Failed to validate belief");

    println!("\n=== Symbolic Layer: Validation Result ===");
    println!("  Valid: {}", validation.valid);
    println!("  Reason: {}", validation.reason);
    println!("  Weighted Score: {:.2}", validation.weighted_score);
    println!("  Strong Evidence Count: {}", validation.strong_evidence_count);
    println!("  Has Diverse Sources: {}", validation.has_diverse_sources);
    println!("  Avg Reliability: {:.2}", validation.avg_reliability);
    println!("  Avg Similarity: {:.2}", validation.avg_similarity);

    // Assertions: Strong evidence should validate
    assert!(validation.valid, "Should be valid with strong evidence");
    assert!(
        validation.weighted_score >= 3.0,
        "Should have weighted score >= 3.0, got {}",
        validation.weighted_score
    );
    assert!(
        validation.strong_evidence_count >= 2,
        "Should have multiple strong evidence"
    );
    assert!(validation.has_diverse_sources, "Should have diverse sources");
}

#[test]
fn test_neuro_symbolic_belief_validation_weak_evidence() {
    // Setup: Create temporary storage and search engine
    let embedder = get_test_embedder();
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut search =
        SemanticSearch::new(temp_dir.path(), embedder).expect("Failed to create search engine");

    // Add weak/irrelevant observations
    let observations = vec![
        ("Maybe Rust is good for some things", "pattern", 0.50, "comment"),
        ("Heard about Rust from colleague", "pattern", 0.60, "comment"),
    ];

    for (content, obs_type, reliability, source_type) in observations {
        let metadata = ObservationMetadata {
            created_at: Some(chrono::Utc::now()),
            updated_at: None,
            source: None,
            source_type: Some(source_type.to_string()),
            reliability: Some(reliability),
        };
        search
            .add_observation_with_metadata(content, obs_type, metadata)
            .expect("Failed to add observation");
    }

    // Neural layer: Semantic search
    let query = "I prefer Rust for systems programming";
    let mut query_embedder = get_test_embedder();
    let query_embedding = query_embedder.embed(query).expect("Failed to embed query");
    let search_results = search
        .observation_storage()
        .search_with_scores(&query_embedding, 10)
        .expect("Search should succeed");

    // Convert to ScoredObservation
    let scored_obs: Vec<ScoredObservation> = search_results
        .into_iter()
        .map(|(obs, similarity)| ScoredObservation {
            id: obs.id.to_string(),
            observation_type: obs.observation_type,
            content: obs.content,
            similarity,
            reliability: obs.metadata.reliability.unwrap_or(0.70),
            source_type: obs.metadata.source_type.unwrap_or_else(|| "unknown".to_string()),
        })
        .collect();

    // Symbolic layer: Prolog validation
    let mut engine = ReasoningEngine::new().expect("Failed to create reasoning engine");
    engine
        .load_observations(&scored_obs)
        .expect("Failed to load observations");
    let validation = engine
        .validate_belief()
        .expect("Failed to validate belief");

    println!("\n=== Weak Evidence Validation ===");
    println!("  Valid: {}", validation.valid);
    println!("  Reason: {}", validation.reason);
    println!("  Weighted Score: {:.2}", validation.weighted_score);

    // Assertions: Weak evidence should fail validation
    assert!(
        !validation.valid,
        "Should be invalid with weak evidence, reason: {}",
        validation.reason
    );
    assert_eq!(validation.reason, "weak_evidence");
    assert!(
        validation.weighted_score < 3.0,
        "Should have weighted score < 3.0"
    );
}

#[test]
fn test_neuro_symbolic_confidence_calculation() {
    let mut engine = ReasoningEngine::new().expect("Failed to create reasoning engine");

    // Test confidence calculation with different evidence counts
    let test_cases = vec![
        (0, 0.50), // No evidence: baseline 0.50
        (1, 0.65), // 1 evidence: 0.50 + (1 * 0.15) = 0.65
        (2, 0.80), // 2 evidence: 0.50 + (2 * 0.15) = 0.80
        (3, 0.80), // 3+ evidence: min(0.85, 0.50 + (3 * 0.1)) = 0.80
        (5, 0.85), // 5 evidence: capped at 0.85
    ];

    println!("\n=== Confidence Calculation Tests ===");
    for (evidence_count, expected) in test_cases {
        let confidence = engine
            .calculate_confidence(evidence_count)
            .expect("Confidence calculation should succeed");
        println!(
            "  Evidence: {} → Confidence: {:.2} (expected: {:.2})",
            evidence_count, confidence, expected
        );
        assert!(
            (confidence - expected).abs() < 0.01,
            "Evidence count {} should yield confidence ~{}, got {}",
            evidence_count,
            expected,
            confidence
        );
    }
}
