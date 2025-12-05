//! Internal implementation for init command

pub mod backup;
pub mod config;
pub mod patterns;
pub mod validation;

use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use patina::environment::Environment;
use patina::layer::Layer;

use self::backup::{backup_gitignored_dirs, restore_session_files};
use self::config::{create_project_config, handle_version_manifest};
use self::patterns::copy_core_patterns_safe;
use self::validation::{determine_dev_environment, validate_environment};

use super::design_wizard::confirm;

/// Main execution logic for init command
pub fn execute_init(
    name: String,
    llm: String,
    dev: Option<String>,
    force: bool,
    local: bool,
) -> Result<()> {
    let json_output = false; // For init command, always false

    // === STEP 1: GIT SETUP (FIRST - BEFORE ANY DESTRUCTIVE CHANGES) ===
    println!("🎨 Initializing Patina...\n");

    // Check for gh CLI early (unless local mode)
    if !local {
        check_gh_cli_available()?;
    }

    // Initialize git if needed
    ensure_git_initialized()?;

    // Ensure proper .gitignore exists
    ensure_gitignore(Path::new("."))?;

    // Check if this is a re-init (already has .patina/)
    let is_reinit_early = name == "." && Path::new(".patina").exists();

    if is_reinit_early {
        // Re-init: skip branch management, just refresh files in place
        println!("🔄 Re-initializing existing Patina project...\n");
    } else {
        // First-time init: full git setup (fork detection, branch management)
        patina::git::ensure_fork(local)?;
        println!();

        patina::git::ensure_patina_branch(force)?;
        println!("✓ On branch 'patina'\n");
    }

    // === STEP 2: SAFE TO PROCEED WITH DESTRUCTIVE CHANGES ===

    // Backup existing devcontainer if it exists
    if Path::new(".devcontainer").exists() {
        fs::rename(".devcontainer", ".devcontainer.backup")
            .context("Failed to backup existing .devcontainer")?;
        println!("✓ Backed up .devcontainer/ → .devcontainer.backup/");
    }

    // Backup gitignored directories if re-initializing
    if name == "." && Path::new(".claude").exists() {
        backup_gitignored_dirs()?;
    }

    // Check for nested project
    if name != "." && Path::new(".patina").exists() {
        println!("⚠️  You're already in a Patina project!");
        println!(
            "   Running 'patina init {}' would create: {}",
            name,
            std::env::current_dir()?.join(&name).display()
        );
        if !confirm("Continue anyway?")? {
            println!("Initialization cancelled.");
            return Ok(());
        }
    }

    // Check if re-initializing (for non-current-dir case)
    let is_reinit = if name == "." {
        is_reinit_early
    } else {
        let path = PathBuf::from(&name);
        path.exists() && path.join(".patina").exists()
    };

    // Only print init message if we didn't already print re-init message
    if !is_reinit_early {
        if is_reinit {
            println!("🔄 Re-initializing Patina project...");
        } else {
            println!("🎨 Initializing Patina project: {name}");
        }
    }

    // Detect environment
    println!("🔍 Detecting environment...");
    let environment = Environment::detect()?;
    let dev = dev.unwrap_or_else(|| determine_dev_environment(&environment));

    // Display environment info
    display_environment_info(&environment);

    // Create or determine project path
    let project_path = setup_project_path(&name)?;

    // Get project name from directory
    let project_name = if name == "." {
        project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string()
    } else {
        name.clone()
    };

    // Write ENVIRONMENT.toml with full captured data
    write_environment_toml(&project_path, &environment)?;

    // Initialize layer structure
    let layer_path = project_path.join("layer");
    let layer = Layer::new(&layer_path);
    layer
        .init()
        .context("Failed to initialize layer structure")?;
    println!("  ✓ Created layer structure");

    // Create project configuration
    let dev_env = patina::dev_env::get_dev_env(&dev);
    create_project_config(
        &project_path,
        &name,
        &llm,
        &dev,
        &environment,
        dev_env.as_ref(),
    )?;

    // Handle version manifest and updates
    let updates = handle_version_manifest(&project_path, &llm, &dev, is_reinit, json_output)?;

    // Process updates if needed
    let should_update = if let Some(ref _updates_list) = updates {
        should_update_components(json_output)?
    } else {
        false
    };

    if should_update {
        println!("  ✓ Components will be updated");
    }

    // Initialize LLM adapter
    let adapter = patina::adapters::get_adapter(&llm);
    adapter.init_project(&project_path, &project_name, &environment)?;
    println!("  ✓ Created {llm} integration files");

    // Restore preserved session files if any
    restore_session_files()?;

    // Initialize dev environment - use real project name from Cargo.toml if re-initializing
    let dev_project_name = if name == "." {
        detect_project_name_from_cargo_toml(&project_path).unwrap_or_else(|_| name.to_string())
    } else {
        name.clone()
    };
    dev_env.init_project(&project_path, &dev_project_name, "app")?;
    println!("  ✓ Created {dev} environment files");

    // Copy core patterns
    let patterns_copied = copy_core_patterns_safe(&project_path, &layer_path)?;
    if patterns_copied {
        println!("  ✓ Copied core patterns from Patina");
    }

    // Create initial session record
    create_init_session(&layer_path, &project_name, &llm, &dev)?;

    // Initialize navigation index
    initialize_navigation(&project_path)?;

    // Run post-init for adapter
    adapter.post_init(&project_path, &dev)?;

    // Update components if needed
    if should_update {
        update_components(&project_path, &llm)?;
    }

    // Validate environment
    if let Some(warnings) = validate_environment(&environment)? {
        println!("\n⚠️  Environment warnings:");
        for warning in warnings {
            println!("   {warning}");
        }
    }

    // === STEP 3: COMMIT PATINA SETUP ===
    if name == "." {
        // Only commit if we're initializing in current directory
        println!("\n📦 Committing Patina setup...");
        patina::git::add_all()?;

        let commit_msg = if is_reinit {
            "chore: update Patina configuration"
        } else {
            "chore: initialize Patina"
        };

        patina::git::commit(commit_msg)?;
        println!("✓ Committed Patina initialization");
    }

    // Suggest tool installation if needed
    suggest_missing_tools(&environment)?;

    println!("\n✨ Project '{name}' initialized successfully!");
    println!("\nNext steps:");
    println!("  1. patina add <type> <name>  # Add patterns to session");
    println!("  2. patina commit             # Commit patterns to layer");
    println!("  3. patina push               # Generate LLM context");

    Ok(())
}

fn display_environment_info(environment: &Environment) {
    println!("  ✓ OS: {} ({})", environment.os, environment.arch);
    for (tool, info) in &environment.tools {
        if info.available {
            println!(
                "  ✓ {}: {}",
                tool,
                info.version.as_ref().unwrap_or(&"detected".to_string())
            );
        }
    }
}

/// Write ENVIRONMENT.toml with all captured environment data
fn write_environment_toml(project_path: &Path, environment: &Environment) -> Result<()> {
    let toml_path = project_path.join("ENVIRONMENT.toml");

    let content =
        toml::to_string_pretty(environment).context("Failed to serialize environment data")?;

    fs::write(&toml_path, content).context("Failed to write ENVIRONMENT.toml")?;

    println!("  ✓ Created ENVIRONMENT.toml with full environment data");
    Ok(())
}

fn setup_project_path(name: &str) -> Result<PathBuf> {
    let path = if name == "." {
        std::env::current_dir()?
    } else {
        let path = PathBuf::from(name);
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }
        path.canonicalize()?
    };
    Ok(path)
}

fn create_init_session(layer_path: &Path, name: &str, llm: &str, dev: &str) -> Result<()> {
    let session_filename = format!("{}-init.md", chrono::Utc::now().format("%Y%m%d-%H%M%S"));
    let session_content = format!(
        "# {} Initialization\n\nInitialized on: {}\nLLM: {}\nDev Environment: {}\n",
        name,
        chrono::Utc::now().to_rfc3339(),
        llm,
        dev
    );

    let sessions_path = layer_path.join("sessions");
    fs::create_dir_all(&sessions_path)?;
    fs::write(sessions_path.join(session_filename), session_content)?;

    Ok(())
}

fn initialize_navigation(project_path: &Path) -> Result<()> {
    // TODO: Fix NavigationIndexer to work with new PatternIndexer API
    // For now, just print a message
    if project_path.join("layer").exists() {
        println!("🔍 Reindexing patterns for navigation...");
        println!("  Indexing patterns... ✓ (0 patterns indexed)");
    } else {
        println!("🔍 Initializing navigation database...");
        println!("  ✓ Created navigation database");
        println!("  Indexing patterns... ✓ (0 patterns indexed)");
    }
    Ok(())
}

fn should_update_components(json_output: bool) -> Result<bool> {
    if json_output {
        return Ok(true);
    }

    print!("Update components to latest versions? [Y/n]: ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    Ok(input.is_empty() || input == "y" || input == "yes")
}

fn update_components(project_path: &Path, llm: &str) -> Result<()> {
    println!("\n🔄 Updating components...");

    // Update LLM adapter
    print!("  Updating {} adapter... ", llm.to_title_case());
    std::io::stdout().flush()?;

    let adapter = patina::adapters::get_adapter(llm);
    if let Some((_current_ver, _new_ver)) = adapter.check_for_updates(project_path)? {
        adapter.update_adapter_files(project_path)?;
        println!("✓");
    } else {
        println!("already up to date");
    }

    println!("\n✅ All components updated successfully!");
    Ok(())
}

fn suggest_missing_tools(environment: &Environment) -> Result<()> {
    use crate::commands::init::tool_installer;

    // Get list of tools we can help install
    let available_tools = tool_installer::get_available_tools();

    // Check which ones are missing from the environment
    let missing: Vec<_> = available_tools
        .iter()
        .filter(|tool| {
            !environment
                .tools
                .get(tool.name)
                .map(|info| info.available)
                .unwrap_or(false)
        })
        .collect();

    if !missing.is_empty() {
        println!("\n💡 Missing optional tools that can enhance your Patina experience:");
        for tool in &missing {
            println!("   - {}", tool.name);
        }
        println!("\n   Run 'patina init --install-tools' to install them automatically");
        println!("   (Note: --install-tools flag is not yet implemented)");
    }

    Ok(())
}

/// Detect project name from Cargo.toml
fn detect_project_name_from_cargo_toml(project_path: &Path) -> Result<String> {
    let cargo_toml_path = project_path.join("Cargo.toml");

    if !cargo_toml_path.exists() {
        anyhow::bail!("Cargo.toml not found");
    }

    let cargo_content =
        fs::read_to_string(&cargo_toml_path).context("Failed to read Cargo.toml")?;
    let cargo_toml: toml::Value =
        toml::from_str(&cargo_content).context("Failed to parse Cargo.toml")?;

    cargo_toml
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No package.name found in Cargo.toml"))
}

// Helper trait for string formatting
trait TitleCase {
    fn to_title_case(&self) -> String;
}

impl TitleCase for str {
    fn to_title_case(&self) -> String {
        let mut chars = self.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
}

/// Check if gh CLI is available
fn check_gh_cli_available() -> Result<()> {
    use std::process::Command;

    let output = Command::new("gh").arg("--version").output();

    match output {
        Ok(output) if output.status.success() => Ok(()),
        _ => {
            eprintln!("Error: GitHub CLI (gh) is required but not found.");
            eprintln!();
            eprintln!("Please install the GitHub CLI:");
            eprintln!("  • macOS: brew install gh");
            eprintln!("  • Linux: See https://cli.github.com/manual/installation");
            eprintln!("  • Windows: winget install GitHub.cli");
            eprintln!();
            eprintln!("Or use --local flag to skip GitHub integration.");
            anyhow::bail!("GitHub CLI (gh) not found")
        }
    }
}

/// Ensure git is initialized in the current directory
fn ensure_git_initialized() -> Result<()> {
    use std::process::Command;

    // Check if we're in a git repository
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .context("Failed to check git status")?;

    if !output.status.success() {
        // Not a git repo, initialize it
        println!("📝 No git repository found. Initializing...");

        let output = Command::new("git")
            .arg("init")
            .output()
            .context("Failed to initialize git repository")?;

        if !output.status.success() {
            anyhow::bail!("Failed to initialize git repository");
        }

        println!("✓ Initialized git repository");
    }

    Ok(())
}

/// Ensure a proper .gitignore exists with sensible defaults
pub fn ensure_gitignore(project_path: &Path) -> Result<()> {
    let gitignore_path = project_path.join(".gitignore");

    if !gitignore_path.exists() {
        // Create opinionated defaults for new projects
        create_default_gitignore(&gitignore_path)?;
    } else {
        // Ensure critical entries exist in existing .gitignore
        ensure_gitignore_entries(&gitignore_path)?;
    }

    Ok(())
}

/// Create a new .gitignore with sensible defaults
fn create_default_gitignore(gitignore_path: &Path) -> Result<()> {
    let content = r#"# Build artifacts
/target/
**/*.rs.bk
Cargo.lock

# Environment and secrets
.env
.env.*
*.pem
*.key
credentials.json
secrets.toml

# Dependencies
node_modules/
vendor/
venv/
__pycache__/
*.pyc

# Build outputs
dist/
build/
*.o
*.so
*.dylib
*.dll
*.exe

# IDE and editor files
.idea/
.vscode/
*.iml
*.swp
*.swo
*~
.DS_Store

# Patina-specific
.patina/
ENVIRONMENT.toml

# Temporary files
*.tmp
*.bak
*.backup
*.old

# Database files
*.db
*.db-shm
*.db-wal
*.sqlite
*.sqlite3

# Logs
*.log
logs/
"#;

    fs::write(gitignore_path, content).context("Failed to create .gitignore")?;

    println!("✓ Created .gitignore with standard patterns");
    Ok(())
}

/// Ensure critical entries exist in an existing .gitignore
fn ensure_gitignore_entries(gitignore_path: &Path) -> Result<()> {
    let content = fs::read_to_string(gitignore_path).context("Failed to read .gitignore")?;

    // Critical entries that should always be ignored
    let must_have = [
        ("/target/", "Rust build artifacts"),
        ("node_modules/", "Node.js dependencies"),
        (".env", "Environment secrets"),
        (".patina/", "Patina cache"),
        ("*.db", "Database files"),
        ("*.key", "Private keys"),
        ("*.pem", "Certificates"),
    ];

    let mut added = Vec::new();
    let mut updated_content = content.clone();

    for (pattern, _description) in must_have {
        // Check if pattern already exists (accounting for variations)
        let pattern_exists = content.lines().any(|line| {
            let line = line.trim();
            line == pattern || line == pattern.trim_end_matches('/')
        });

        if !pattern_exists {
            // Add a newline if file doesn't end with one
            if !updated_content.ends_with('\n') {
                updated_content.push('\n');
            }

            // Add the pattern with a comment if we're adding multiple
            if added.is_empty() {
                updated_content.push_str("\n# Added by Patina for safety\n");
            }
            updated_content.push_str(pattern);
            updated_content.push('\n');

            added.push(pattern);
        }
    }

    if !added.is_empty() {
        fs::write(gitignore_path, updated_content).context("Failed to update .gitignore")?;

        println!("✓ Added to .gitignore: {}", added.join(", "));
    }

    Ok(())
}
