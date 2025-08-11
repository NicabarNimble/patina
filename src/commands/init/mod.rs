pub mod design_wizard;
pub mod tool_installer;

use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use toml::Value;

use patina::layer::{Layer, Pattern, PatternType};
use patina::version::{UpdateChecker, VersionManifest};

use self::design_wizard::{confirm, create_project_design_wizard};
use self::tool_installer::{detect_missing_tools, get_available_tools, install_tools};

pub fn execute(name: String, llm: String, design: String, dev: Option<String>) -> Result<()> {
    // Check if we're in JSON output mode (for init command, always false)
    let json_output = false;
    // First, check if we're trying to create a nested project
    if name != "." && (Path::new(".patina").exists() || Path::new("PROJECT_DESIGN.toml").exists()) {
        println!("⚠️  You're already in a Patina project!");
        println!(
            "   Running 'patina init {}' would create: {}",
            name,
            env::current_dir()?.join(&name).display()
        );
        if !confirm("Continue anyway?")? {
            println!("Initialization cancelled.");
            return Ok(());
        }
    }

    // Check if we're re-initializing an existing Patina project
    let is_reinit = if name == "." {
        Path::new(".patina").exists() || Path::new("PROJECT_DESIGN.toml").exists()
    } else {
        let path = PathBuf::from(&name);
        path.exists()
            && (path.join(".patina").exists() || path.join("PROJECT_DESIGN.toml").exists())
    };

    if is_reinit {
        println!("🔄 Re-initializing Patina project...");
    } else {
        println!("🎨 Initializing Patina project: {name}");
    }

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
            println!(
                "  ✓ {}: {}",
                tool,
                info.version.as_ref().unwrap_or(&"detected".to_string())
            );
        }
    }

    // 2. Handle PROJECT_DESIGN.toml
    let design_path = PathBuf::from(&design);

    // If user specified a non-default design file, inform them it will be copied
    if design != "PROJECT_DESIGN.toml" && design_path.exists() {
        println!("📄 Using design file: {}", design_path.display());
        println!("   (Will be copied to project as PROJECT_DESIGN.toml)");
    }

    let design_content = if !design_path.exists() {
        println!("\n📋 No PROJECT_DESIGN.toml found.");

        if confirm("Create one interactively?")? {
            let content = create_project_design_wizard(&name, &environment)?;

            // Save it to the requested location
            fs::write(&design_path, &content).with_context(|| {
                format!(
                    "Failed to write PROJECT_DESIGN.toml to {}",
                    design_path.display()
                )
            })?;

            println!("✅ Created PROJECT_DESIGN.toml");
            content
        } else {
            println!("\nCannot initialize without PROJECT_DESIGN.toml");
            println!("You can:");
            println!("  • Run this command again and choose to create one");
            println!("  • Create one manually");
            println!("  • Copy from another project");
            return Ok(());
        }
    } else {
        fs::read_to_string(&design_path)
            .with_context(|| format!("Failed to read design file: {}", design_path.display()))?
    };

    // Parse the design content
    let design_toml: Value = toml::from_str(&design_content)
        .with_context(|| format!("Failed to parse design file: {design}"))?;

    // Validate TOML structure has minimum required fields
    validate_design_structure(&design_toml)?;

    // 3. Validate environment against requirements
    if let Some(validation_warnings) = validate_environment(&environment, &design_toml)? {
        if !validation_warnings.is_empty() {
            println!("\n📋 Validation warnings:");
            for warning in &validation_warnings {
                println!("  {warning}");
            }

            // Ask user if they want to continue
            if !confirm("\nContinue anyway?")? {
                println!("Initialization cancelled.");
                return Ok(());
            }
        }
    }

    // 3.5 Check for missing tools and offer to install
    let available_tools = get_available_tools();
    let missing_tools = detect_missing_tools(&available_tools);

    // Filter to only tools that make sense for the project type
    let project_type = design_toml
        .get("project")
        .and_then(|p| p.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("app");

    let recommended_missing: Vec<_> = missing_tools
        .into_iter()
        .filter(|tool| {
            match (project_type, tool.name) {
                // Apps may want containerization
                ("app", "docker") => true,
                ("app", "dagger") => &dev == "dagger",
                // All projects benefit from gh for PRs
                (_, "gh") => true,
                // jq is useful for automation
                (_, "jq") => true,
                // Go needed for dagger
                (_, "go") => &dev == "dagger",
                _ => false,
            }
        })
        .collect();

    if !recommended_missing.is_empty() {
        println!("\n🔧 Missing recommended tools:");
        for tool in &recommended_missing {
            println!("  • {}", tool.name);
        }

        if confirm("\nInstall missing tools?")? {
            install_tools(&recommended_missing)?;
            println!();
        }
    }

    // 4. Create or verify project directory
    let project_path = if name == "." {
        // Initialize in current directory
        env::current_dir().context("Failed to get current directory")?
    } else {
        // Create new directory or use existing for re-init
        let path = PathBuf::from(&name);
        if path.exists() {
            // Check if it's a re-init scenario
            let has_patina = path.join(".patina").exists();
            let has_design = path.join("PROJECT_DESIGN.toml").exists();

            if has_patina || has_design {
                println!("  ℹ️  Found existing project at: {}", path.display());
                if !confirm("Re-initialize this project?")? {
                    println!("Initialization cancelled.");
                    return Ok(());
                }
            } else {
                anyhow::bail!(
                    "Directory already exists but is not a Patina project: {}",
                    name
                );
            }
        } else {
            fs::create_dir_all(&path)
                .with_context(|| format!("Failed to create project directory: {name}"))?;
        }
        path
    };

    // 5. Handle PROJECT_DESIGN.toml
    let project_design_path = project_path.join("PROJECT_DESIGN.toml");

    // Copy design file if source and destination are different
    let source_canonical = fs::canonicalize(&design_path)?;
    let dest_canonical = fs::canonicalize(&project_design_path).ok();

    if dest_canonical.is_none() || source_canonical != dest_canonical.unwrap() {
        fs::copy(&design_path, &project_design_path)
            .with_context(|| "Failed to copy PROJECT_DESIGN.toml")?;
        println!("  ✓ Copied PROJECT_DESIGN.toml to project");
    }

    // 6. Set up layer directories
    let layer_path = project_path.join("layer");
    let layer = Layer::new(&layer_path);
    layer
        .init()
        .with_context(|| "Failed to initialize layer structure")?;

    println!("  ✓ Created layer structure");

    // 7. Create .patina directory for session state
    let patina_dir = project_path.join(".patina");
    fs::create_dir_all(&patina_dir).with_context(|| "Failed to create .patina directory")?;

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

    // Handle version manifest and check for updates in re-init scenarios
    let mut should_update_components = false;
    let mut component_updates = vec![];

    let mut version_manifest = if is_reinit && project_path.join(".patina/versions.json").exists() {
        // Load existing config to get current LLM for re-init scenarios
        let reinit_llm = if project_path.join(".patina/config.json").exists() {
            let config_content = fs::read_to_string(project_path.join(".patina/config.json"))?;
            let config: serde_json::Value = serde_json::from_str(&config_content)?;
            config
                .get("llm")
                .and_then(|l| l.as_str())
                .unwrap_or(&llm)
                .to_string()
        } else {
            llm.clone()
        };

        // Load existing manifest and check for updates
        println!("🔍 Checking for component updates...");
        let existing_manifest = VersionManifest::load(&project_path)?;
        let updates = UpdateChecker::check_for_updates(&existing_manifest);

        if !updates.is_empty() {
            println!("\n📦 Component updates available:");
            for (component, current, available) in &updates {
                println!("  • {component}: {current} → {available}");
            }

            // Show what's new in the updates
            for (component, current_version, _new_version) in &updates {
                match component.as_str() {
                    "claude-adapter" if reinit_llm == "claude" => {
                        let adapter = patina::adapters::get_adapter(&reinit_llm);
                        let changelog = adapter.get_changelog_since(current_version);
                        if !changelog.is_empty() {
                            println!("\n  What's new in Claude adapter:");
                            for change in changelog {
                                println!("    {change}");
                            }
                        }
                    }
                    _ => {}
                }
            }

            if confirm("\nUpdate components to latest versions?")? {
                should_update_components = true;
                component_updates = updates;
                println!("  ✓ Components will be updated");
            }
        }

        existing_manifest
    } else {
        // Create new manifest for first-time init
        let new_manifest = VersionManifest::new();
        new_manifest
            .save(&project_path)
            .with_context(|| "Failed to create version manifest")?;
        println!("  ✓ Created version manifest");
        new_manifest
    };

    // 8. Create LLM-specific files using adapter (now with environment)
    let adapter = patina::adapters::get_adapter(&llm);
    adapter.init_project(&project_path, &design_toml, &environment)?;
    println!("  ✓ Created {} integration files", adapter.name());

    // 9. Create development environment files using modular dev environments
    let project_type = design_toml
        .get("project")
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
    let mut config = serde_json::from_str::<serde_json::Value>(&fs::read_to_string(&config_path)?)?;
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

    // 13. Initialize navigation database
    let navigation_db_path = patina_dir.join("navigation.db");
    if !navigation_db_path.exists() {
        println!("🔍 Initializing navigation database...");

        // Create navigation database using HybridDatabase or SqliteClient
        let enable_crdt = std::env::var("PATINA_ENABLE_CRDT").is_ok();

        if enable_crdt {
            match patina::indexer::advanced::HybridDatabase::new(&navigation_db_path, true) {
                Ok(db) => {
                    db.initialize_schema()?;
                    println!("  ✓ Created navigation database with CRDT support");
                }
                Err(e) => {
                    eprintln!("  ⚠️  Could not create HybridDatabase: {e}");
                    eprintln!("     Falling back to regular SQLite...");

                    let db = patina::indexer::advanced::SqliteClient::new(&navigation_db_path)?;
                    db.initialize_schema()?;
                    println!("  ✓ Created navigation database (SQLite only)");
                }
            }
        } else {
            let db = patina::indexer::advanced::SqliteClient::new(&navigation_db_path)?;
            db.initialize_schema()?;
            println!("  ✓ Created navigation database");
        }
    } else if is_reinit {
        println!("🔍 Reindexing patterns for navigation...");
    }

    // Index initial patterns
    if layer_path.exists() {
        let enable_crdt = std::env::var("PATINA_ENABLE_CRDT").is_ok();
        let indexer = if enable_crdt {
            patina::indexer::PatternIndexer::with_hybrid_database(&navigation_db_path, true)
                .or_else(|_| patina::indexer::PatternIndexer::with_database(&navigation_db_path))?
        } else {
            patina::indexer::PatternIndexer::with_database(&navigation_db_path)?
        };

        print!("  Indexing patterns... ");
        std::io::stdout().flush()?;

        indexer.index_directory(&layer_path)?;

        // Query to see what was indexed
        let results = indexer.navigate("");
        let count = results.locations.len();
        println!("✓ ({count} patterns indexed)");

        // Show what was discovered
        if count > 0 && !json_output {
            println!("\n  Discovered patterns:");
            let mut shown = 0;
            for location in results.locations.iter().take(5) {
                let path_str = location.path.to_string_lossy();
                if let Some(pos) = path_str.rfind("layer/") {
                    println!("    • {}", &path_str[pos + 6..]);
                    shown += 1;
                }
            }
            if count > shown {
                println!("    ... and {} more", count - shown);
            }
        }
    }

    // 14. Perform component updates if requested during re-init
    if should_update_components && !component_updates.is_empty() {
        println!("\n🔄 Updating components...");

        for (component, _, new_version) in &component_updates {
            match component.as_str() {
                "claude-adapter" if llm == "claude" => {
                    print!("  Updating Claude adapter... ");
                    std::io::stdout().flush()?;

                    // Re-initialize adapter files with latest version
                    adapter.init_project(&project_path, &design_toml, &environment)?;
                    version_manifest.update_component_version(component, new_version);
                    println!("✓");
                }
                "gemini-adapter" if llm == "gemini" => {
                    print!("  Updating Gemini adapter... ");
                    std::io::stdout().flush()?;

                    // Re-initialize adapter files with latest version
                    adapter.init_project(&project_path, &design_toml, &environment)?;
                    version_manifest.update_component_version(component, new_version);
                    println!("✓");
                }
                "dagger-templates" => {
                    // Check if this project uses dagger
                    let config_content = fs::read_to_string(&config_path)?;
                    let config: serde_json::Value = serde_json::from_str(&config_content)?;
                    let project_dev = config
                        .get("dev")
                        .and_then(|d| d.as_str())
                        .unwrap_or("docker");

                    if project_dev == "dagger" {
                        print!("  Updating Dagger templates... ");
                        std::io::stdout().flush()?;

                        // Re-create dagger files with latest templates
                        let dev_environment = patina::dev_env::get_dev_env("dagger");
                        dev_environment.init_project(&project_path, &name, project_type)?;
                        version_manifest.update_component_version(component, new_version);
                        println!("✓");
                    } else {
                        println!("  Skipping {component} (project uses {project_dev})");
                    }
                }
                _ => {
                    println!("  Skipping {component} (not applicable to current config)");
                }
            }
        }

        // Save updated manifest
        version_manifest.save(&project_path)?;
        println!("\n✅ All components updated successfully!");
    }

    println!("\n✨ Project '{name}' initialized successfully!");

    if name != "." {
        println!("\nNext steps:");
        println!("  1. cd {name}");
        println!("  2. patina add <type> <name>  # Add patterns to session");
        println!("  3. patina commit             # Commit patterns to layer");
        println!("  4. patina push               # Generate LLM context");
    } else {
        println!("\nNext steps:");
        println!("  1. patina add <type> <name>  # Add patterns to session");
        println!("  2. patina commit             # Commit patterns to layer");
        println!("  3. patina push               # Generate LLM context");
    }

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
            if source_file.is_file() && source_file.extension().is_some_and(|ext| ext == "md") {
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

    // For Patina projects, we always want Rust
    let _project_type = design
        .get("project")
        .and_then(|p| p.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("tool");

    // All Patina project types benefit from Rust
    if !env.languages.get("rust").is_some_and(|info| info.available) {
        warnings.push(
            "⚠️  Rust not detected - Patina is built for Rust projects (install via rustup)"
                .to_string(),
        );
    }

    // Check development environment requirements
    if let Some(dev) = design.get("development") {
        if let Some(env_section) = dev.get("environment") {
            // Check required tools
            if let Some(required) = env_section.get("required_tools").and_then(|v| v.as_array()) {
                for tool in required {
                    if let Some(tool_name) = tool.as_str() {
                        // Special handling for tools that are detected differently
                        match tool_name {
                            "rust" => {
                                // Rust is detected as a language via rustc
                                if !env.languages.get("rust").is_some_and(|info| info.available) {
                                    warnings.push("⚠️  Required: Rust language not found (install via rustup)".to_string());
                                }
                            }
                            _ => {
                                // Standard tool check
                                if !env.tools.get(tool_name).is_some_and(|info| info.available) {
                                    warnings
                                        .push(format!("⚠️  Required tool '{tool_name}' not found"));
                                }
                            }
                        }
                    }
                }
            }

            // Check recommended tools
            if let Some(recommended) = env_section
                .get("recommended_tools")
                .and_then(|v| v.as_array())
            {
                for tool in recommended {
                    if let Some(tool_name) = tool.as_str() {
                        // Skip validation for tools detected elsewhere
                        match tool_name {
                            "rust" => {
                                // Already checked in required tools or languages
                                if !env.languages.get("rust").is_some_and(|info| info.available) {
                                    warnings.push(
                                        "💡 Recommended: Rust language not found".to_string(),
                                    );
                                }
                            }
                            _ => {
                                if !env.tools.get(tool_name).is_some_and(|info| info.available) {
                                    warnings.push(format!(
                                        "💡 Recommended tool '{tool_name}' not found"
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(if warnings.is_empty() {
        None
    } else {
        Some(warnings)
    })
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
    if env::var("CI").is_ok()
        || env::var("GITHUB_ACTIONS").is_ok()
        || env::var("GITLAB_CI").is_ok()
        || env::var("JENKINS_URL").is_ok()
        || env::var("BUILDKITE").is_ok()
        || env::var("CIRCLECI").is_ok()
    {
        return EnvironmentMode::CI;
    }

    // Default to interactive
    EnvironmentMode::Interactive
}

fn determine_dev_environment(environment: &patina::Environment) -> String {
    let mode = detect_environment_mode();

    // Check explicit override first
    if let Ok(dev) = env::var("PATINA_DEV") {
        eprintln!("📦 Using development environment from PATINA_DEV: {dev}");
        return dev;
    }

    // In CI/headless mode, default to Docker for predictability
    if mode == EnvironmentMode::CI || mode == EnvironmentMode::Headless {
        eprintln!("🤖 Headless mode detected: defaulting to Docker");
        eprintln!("   Set PATINA_DEV=dagger to use Dagger in CI");
        return "docker".to_string();
    }

    // Interactive mode: smart detection (best to worst)
    let has_docker = environment
        .tools
        .get("docker")
        .map(|t| t.available)
        .unwrap_or(false);

    let has_go = environment
        .languages
        .get("go")
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

fn validate_design_structure(design: &Value) -> Result<()> {
    // Check for required top-level sections
    let project = design
        .get("project")
        .ok_or_else(|| anyhow::anyhow!("PROJECT_DESIGN.toml missing required [project] section"))?;

    // Check for required project fields
    project
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!("PROJECT_DESIGN.toml missing required field: project.name")
        })?;

    project
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!("PROJECT_DESIGN.toml missing required field: project.type")
        })?;

    // Validate project type
    let project_type = project.get("type").and_then(|v| v.as_str()).unwrap();
    match project_type {
        "app" | "tool" | "library" => {}
        _ => anyhow::bail!(
            "Invalid project.type: '{}'. Must be one of: app, tool, library",
            project_type
        ),
    }

    Ok(())
}
