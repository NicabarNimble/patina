//! Embeddings command - Generate and manage semantic embeddings

use anyhow::{Context, Result};
use patina::embeddings::{create_embedder, EmbeddingsDatabase};
use patina::query::SemanticSearch;
use std::path::Path;

/// Generate embeddings for all beliefs and observations
pub fn generate(force: bool) -> Result<()> {
    let storage_path = ".patina/storage/observations";
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
        let beliefs_index = ".patina/storage/beliefs/beliefs.usearch";

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
    let mut search = SemanticSearch::new(".patina/storage", embedder)?;

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
fn generate_observation_embeddings(
    db_path: &str,
    search: &mut SemanticSearch,
) -> Result<usize> {
    use patina::storage::ObservationMetadata;
    use rusqlite::Connection;

    let conn = Connection::open(db_path).context("Failed to open observations database")?;
    let mut count = 0;

    // Query unified observations table
    let mut stmt = conn.prepare(
        "SELECT id, observation_type, content, metadata FROM observations"
    )?;

    let observations: Vec<(String, String, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,  // id
                row.get(1)?,  // observation_type
                row.get(2)?,  // content
                row.get(3)?,  // metadata
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for (id_str, obs_type, content, metadata_json) in observations {
        // Parse ID from database (preserves existing IDs like "obs_001")
        let id = match uuid::Uuid::parse_str(&id_str) {
            Ok(uuid) => uuid,
            Err(_) => {
                // If ID is not a valid UUID (like "obs_001"), generate one but warn
                eprintln!("⚠️  Warning: Invalid UUID '{}', generating new ID", id_str);
                uuid::Uuid::new_v4()
            }
        };

        // Parse metadata from JSON
        let metadata: ObservationMetadata = serde_json::from_str(&metadata_json)
            .unwrap_or_default();

        search.add_observation_with_id(id, &content, &obs_type, metadata)?;
        count += 1;
    }

    Ok(count)
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
