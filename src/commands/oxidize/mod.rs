//! Oxidize command - Build embeddings and projections from recipe
//!
//! Phase 2: Training + safetensors export + USearch index building

pub mod dependency;
pub mod pairs;
pub mod recipe;
pub mod temporal;
pub mod trainer;

use anyhow::{Context, Result};
use dependency::generate_dependency_pairs;
use pairs::{generate_same_session_pairs, TrainingPair};
use recipe::{OxidizeRecipe, ProjectionConfig};
use temporal::generate_temporal_pairs;
use trainer::Projection;

/// Run oxidize command
pub fn oxidize() -> Result<()> {
    println!("🧪 Oxidize - Build embeddings and projections");

    // Load recipe
    let recipe = OxidizeRecipe::load()?;

    println!("✅ Recipe loaded: {}", recipe.embedding_model);
    println!("   Projections: {}", recipe.projections.len());

    for (name, config) in &recipe.projections {
        println!(
            "   - {}: {}→{}→{} ({} epochs)",
            name,
            config.input_dim(),
            config.hidden_dim(),
            config.output_dim(),
            config.epochs
        );
    }

    let db_path = ".patina/data/patina.db";
    let output_dir = format!(
        ".patina/data/embeddings/{}/projections",
        recipe.embedding_model
    );
    std::fs::create_dir_all(&output_dir)?;

    // Create embedder once, reuse for all projections
    use patina::embeddings::create_embedder;
    let mut embedder = create_embedder()?;

    // Train each projection
    for (name, config) in &recipe.projections {
        println!("\n{}", "=".repeat(60));
        println!("📊 Training {} projection...", name);
        println!("{}", "=".repeat(60));

        let projection = train_projection(name, config, db_path, &mut embedder)?;

        // Save trained weights
        println!("\n💾 Saving projection weights...");
        let weights_path = format!("{}/{}.safetensors", output_dir, name);
        projection.save_safetensors(std::path::Path::new(&weights_path))?;
        println!("   Saved to: {}", weights_path);

        // Build USearch index
        println!("\n🔍 Building USearch index...");
        build_projection_index(
            name,
            db_path,
            &mut embedder,
            &projection,
            config.output_dim(),
            &output_dir,
        )?;

        println!("\n✅ {} projection complete!", name);
    }

    println!("\n{}", "=".repeat(60));
    println!("✅ All projections trained!");
    println!("   Output: {}", output_dir);

    Ok(())
}

/// Train a projection based on its name
fn train_projection(
    name: &str,
    config: &ProjectionConfig,
    db_path: &str,
    embedder: &mut Box<dyn patina::embeddings::EmbeddingEngine>,
) -> Result<Projection> {
    let num_pairs = 100; // Start with 100 pairs for MVP

    // Generate pairs based on projection type
    let pairs: Vec<TrainingPair> = match name {
        "semantic" => {
            println!("   Strategy: observations from same session are similar");
            generate_same_session_pairs(db_path, num_pairs)?
        }
        "temporal" => {
            println!("   Strategy: files that co-change are related");
            generate_temporal_pairs(db_path, num_pairs)?
        }
        "dependency" => {
            println!("   Strategy: functions that call each other are related");
            generate_dependency_pairs(db_path, num_pairs)?
        }
        _ => {
            anyhow::bail!(
                "Unknown projection type: {}. Supported: semantic, temporal, dependency",
                name
            );
        }
    };

    println!("   Generated {} training pairs", pairs.len());

    // Generate embeddings
    println!("\n🔮 Generating embeddings...");
    let mut anchors = Vec::new();
    let mut positives = Vec::new();
    let mut negatives = Vec::new();

    for pair in &pairs {
        anchors.push(embedder.embed_passage(&pair.anchor)?);
        positives.push(embedder.embed_passage(&pair.positive)?);
        negatives.push(embedder.embed_passage(&pair.negative)?);
    }

    println!("   Embedded {} triplets", anchors.len());

    // Train projection
    println!(
        "\n🧠 Training MLP: {}→{}→{}...",
        config.input_dim(),
        config.hidden_dim(),
        config.output_dim()
    );

    let mut projection =
        Projection::new(config.input_dim(), config.hidden_dim(), config.output_dim());

    let learning_rate = 0.001;
    let _losses = projection.train(
        &anchors,
        &positives,
        &negatives,
        config.epochs,
        learning_rate,
    )?;

    println!("   Training complete!");

    Ok(projection)
}

/// Build USearch index from projected embeddings
fn build_projection_index(
    projection_name: &str,
    db_path: &str,
    embedder: &mut Box<dyn patina::embeddings::EmbeddingEngine>,
    projection: &Projection,
    output_dim: usize,
    output_dir: &str,
) -> Result<()> {
    use rusqlite::Connection;
    use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

    // Open database
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Get content to index based on projection type
    let events: Vec<(i64, String)> = match projection_name {
        "semantic" => query_session_events(&conn)?,
        "temporal" => query_file_events(&conn)?,
        "dependency" => dependency::query_function_events(&conn)?,
        _ => {
            println!("   ⚠️  No index builder for {} - skipping", projection_name);
            return Ok(());
        }
    };

    println!("   Found {} items to index", events.len());

    if events.is_empty() {
        println!("   ⚠️  No items found - skipping index build");
        return Ok(());
    }

    // Create USearch index
    let options = IndexOptions {
        dimensions: output_dim,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        ..Default::default()
    };

    let index = Index::new(&options).context("Failed to create USearch index")?;
    index
        .reserve(events.len())
        .context("Failed to reserve index capacity")?;

    // Embed, project, and add to index
    println!("   Embedding and projecting vectors...");
    for (id, content) in &events {
        let embedding = embedder
            .embed_passage(content)
            .context("Failed to generate embedding")?;
        let projected = projection.forward(&embedding);
        index
            .add(*id as u64, &projected)
            .context("Failed to add vector to index")?;
    }

    // Save index
    let index_path = format!("{}/{}.usearch", output_dir, projection_name);
    index
        .save(&index_path)
        .context("Failed to save USearch index")?;

    println!("   ✅ Index built: {} vectors", events.len());
    println!("   Saved to: {}", index_path);

    Ok(())
}

/// Query session events for semantic index
fn query_session_events(conn: &rusqlite::Connection) -> Result<Vec<(i64, String)>> {
    let mut stmt = conn.prepare(
        "SELECT seq, json_extract(data, '$.content') as content
         FROM eventlog
         WHERE event_type IN ('session.decision', 'session.pattern', 'session.goal', 'session.work', 'session.context')
           AND content IS NOT NULL
           AND length(content) > 20
         ORDER BY seq",
    )?;

    let mut events = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let seq: i64 = row.get(0)?;
        let content: String = row.get(1)?;
        events.push((seq, content));
    }

    Ok(events)
}

/// Query file events for temporal index
fn query_file_events(conn: &rusqlite::Connection) -> Result<Vec<(i64, String)>> {
    // Get unique files from co_changes with their index
    let mut stmt = conn.prepare(
        "SELECT DISTINCT file_a FROM co_changes
         UNION
         SELECT DISTINCT file_b FROM co_changes
         ORDER BY 1",
    )?;

    let mut events = Vec::new();
    let mut rows = stmt.query([])?;
    let mut idx: i64 = 0;
    while let Some(row) = rows.next()? {
        let file_path: String = row.get(0)?;
        // Convert file path to descriptive text for embedding
        let text = temporal::file_to_text(&file_path);
        events.push((idx, text));
        idx += 1;
    }

    Ok(events)
}
