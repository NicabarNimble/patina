//! Oxidize command - Build embeddings and projections from recipe
//!
//! Phase 2: Training + safetensors export + USearch index building

pub mod pairs;
pub mod recipe;
pub mod temporal;
pub mod trainer;

use anyhow::{Context, Result};
use pairs::generate_same_session_pairs;
use recipe::OxidizeRecipe;
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

    // Generate training pairs for semantic projection
    if let Some(config) = recipe.projections.get("semantic") {
        println!("\n📊 Generating training pairs for semantic projection...");
        let db_path = ".patina/data/patina.db";
        let num_pairs = 100; // Start with 100 pairs for MVP

        let pairs = generate_same_session_pairs(db_path, num_pairs)?;
        println!("   Generated {} training pairs", pairs.len());

        // Generate embeddings for training
        println!("\n🔮 Generating embeddings with {}...", recipe.embedding_model);
        use patina::embeddings::create_embedder;

        let mut embedder = create_embedder()?;
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
        println!("\n🧠 Training projection: {}→{}→{}...",
                 config.input_dim(), config.hidden_dim(), config.output_dim());

        let mut projection = Projection::new(
            config.input_dim(),
            config.hidden_dim(),
            config.output_dim(),
        );

        let learning_rate = 0.001;
        let _losses = projection.train(&anchors, &positives, &negatives, config.epochs, learning_rate)?;

        println!("\n✅ Training complete!");
        println!("   Output dimension: {} (from {})", config.output_dim(), config.input_dim());

        // Save trained weights
        println!("\n💾 Saving projection weights...");
        let output_dir = format!(".patina/data/embeddings/{}/projections", recipe.embedding_model);
        std::fs::create_dir_all(&output_dir)?;

        let weights_path = format!("{}/semantic.safetensors", output_dir);
        projection.save_safetensors(std::path::Path::new(&weights_path))?;
        println!("   Saved to: {}", weights_path);

        // Build USearch index from projected vectors
        println!("\n🔍 Building USearch index from projected vectors...");
        build_projection_index(
            db_path,
            &mut embedder,
            &projection,
            config.output_dim(),
            &output_dir,
            "semantic"
        )?;

        println!("\n✅ Phase 2 complete!");
        println!("   Projection ready for semantic search");
    }

    Ok(())
}

/// Build USearch index from projected embeddings
fn build_projection_index(
    db_path: &str,
    embedder: &mut Box<dyn patina::embeddings::EmbeddingEngine>,
    projection: &Projection,
    output_dim: usize,
    output_dir: &str,
    projection_name: &str,
) -> Result<()> {
    use rusqlite::Connection;
    use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

    // Open database
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Query all session events with content
    let mut stmt = conn.prepare(
        "SELECT seq, json_extract(data, '$.content') as content
         FROM eventlog
         WHERE event_type IN ('session.decision', 'session.pattern', 'session.goal', 'session.work', 'session.context')
           AND content IS NOT NULL
           AND length(content) > 20
         ORDER BY seq",
    )?;

    // Collect events
    let mut events: Vec<(i64, String)> = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let seq: i64 = row.get(0)?;
        let content: String = row.get(1)?;
        events.push((seq, content));
    }

    println!("   Found {} events to index", events.len());

    if events.is_empty() {
        println!("   ⚠️  No events found - skipping index build");
        return Ok(());
    }

    // Create USearch index
    let options = IndexOptions {
        dimensions: output_dim,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        ..Default::default()
    };

    let index = Index::new(&options)
        .context("Failed to create USearch index")?;

    index.reserve(events.len())
        .context("Failed to reserve index capacity")?;

    // Embed, project, and add to index
    println!("   Embedding and projecting vectors...");
    for (seq, content) in &events {
        // Embed
        let embedding = embedder.embed_passage(content)
            .context("Failed to generate embedding")?;

        // Project
        let projected = projection.forward(&embedding);

        // Add to index (using seq as key)
        index.add(*seq as u64, &projected)
            .context("Failed to add vector to index")?;
    }

    // Save index
    let index_path = format!("{}/{}.usearch", output_dir, projection_name);
    index.save(&index_path)
        .context("Failed to save USearch index")?;

    println!("   ✅ Index built: {} vectors", events.len());
    println!("   Saved to: {}", index_path);

    Ok(())
}
