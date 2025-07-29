use anyhow::{Context, Result};
use patina::layer::Layer;
use patina::session::SessionManager;
use std::fs;

pub fn execute() -> Result<()> {
    // Find project root
    let project_root = SessionManager::find_project_root()
        .context("Not in a Patina project directory. Run 'patina init' first.")?;

    println!("🚀 Generating context for LLM...");

    // Read project config to determine LLM type
    let config_path = project_root.join(".patina").join("config.json");
    let config_content =
        fs::read_to_string(&config_path).context("Failed to read project config")?;
    let config: serde_json::Value = serde_json::from_str(&config_content)?;

    let llm = config
        .get("llm")
        .and_then(|l| l.as_str())
        .unwrap_or("claude");
    let project_name = config
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("unknown");

    // Read PROJECT_DESIGN.toml
    let design_path = project_root.join("PROJECT_DESIGN.toml");
    let design_content =
        fs::read_to_string(&design_path).context("Failed to read PROJECT_DESIGN.toml")?;

    // Initialize layer and gather all patterns
    let layer_path = project_root.join("layer");
    let layer = Layer::new(&layer_path);

    let all_patterns = layer
        .get_all_patterns()
        .context("Failed to retrieve patterns from layer")?;

    // Generate context using adapter
    let environment = patina::Environment::detect()?;
    let adapter = patina::adapters::get_adapter(llm);
    adapter.generate_context(
        &project_root,
        project_name,
        &design_content,
        &all_patterns,
        &environment,
    )?;

    println!("✨ Context generated successfully!");

    Ok(())
}
