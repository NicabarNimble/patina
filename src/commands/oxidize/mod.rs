//! Oxidize command - Build embeddings and projections from recipe
//!
//! Phase 2 MVP: Semantic projection only, in-memory training

pub mod recipe;

use anyhow::Result;
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

    println!("\n⚠️  Training not implemented yet (Phase 2 in progress)");

    Ok(())
}
