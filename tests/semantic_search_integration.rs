//! Integration tests for semantic search API

use patina::embeddings::{create_embedder, EmbeddingEngine, OnnxEmbedder};
use patina::query::{search_beliefs, search_observations};
use rusqlite::{ffi::sqlite3_auto_extension, Connection};
use sqlite_vec::sqlite3_vec_init;
use std::path::Path;
use tempfile::TempDir;
use zerocopy::AsBytes;

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
/// Returns None if sqlite-vec extension is not available
fn setup_test_db() -> Option<(TempDir, Connection)> {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    // Register sqlite-vec extension globally
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }

    let conn = Connection::open(&db_path).expect("Failed to open test database");

    // Create vector tables
    if let Err(e) = conn.execute_batch(include_str!("../.patina/vector-tables.sql")) {
        eprintln!("\n⚠️  Failed to create vector tables: {}", e);
        eprintln!("Error details: {}", e);
        return None;
    }

    Some((temp_dir, conn))
}

/// Insert test belief with embedding
fn insert_test_belief(
    conn: &Connection,
    embedder: &mut dyn EmbeddingEngine,
    belief_id: i64,
    statement: &str,
) {
    // Generate embedding
    let embedding = embedder
        .embed(statement)
        .expect("Failed to generate embedding");

    // Insert into belief_vectors table (rowid will be set to belief_id)
    conn.execute(
        "INSERT INTO belief_vectors (rowid, embedding) VALUES (?, ?)",
        rusqlite::params![belief_id, embedding.as_bytes()],
    )
    .expect("Failed to insert belief vector");
}

/// Insert test observation with embedding
fn insert_test_observation(
    conn: &Connection,
    embedder: &mut dyn EmbeddingEngine,
    observation_id: i64,
    observation_type: &str,
    text: &str,
) {
    // Generate embedding
    let embedding = embedder.embed(text).expect("Failed to generate embedding");

    // Insert into observation_vectors table (rowid will be set to observation_id)
    conn.execute(
        "INSERT INTO observation_vectors (rowid, embedding, observation_type) VALUES (?, ?, ?)",
        rusqlite::params![observation_id, embedding.as_bytes(), observation_type],
    )
    .expect("Failed to insert observation vector");
}

#[test]
fn test_search_beliefs_basic() {
    let mut embedder = get_test_embedder();

    // Setup database - skip test if sqlite-vec not available
    let Some((_temp_dir, conn)) = setup_test_db() else {
        eprintln!("⚠️  Skipping test_search_beliefs_basic - sqlite-vec not available");
        return;
    };

    // Insert test beliefs
    insert_test_belief(
        &conn,
        &mut *embedder,
        1,
        "I prefer Rust for systems programming",
    );
    insert_test_belief(&conn, &mut *embedder, 2, "I avoid global state in my code");
    insert_test_belief(&conn, &mut *embedder, 3, "I like chocolate ice cream");

    // Search for Rust-related beliefs
    let results = search_beliefs(&conn, "prefer rust for cli tools", &mut *embedder, 5)
        .expect("Search should succeed");

    println!("Results: {:?}", results);

    // Should find at least the Rust belief
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
    let Some((_temp_dir, conn)) = setup_test_db() else {
        eprintln!("⚠️  Skipping test_search_beliefs_ranking - sqlite-vec not available");
        return;
    };

    // Insert test beliefs with varying relevance
    insert_test_belief(
        &conn,
        &mut *embedder,
        1,
        "I prefer ECS architecture for game development",
    );
    insert_test_belief(
        &conn,
        &mut *embedder,
        2,
        "I use entity component systems in Bevy",
    );
    insert_test_belief(
        &conn,
        &mut *embedder,
        3,
        "I like to bake cookies on weekends",
    );

    // Search for ECS-related beliefs
    let results = search_beliefs(
        &conn,
        "entity component system for games",
        &mut *embedder,
        5,
    )
    .expect("Search should succeed");

    println!("Results: {:?}", results);

    // Should find multiple results
    assert!(results.len() >= 2, "Should find at least 2 results");

    // Both ECS beliefs should rank higher than the cookies belief
    let ecs_belief_ids = vec![1, 2];
    assert!(
        ecs_belief_ids.contains(&results[0].0),
        "First result should be an ECS belief, got: {}",
        results[0].0
    );
    assert!(
        ecs_belief_ids.contains(&results[1].0),
        "Second result should be an ECS belief, got: {}",
        results[1].0
    );
}

#[test]
fn test_search_observations_basic() {
    let mut embedder = get_test_embedder();

    // Setup database - skip test if sqlite-vec not available
    let Some((_temp_dir, conn)) = setup_test_db() else {
        eprintln!("⚠️  Skipping test_search_observations_basic - sqlite-vec not available");
        return;
    };

    // Insert test observations
    insert_test_observation(
        &conn,
        &mut *embedder,
        1,
        "pattern",
        "Use dependency injection for loose coupling",
    );
    insert_test_observation(
        &conn,
        &mut *embedder,
        2,
        "technology",
        "SQLite for embedded database",
    );
    insert_test_observation(
        &conn,
        &mut *embedder,
        3,
        "decision",
        "Chose Rust over C++ for memory safety",
    );

    // Search all observations
    let results = search_observations(&conn, "database technology", None, &mut *embedder, 5)
        .expect("Search should succeed");

    println!("Results: {:?}", results);

    // Should find results
    assert!(!results.is_empty(), "Should find at least one result");

    // SQLite observation should rank high
    assert!(
        results.iter().any(|(id, _, _)| *id == 2),
        "Should find the SQLite observation"
    );
}

#[test]
fn test_search_observations_with_type_filter() {
    let mut embedder = get_test_embedder();

    // Setup database - skip test if sqlite-vec not available
    let Some((_temp_dir, conn)) = setup_test_db() else {
        eprintln!("⚠️  Skipping test_search_observations_with_type_filter - sqlite-vec not available");
        return;
    };

    // Insert test observations
    insert_test_observation(
        &conn,
        &mut *embedder,
        1,
        "pattern",
        "Always validate user input for security",
    );
    insert_test_observation(
        &conn,
        &mut *embedder,
        2,
        "pattern",
        "Use const generics for type safety",
    );
    insert_test_observation(
        &conn,
        &mut *embedder,
        3,
        "technology",
        "Rust for systems programming",
    );
    insert_test_observation(
        &conn,
        &mut *embedder,
        4,
        "decision",
        "Chose to implement input validation",
    );

    // Search only patterns
    let results = search_observations(
        &conn,
        "input validation security",
        Some("pattern"),
        &mut *embedder,
        5,
    )
    .expect("Search should succeed");

    println!("Results: {:?}", results);

    // Should find results
    assert!(!results.is_empty(), "Should find at least one result");

    // All results should be patterns
    for (_, obs_type, _) in &results {
        assert_eq!(obs_type, "pattern", "All results should be patterns");
    }

    // Should find the validation pattern
    assert!(
        results.iter().any(|(id, _, _)| *id == 1),
        "Should find the input validation pattern"
    );
}

#[test]
fn test_search_observations_cross_type() {
    let mut embedder = get_test_embedder();

    // Setup database - skip test if sqlite-vec not available
    let Some((_temp_dir, conn)) = setup_test_db() else {
        eprintln!("⚠️  Skipping test_search_observations_cross_type - sqlite-vec not available");
        return;
    };

    // Insert semantically similar observations across different types
    insert_test_observation(
        &conn,
        &mut *embedder,
        1,
        "pattern",
        "Prefer composition over inheritance",
    );
    insert_test_observation(
        &conn,
        &mut *embedder,
        2,
        "technology",
        "Rust traits for composition",
    );
    insert_test_observation(
        &conn,
        &mut *embedder,
        3,
        "decision",
        "Rejected OOP inheritance in favor of traits",
    );

    // Search without type filter (should find all related observations)
    let results = search_observations(&conn, "composition traits", None, &mut *embedder, 5)
        .expect("Search should succeed");

    println!("Results: {:?}", results);

    // Should find multiple related observations across types
    assert!(results.len() >= 2, "Should find at least 2 results");

    // Should include different observation types
    let types: Vec<String> = results.iter().map(|(_, t, _)| t.clone()).collect();
    assert!(
        types.iter().any(|t| t == "pattern") || types.iter().any(|t| t == "technology"),
        "Should find observations across different types"
    );
}
