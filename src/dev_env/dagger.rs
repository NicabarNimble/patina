use super::DevEnvironment;
use anyhow::{Context, Result};
use std::path::Path;
use std::fs;
use std::process::Command;

pub const DAGGER_VERSION: &str = "1.0.0";

pub struct DaggerEnvironment;

impl DevEnvironment for DaggerEnvironment {
    fn name(&self) -> &'static str {
        "dagger"
    }
    
    fn version(&self) -> &'static str {
        DAGGER_VERSION
    }
    
    fn init_project(&self, project_path: &Path, project_name: &str, project_type: &str) -> Result<()> {
        // Create pipelines directory
        let pipelines_dir = project_path.join("pipelines");
        fs::create_dir_all(&pipelines_dir)?;
        
        // Generate main.go from template
        let main_go_content = include_str!("../../resources/templates/dagger/main.go.tmpl")
            .replace("{{.name}}", project_name)
            .replace("{{.type}}", project_type);
        
        fs::write(pipelines_dir.join("main.go"), main_go_content)?;
        
        // Generate go.mod
        let go_mod_content = include_str!("../../resources/templates/dagger/go.mod.tmpl")
            .replace("{{.name}}", project_name);
            
        fs::write(pipelines_dir.join("go.mod"), go_mod_content)?;
        
        // Read config to get current LLM
        let config_path = project_path.join(".patina").join("config.json");
        let llm = if config_path.exists() {
            let config_content = fs::read_to_string(&config_path)?;
            let config: serde_json::Value = serde_json::from_str(&config_content)?;
            config.get("llm")
                .and_then(|l| l.as_str())
                .unwrap_or("claude")
                .to_string()
        } else {
            "claude".to_string()
        };
        
        // Copy constraints with LLM-specific filename
        let constraints_content = include_str!("../../resources/templates/dagger/CONSTRAINTS.md");
        let llm_filename = match llm.as_str() {
            "claude" => "CLAUDE.md",
            "gemini" => "GEMINI.md",
            _ => "CONSTRAINTS.md"
        };
        fs::write(pipelines_dir.join(llm_filename), constraints_content)?;
        
        Ok(())
    }
    
    fn build(&self, project_path: &Path) -> Result<()> {
        if !self.is_available() {
            anyhow::bail!("Dagger requires Go to be installed");
        }
        
        let pipelines_path = project_path.join("pipelines");
        if !pipelines_path.join("main.go").exists() {
            anyhow::bail!("No Dagger pipeline found");
        }
        
        println!("🔧 Building with Dagger pipeline...");
        
        let output = Command::new("go")
            .current_dir(&pipelines_path)
            .env("PATINA_PROJECT_ROOT", project_path)
            .args(&["run", ".", "build"])
            .status()
            .context("Failed to run Dagger pipeline")?;
        
        if output.success() {
            println!("✅ Build completed successfully with Dagger");
            Ok(())
        } else {
            anyhow::bail!("Dagger pipeline failed")
        }
    }
    
    fn test(&self, project_path: &Path) -> Result<()> {
        if !self.is_available() {
            anyhow::bail!("Dagger requires Go to be installed");
        }
        
        let pipelines_path = project_path.join("pipelines");
        if !pipelines_path.join("main.go").exists() {
            anyhow::bail!("No Dagger pipeline found");
        }
        
        println!("🧪 Testing with Dagger pipeline...");
        
        let output = Command::new("go")
            .current_dir(&pipelines_path)
            .env("PATINA_PROJECT_ROOT", project_path)
            .args(&["run", ".", "test"])
            .status()
            .context("Failed to run Dagger pipeline")?;
        
        if output.success() {
            println!("✅ Tests completed successfully with Dagger");
            Ok(())
        } else {
            anyhow::bail!("Dagger pipeline failed")
        }
    }
    
    fn is_available(&self) -> bool {
        which::which("go").is_ok()
    }
    
    fn fallback(&self) -> Option<&'static str> {
        Some("docker")
    }
}