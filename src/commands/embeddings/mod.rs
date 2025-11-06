//! Embeddings command - Generate and manage semantic embeddings

use anyhow::{Context, Result};
use patina::embeddings::{create_embedder, EmbeddingsDatabase};
use patina::query::SemanticSearch;
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
    let embedder = create_embedder().context("Failed to create ONNX embedder")?;

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

    // Create semantic search engine (for vector storage)
    let mut search = SemanticSearch::new(".patina/storage", embedder)?;

    // Generate embeddings for beliefs (not implemented yet - TODO)
    println!();
    println!("📊 Generating embeddings for beliefs...");
    let belief_count = 0; // TODO: generate_belief_embeddings
    println!("✓ Generated {} belief embeddings", belief_count);

    // Generate embeddings for observations
    println!();
    println!("📊 Generating embeddings for observations...");
    let obs_count = generate_observation_embeddings(&db, &mut search)?;
    println!("✓ Generated {} observation embeddings", obs_count);

    // Record metadata
    db.record_metadata(
        search.observation_storage().count()?.to_string().as_str(),
        "1.0",
        384,
        belief_count,
        obs_count,
    )?;

    println!();
    println!("✅ Embeddings generation complete!");
    println!("   Total: {} embeddings", belief_count + obs_count);

    Ok(())
}

/// Generate observation embeddings and store in ObservationStorage
fn generate_observation_embeddings(
    db: &EmbeddingsDatabase,
    search: &mut SemanticSearch,
) -> Result<usize> {
    use patina::storage::ObservationMetadata;

    let conn = db.database().connection();
    let mut count = 0;

    // Patterns - sourced from session distillations (reliability: 0.85)
    let mut stmt = conn.prepare("SELECT id, pattern_name, description FROM patterns")?;
    let patterns: Vec<(i64, String, Option<String>)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    for (_id, name, desc) in patterns {
        let content = match desc {
            Some(d) => format!("{}: {}", name, d),
            None => name.clone(),
        };

        let metadata = ObservationMetadata {
            source_type: Some("session_distillation".to_string()),
            reliability: Some(0.85),
            ..Default::default()
        };

        search.add_observation_with_metadata(&content, "pattern", metadata)?;
        count += 1;
    }

    // Technologies - sourced from session distillations (reliability: 0.85)
    let mut stmt = conn.prepare("SELECT id, tech_name, purpose FROM technologies")?;
    let technologies: Vec<(i64, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    for (_id, name, purpose) in technologies {
        let content = format!("{}: {}", name, purpose);

        let metadata = ObservationMetadata {
            source_type: Some("session_distillation".to_string()),
            reliability: Some(0.85),
            ..Default::default()
        };

        search.add_observation_with_metadata(&content, "technology", metadata)?;
        count += 1;
    }

    // Decisions - sourced from session distillations (reliability: 0.85)
    let mut stmt = conn.prepare("SELECT id, choice, rationale FROM decisions")?;
    let decisions: Vec<(i64, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    for (_id, choice, rationale) in decisions {
        let content = format!("{}: {}", choice, rationale);

        let metadata = ObservationMetadata {
            source_type: Some("session_distillation".to_string()),
            reliability: Some(0.85),
            ..Default::default()
        };

        search.add_observation_with_metadata(&content, "decision", metadata)?;
        count += 1;
    }

    // Challenges - sourced from session distillations (reliability: 0.85)
    let mut stmt = conn.prepare("SELECT id, problem, solution FROM challenges")?;
    let challenges: Vec<(i64, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    for (_id, problem, solution) in challenges {
        let content = format!("{}: {}", problem, solution);

        let metadata = ObservationMetadata {
            source_type: Some("session_distillation".to_string()),
            reliability: Some(0.85),
            ..Default::default()
        };

        search.add_observation_with_metadata(&content, "challenge", metadata)?;
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
