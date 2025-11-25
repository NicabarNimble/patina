//! Oxidize command - Build embeddings and projections from recipe
//!
//! Phase 2 MVP: Semantic projection only, in-memory training

pub mod pairs;
pub mod recipe;
pub mod trainer;

use anyhow::Result;
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

        println!("\n✅ Phase 2 MVP complete!");
        println!("   Next: Build USearch index from projected vectors");
    }

    Ok(())
}
