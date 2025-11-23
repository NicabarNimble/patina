//! Semantic search API demo
//!
//! This example demonstrates how to use the semantic search API with USearch.
//!
//! To run: cargo run --example semantic_search_demo

use anyhow::Result;
use patina::embeddings::create_embedder;
use patina::query::SemanticSearch;
use tempfile::TempDir;

fn main() -> Result<()> {
    println!("🔍 Semantic Search API Demo\n");

    // Setup
    println!("Setting up demo...");
    let temp_dir = TempDir::new()?;
    let embedder = create_embedder()?;

    let mut search = SemanticSearch::new(temp_dir.path(), embedder)?;

    println!("Inserting sample beliefs...\n");

    // Add sample beliefs
    search.add_belief("I prefer Rust for systems programming due to memory safety")?;
    search.add_belief("I value type safety and compile-time guarantees")?;
    search.add_belief("I avoid global mutable state in my code")?;
    search.add_belief("I prefer composition over inheritance")?;
    search.add_belief("I use ECS architecture for game development")?;
    search.add_belief("Always validate user input for security")?;
    search.add_belief("Use dependency injection for loose coupling")?;
    search.add_belief("SQLite is great for embedded database storage")?;
    search.add_belief("Chose Rust over C++ for memory safety guarantees")?;

    // Demo 1: Search beliefs
    println!("📚 Demo 1: Searching beliefs");
    println!("Query: \"type safe programming languages\"");
    let results = search.search_beliefs("type safe programming languages", 3)?;

    for (i, belief) in results.iter().enumerate() {
        println!("  {}. {}", i + 1, belief.content);
    }

    println!("\n");

    // Demo 2: Search for security patterns
    println!("🔒 Demo 2: Searching for security-related beliefs");
    println!("Query: \"code security\"");
    let results = search.search_beliefs("code security", 3)?;

    for (i, belief) in results.iter().enumerate() {
        println!("  {}. {}", i + 1, belief.content);
    }

    println!("\n");

    // Demo 3: Search for architecture patterns
    println!("🏗️  Demo 3: Searching for architecture patterns");
    println!("Query: \"software design principles\"");
    let results = search.search_beliefs("software design principles", 3)?;

    for (i, belief) in results.iter().enumerate() {
        println!("  {}. {}", i + 1, belief.content);
    }

    println!("\n");

    // Demo 4: Cross-domain search
    println!("🌐 Demo 4: Cross-domain search");
    println!("Query: \"memory safety in system programming\"");
    let results = search.search_beliefs("memory safety in system programming", 5)?;

    for (i, belief) in results.iter().enumerate() {
        println!("  {}. {}", i + 1, belief.content);
    }

    println!("\n✅ Demo complete!");
    println!("\n💡 The demo uses temporary storage. In a real application,");
    println!("   you'd persist beliefs to a permanent location like .patina/data/beliefs");

    Ok(())
}
