use anyhow::{Context, Result};
use std::fs;
use std::process::Command;
use std::path::PathBuf;

pub fn execute() -> Result<()> {
    // Find project root
    let project_root = find_project_root()?;
    
    // Read project config
    let config_path = project_root.join(".patina").join("config.json");
    let config_content = fs::read_to_string(&config_path)
        .context("Failed to read project config")?;
    let config: serde_json::Value = serde_json::from_str(&config_content)?;
    
    // Get dev manifest
    let dev_manifest = config.get("dev_manifest")
        .ok_or_else(|| anyhow::anyhow!("No dev_manifest found. Run 'patina update' to refresh."))?;
    
    // Get test command
    let test_command = dev_manifest.get("test_command")
        .and_then(|cmd| cmd.as_str())
        .ok_or_else(|| anyhow::anyhow!("No test_command found in manifest"))?;
    
    // Get requirements and check them
    if let Some(requirements) = dev_manifest.get("requirements").and_then(|r| r.as_object()) {
        for (tool, version) in requirements {
            match tool.as_str() {
                "go" => {
                    if which::which("go").is_err() {
                        anyhow::bail!("Go {} is required but not found in PATH", version);
                    }
                }
                "docker" => {
                    if which::which("docker").is_err() {
                        anyhow::bail!("Docker {} is required but not found in PATH", version);
                    }
                }
                "cargo" => {
                    if which::which("cargo").is_err() {
                        anyhow::bail!("Cargo {} is required but not found in PATH", version);
                    }
                }
                _ => {
                    // Unknown requirement, just warn
                    println!("⚠️  Unknown requirement: {} {}", tool, version);
                }
            }
        }
    }
    
    // Execute test command
    println!("🧪 Running tests: {}", test_command);
    println!();
    
    let output = Command::new("sh")
        .arg("-c")
        .arg(test_command)
        .current_dir(&project_root)
        .status()
        .context("Failed to execute test command")?;
    
    if !output.success() {
        anyhow::bail!("Tests failed");
    }
    
    println!();
    println!("✅ Tests completed successfully");
    
    Ok(())
}

fn find_project_root() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;
    let mut path = current_dir.as_path();
    
    loop {
        if path.join(".patina").exists() {
            return Ok(path.to_path_buf());
        }
        
        match path.parent() {
            Some(parent) => path = parent,
            None => anyhow::bail!("Not in a Patina project (no .patina directory found)"),
        }
    }
}