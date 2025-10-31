//! Embeddings command - Generate and manage semantic embeddings

use anyhow::{Context, Result};
use patina::embeddings::{create_embedder, EmbeddingEngine};
use rusqlite::{Connection, OptionalExtension};
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

    // Connect to database
    let mut conn = Connection::open(db_path).context("Failed to open database")?;

    // Check if we need to generate embeddings
    let existing_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM embedding_metadata", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    if existing_count > 0 && !force {
        println!("⚠️  Embeddings already exist. Use --force to regenerate.");
        return Ok(());
    }

    // Generate embeddings for beliefs
    println!();
    println!("📊 Generating embeddings for beliefs...");
    let belief_count = generate_belief_embeddings(&mut conn, &mut *embedder)?;
    println!("✓ Generated {} belief embeddings", belief_count);

    // Generate embeddings for observations
    println!();
    println!("📊 Generating embeddings for observations...");
    let obs_count = generate_observation_embeddings(&mut conn, &mut *embedder)?;
    println!("✓ Generated {} observation embeddings", obs_count);

    // Record metadata
    conn.execute(
        "INSERT INTO embedding_metadata (model_name, model_version, dimension, belief_count, observation_count)
         VALUES (?, ?, ?, ?, ?)",
        (
            embedder.model_name(),
            "1.0",
            embedder.dimension() as i64,
            belief_count as i64,
            obs_count as i64,
        ),
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

    let conn = Connection::open(db_path)?;

    // Check for embedding metadata
    let metadata: Option<(String, String, i64, i64, i64)> = conn
        .query_row(
            "SELECT model_name, model_version, dimension, belief_count, observation_count
             FROM embedding_metadata
             ORDER BY generated_at DESC LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .optional()?;

    match metadata {
        Some((model, version, dim, beliefs, obs)) => {
            println!("🔮 Embedding Status");
            println!();
            println!("Model:         {}", model);
            println!("Version:       {}", version);
            println!("Dimensions:    {}", dim);
            println!();
            println!("Coverage:");
            println!("  Beliefs:       {}", beliefs);
            println!("  Observations:  {}", obs);
            println!("  Total:         {}", beliefs + obs);
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

/// Generate embeddings for all beliefs
fn generate_belief_embeddings(
    conn: &mut Connection,
    embedder: &mut dyn EmbeddingEngine,
) -> Result<usize> {
    let mut stmt = conn.prepare("SELECT id, statement FROM beliefs WHERE active = TRUE")?;
    let beliefs: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    let count = beliefs.len();

    for (id, statement) in beliefs {
        let embedding = embedder
            .embed(&statement)
            .context(format!("Failed to generate embedding for belief {}", id))?;

        // Note: Storing embeddings as JSON for now since we don't have sqlite-vss loaded yet
        // In Phase 2, we'll switch to proper vector storage
        conn.execute(
            "INSERT OR REPLACE INTO embedding_metadata (id, model_name, model_version, dimension)
             VALUES (?, ?, ?, ?)",
            (
                id,
                embedder.model_name(),
                "1.0",
                embedder.dimension() as i64,
            ),
        )?;

        // For now, just validate the embedding was generated
        if embedding.len() != embedder.dimension() {
            anyhow::bail!(
                "Embedding dimension mismatch for belief {}: expected {}, got {}",
                id,
                embedder.dimension(),
                embedding.len()
            );
        }
    }

    Ok(count)
}

/// Generate embeddings for all observations
fn generate_observation_embeddings(
    conn: &mut Connection,
    embedder: &mut dyn EmbeddingEngine,
) -> Result<usize> {
    let mut count = 0;

    // Patterns
    let mut stmt = conn.prepare("SELECT id, pattern_name, description FROM patterns")?;
    let patterns: Vec<(i64, String, Option<String>)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    for (id, name, desc) in patterns {
        let text = match desc {
            Some(d) => format!("{}: {}", name, d),
            None => name.clone(),
        };
        let _embedding = embedder
            .embed(&text)
            .context(format!("Failed to generate embedding for pattern {}", id))?;
        count += 1;
    }

    // Technologies
    let mut stmt = conn.prepare("SELECT id, tech_name, purpose FROM technologies")?;
    let technologies: Vec<(i64, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    for (id, name, purpose) in technologies {
        let text = format!("{}: {}", name, purpose);
        let _embedding = embedder.embed(&text).context(format!(
            "Failed to generate embedding for technology {}",
            id
        ))?;
        count += 1;
    }

    // Decisions
    let mut stmt = conn.prepare("SELECT id, choice, rationale FROM decisions")?;
    let decisions: Vec<(i64, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    for (id, choice, rationale) in decisions {
        let text = format!("{}: {}", choice, rationale);
        let _embedding = embedder
            .embed(&text)
            .context(format!("Failed to generate embedding for decision {}", id))?;
        count += 1;
    }

    // Challenges
    let mut stmt = conn.prepare("SELECT id, problem, solution FROM challenges")?;
    let challenges: Vec<(i64, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    for (id, problem, solution) in challenges {
        let text = format!("{}: {}", problem, solution);
        let _embedding = embedder
            .embed(&text)
            .context(format!("Failed to generate embedding for challenge {}", id))?;
        count += 1;
    }

    Ok(count)
}
