//! Semantic search API demo
//!
//! This example demonstrates how to use the semantic search API with sqlite-vec.
//!
//! To run: cargo run --example semantic_search_demo

use anyhow::Result;
use patina::db::{DatabaseBackend, SqliteDatabase};
use patina::embeddings::{create_embedder, EmbeddingEngine};
use patina::query::SemanticSearch;
use tempfile::TempDir;

fn setup_demo_db() -> Result<(TempDir, SqliteDatabase)> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("demo.db");

    // Open database (sqlite-vec loads automatically)
    let db = SqliteDatabase::open(&db_path)?;

    // Create vector tables
    db.execute_batch(include_str!("../.patina/vector-tables.sql"))?;

    // Create beliefs table
    db.execute_batch(
        "CREATE TABLE beliefs (
            id INTEGER PRIMARY KEY,
            statement TEXT NOT NULL
        )",
    )?;

    // Create observations tables
    db.execute_batch(
        "CREATE TABLE patterns (
            id INTEGER PRIMARY KEY,
            description TEXT NOT NULL
        )",
    )?;

    Ok((temp_dir, db))
}

fn insert_sample_beliefs(
    db: &SqliteDatabase,
    embedder: &mut dyn EmbeddingEngine,
) -> Result<()> {
    let beliefs = vec![
        (1, "I prefer Rust for systems programming due to memory safety"),
        (2, "I value type safety and compile-time guarantees"),
        (3, "I avoid global mutable state in my code"),
        (4, "I prefer composition over inheritance"),
        (5, "I use ECS architecture for game development"),
    ];

    for (id, statement) in beliefs {
        // Insert belief
        db.execute(
            "INSERT INTO beliefs (id, statement) VALUES (?, ?)",
            &[&id, &statement],
        )?;

        // Generate and insert embedding
        let embedding = embedder.embed(statement)?;
        db.vector_insert(
            patina::db::VectorTable::Beliefs,
            id,
            &embedding,
            None,
        )?;
    }

    Ok(())
}

fn insert_sample_observations(
    db: &SqliteDatabase,
    embedder: &mut dyn EmbeddingEngine,
) -> Result<()> {
    let observations = vec![
        (1, "pattern", "Always validate user input for security"),
        (2, "pattern", "Use dependency injection for loose coupling"),
        (3, "technology", "SQLite for embedded database storage"),
        (4, "technology", "Rust for safe systems programming"),
        (5, "decision", "Chose Rust over C++ for memory safety guarantees"),
        (6, "challenge", "Debugging borrow checker errors in complex async code"),
    ];

    for (id, obs_type, description) in observations {
        // Generate and insert embedding
        let embedding = embedder.embed(description)?;
        db.vector_insert(
            patina::db::VectorTable::Observations,
            id,
            &embedding,
            Some(obs_type),
        )?;
    }

    Ok(())
}

fn main() -> Result<()> {
    println!("🔍 Semantic Search API Demo\n");

    // Setup
    println!("Setting up demo database...");
    let (_temp_dir, db) = match setup_demo_db() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to setup database: {}", e);
            eprintln!("\nThis demo uses sqlite-vec (should work out of the box).");
            return Ok(());
        }
    };

    let mut embedder = create_embedder()?;

    println!("Inserting sample data...");
    insert_sample_beliefs(&db, &mut *embedder)?;
    insert_sample_observations(&db, &mut *embedder)?;

    println!();

    // Create semantic search engine
    let mut search = SemanticSearch::new(DatabaseBackend::Sqlite(db), embedder);

    // Demo 1: Search beliefs
    println!("📚 Demo 1: Searching beliefs");
    println!("Query: \"type safe programming languages\"");
    let results = search.search_beliefs("type safe programming languages", 3)?;

    for (i, (belief_id, similarity)) in results.iter().enumerate() {
        let statement: String = search
            .database()
            .connection()
            .query_row(
                "SELECT statement FROM beliefs WHERE id = ?",
                [belief_id],
                |row| row.get(0),
            )?;
        println!(
            "  {}. [Similarity: {:.3}] {}",
            i + 1,
            similarity,
            statement
        );
    }

    println!("\n");

    // Demo 2: Search observations (all types)
    println!("📋 Demo 2: Searching all observations");
    println!("Query: \"database technology\"");
    let results = search.search_observations("database technology", None, 3)?;

    for (i, (obs_id, obs_type, similarity)) in results.iter().enumerate() {
        println!(
            "  {}. [Type: {}, Similarity: {:.3}] ID: {}",
            i + 1,
            obs_type,
            similarity,
            obs_id
        );
    }

    println!("\n");

    // Demo 3: Search observations with type filter
    println!("🔧 Demo 3: Searching patterns only");
    println!("Query: \"code security\"");
    let results = search.search_observations("code security", Some("pattern"), 3)?;

    for (i, (obs_id, obs_type, similarity)) in results.iter().enumerate() {
        println!(
            "  {}. [Type: {}, Similarity: {:.3}] ID: {}",
            i + 1,
            obs_type,
            similarity,
            obs_id
        );
    }

    println!("\n");

    // Demo 4: Cross-domain search
    println!("🌐 Demo 4: Cross-domain search");
    println!("Query: \"memory safety in system programming\"");
    let results = search.search_observations("memory safety in system programming", None, 5)?;

    for (i, (obs_id, obs_type, similarity)) in results.iter().enumerate() {
        println!(
            "  {}. [Type: {}, Similarity: {:.3}] ID: {}",
            i + 1,
            obs_type,
            similarity,
            obs_id
        );
    }

    println!("\n✅ Demo complete!");

    Ok(())
}
