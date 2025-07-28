use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::env;
use toml::Value;

use patina::layer::{Layer, Pattern, PatternType};
use patina::version::VersionManifest;

pub fn execute(name: String, llm: String, design: String, dev: Option<String>) -> Result<()> {
    println!("🎨 Initializing Patina project: {}", name);
    
    // 1. Detect environment first
    println!("🔍 Detecting environment...");
    let environment = patina::Environment::detect()?;
    
    // Determine development environment if not specified
    let dev = dev.unwrap_or_else(|| determine_dev_environment(&environment));
    
    // Display key environment info
    println!("  ✓ OS: {} ({})", environment.os, environment.arch);
    
    // Show detected tools
    for (tool, info) in &environment.tools {
        if info.available {
            println!("  ✓ {}: {}", tool, info.version.as_ref().unwrap_or(&"detected".to_string()));
        }
    }
    
    // 2. Read and parse design.toml
    let design_content = fs::read_to_string(&design)
        .with_context(|| format!("Failed to read design file: {}", design))?;
    
    let design_toml: Value = toml::from_str(&design_content)
        .with_context(|| format!("Failed to parse design file: {}", design))?;
    
    // 3. Validate environment against requirements
    if let Some(validation_warnings) = validate_environment(&environment, &design_toml)? {
        if !validation_warnings.is_empty() {
            println!("\n📋 Validation warnings:");
            for warning in &validation_warnings {
                println!("  {}", warning);
            }
            
            // Ask user if they want to continue
            print!("\nContinue anyway? [y/N] ");
            use std::io::{self, Write};
            io::stdout().flush()?;
            let mut response = String::new();
            io::stdin().read_line(&mut response)?;
            
            if !response.trim().eq_ignore_ascii_case("y") {
                println!("Initialization cancelled.");
                return Ok(());
            }
        }
    }
    
    // 4. Create project directory
    let project_path = PathBuf::from(&name);
    if project_path.exists() {
        anyhow::bail!("Project directory already exists: {}", name);
    }
    
    fs::create_dir_all(&project_path)
        .with_context(|| format!("Failed to create project directory: {}", name))?;
    
    // 5. Copy PROJECT_DESIGN.toml to project
    let project_design_path = project_path.join("PROJECT_DESIGN.toml");
    fs::copy(&design, &project_design_path)
        .with_context(|| "Failed to copy PROJECT_DESIGN.toml")?;
    
    // 6. Set up layer directories
    let layer_path = project_path.join("layer");
    let layer = Layer::new(&layer_path);
    layer.init()
        .with_context(|| "Failed to initialize layer structure")?;
    
    println!("  ✓ Created layer structure");
    
    // 7. Create .patina directory for session state
    let patina_dir = project_path.join(".patina");
    fs::create_dir_all(&patina_dir)
        .with_context(|| "Failed to create .patina directory")?;
    
    // Store current project configuration with environment snapshot
    let config = serde_json::json!({
        "name": name,
        "llm": llm,
        "dev": dev,
        "created": chrono::Utc::now().to_rfc3339(),
        "environment_snapshot": {
            "os": environment.os,
            "arch": environment.arch,
            "detected_tools": environment.tools.iter()
                .filter(|(_, info)| info.available)
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
        }
    });
    
    let config_path = patina_dir.join("config.json");
    fs::write(&config_path, serde_json::to_string_pretty(&config)?)
        .with_context(|| "Failed to write project config")?;
    
    // Create version manifest
    let version_manifest = VersionManifest::new();
    version_manifest.save(&project_path)
        .with_context(|| "Failed to create version manifest")?;
    println!("  ✓ Created version manifest");
    
    // 8. Create LLM-specific files using adapter (now with environment)
    let adapter = patina::adapters::get_adapter(&llm);
    adapter.init_project(&project_path, &design_toml, &environment)?;
    println!("  ✓ Created {} integration files", adapter.name());
    
    // 9. Create development environment files using modular dev environments
    let project_type = design_toml.get("project")
        .and_then(|p| p.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("app");
        
    let dev_environment = patina::dev_env::get_dev_env(&dev);
    dev_environment.init_project(&project_path, &name, project_type)?;
    println!("  ✓ Created {} environment files", dev_environment.name());
    
    // Create dev manifest for config
    let dev_manifest = serde_json::json!({
        "environment": dev_environment.name(),
        "version": dev_environment.version(),
        "available": dev_environment.is_available(),
    });
    
    // Update config with dev manifest
    let mut config = serde_json::from_str::<serde_json::Value>(
        &fs::read_to_string(&config_path)?
    )?;
    config["dev_manifest"] = dev_manifest;
    fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    
    // 10. Initialize with core patterns from Patina's layer
    if let Ok(patina_exe_path) = std::env::current_exe() {
        if let Some(patina_root) = patina_exe_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            let source_layer_path = patina_root.join("layer");
            if source_layer_path.exists() {
                copy_core_patterns(&source_layer_path, &layer_path)?;
                println!("  ✓ Copied core patterns from Patina");
            }
        }
    }
    
    // 11. Create initial project pattern
    let project_pattern = Pattern {
        name: "initialization".to_string(),
        pattern_type: PatternType::Project(name.clone()),
        content: format!(
            "# {} Initialization\n\n\
            Initialized on: {}\n\
            LLM: {}\n\
            Dev Environment: {}\n\n\
            ## Design Source\n\
            {}\n",
            name,
            chrono::Utc::now().to_rfc3339(),
            llm,
            dev,
            design_content
        ),
    };
    
    layer.store_pattern(&project_pattern)?;
    
    // 12. Call adapter post_init for any additional setup
    adapter.post_init(&project_path, &design_toml, &dev)?;
    
    println!("\n✨ Project '{}' initialized successfully!", name);
    println!("\nNext steps:");
    println!("  1. cd {}", name);
    println!("  2. patina add <type> <name>  # Add patterns to session");
    println!("  3. patina commit             # Commit patterns to layer");
    println!("  4. patina push               # Generate LLM context");
    
    Ok(())
}

fn copy_core_patterns(source_layer: &Path, target_layer: &Path) -> Result<()> {
    let source_core = source_layer.join("core");
    let target_core = target_layer.join("core");
    
    if source_core.exists() {
        fs::create_dir_all(&target_core)?;
        
        for entry in fs::read_dir(&source_core)? {
            let entry = entry?;
            let source_file = entry.path();
            if source_file.is_file() && source_file.extension().map_or(false, |ext| ext == "md") {
                let file_name = source_file.file_name().unwrap();
                let target_file = target_core.join(file_name);
                fs::copy(&source_file, &target_file)?;
            }
        }
    }
    
    Ok(())
}

fn validate_environment(env: &patina::Environment, design: &Value) -> Result<Option<Vec<String>>> {
    let mut warnings = Vec::new();
    
    // Check language requirement
    if let Some(technical) = design.get("technical") {
        if let Some(language) = technical.get("language").and_then(|v| v.as_str()) {
            match language {
                "rust" => {
                    if !env.languages.get("rust").map_or(false, |info| info.available) {
                        warnings.push(format!("⚠️  Required language '{}' not detected", language));
                    }
                }
                _ => {
                    warnings.push(format!("💡 Note: Patina is designed for Rust development (specified: {})", language));
                }
            }
        }
        
        // Check for database requirements
        if let Some(database) = technical.get("database").and_then(|v| v.as_str()) {
            match database {
                "postgres" | "postgresql" => {
                    if !env.tools.get("psql").map_or(false, |info| info.available) {
                        warnings.push("💡 PostgreSQL client not found - you may need it for database operations".to_string());
                    }
                }
                _ => {}
            }
        }
    }
    
    // Check development environment requirements
    if let Some(dev) = design.get("development") {
        if let Some(env_section) = dev.get("environment") {
            // Check required tools
            if let Some(required) = env_section.get("required_tools").and_then(|v| v.as_array()) {
                for tool in required {
                    if let Some(tool_name) = tool.as_str() {
                        if !env.tools.get(tool_name).map_or(false, |info| info.available) {
                            warnings.push(format!("⚠️  Required tool '{}' not found", tool_name));
                        }
                    }
                }
            }
            
            // Check recommended tools
            if let Some(recommended) = env_section.get("recommended_tools").and_then(|v| v.as_array()) {
                for tool in recommended {
                    if let Some(tool_name) = tool.as_str() {
                        if !env.tools.get(tool_name).map_or(false, |info| info.available) {
                            warnings.push(format!("💡 Recommended tool '{}' not found", tool_name));
                        }
                    }
                }
            }
        }
    }
    
    Ok(if warnings.is_empty() { None } else { Some(warnings) })
}

#[derive(Debug, PartialEq)]
enum EnvironmentMode {
    Interactive,
    CI,
    Headless,
}

fn detect_environment_mode() -> EnvironmentMode {
    // Explicit overrides first
    if env::var("PATINA_NONINTERACTIVE").is_ok() {
        return EnvironmentMode::Headless;
    }
    
    // Common CI environments
    if env::var("CI").is_ok() || 
       env::var("GITHUB_ACTIONS").is_ok() ||
       env::var("GITLAB_CI").is_ok() ||
       env::var("JENKINS_URL").is_ok() ||
       env::var("BUILDKITE").is_ok() ||
       env::var("CIRCLECI").is_ok() {
        return EnvironmentMode::CI;
    }
    
    // Default to interactive
    EnvironmentMode::Interactive
}

fn determine_dev_environment(environment: &patina::Environment) -> String {
    let mode = detect_environment_mode();
    
    // Check explicit override first
    if let Ok(dev) = env::var("PATINA_DEV") {
        eprintln!("📦 Using development environment from PATINA_DEV: {}", dev);
        return dev;
    }
    
    // In CI/headless mode, default to Docker for predictability
    if mode == EnvironmentMode::CI || mode == EnvironmentMode::Headless {
        eprintln!("🤖 Headless mode detected: defaulting to Docker");
        eprintln!("   Set PATINA_DEV=dagger to use Dagger in CI");
        return "docker".to_string();
    }
    
    // Interactive mode: smart detection (best to worst)
    let has_docker = environment.tools.get("docker")
        .map(|t| t.available)
        .unwrap_or(false);
    
    let has_go = environment.languages.get("go")
        .map(|l| l.available)
        .unwrap_or(false);
    
    if has_docker && has_go {
        println!("📦 Using Dagger for development (fastest, cached builds)");
        println!("   Detected: Docker ✓ Go ✓");
        "dagger".to_string()
    } else if has_docker {
        println!("📦 Using Docker for development");
        println!("   💡 Install Go to unlock Dagger's fast, cached builds");
        "docker".to_string()
    } else {
        println!("📦 Using native builds (no containerization)");
        println!("   ⚠️  Install Docker for reproducible builds");
        "native".to_string()
    }
}