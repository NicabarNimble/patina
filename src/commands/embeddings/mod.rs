//! Embeddings command - Generate and manage semantic embeddings

use anyhow::{Context, Result};
use patina::embeddings::{create_embedder, EmbeddingsDatabase};
use std::path::Path;

/// Generate embeddings for all beliefs and observations
pub fn generate(force: bool) -> Result<()> {
    let db_path = ".patina/db/facts.db";

    if !Path::new(db_path).exists() {
        anyhow::bail!(
            "Database not found at {}\n\nRun `patina scrape` first to create the knowledge database.",
            db_path
        );
    }

    println!("🔮 Generating embeddings...");
    println!();

    // Create embedder
    let mut embedder = create_embedder().context("Failed to create ONNX embedder")?;

    println!(
        "✓ Loaded {} model ({} dimensions)",
        embedder.model_name(),
        embedder.dimension()
    );

    // Open database wrapper
    let db = EmbeddingsDatabase::open(db_path).context("Failed to open database")?;

    // Check if we need to generate embeddings
    if db.has_embeddings()? && !force {
        println!("⚠️  Embeddings already exist. Use --force to regenerate.");
        return Ok(());
    }

    // Generate embeddings for beliefs
    println!();
    println!("📊 Generating embeddings for beliefs...");
    let belief_count = db.generate_belief_embeddings(&mut *embedder)?;
    println!("✓ Generated {} belief embeddings", belief_count);

    // Generate embeddings for observations
    println!();
    println!("📊 Generating embeddings for observations...");
    let obs_count = db.generate_observation_embeddings(&mut *embedder)?;
    println!("✓ Generated {} observation embeddings", obs_count);

    // Record metadata
    db.record_metadata(
        embedder.model_name(),
        "1.0",
        embedder.dimension(),
        belief_count,
        obs_count,
    )?;

    println!();
    println!("✅ Embeddings generation complete!");
    println!("   Total: {} embeddings", belief_count + obs_count);

    Ok(())
}

/// Show embedding coverage status
pub fn status() -> Result<()> {
    let db_path = ".patina/db/facts.db";

    if !Path::new(db_path).exists() {
        anyhow::bail!(
            "Database not found at {}\n\nRun `patina scrape` first to create the knowledge database.",
            db_path
        );
    }

    // Open database wrapper
    let db = EmbeddingsDatabase::open(db_path)?;

    // Get metadata
    let metadata = db.get_metadata()?;

    match metadata {
        Some(meta) => {
            println!("🔮 Embedding Status");
            println!();
            println!("Model:         {}", meta.model_name);
            println!("Version:       {}", meta.model_version);
            println!("Dimensions:    {}", meta.dimension);
            println!();
            println!("Coverage:");
            println!("  Beliefs:       {}", meta.belief_count);
            println!("  Observations:  {}", meta.observation_count);
            println!("  Total:         {}", meta.belief_count + meta.observation_count);
            println!();
            println!("✅ Embeddings are ready for semantic search");
        }
        None => {
            println!("⚠️  No embeddings found");
            println!();
            println!("Run `patina embeddings generate` to create embeddings.");
        }
    }

    Ok(())
}
