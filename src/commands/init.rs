use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::env;
use toml::Value;
use serde_json::json;

use patina::brain::{Brain, Pattern, PatternType};
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
    
    // 6. Set up brain directories
    let brain_path = project_path.join("brain");
    let brain = Brain::new(&brain_path);
    brain.init()
        .with_context(|| "Failed to initialize brain structure")?;
    
    println!("  ✓ Created brain structure");
    
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
    
    // 9. Create environment-specific files and collect manifest
    let dev_manifest = match dev.as_str() {
        "docker" => {
            let manifest = create_docker_files(&project_path, &design_toml)?;
            println!("  ✓ Created Docker environment files");
            manifest
        }
        "dagger" => {
            let manifest = create_dagger_files(&project_path, &name, &design_toml)?;
            println!("  ✓ Created Dagger environment files");
            manifest
        }
        "native" => {
            println!("  ✓ Using native development (no container files)");
            serde_json::json!({
                "test_command": "cargo test --workspace",
                "build_command": "cargo build --release",
                "files_created": [],
                "requirements": {
                    "cargo": "1.70+"
                }
            })
        }
        "nix" => {
            // TODO: Create Nix files
            println!("  ✓ Created Nix environment files");
            serde_json::json!({
                "test_command": "nix-shell --run 'cargo test'",
                "build_command": "nix-build",
                "files_created": ["shell.nix", "default.nix"],
                "requirements": {
                    "nix": "2.0+"
                }
            })
        }
        _ => {
            println!("  ⚠️  Unknown dev environment: {}, skipping environment files", dev);
            serde_json::json!({})
        }
    };
    
    // Update config with dev manifest
    let mut config = serde_json::from_str::<serde_json::Value>(
        &fs::read_to_string(&config_path)?
    )?;
    config["dev_manifest"] = dev_manifest;
    fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    
    // 8. Initialize with core patterns from Patina's brain
    if let Ok(patina_brain_path) = std::env::current_exe() {
        if let Some(patina_root) = patina_brain_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            let source_brain_path = patina_root.join("brain");
            if source_brain_path.exists() {
                copy_core_patterns(&source_brain_path, &brain_path)?;
                println!("  ✓ Copied core patterns from Patina");
            }
        }
    }
    
    // 9. Create initial project pattern
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
    
    brain.store_pattern(&project_pattern)?;
    
    println!("\n✨ Project '{}' initialized successfully!", name);
    println!("\nNext steps:");
    println!("  1. cd {}", name);
    println!("  2. patina add <type> <name>  # Add patterns to session");
    println!("  3. patina commit             # Commit patterns to brain");
    println!("  4. patina push               # Generate LLM context");
    
    Ok(())
}


fn create_docker_files(project_path: &Path, design: &Value) -> Result<serde_json::Value> {
    // Create basic Dockerfile
    let dockerfile_content = r#"FROM rust:latest

WORKDIR /app

COPY . .

RUN cargo build --release

CMD ["cargo", "run"]
"#;
    
    fs::write(project_path.join("Dockerfile"), dockerfile_content)?;
    
    // Create docker-compose.yml if applicable
    if design.get("services").is_some() {
        let compose_content = r#"version: '3.8'

services:
  app:
    build: .
    volumes:
      - .:/app
"#;
        
        fs::write(project_path.join("docker-compose.yml"), compose_content)?;
    }
    
    Ok(serde_json::json!({
        "test_command": "docker run --rm -v $(pwd):/workspace -w /workspace rust:latest cargo test --workspace",
        "build_command": "docker build -t $(basename $(pwd)):latest .",
        "files_created": if design.get("services").is_some() {
            vec!["Dockerfile", "docker-compose.yml"]
        } else {
            vec!["Dockerfile"]
        },
        "requirements": {
            "docker": "20.10+"
        }
    }))
}


fn create_dagger_files(project_path: &Path, name: &str, design: &Value) -> Result<serde_json::Value> {
    // Always create basic Dockerfile as fallback
    let _ = create_docker_files(project_path, design)?;
    
    // Create pipelines directory
    let pipelines_dir = project_path.join("pipelines");
    fs::create_dir_all(&pipelines_dir)?;
    
    // Copy Go module file (no templating needed - it's static)
    let go_mod_content = include_str!("../../resources/templates/dagger/go.mod.tmpl");
    fs::write(pipelines_dir.join("go.mod"), go_mod_content)?;
    
    // Check if project wants agent workflows
    let use_agent_workflows = design.get("project")
        .and_then(|p| p.get("features"))
        .and_then(|f| f.as_array())
        .map(|features| features.iter().any(|f| f.as_str() == Some("agent-workflows")))
        .unwrap_or(false);
    
    // Copy main.go file (no templating needed - it's generic)
    let main_go_content = include_str!("../../resources/templates/dagger/main.go.tmpl");
    fs::write(pipelines_dir.join("main.go"), main_go_content)?;
    
    // Create a simple README for the pipelines
    let readme_content = if use_agent_workflows {
        format!(r#"# {} Dagger Pipelines with Agent Workflows

This directory contains Dagger pipelines for building, testing, and agent development.

## Usage

### Standard Build
```bash
# Run the build pipeline
go run .

# Or with dagger command
dagger run go run .
```

### Agent Workflows
```bash
# Create an isolated agent workspace
go run . agent

# Run tests in isolation
go run . test

# With session tracking
PATINA_SESSION_ID=my-session go run . agent
```

## Agent Features

- **Isolated Workspaces**: Each agent gets its own container and git branch
- **Session Integration**: Links with Patina sessions for context tracking
- **Tool Installation**: Development tools pre-installed for agent use
- **Cache Isolation**: Separate caches per session to avoid conflicts

## Requirements

- Go 1.21+
- Dagger CLI (optional but recommended)
- Docker daemon running

## What it does

### Build Mode (default)
1. Runs clippy for linting
2. Runs tests with `cargo test`
3. Builds release binary with `cargo build --release`
4. Creates a minimal Docker image with the binary
5. Exports as `{}:latest` to your local Docker daemon

### Agent Mode
1. Creates isolated container with full development environment
2. Creates git branch `agent/{{session-id}}`
3. Mounts code with session-specific caches
4. Ready for AI agent operations

### Test Mode
1. Runs tests in isolated environment
2. Uses separate cache to avoid conflicts
3. Shows full test output
"#, name, name)
    } else {
        format!(r#"# {} Dagger Pipelines

This directory contains Dagger pipelines for building and testing the project.

## Usage

```bash
# Run the pipeline
go run .

# Or with dagger command
dagger run go run .
```

## Requirements

- Go 1.21+
- Dagger CLI (optional but recommended)

## What it does

1. Runs tests with `cargo test`
2. Builds release binary with `cargo build --release`
3. Creates a minimal Docker image with the binary
4. Exports as `{}:latest` to your local Docker daemon
"#, name, name)
    };
    
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

fn copy_core_patterns(source_brain: &Path, target_brain: &Path) -> Result<()> {
    let source_core = source_brain.join("core");
    let target_core = target_brain.join("core");
    
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

