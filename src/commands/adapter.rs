//! Adapter command - Manage AI frontend configurations
//!
//! Adapters are AI frontends (Claude Code, Gemini CLI, etc.) that can be used
//! to interact with a patina project. This command manages:
//! - Global frontend availability (detected from system)
//! - Project-level allowed frontends (configured per-project)
//!
//! # Example
//!
//! ```no_run
//! # fn main() -> anyhow::Result<()> {
//! // List available and allowed frontends
//! // patina adapter list
//!
//! // Set global default
//! // patina adapter default claude
//!
//! // Set project default
//! // patina adapter default gemini --project
//!
//! // Add frontend to project
//! // patina adapter add gemini
//!
//! // Remove frontend from project (with backup)
//! // patina adapter remove claude
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use patina::adapters::launch as frontend;
use patina::project;

/// Adapter subcommands (re-exported for main.rs)
#[derive(Debug, Clone, clap::Subcommand)]
pub enum AdapterCommands {
    /// List available frontends (global) and allowed frontends (project)
    List,

    /// Set default frontend (global or project with --project)
    Default {
        /// Frontend name (claude, gemini, codex)
        name: String,

        /// Set default for current project (not global)
        #[arg(short, long)]
        project: bool,
    },

    /// Check frontend installation status
    Check {
        /// Frontend name (optional, checks all if not specified)
        name: Option<String>,
    },

    /// Add a frontend to project's allowed list
    Add {
        /// Frontend name (claude, gemini, codex)
        name: String,

        /// Skip automatic git commit
        #[arg(long)]
        no_commit: bool,
    },

    /// Remove a frontend from project's allowed list
    Remove {
        /// Frontend name (claude, gemini, codex)
        name: String,

        /// Don't backup files before removing
        #[arg(long)]
        no_backup: bool,

        /// Skip automatic git commit
        #[arg(long)]
        no_commit: bool,
    },

    /// Refresh adapter files (backup, update templates, restore sessions)
    Refresh {
        /// Frontend name (claude, gemini, codex)
        name: String,

        /// Skip automatic git commit
        #[arg(long)]
        no_commit: bool,
    },

    /// Health check all configured adapters
    Doctor,

    /// Configure MCP server for a frontend
    Mcp {
        /// Frontend name (claude)
        name: String,

        /// Remove MCP configuration instead of adding
        #[arg(long)]
        remove: bool,
    },
}

/// Execute the adapter command (main entry point from CLI)
pub fn execute(command: Option<AdapterCommands>) -> Result<()> {
    match command {
        None | Some(AdapterCommands::List) => list(),
        Some(AdapterCommands::Default { name, project }) => set_default(&name, project),
        Some(AdapterCommands::Check { name }) => check(name.as_deref()),
        Some(AdapterCommands::Add { name, no_commit }) => add(&name, no_commit),
        Some(AdapterCommands::Remove {
            name,
            no_backup,
            no_commit,
        }) => remove(&name, no_backup, no_commit),
        Some(AdapterCommands::Refresh { name, no_commit }) => refresh(&name, no_commit),
        Some(AdapterCommands::Doctor) => doctor(),
        Some(AdapterCommands::Mcp { name, remove }) => configure_mcp(&name, remove),
    }
}

/// List available frontends (global) and allowed frontends (project)
fn list() -> Result<()> {
    // Show global frontends
    let frontends = frontend::list()?;
    println!("📱 Available AI Frontends (Global)\n");
    println!("{:<12} {:<15} {:<10} VERSION", "NAME", "DISPLAY", "STATUS");
    println!("{}", "─".repeat(50));
    for f in frontends {
        let status = if f.detected {
            "✓ found"
        } else {
            "✗ missing"
        };
        let version = f.version.unwrap_or_else(|| "-".to_string());
        println!(
            "{:<12} {:<15} {:<10} {}",
            f.name, f.display, status, version
        );
    }

    let default = frontend::default_name()?;
    println!("\nGlobal default: {}", default);

    // Show project frontends if in a patina project
    let cwd = std::env::current_dir()?;
    if project::is_patina_project(&cwd) {
        let config = project::load_with_migration(&cwd)?;
        println!("\n📁 Project Allowed Frontends\n");
        println!("Allowed: {:?}", config.frontends.allowed);
        println!("Project default: {}", config.frontends.default);
    }

    Ok(())
}

/// Set default frontend (global or project-level)
fn set_default(name: &str, is_project: bool) -> Result<()> {
    if is_project {
        // Set project default
        let cwd = std::env::current_dir()?;
        if !project::is_patina_project(&cwd) {
            anyhow::bail!("Not a patina project. Run `patina init .` first.");
        }
        let mut config = project::load_with_migration(&cwd)?;
        if !config.frontends.allowed.contains(&name.to_string()) {
            anyhow::bail!(
                "Frontend '{}' is not in allowed list. Add it first: patina adapter add {}",
                name,
                name
            );
        }
        config.frontends.default = name.to_string();
        project::save(&cwd, &config)?;
        println!("✓ Project default frontend set to: {}", name);
    } else {
        // Set global default
        frontend::set_default(name)?;
        println!("✓ Global default frontend set to: {}", name);
    }
    Ok(())
}

/// Check frontend installation status
fn check(name: Option<&str>) -> Result<()> {
    if let Some(n) = name {
        let f = frontend::get(n)?;
        if f.detected {
            println!("✓ {} is installed", f.display);
            if let Some(v) = f.version {
                println!("  Version: {}", v);
            }
        } else {
            println!("✗ {} is not installed", f.display);
        }
    } else {
        // Check all
        for f in frontend::list()? {
            let status = if f.detected { "✓" } else { "✗" };
            println!("{} {}", status, f.display);
        }
    }
    Ok(())
}

/// Add a frontend to project's allowed list
fn add(name: &str, no_commit: bool) -> Result<()> {
    // Verify frontend exists
    let _ = frontend::get(name)?;

    let cwd = std::env::current_dir()?;
    if !project::is_patina_project(&cwd) {
        anyhow::bail!("Not a patina project. Run `patina init .` first.");
    }

    let mut config = project::load_with_migration(&cwd)?;
    let already_allowed = config.frontends.allowed.contains(&name.to_string());

    if !already_allowed {
        config.frontends.allowed.push(name.to_string());
        // Set as default if this is the first adapter
        if config.frontends.default.is_empty() {
            config.frontends.default = name.to_string();
        }
        project::save(&cwd, &config)?;
        println!("✓ Added '{}' to allowed frontends", name);
        println!("  Allowed: {:?}", config.frontends.allowed);
    } else {
        println!("Frontend '{}' is already in allowed list.", name);
    }

    // Create adapter files if they don't exist
    let adapter_dir = cwd.join(format!(".{}", name));
    let bootstrap_file = get_bootstrap_filename(name);
    let bootstrap_path = cwd.join(&bootstrap_file);
    let created_files = !adapter_dir.exists() || !bootstrap_path.exists();

    if !adapter_dir.exists() {
        println!("  Creating .{}/ directory...", name);
        patina::adapters::templates::copy_to_project(name, &cwd)?;
        println!("  ✓ Created adapter files");
    }

    // Create bootstrap file (CLAUDE.md, GEMINI.md, etc.) if it doesn't exist
    if !bootstrap_path.exists() {
        println!("  Creating {}...", bootstrap_file);
        frontend::generate_bootstrap(name, &cwd)?;
        println!("  ✓ Created {}", bootstrap_file);
    }

    // Commit if files were created and not in no_commit mode
    if created_files && !no_commit {
        println!("\n📦 Committing adapter setup...");
        let adapter_dir = format!(".{}", name);
        let mut files_to_add = Vec::new();
        if cwd.join(&adapter_dir).exists() {
            files_to_add.push(adapter_dir);
        }
        if cwd.join(&bootstrap_file).exists() {
            files_to_add.push(bootstrap_file.clone());
        }
        if cwd.join(".patina/config.toml").exists() {
            files_to_add.push(".patina/config.toml".to_string());
        }

        let refs: Vec<&str> = files_to_add.iter().map(|s| s.as_str()).collect();
        patina::git::add_paths(&refs)?;
        patina::git::commit(&format!("chore: add {} adapter", name))?;
        println!("✓ Committed adapter files");
    }

    Ok(())
}

/// Remove a frontend from project's allowed list
fn remove(name: &str, no_backup: bool, _no_commit: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    if !project::is_patina_project(&cwd) {
        anyhow::bail!("Not a patina project. Run `patina init .` first.");
    }

    let mut config = project::load_with_migration(&cwd)?;
    if !config.frontends.allowed.contains(&name.to_string()) {
        println!("Frontend '{}' is not in allowed list.", name);
        return Ok(());
    }

    // Backup files if requested
    if !no_backup {
        backup_frontend_files(&cwd, name)?;
    }

    // Remove from allowed list
    config.frontends.allowed.retain(|f| f != name);

    // Update default if we removed it
    if config.frontends.default == name {
        config.frontends.default = config
            .frontends
            .allowed
            .first()
            .cloned()
            .unwrap_or_default();
        if !config.frontends.default.is_empty() {
            println!("  ✓ Default changed to: {}", config.frontends.default);
        }
    }

    project::save(&cwd, &config)?;

    println!("✓ Removed '{}' from allowed frontends", name);
    println!("  Allowed: {:?}", config.frontends.allowed);
    println!(
        "\n💡 To also remove files: rm -rf .{}/ {}",
        name,
        get_bootstrap_filename(name)
    );

    // Note: We don't auto-commit removal since files still exist
    // User should manually delete files and commit

    Ok(())
}

/// Get the bootstrap filename for a frontend (CLAUDE.md, GEMINI.md, etc.)
fn get_bootstrap_filename(name: &str) -> String {
    match name {
        "claude" => "CLAUDE.md".to_string(),
        "gemini" => "GEMINI.md".to_string(),
        "codex" => "AGENTS.md".to_string(),
        "opencode" => "OPENCODE.md".to_string(),
        _ => format!("{}.md", name.to_uppercase()),
    }
}

/// Refresh adapter files - backup, update templates, restore sessions
fn refresh(name: &str, no_commit: bool) -> Result<()> {
    // Verify frontend exists
    let _ = frontend::get(name)?;

    let cwd = std::env::current_dir()?;
    if !project::is_patina_project(&cwd) {
        anyhow::bail!("Not a patina project. Run `patina init .` first.");
    }

    let config = project::load_with_migration(&cwd)?;
    if !config.frontends.allowed.contains(&name.to_string()) {
        anyhow::bail!(
            "Frontend '{}' is not in allowed list. Add it first: patina adapter add {}",
            name,
            name
        );
    }

    println!("🔄 Refreshing {} adapter...\n", name);

    // Step 1: Backup existing files (including session files)
    println!("📦 Backing up existing files...");
    backup_frontend_files(&cwd, name)?;

    // Step 2: Preserve session files before removing adapter directory
    let adapter_dir = cwd.join(format!(".{}", name));
    let preserved_sessions = preserve_session_files(&adapter_dir)?;

    // Step 3: Remove old adapter directory
    if adapter_dir.exists() {
        std::fs::remove_dir_all(&adapter_dir)?;
        println!("  ✓ Removed old .{}/ directory", name);
    }

    // Step 4: Copy fresh templates
    println!("\n📋 Copying fresh templates...");
    patina::adapters::templates::copy_to_project(name, &cwd)?;
    println!("  ✓ Copied fresh adapter files");

    // Create/refresh bootstrap file (CLAUDE.md, GEMINI.md, etc.)
    let bootstrap_file = get_bootstrap_filename(name);
    println!("  Generating {}...", bootstrap_file);
    frontend::generate_bootstrap(name, &cwd)?;
    println!("  ✓ Created {}", bootstrap_file);

    // Step 5: Restore preserved session files
    if !preserved_sessions.is_empty() {
        println!("\n📁 Restoring session files...");
        restore_session_files(&adapter_dir, &preserved_sessions)?;
        println!("  ✓ Restored {} session files", preserved_sessions.len());
    }

    // Step 6: Commit if not in no_commit mode
    if !no_commit {
        println!("\n📦 Committing refresh...");
        let adapter_dir_name = format!(".{}", name);
        let mut files_to_add = Vec::new();
        if cwd.join(&adapter_dir_name).exists() {
            files_to_add.push(adapter_dir_name);
        }
        if cwd.join(&bootstrap_file).exists() {
            files_to_add.push(bootstrap_file);
        }

        let refs: Vec<&str> = files_to_add.iter().map(|s| s.as_str()).collect();
        patina::git::add_paths(&refs)?;
        patina::git::commit(&format!("chore: refresh {} adapter", name))?;
        println!("✓ Committed adapter refresh");
    }

    println!("\n✨ {} adapter refreshed successfully!", name);
    Ok(())
}

/// Preserve session files from adapter directory
fn preserve_session_files(adapter_dir: &std::path::Path) -> Result<Vec<(String, Vec<u8>)>> {
    let mut preserved = Vec::new();

    // Look for context directory which typically has session files
    let context_dir = adapter_dir.join("context");
    if context_dir.exists() {
        for entry in std::fs::read_dir(&context_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                // Preserve session-related files
                if filename.contains("session") || filename.ends_with(".md") {
                    let content = std::fs::read(&path)?;
                    preserved.push((format!("context/{}", filename), content));
                }
            }
        }
    }

    Ok(preserved)
}

/// Restore preserved session files to adapter directory
fn restore_session_files(
    adapter_dir: &std::path::Path,
    files: &[(String, Vec<u8>)],
) -> Result<()> {
    for (relative_path, content) in files {
        let full_path = adapter_dir.join(relative_path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full_path, content)?;
    }
    Ok(())
}

/// Health check all configured adapters
fn doctor() -> Result<()> {
    let cwd = std::env::current_dir()?;
    if !project::is_patina_project(&cwd) {
        anyhow::bail!("Not a patina project. Run `patina init .` first.");
    }

    let config = project::load_with_migration(&cwd)?;

    println!("🩺 Adapter Health Check\n");

    if config.frontends.allowed.is_empty() {
        println!("⚠️  No adapters configured.");
        println!("   Run: patina adapter add <claude|gemini|opencode>");
        return Ok(());
    }

    let mut all_healthy = true;

    for adapter_name in &config.frontends.allowed {
        println!("📱 {} adapter:", adapter_name);

        // Check 1: Frontend installed on system
        let frontend_info = frontend::get(adapter_name);
        match frontend_info {
            Ok(f) if f.detected => {
                println!("  ✓ CLI installed: {}", f.version.unwrap_or_default());
            }
            Ok(_) => {
                println!("  ✗ CLI not found on system");
                all_healthy = false;
            }
            Err(_) => {
                println!("  ✗ Unknown frontend type");
                all_healthy = false;
            }
        }

        // Check 2: Adapter directory exists
        let adapter_dir = cwd.join(format!(".{}", adapter_name));
        if adapter_dir.exists() {
            println!("  ✓ .{}/ directory exists", adapter_name);
        } else {
            println!("  ✗ .{}/ directory missing", adapter_name);
            println!("    Fix: patina adapter refresh {}", adapter_name);
            all_healthy = false;
        }

        // Check 3: Bootstrap file exists
        let bootstrap_file = get_bootstrap_filename(adapter_name);
        let bootstrap_path = cwd.join(&bootstrap_file);
        if bootstrap_path.exists() {
            println!("  ✓ {} exists", bootstrap_file);
        } else {
            println!("  ✗ {} missing", bootstrap_file);
            println!("    Fix: patina adapter refresh {}", adapter_name);
            all_healthy = false;
        }

        // Check 4: MCP configuration (Claude only)
        if adapter_name == "claude" {
            match check_mcp_configured() {
                Ok(true) => println!("  ✓ MCP server configured"),
                Ok(false) => {
                    println!("  ⚠ MCP server not configured (optional)");
                    println!("    Setup: patina adapter mcp claude");
                }
                Err(_) => {
                    println!("  ⚠ Could not check MCP status");
                }
            }
        }

        println!();
    }

    // Summary
    if all_healthy {
        println!("✅ All adapters healthy!");
    } else {
        println!("⚠️  Some issues found. See above for fixes.");
    }

    Ok(())
}

/// Check if MCP is configured for Claude
fn check_mcp_configured() -> Result<bool> {
    use std::process::Command;

    let output = Command::new("claude")
        .args(["mcp", "list"])
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains("patina"))
    } else {
        Ok(false)
    }
}

/// Backup frontend-specific files before removal or refresh
fn backup_frontend_files(project_root: &std::path::Path, name: &str) -> Result<()> {
    let bootstrap_file = get_bootstrap_filename(name);
    let file_path = project_root.join(&bootstrap_file);
    if let Some(backup_path) = project::backup_file(project_root, &file_path)? {
        println!(
            "  ✓ Backed up {} to {}",
            bootstrap_file,
            backup_path.display()
        );
    }

    // Backup adapter directory (.claude/, .gemini/, etc.) to .patina/backups/
    let adapter_dir = project_root.join(format!(".{}", name));
    if adapter_dir.exists() {
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let backup_dir = project::backups_dir(project_root).join(format!("{}-{}", name, timestamp));
        std::fs::create_dir_all(&backup_dir)?;

        // Copy adapter directory contents
        copy_dir_recursive(&adapter_dir, &backup_dir)?;
        println!(
            "  ✓ Backed up .{}/ to {}",
            name,
            backup_dir.display()
        );
    }

    Ok(())
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    use std::fs;

    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Configure MCP server for a frontend
fn configure_mcp(name: &str, remove: bool) -> Result<()> {
    use std::process::Command;

    match name {
        "claude" => {
            // Find patina binary path
            let patina_path = std::env::current_exe()?;

            if remove {
                // Remove MCP configuration
                println!("Removing patina MCP server from Claude Code...");
                let status = Command::new("claude")
                    .args(["mcp", "remove", "patina"])
                    .status()?;

                if status.success() {
                    println!("✓ Removed patina MCP server");
                } else {
                    anyhow::bail!("Failed to remove MCP server. Is Claude Code installed?");
                }
            } else {
                // Add MCP configuration
                println!("Adding patina MCP server to Claude Code...");
                let status = Command::new("claude")
                    .args([
                        "mcp",
                        "add",
                        "--transport",
                        "stdio",
                        "-s",
                        "user",
                        "patina",
                        "--",
                        patina_path.to_str().unwrap(),
                        "serve",
                        "--mcp",
                    ])
                    .status()?;

                if status.success() {
                    println!("✓ Added patina MCP server");
                    println!("\n  Restart Claude Code to use scry and context tools.");
                    println!("  Verify with: claude mcp list");
                } else {
                    anyhow::bail!("Failed to add MCP server. Is Claude Code installed?");
                }
            }
        }
        "gemini" => {
            anyhow::bail!("Gemini MCP configuration not yet supported");
        }
        _ => {
            anyhow::bail!("Unknown frontend: {}. Supported: claude", name);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_commands_variants() {
        let list = AdapterCommands::List;
        assert!(matches!(list, AdapterCommands::List));

        let add = AdapterCommands::Add {
            name: "claude".to_string(),
        };
        assert!(matches!(add, AdapterCommands::Add { .. }));
    }
}
