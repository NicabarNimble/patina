//! Semantic search API demo
//!
//! This example demonstrates how to use the semantic search API.
//!
//! Note: Requires sqlite-vss extension to be installed.
//! To run: cargo run --example semantic_search_demo

use anyhow::Result;
use patina::embeddings::create_embedder;
use patina::query::{search_beliefs, search_observations};
use rusqlite::Connection;
use tempfile::TempDir;

fn setup_demo_db() -> Result<(TempDir, Connection)> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("demo.db");
    let conn = Connection::open(&db_path)?;

    // Enable extension loading and load sqlite-vss
    unsafe {
        conn.load_extension_enable()?;
        conn.load_extension("vss0", None)?;
    }

    // Create vector tables
    conn.execute_batch(include_str!("../.patina/vector-tables.sql"))?;

    // Create beliefs table
    conn.execute_batch(
        "CREATE TABLE beliefs (
            id INTEGER PRIMARY KEY,
            statement TEXT NOT NULL
        )",
    )?;

    // Create observations tables
    conn.execute_batch(
        "CREATE TABLE patterns (
            id INTEGER PRIMARY KEY,
            description TEXT NOT NULL
        )",
    )?;

    Ok((temp_dir, conn))
}

fn insert_sample_beliefs(conn: &Connection, embedder: &mut dyn patina::embeddings::EmbeddingEngine) -> Result<()> {
    let beliefs = vec![
        (1, "I prefer Rust for systems programming due to memory safety"),
        (2, "I value type safety and compile-time guarantees"),
        (3, "I avoid global mutable state in my code"),
        (4, "I prefer composition over inheritance"),
        (5, "I use ECS architecture for game development"),
    ];

    for (id, statement) in beliefs {
        // Insert belief
        conn.execute(
            "INSERT INTO beliefs (id, statement) VALUES (?, ?)",
            rusqlite::params![id, statement],
        )?;

        // Generate and insert embedding
        let embedding = embedder.embed(statement)?;
        let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|&f| f.to_le_bytes()).collect();

        conn.execute(
            "INSERT INTO belief_vectors (belief_id, embedding) VALUES (?, ?)",
            rusqlite::params![id, &embedding_bytes[..]],
        )?;
    }

    Ok(())
}

fn insert_sample_observations(conn: &Connection, embedder: &mut dyn patina::embeddings::EmbeddingEngine) -> Result<()> {
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
        let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|&f| f.to_le_bytes()).collect();

        conn.execute(
            "INSERT INTO observation_vectors (observation_id, observation_type, embedding) VALUES (?, ?, ?)",
            rusqlite::params![id, obs_type, &embedding_bytes[..]],
        )?;
    }

    Ok(())
}

fn main() -> Result<()> {
    println!("🔍 Semantic Search API Demo\n");

    // Setup
    println!("Setting up demo database...");
    let (_temp_dir, conn) = match setup_demo_db() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("❌ Failed to setup database: {}", e);
            eprintln!("\nThis demo requires sqlite-vss extension.");
            eprintln!("Install from: https://github.com/asg017/sqlite-vss");
            return Ok(());
        }
    };

    let mut embedder = create_embedder()?;

    println!("Inserting sample data...");
    insert_sample_beliefs(&conn, &mut *embedder)?;
    insert_sample_observations(&conn, &mut *embedder)?;

    println!();

    // Demo 1: Search beliefs
    println!("📚 Demo 1: Searching beliefs");
    println!("Query: \"type safe programming languages\"");
    let results = search_beliefs(&conn, "type safe programming languages", &mut *embedder, 3)?;

    for (i, (belief_id, similarity)) in results.iter().enumerate() {
        let statement: String = conn.query_row(
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
    let results = search_observations(&conn, "database technology", None, &mut *embedder, 3)?;

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
    let results = search_observations(&conn, "code security", Some("pattern"), &mut *embedder, 3)?;

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
    let results = search_observations(&conn, "memory safety in system programming", None, &mut *embedder, 5)?;

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
