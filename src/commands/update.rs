use anyhow::{Context, Result};
use patina::session::SessionManager;
use patina::version::{VersionManifest, UpdateChecker};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use serde::{Serialize, Deserialize};
use toml::Value;

#[derive(Serialize, Deserialize)]
struct UpdateResult {
    patina_version: String,
    components: Vec<ComponentUpdate>,
    updates_available: bool,
    updates_applied: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct ComponentUpdate {
    name: String,
    current_version: String,
    available_version: String,
    updated: bool,
}

pub fn execute(
    check_only: bool,
    auto_yes: bool,
    auto_no: bool,
    json_output: bool,
    llm: Option<String>,
    dev: Option<String>,
) -> Result<i32> {
    // Find project root
    let project_root = SessionManager::find_project_root()
        .context("Not in a Patina project directory. Run 'patina init' first.")?;
    
    // If LLM or dev environment change requested, handle that separately
    if llm.is_some() || dev.is_some() {
        return handle_config_updates(&project_root, llm, dev, check_only, json_output);
    }
    
    // Check for non-interactive mode via environment variable
    let non_interactive = auto_yes || auto_no || json_output ||
        std::env::var("PATINA_NONINTERACTIVE").is_ok();
    
    if !json_output {
        println!("🔍 Checking for updates...");
    }
    
    // Load version manifest
    let mut manifest = VersionManifest::load(&project_root)?;
    let updates = UpdateChecker::check_for_updates(&manifest);
    
    // Read project config for LLM info
    let config_path = project_root.join(".patina").join("config.json");
    let config_content = fs::read_to_string(&config_path)
        .context("Failed to read project config")?;
    let config: serde_json::Value = serde_json::from_str(&config_content)?;
    
    let llm = config.get("llm")
        .and_then(|l| l.as_str())
        .unwrap_or("claude");
    
    let mut result = UpdateResult {
        patina_version: manifest.patina.clone(),
        components: vec![],
        updates_available: !updates.is_empty(),
        updates_applied: vec![],
    };
    
    // Check current Patina version against Cargo version
    let current_patina = env!("CARGO_PKG_VERSION");
    if !json_output && manifest.patina != current_patina {
        println!("⚠️  Patina version mismatch:");
        println!("   Project expects: v{}", manifest.patina);
        println!("   Installed: v{}", current_patina);
        println!("   Run: cargo install patina --version {}", manifest.patina);
        println!();
    }
    
    if updates.is_empty() {
        if !json_output {
            println!("✓ All components are up to date");
        }
        if json_output {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        return Ok(0);
    }
    
    // Build component updates list
    for (component, current, available) in &updates {
        result.components.push(ComponentUpdate {
            name: component.clone(),
            current_version: current.clone(),
            available_version: available.clone(),
            updated: false,
        });
        
        if !json_output {
            println!("📦 {} update available: {} → {}", component, current, available);
        }
    }
    
    // Show what's new for specific components
    if !json_output && !check_only {
        for (component, current_version, new_version) in &updates {
            match component.as_str() {
                "claude-adapter" if llm == "claude" => {
                    let adapter = patina::adapters::get_adapter(llm);
                    let changelog = adapter.get_changelog_since(current_version);
                    if !changelog.is_empty() {
                        println!("\nWhat's new since claude-adapter {}:", current_version);
                        for change in changelog {
                            println!("{}", change);
                        }
                    }
                }
                "gemini-adapter" if llm == "gemini" => {
                    println!("\nGemini adapter {} is now available", new_version);
                    println!("  - Initial release with basic GEMINI.md generation");
                }
                _ => {}
            }
        }
    }
    
    if check_only || auto_no {
        if json_output {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        return Ok(if check_only { 2 } else { 0 });
    }
    
    // Determine whether to update
    let should_update = if non_interactive {
        auto_yes || std::env::var("PATINA_AUTO_APPROVE").is_ok()
    } else {
        // Interactive prompt
        print!("\nUpdate components? [Y/n] ");
        io::stdout().flush()?;
        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        response.trim().is_empty() || response.trim().eq_ignore_ascii_case("y")
    };
    
    if should_update {
        // Update each component
        for (component, _, new_version) in &updates {
            if !json_output {
                print!("  Updating {}... ", component);
                io::stdout().flush()?;
            }
            
            // Component-specific update logic
            match component.as_str() {
                "claude-adapter" if llm == "claude" => {
                    let adapter = patina::adapters::get_adapter(llm);
                    adapter.update_adapter_files(&project_root)?;
                    manifest.update_component_version(component, new_version);
                    result.updates_applied.push(component.clone());
                    
                    // Mark component as updated in results
                    if let Some(comp) = result.components.iter_mut().find(|c| c.name == *component) {
                        comp.updated = true;
                    }
                    
                    if !json_output {
                        println!("✓");
                    }
                }
                "dagger-templates" => {
                    // Future: Update Dagger templates
                    if !json_output {
                        println!("(not implemented yet)");
                    }
                }
                _ => {
                    if !json_output {
                        println!("(unknown component)");
                    }
                }
            }
        }
        
        // Save updated manifest
        manifest.save(&project_root)?;
        
        if !json_output {
            println!("\n✨ Updates completed successfully!");
            if result.updates_applied.contains(&"claude-adapter".to_string()) {
                println!("\nNote: Use 'patina push' to regenerate CLAUDE.md");
            }
        }
    } else if !json_output {
        println!("Update cancelled.");
    }
    
    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    
    Ok(0)
}

fn handle_config_updates(
    project_root: &Path,
    llm: Option<String>,
    dev: Option<String>,
    check_only: bool,
    json_output: bool,
) -> Result<i32> {
    // Load current project config
    let config_path = project_root.join(".patina").join("config.json");
    let config_content = fs::read_to_string(&config_path)
        .context("Failed to read project config")?;
    let mut config: serde_json::Value = serde_json::from_str(&config_content)?;
    
    // Load PROJECT_DESIGN.toml for context
    let design_path = project_root.join("PROJECT_DESIGN.toml");
    let design_content = fs::read_to_string(&design_path)
        .context("Failed to read PROJECT_DESIGN.toml")?;
    let design_toml: Value = toml::from_str(&design_content)
        .context("Failed to parse PROJECT_DESIGN.toml")?;
    
    let mut changes_made = false;
    
    // Handle LLM change
    if let Some(ref new_llm) = llm {
        let current_llm = config.get("llm")
            .and_then(|l| l.as_str())
            .unwrap_or("claude");
        
        if !json_output {
            if current_llm == new_llm.as_str() {
                println!("✓ LLM adapter already set to {}", new_llm);
            } else {
                println!("🔄 Changing LLM adapter from {} to {}", current_llm, new_llm);
                
                if check_only {
                    println!("  Would:");
                    println!("  - Remove {} adapter files", current_llm);
                    println!("  - Create {} adapter files", new_llm);
                    println!("  - Update project configuration");
                } else {
                    // Get current environment for adapter initialization
                    let environment = patina::Environment::detect()?;
                    
                    // Initialize new adapter
                    let adapter = patina::adapters::get_adapter(&new_llm);
                    adapter.init_project(project_root, &design_toml, &environment)?;
                    println!("  ✓ Created {} adapter files", new_llm);
                    
                    // Update config
                    config["llm"] = serde_json::Value::String(new_llm.clone());
                    changes_made = true;
                }
            }
        }
    }
    
    // Handle dev environment change
    if let Some(ref new_dev) = dev {
        let current_dev = config.get("dev")
            .and_then(|d| d.as_str())
            .unwrap_or("docker")
            .to_string(); // Make owned copy to avoid borrow issues
        
        if !json_output {
            match new_dev.as_str() {
                "dagger" => {
                    if Path::new(project_root).join("pipelines").exists() {
                        println!("✓ Dagger pipeline already exists");
                    } else {
                        println!("➕ Adding Dagger pipeline support");
                        
                        if check_only {
                            println!("  Would:");
                            println!("  - Create pipelines/ directory");
                            println!("  - Generate main.go and go.mod from templates");
                            println!("  - Create Dockerfile if missing");
                            println!("  - Update project configuration");
                        } else {
                            // Extract project name from config
                            let project_name = config.get("name")
                                .and_then(|n| n.as_str())
                                .ok_or_else(|| anyhow::anyhow!("Project name not found in config"))?;
                            
                            // Use the existing create_dagger_files function from init
                            let manifest = create_dagger_files(project_root, project_name, &design_toml)?;
                            println!("  ✓ Created Dagger pipeline files");
                            
                            // Update config with dev and manifest
                            config["dev"] = serde_json::Value::String("dagger".to_string());
                            config["dev_manifest"] = manifest;
                            changes_made = true;
                        }
                    }
                }
                "docker" => {
                    if !Path::new(project_root).join("Dockerfile").exists() {
                        println!("➕ Adding Docker support");
                        
                        if check_only {
                            println!("  Would:");
                            println!("  - Create Dockerfile");
                            println!("  - Update project configuration");
                        } else {
                            let manifest = create_docker_files(project_root, &design_toml)?;
                            println!("  ✓ Created Docker files");
                            
                            config["dev"] = serde_json::Value::String("docker".to_string());
                            config["dev_manifest"] = manifest;
                            changes_made = true;
                        }
                    } else {
                        println!("✓ Docker support already configured");
                    }
                    
                    // Update config if different from current
                    if current_dev != "docker" && !check_only {
                        config["dev"] = serde_json::Value::String("docker".to_string());
                        let manifest = serde_json::json!({
                            "test_command": "docker run --rm -v $(pwd):/workspace -w /workspace rust:latest cargo test --workspace",
                            "build_command": "docker build -t $(basename $(pwd)):latest .",
                            "files_created": vec!["Dockerfile"],
                            "requirements": {
                                "docker": "20.10+"
                            }
                        });
                        config["dev_manifest"] = manifest;
                        changes_made = true;
                    }
                }
                "native" => {
                    println!("☑ Switching to native development environment");
                    
                    if check_only {
                        println!("  Would:");
                        println!("  - Remove container files");
                        println!("  - Use cargo directly for builds and tests");
                    } else {
                        config["dev"] = serde_json::Value::String("native".to_string());
                        let manifest = serde_json::json!({
                            "test_command": "cargo test --workspace",
                            "build_command": "cargo build --release",
                            "files_created": [],
                            "requirements": {
                                "cargo": "1.70+"
                            }
                        });
                        config["dev_manifest"] = manifest;
                        changes_made = true;
                    }
                }
                _ => {
                    anyhow::bail!("Unknown development environment: {}. Supported: docker, dagger, native", new_dev);
                }
            }
        }
    }
    
    // Save updated config if not in check mode
    if changes_made && !check_only {
        fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
        
        if !json_output {
            println!("\n✨ Project configuration updated successfully!");
            
            // Provide next steps based on what was changed
            if llm.is_some() {
                println!("\nNext steps:");
                println!("  - Run 'patina push' to generate context for the new LLM adapter");
            }
            
            if dev.is_some() && dev.as_deref() == Some("dagger") {
                println!("\nTry the new Dagger pipeline:");
                println!("  - 'patina build' to build with Dagger");
                println!("  - 'patina agent test' to run tests in container");
            }
        }
    }
    
    if json_output {
        let result = serde_json::json!({
            "config_updated": changes_made,
            "llm": llm,
            "dev": dev,
            "check_only": check_only
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    
    Ok(0)
}

// Import the create_dagger_files function from init.rs
// For now, we'll duplicate it here, but in a real implementation
// we'd extract it to a shared module
fn create_dagger_files(project_path: &Path, name: &str, design: &Value) -> Result<serde_json::Value> {
    
    
    // Always create basic Dockerfile as fallback
    let _ = create_docker_files(project_path, design)?;
    
    // Create pipelines directory
    let pipelines_dir = project_path.join("pipelines");
    fs::create_dir_all(&pipelines_dir)?;
    
    // Copy Go module file (no templating needed - it's static)
    let go_mod_content = include_str!("../../resources/templates/dagger/go.mod.tmpl");
    fs::write(pipelines_dir.join("go.mod"), go_mod_content)?;
    
    // Copy main.go file (no templating needed - it's generic)
    let main_go_content = include_str!("../../resources/templates/dagger/main.go.tmpl");
    fs::write(pipelines_dir.join("main.go"), main_go_content)?;
    
    // Create a simple README for the pipelines
    let readme_content = format!(r#"# {} Dagger Pipelines

This directory contains Dagger pipelines for building and testing the project.

## Usage

```bash
# Run the build pipeline
go run . build

# Run tests in container
go run . test

# Execute arbitrary commands
go run . exec cargo --version
```

## Requirements

- Go 1.21+
- Docker daemon running
- Dagger CLI (optional but recommended)
"#, name);
    
    fs::write(pipelines_dir.join("README.md"), readme_content)?;
    
    Ok(serde_json::json!({
        "test_command": "cd pipelines && go run . test",
        "build_command": "cd pipelines && go run . build",
        "files_created": vec!["pipelines/main.go", "pipelines/go.mod", "pipelines/README.md", "Dockerfile"],
        "requirements": {
            "go": "1.21+",
            "docker": "20.10+"
        }
    }))
}

fn create_docker_files(project_path: &Path, _design: &Value) -> Result<serde_json::Value> {
    // Create basic Dockerfile if it doesn't exist
    if !project_path.join("Dockerfile").exists() {
        let dockerfile_content = r#"FROM rust:latest

WORKDIR /app

COPY . .

RUN cargo build --release

CMD ["cargo", "run"]
"#;
        
        fs::write(project_path.join("Dockerfile"), dockerfile_content)?;
    }
    
    Ok(serde_json::json!({
        "test_command": "docker run --rm -v $(pwd):/workspace -w /workspace rust:latest cargo test --workspace",
        "build_command": "docker build -t $(basename $(pwd)):latest .",
        "files_created": vec!["Dockerfile"],
        "requirements": {
            "docker": "20.10+"
        }
    }))
}