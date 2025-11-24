//! Oxidize command - Build embeddings and projections from recipe
//!
//! Phase 2 MVP: Semantic projection only, in-memory training

pub mod pairs;
pub mod recipe;

use anyhow::Result;
use pairs::generate_same_session_pairs;
use recipe::OxidizeRecipe;

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
    if recipe.projections.contains_key("semantic") {
        println!("\n📊 Generating training pairs for semantic projection...");
        let db_path = ".patina/data/patina.db";
        let num_pairs = 100; // Start with 100 pairs for MVP

        let pairs = generate_same_session_pairs(db_path, num_pairs)?;
        println!("   Generated {} training pairs", pairs.len());
        println!(
            "   Sample: anchor=\"{}...\"",
            &pairs[0].anchor.chars().take(50).collect::<String>()
        );
    }

    println!("\n⚠️  Training not yet implemented (next step)");

    Ok(())
}
