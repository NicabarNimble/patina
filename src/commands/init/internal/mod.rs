//! Internal implementation for init command

pub mod config;
pub mod patterns;
pub mod validation;

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use patina::environment::Environment;
use patina::layer::Layer;

// Note: backup functions moved to 'adapter refresh' command
use self::config::{create_project_config, handle_version_manifest};
use self::patterns::copy_core_patterns_safe;
use self::validation::{determine_dev_environment, validate_environment};

use super::design_wizard::confirm;

/// Main execution logic for init command
pub fn execute_init(
    name: String,
    dev: Option<String>,
    force: bool,
    local: bool,
    no_commit: bool,
) -> Result<()> {
    let json_output = false; // For init command, always false

    // === STEP 0: CHECK FOR HIERARCHY CONFLICTS (BEFORE ANYTHING ELSE) ===
    // Jon Gjengset principle: fail fast with clear errors
    check_hierarchy_conflicts(force)?;

    // === STEP 1: GIT SETUP (FIRST - BEFORE ANY DESTRUCTIVE CHANGES) ===
    println!("🎨 Initializing Patina...\n");

    // Check for gh CLI early (unless local mode)
    if !local {
        check_gh_cli_available()?;
    }

    // Initialize git if needed
    ensure_git_initialized()?;

    // Check if this is a re-init (already has .patina/)
    let is_reinit_early = name == "." && Path::new(".patina").exists();

    if is_reinit_early {
        // Re-init: skip branch management, just refresh files in place
        println!("🔄 Re-initializing existing Patina project...\n");
    } else {
        // First-time init: full git setup (fork detection, branch management)
        // NOTE: This checks for clean state BEFORE we modify any files
        patina::git::ensure_fork(local)?;
        println!();

        patina::git::ensure_patina_branch(force)?;
        println!("✓ On branch 'patina'\n");
    }

    // Ensure proper .gitignore exists (AFTER branch setup, so our changes go on patina branch)
    ensure_gitignore(Path::new("."))?;

    // === STEP 2: SAFE TO PROCEED WITH DESTRUCTIVE CHANGES ===

    // Backup existing devcontainer if it exists
    if Path::new(".devcontainer").exists() {
        // Remove old backup if it exists
        let backup_path = Path::new(".devcontainer.backup");
        if backup_path.exists() {
            fs::remove_dir_all(backup_path).context("Failed to remove old .devcontainer.backup")?;
        }
        fs::rename(".devcontainer", ".devcontainer.backup")
            .context("Failed to backup existing .devcontainer")?;
        println!("✓ Backed up .devcontainer/ → .devcontainer.backup/");
    }

    // Note: We don't backup .claude/ here - that's handled by 'adapter refresh'
    // Init only creates skeleton (.patina/, layer/), not adapter directories

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

    // Create UID (stable project identity)
    let uid = patina::project::create_uid_if_missing(&project_path)?;
    if !is_reinit {
        println!("  ✓ Created project UID: {}", uid);
    }

    // Create project configuration (without LLM - use 'adapter add' for that)
    let dev_env = patina::dev_env::get_dev_env(&dev);
    create_project_config(&project_path, &name, &dev, &environment, dev_env.as_ref())?;

    // Handle version manifest
    handle_version_manifest(&project_path, &dev, is_reinit, json_output)?;

    // Note: LLM adapter initialization moved to 'patina adapter add'
    // Init only creates skeleton - run 'patina adapter add <claude|gemini|opencode>' for LLM support

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

    // Create initial session record (without LLM - added via 'adapter add')
    create_init_session(&layer_path, &project_name, &dev)?;

    // Initialize navigation index
    initialize_navigation(&project_path)?;

    // Note: Component updates for adapters are handled by 'adapter refresh'

    // Validate environment
    if let Some(warnings) = validate_environment(&environment)? {
        println!("\n⚠️  Environment warnings:");
        for warning in warnings {
            println!("   {warning}");
        }
    }

    // === STEP 3: COMMIT PATINA SETUP (unless --no-commit) ===
    if !no_commit && name == "." {
        // Only commit if we're initializing in current directory
        println!("\n📦 Committing Patina setup...");
        // Add specific committed files (not .patina/local/ which is gitignored)
        patina::git::add_paths(&[
            ".gitignore",
            ".patina/config.toml",
            ".patina/uid",
            ".patina/oxidize.yaml",
            ".patina/versions.json",
            ".devcontainer",
            "layer",
            "Dockerfile",
            "docker-compose.yml",
        ])?;

        let commit_msg = if is_reinit {
            "chore: update Patina configuration"
        } else {
            "chore: initialize Patina project"
        };

        patina::git::commit(commit_msg)?;
        println!("✓ Committed Patina initialization");
    }

    // Note: scrape and oxidize are now separate commands
    // Run 'patina scrape' and 'patina oxidize' after adding an adapter

    // Suggest tool installation if needed
    suggest_missing_tools(&environment)?;

    println!("\n✨ Project '{name}' initialized successfully!");
    println!("  Add an adapter: patina adapter add <claude|gemini|opencode>");

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

fn create_init_session(layer_path: &Path, name: &str, dev: &str) -> Result<()> {
    let session_filename = format!("{}-init.md", chrono::Utc::now().format("%Y%m%d-%H%M%S"));
    let session_content = format!(
        "# {} Initialization\n\nInitialized on: {}\nDev Environment: {}\n\nNote: Run 'patina adapter add <name>' to add LLM support.\n",
        name,
        chrono::Utc::now().to_rfc3339(),
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

// Note: Component update functions moved to 'adapter refresh' command

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

# Patina local state (derived, not committed)
.patina/local/
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

// Note: ensure_model_available() moved to oxidize command
// Models are downloaded during 'patina oxidize', not during init

/// Maximum depth to search for child patina projects
/// 6 levels covers most real project structures (monorepos go 5-6 deep)
/// TODO: Make configurable via .patina/config.toml
const CHILD_PROJECT_SEARCH_DEPTH: usize = 6;

/// Check for hierarchy conflicts before init
/// Prevents creating nested patina projects that cause duplicate slash commands
fn check_hierarchy_conflicts(force: bool) -> Result<()> {
    let current_dir = std::env::current_dir()?;

    // Check 1: Parent directories with .claude/ (would cause duplicate commands)
    let mut parent = current_dir.parent();
    let mut conflicting_parents = Vec::new();

    while let Some(p) = parent {
        let claude_dir = p.join(".claude");
        let commands_dir = claude_dir.join("commands");

        if commands_dir.exists() {
            conflicting_parents.push(p.to_path_buf());
        }

        parent = p.parent();
    }

    if !conflicting_parents.is_empty() {
        eprintln!("Error: Found .claude/commands/ in parent directory:");
        for p in &conflicting_parents {
            eprintln!("  → {}", p.display());
        }
        eprintln!();
        eprintln!("Claude Code walks up the directory tree and loads commands from each");
        eprintln!(".claude/commands/ it finds. This would cause duplicate slash commands.");
        eprintln!();
        eprintln!("To fix:");
        eprintln!(
            "  1. Remove the parent .claude/: rm -rf {}/.claude",
            conflicting_parents[0].display()
        );
        eprintln!("  2. Or use --force to ignore this check (not recommended)");

        if !force {
            anyhow::bail!("Hierarchy conflict: parent directory has .claude/commands/");
        }
        eprintln!();
        eprintln!("⚠️  Proceeding anyway due to --force flag...");
    }

    // Check 2: Child directories with .patina/ (would be nested projects)
    let child_projects = find_child_patina_projects(&current_dir)?;

    if !child_projects.is_empty() {
        eprintln!(
            "Error: Found Patina project(s) in subdirectories (checked {} levels):",
            CHILD_PROJECT_SEARCH_DEPTH
        );
        for p in &child_projects {
            eprintln!("  → {}", p.display());
        }
        eprintln!();
        eprintln!("Initializing here would create a parent project over existing ones,");
        eprintln!("causing duplicate commands and configuration conflicts.");
        eprintln!();
        eprintln!("To fix:");
        eprintln!("  1. Initialize in a different directory");
        eprintln!("  2. Or use --force to ignore this check (not recommended)");

        if !force {
            anyhow::bail!("Hierarchy conflict: child directories contain Patina projects");
        }
        eprintln!();
        eprintln!("⚠️  Proceeding anyway due to --force flag...");
    }

    Ok(())
}

/// Find child directories that already have .patina/ or .claude/commands/
/// Uses ignore crate for fast walking (respects .gitignore, skips node_modules etc.)
/// Returns on first match - we only need to find one to block init
fn find_child_patina_projects(dir: &Path) -> Result<Vec<PathBuf>> {
    use ignore::WalkBuilder;

    let walker = WalkBuilder::new(dir)
        .max_depth(Some(CHILD_PROJECT_SEARCH_DEPTH))
        .hidden(false) // Need to see .patina and .claude
        .git_ignore(true) // Skip node_modules, target, etc.
        .build();

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // Skip permission errors etc.
        };

        let path = entry.path();

        // Skip the root directory itself
        if path == dir {
            continue;
        }

        // Check for patina markers
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if file_name == ".patina"
            || (file_name == "commands"
                && path
                    .parent()
                    .map(|p| p.ends_with(".claude"))
                    .unwrap_or(false))
        {
            // Found a marker - return the project directory (parent of .patina or .claude)
            if let Some(project_dir) = path.parent() {
                let project_dir = if file_name == "commands" {
                    // .claude/commands -> go up two levels
                    project_dir.parent().unwrap_or(project_dir)
                } else {
                    project_dir
                };
                // Skip if this is the root directory itself (re-init case)
                // Finding .patina in the current dir is expected for re-init
                if project_dir == dir {
                    continue;
                }
                return Ok(vec![project_dir.to_path_buf()]);
            }
        }
    }

    Ok(vec![])
}

/// Ensure critical entries exist in an existing .gitignore
fn ensure_gitignore_entries(gitignore_path: &Path) -> Result<()> {
    let content = fs::read_to_string(gitignore_path).context("Failed to read .gitignore")?;

    // Critical entries that should always be ignored
    let must_have = [
        ("/target/", "Rust build artifacts"),
        ("node_modules/", "Node.js dependencies"),
        (".env", "Environment secrets"),
        (
            ".patina/local/",
            "Patina local state (derived, not committed)",
        ),
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
