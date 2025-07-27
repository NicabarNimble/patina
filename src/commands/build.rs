use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn execute() -> Result<()> {
    // Check if we have a Dagger pipeline
    if Path::new("pipelines/main.go").exists() {
        println!("🔧 Building with Dagger pipeline...");
        
        // Check if Go is available
        if which::which("go").is_ok() {
            let output = Command::new("go")
                .current_dir("pipelines")
                .args(&["run", ".", "build"])
                .status()
                .context("Failed to run Dagger pipeline")?;
            
            if output.success() {
                println!("✅ Build completed successfully with Dagger");
                return Ok(());
            } else {
                println!("⚠️  Dagger pipeline failed, falling back to Docker");
            }
        } else {
            println!("⚠️  Go not found, falling back to Docker build");
        }
    }
    
    // Fallback to Docker
    if !Path::new("Dockerfile").exists() {
        anyhow::bail!("No Dockerfile found in current directory");
    }
    
    println!("🐳 Building with Docker...");
    
    // Get project name from config
    let config_path = Path::new(".patina/config.json");
    let project_name = if config_path.exists() {
        let config_content = fs::read_to_string(config_path)?;
        let config: serde_json::Value = serde_json::from_str(&config_content)?;
        config.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("app")
            .to_string()
    } else {
        "app".to_string()
    };
    
    let output = Command::new("docker")
        .args(&["build", "-t", &format!("{}:latest", project_name), "."])
        .status()
        .context("Failed to run docker build")?;
    
    if output.success() {
        println!("✅ Successfully built {}:latest", project_name);
    } else {
        anyhow::bail!("Docker build failed");
    }
    
    Ok(())
}