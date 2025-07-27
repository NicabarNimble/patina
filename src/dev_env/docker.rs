use super::DevEnvironment;
use anyhow::{Context, Result};
use std::path::Path;
use std::fs;
use std::process::Command;

pub const DOCKER_VERSION: &str = "0.1.0";

pub struct DockerEnvironment;

impl DevEnvironment for DockerEnvironment {
    fn name(&self) -> &'static str {
        "docker"
    }
    
    fn version(&self) -> &'static str {
        DOCKER_VERSION
    }
    
    fn init_project(&self, project_path: &Path, project_name: &str, project_type: &str) -> Result<()> {
        // Create appropriate Dockerfile based on project type
        let dockerfile_content = match project_type {
            "app" => include_str!("../../resources/templates/docker/Dockerfile.app.tmpl"),
            "tool" => include_str!("../../resources/templates/docker/Dockerfile.tool.tmpl"),
            _ => include_str!("../../resources/templates/docker/Dockerfile.app.tmpl"),
        };
        
        let dockerfile_content = dockerfile_content.replace("{{.name}}", project_name);
        fs::write(project_path.join("Dockerfile"), dockerfile_content)?;
        
        // Create docker-compose.yml for apps
        if project_type == "app" {
            let compose_content = include_str!("../../resources/templates/docker/docker-compose.tmpl")
                .replace("{{.name}}", project_name);
            fs::write(project_path.join("docker-compose.yml"), compose_content)?;
        }
        
        // Create .dockerignore
        let dockerignore = r#"target/
Dockerfile
.dockerignore
.git/
.gitignore
*.md
.patina/
.claude/
"#;
        fs::write(project_path.join(".dockerignore"), dockerignore)?;
        
        Ok(())
    }
    
    fn build(&self, project_path: &Path) -> Result<()> {
        if !self.is_available() {
            anyhow::bail!("Docker is not installed");
        }
        
        if !project_path.join("Dockerfile").exists() {
            anyhow::bail!("No Dockerfile found in current directory");
        }
        
        println!("🐳 Building with Docker...");
        
        // Get project name from config
        let config_path = project_path.join(".patina/config.json");
        let project_name = if config_path.exists() {
            let config_content = fs::read_to_string(&config_path)?;
            let config: serde_json::Value = serde_json::from_str(&config_content)?;
            config.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("app")
                .to_string()
        } else {
            "app".to_string()
        };
        
        let output = Command::new("docker")
            .current_dir(project_path)
            .args(&["build", "-t", &format!("{}:latest", project_name), "."])
            .status()
            .context("Failed to run docker build")?;
        
        if output.success() {
            println!("✅ Successfully built {}:latest", project_name);
            Ok(())
        } else {
            anyhow::bail!("Docker build failed")
        }
    }
    
    fn test(&self, project_path: &Path) -> Result<()> {
        // For now, run tests in Docker container
        self.build(project_path)?;
        
        println!("🧪 Running tests in Docker container...");
        
        let config_path = project_path.join(".patina/config.json");
        let project_name = if config_path.exists() {
            let config_content = fs::read_to_string(&config_path)?;
            let config: serde_json::Value = serde_json::from_str(&config_content)?;
            config.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("app")
                .to_string()
        } else {
            "app".to_string()
        };
        
        let output = Command::new("docker")
            .current_dir(project_path)
            .args(&["run", "--rm", &format!("{}:latest", project_name), "cargo", "test"])
            .status()
            .context("Failed to run tests in Docker")?;
        
        if output.success() {
            println!("✅ Tests passed");
            Ok(())
        } else {
            anyhow::bail!("Tests failed")
        }
    }
    
    fn is_available(&self) -> bool {
        which::which("docker").is_ok()
    }
}