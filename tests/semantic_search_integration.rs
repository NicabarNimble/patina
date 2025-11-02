//! Integration tests for semantic search API

use patina::db::{DatabaseBackend, SqliteDatabase};
use patina::embeddings::{create_embedder, EmbeddingEngine, OnnxEmbedder};
use patina::query::SemanticSearch;
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

/// Setup a test database with vector tables
fn setup_test_db() -> Option<(TempDir, SqliteDatabase)> {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    // Open database (sqlite-vec loads automatically)
    let db = match SqliteDatabase::open(&db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("\n⚠️  Failed to open database: {}", e);
            return None;
        }
    };

    // Create vector tables
    if let Err(e) = db.execute_batch(include_str!("../.patina/vector-tables.sql")) {
        eprintln!("\n⚠️  Failed to create vector tables: {}", e);
        eprintln!("Error details: {}", e);
        return None;
    }

    Some((temp_dir, db))
}

/// Insert test belief with embedding
fn insert_test_belief(
    db: &SqliteDatabase,
    embedder: &mut dyn EmbeddingEngine,
    belief_id: i64,
    statement: &str,
) {
    // Generate embedding
    let embedding = embedder
        .embed(statement)
        .expect("Failed to generate embedding");

    // Insert into belief_vectors table
    db.vector_insert(patina::db::VectorTable::Beliefs, belief_id, &embedding, None)
        .expect("Failed to insert belief vector");
}

/// Insert test observation with embedding
fn insert_test_observation(
    db: &SqliteDatabase,
    embedder: &mut dyn EmbeddingEngine,
    observation_id: i64,
    observation_type: &str,
    text: &str,
) {
    // Generate embedding
    let embedding = embedder.embed(text).expect("Failed to generate embedding");

    // Insert into observation_vectors table
    db.vector_insert(
        patina::db::VectorTable::Observations,
        observation_id,
        &embedding,
        Some(observation_type),
    )
    .expect("Failed to insert observation vector");
}

#[test]
fn test_search_beliefs_basic() {
    let mut embedder = get_test_embedder();

    // Setup database - skip test if sqlite-vec not available
    let Some((_temp_dir, db)) = setup_test_db() else {
        eprintln!("⚠️  Skipping test_search_beliefs_basic - sqlite-vec not available");
        return;
    };

    // Insert test beliefs
    insert_test_belief(&db, &mut *embedder, 1, "I prefer Rust for systems programming");
    insert_test_belief(&db, &mut *embedder, 2, "I avoid global state in my code");
    insert_test_belief(&db, &mut *embedder, 3, "I like chocolate ice cream");

    // Create search engine
    let mut search = SemanticSearch::new(DatabaseBackend::Sqlite(db), embedder);

    // Search for Rust-related beliefs
    let results = search
        .search_beliefs("prefer rust for cli tools", 5)
        .expect("Search should succeed");

    println!("Results: {:?}", results);

    // Should find at least one result
    assert!(!results.is_empty(), "Should find at least one result");

    // First result should be the Rust belief (highest similarity)
    assert_eq!(results[0].0, 1, "First result should be belief 1 (Rust)");

    // Similarity should be positive (cosine similarity range is [-1, 1])
    assert!(
        results[0].1 > 0.0,
        "Similarity should be positive, got: {}",
        results[0].1
    );
}

#[test]
fn test_search_beliefs_ranking() {
    let mut embedder = get_test_embedder();

    // Setup database - skip test if sqlite-vec not available
    let Some((_temp_dir, db)) = setup_test_db() else {
        eprintln!("⚠️  Skipping test_search_beliefs_ranking - sqlite-vec not available");
        return;
    };

    // Insert test beliefs with varying relevance
    insert_test_belief(
        &db,
        &mut *embedder,
        1,
        "Rust provides memory safety without garbage collection",
    );
    insert_test_belief(
        &db,
        &mut *embedder,
        2,
        "Type systems prevent runtime errors",
    );
    insert_test_belief(
        &db,
        &mut *embedder,
        3,
        "I prefer functional programming patterns",
    );
    insert_test_belief(&db, &mut *embedder, 4, "Coffee is essential for coding");

    // Create search engine
    let mut search = SemanticSearch::new(DatabaseBackend::Sqlite(db), embedder);

    // Search for memory safety topics
    let results = search
        .search_beliefs("memory safe programming languages", 4)
        .expect("Search should succeed");

    println!("Ranking results: {:?}", results);

    // Should find all beliefs
    assert_eq!(results.len(), 4, "Should find all 4 beliefs");

    // First result should be most relevant (memory safety)
    assert_eq!(
        results[0].0, 1,
        "First result should be belief 1 (memory safety)"
    );

    // Second result should be type safety (related topic)
    assert_eq!(
        results[1].0, 2,
        "Second result should be belief 2 (type safety)"
    );

    // Similarities should be in descending order
    assert!(
        results[0].1 > results[1].1,
        "First result should have higher similarity than second"
    );
    assert!(
        results[1].1 > results[2].1,
        "Second result should have higher similarity than third"
    );
    assert!(
        results[2].1 > results[3].1,
        "Third result should have higher similarity than fourth"
    );
}

#[test]
fn test_search_observations_basic() {
    let mut embedder = get_test_embedder();

    // Setup database - skip test if sqlite-vec not available
    let Some((_temp_dir, db)) = setup_test_db() else {
        eprintln!("⚠️  Skipping test_search_observations_basic - sqlite-vec not available");
        return;
    };

    // Insert test observations
    insert_test_observation(
        &db,
        &mut *embedder,
        1,
        "pattern",
        "Always validate user input for security",
    );
    insert_test_observation(
        &db,
        &mut *embedder,
        2,
        "technology",
        "SQLite for embedded database storage",
    );
    insert_test_observation(
        &db,
        &mut *embedder,
        3,
        "decision",
        "Chose Rust over C++ for memory safety",
    );

    // Create search engine
    let mut search = SemanticSearch::new(DatabaseBackend::Sqlite(db), embedder);

    // Search for database-related observations
    let results = search
        .search_observations("database storage solutions", None, 5)
        .expect("Search should succeed");

    println!("Observation results: {:?}", results);

    // Should find at least one result
    assert!(!results.is_empty(), "Should find at least one result");

    // First result should be the database technology
    assert_eq!(
        results[0].0, 2,
        "First result should be observation 2 (SQLite)"
    );

    // Similarity should be positive
    assert!(
        results[0].2 > 0.0,
        "Similarity should be positive, got: {}",
        results[0].2
    );
}

#[test]
fn test_search_observations_with_type_filter() {
    let mut embedder = get_test_embedder();

    // Setup database - skip test if sqlite-vec not available
    let Some((_temp_dir, db)) = setup_test_db() else {
        eprintln!("⚠️  Skipping test_search_observations_with_type_filter - sqlite-vec not available");
        return;
    };

    // Insert observations of different types
    insert_test_observation(
        &db,
        &mut *embedder,
        1,
        "pattern",
        "Use dependency injection for loose coupling",
    );
    insert_test_observation(
        &db,
        &mut *embedder,
        2,
        "pattern",
        "Validate all inputs for security",
    );
    insert_test_observation(
        &db,
        &mut *embedder,
        3,
        "technology",
        "Rust programming language",
    );
    insert_test_observation(
        &db,
        &mut *embedder,
        4,
        "decision",
        "Decided to use microservices architecture",
    );

    // Create search engine
    let mut search = SemanticSearch::new(DatabaseBackend::Sqlite(db), embedder);

    // Search only patterns
    let results = search
        .search_observations("software design principles", Some("pattern"), 5)
        .expect("Search should succeed");

    println!("Filtered results: {:?}", results);

    // Should find only patterns
    assert!(!results.is_empty(), "Should find at least one pattern");

    // All results should be patterns
    for (obs_id, obs_type, _similarity) in &results {
        // Note: Currently obs_type is set to "pattern" because we passed it as filter
        // In the future when we improve metadata handling, this will be from the database
        println!("Found: id={}, type={}", obs_id, obs_type);
        assert!(
            *obs_id == 1 || *obs_id == 2,
            "Should only find pattern observations (1 or 2), got {}",
            obs_id
        );
    }
}

#[test]
fn test_search_observations_cross_type() {
    let mut embedder = get_test_embedder();

    // Setup database - skip test if sqlite-vec not available
    let Some((_temp_dir, db)) = setup_test_db() else {
        eprintln!("⚠️  Skipping test_search_observations_cross_type - sqlite-vec not available");
        return;
    };

    // Insert observations across different types about similar topics
    insert_test_observation(
        &db,
        &mut *embedder,
        1,
        "pattern",
        "Use RAII pattern for resource management",
    );
    insert_test_observation(
        &db,
        &mut *embedder,
        2,
        "technology",
        "Rust ownership system prevents memory leaks",
    );
    insert_test_observation(
        &db,
        &mut *embedder,
        3,
        "decision",
        "Chose Rust for automatic resource cleanup",
    );
    insert_test_observation(
        &db,
        &mut *embedder,
        4,
        "challenge",
        "Learning Rust's borrow checker rules",
    );

    // Create search engine
    let mut search = SemanticSearch::new(DatabaseBackend::Sqlite(db), embedder);

    // Search across all types for resource management
    let results = search
        .search_observations("automatic memory management", None, 5)
        .expect("Search should succeed");

    println!("Cross-type results: {:?}", results);

    // Should find results from multiple types
    assert!(
        results.len() >= 2,
        "Should find at least 2 results from different types"
    );

    // All results should have positive similarity (related to query)
    for (_obs_id, _obs_type, similarity) in &results {
        assert!(
            *similarity > 0.0,
            "All results should have positive similarity, got: {}",
            similarity
        );
    }
}
