use anyhow::{Context, Result};
use patina::brain::Brain;
use patina::environment::Environment;
use patina::session::SessionManager;
use std::fs;

pub fn execute() -> Result<()> {
    // Find project root
    let project_root = SessionManager::find_project_root()
        .context("Not in a Patina project directory. Run 'patina init' first.")?;
    
    println!("🔄 Updating CLAUDE.md with latest context...");
    
    // Read project config
    let config_path = project_root.join(".patina").join("config.json");
    let config_content = fs::read_to_string(&config_path)
        .context("Failed to read project config")?;
    let config: serde_json::Value = serde_json::from_str(&config_content)?;
    
    let llm = config.get("llm")
        .and_then(|l| l.as_str())
        .unwrap_or("claude");
    
    let project_name = config.get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("Unknown Project");
    
    // Read PROJECT_DESIGN.toml
    let design_path = project_root.join("PROJECT_DESIGN.toml");
    let design_content = fs::read_to_string(&design_path)
        .context("Failed to read PROJECT_DESIGN.toml")?;
    let design_toml: toml::Value = toml::from_str(&design_content)?;
    
    // Get brain patterns
    let brain_path = project_root.join("brain");
    let brain = Brain::new(&brain_path);
    let patterns = brain.get_all_patterns()
        .unwrap_or_default();
    
    // Detect environment
    let environment = Environment::detect()?;
    
    // Update using adapter
    let adapter = patina::adapters::get_adapter(llm);
    adapter.update_context(&project_root, project_name, &design_toml, &patterns, &environment)?;
    
    println!("✨ Context updated successfully at: {}", adapter.get_context_file_path(&project_root).display());
    
    Ok(())
}

