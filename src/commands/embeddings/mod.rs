//! Embeddings command - Generate and manage semantic embeddings

use anyhow::{Context, Result};
use patina::embeddings::{create_embedder, EmbeddingsDatabase};
use patina::query::SemanticSearch;
use std::path::Path;

/// Generate embeddings for all beliefs and observations
pub fn generate(force: bool) -> Result<()> {
    let storage_path = ".patina/data/observations";
    let db_path = format!("{}/observations.db", storage_path);

    if !Path::new(&db_path).exists() {
        anyhow::bail!(
            "Database not found at {}\n\nNo observations found. Run Topic 0 manual smoke test or session extraction first.",
            db_path
        );
    }

    println!("🔮 Generating embeddings...");
    println!();

    // Create embedder
    let embedder = create_embedder().context("Failed to create ONNX embedder")?;

    println!(
        "✓ Loaded {} model ({} dimensions)",
        embedder.model_name(),
        embedder.dimension()
    );

    // If force flag, delete existing indices to rebuild from scratch
    if force {
        let obs_index = format!("{}/observations.usearch", storage_path);
        let beliefs_index = ".patina/data/beliefs/beliefs.usearch";

        if Path::new(&obs_index).exists() {
            std::fs::remove_file(&obs_index).context("Failed to remove observations index")?;
            println!("🗑️  Removed existing observations index");
        }
        if Path::new(beliefs_index).exists() {
            std::fs::remove_file(beliefs_index).context("Failed to remove beliefs index")?;
            println!("🗑️  Removed existing beliefs index");
        }
    }

    // Create semantic search engine (for vector storage)
    let mut search = SemanticSearch::new(".patina/data", embedder)?;

    // Generate embeddings for beliefs (not implemented yet - TODO)
    println!();
    println!("📊 Generating embeddings for beliefs...");
    let belief_count = 0; // TODO: generate_belief_embeddings
    println!("✓ Generated {} belief embeddings", belief_count);

    // Generate embeddings for observations from unified observations table
    println!();
    println!("📊 Generating embeddings for observations...");
    let obs_count = generate_observation_embeddings(&db_path, &mut search)?;
    println!("✓ Generated {} observation embeddings", obs_count);

    println!();
    println!("✅ Embeddings generation complete!");
    println!("   Total: {} observation embeddings", obs_count);

    Ok(())
}

/// Generate observation embeddings and store in ObservationStorage
///
/// Rebuilds USearch index from existing SQLite observations.
/// Does NOT insert into SQLite - only generates embeddings and builds vector index.
fn generate_observation_embeddings(_db_path: &str, search: &mut SemanticSearch) -> Result<usize> {
    // Get mutable access to observation storage
    let obs_storage = search.observation_storage_mut();

    // Query all observations from SQLite (includes rowid)
    let observations = obs_storage
        .query_all()
        .context("Failed to query observations from SQLite")?;

    let total = observations.len();
    println!("   Found {} observations in SQLite", total);

    let mut count = 0;

    // Drop obs_storage reference so we can use search methods
    let _ = obs_storage;

    // Process each observation: generate embedding and add to index
    for (rowid, _id, _obs_type, content, _metadata_json) in observations {
        // Generate embedding (passages - observations being stored)
        let embedding = search
            .embed_passage(&content)
            .context("Failed to generate embedding")?;

        // Add to USearch index only (not SQLite)
        search
            .observation_storage_mut()
            .add_to_index_only(rowid, &embedding)
            .context("Failed to add to vector index")?;

        count += 1;

        if count % 100 == 0 {
            println!("   Progress: {}/{} embeddings generated", count, total);
        }
    }

    // Save the index
    search
        .observation_storage_mut()
        .save_index()
        .context("Failed to save vector index")?;

    Ok(count)
}

/// Show embedding coverage status
pub fn status() -> Result<()> {
    let db_path = ".patina/data/facts.db";

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
            println!(
                "  Total:         {}",
                meta.belief_count + meta.observation_count
            );
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
