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
    },

    /// Remove a frontend from project's allowed list
    Remove {
        /// Frontend name (claude, gemini, codex)
        name: String,

        /// Don't backup files before removing
        #[arg(long)]
        no_backup: bool,
    },

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
        Some(AdapterCommands::Add { name }) => add(&name),
        Some(AdapterCommands::Remove { name, no_backup }) => remove(&name, no_backup),
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
fn add(name: &str) -> Result<()> {
    // Verify frontend exists
    let _ = frontend::get(name)?;

    let cwd = std::env::current_dir()?;
    if !project::is_patina_project(&cwd) {
        anyhow::bail!("Not a patina project. Run `patina init .` first.");
    }

    let mut config = project::load_with_migration(&cwd)?;
    if config.frontends.allowed.contains(&name.to_string()) {
        println!("Frontend '{}' is already in allowed list.", name);
        return Ok(());
    }

    config.frontends.allowed.push(name.to_string());
    project::save(&cwd, &config)?;

    println!("✓ Added '{}' to allowed frontends", name);
    println!("  Allowed: {:?}", config.frontends.allowed);

    // Create adapter files if they don't exist
    let adapter_dir = cwd.join(format!(".{}", name));
    if !adapter_dir.exists() {
        println!("  Creating .{}/ directory...", name);
        patina::adapters::templates::copy_to_project(name, &cwd)?;
        println!("  ✓ Created adapter files");
    }

    Ok(())
}

/// Remove a frontend from project's allowed list
fn remove(name: &str, no_backup: bool) -> Result<()> {
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
        "\n💡 To also remove files: rm -rf .{}/ {}.md",
        name,
        name.to_uppercase()
    );

    Ok(())
}

/// Backup frontend-specific files before removal
fn backup_frontend_files(project_root: &std::path::Path, name: &str) -> Result<()> {
    // Backup bootstrap file (CLAUDE.md, GEMINI.md, etc.)
    let bootstrap_file = match name {
        "claude" => "CLAUDE.md",
        "gemini" => "GEMINI.md",
        "codex" => "AGENTS.md",
        _ => "",
    };
    if !bootstrap_file.is_empty() {
        let file_path = project_root.join(bootstrap_file);
        if let Some(backup_path) = project::backup_file(project_root, &file_path)? {
            println!(
                "  ✓ Backed up {} to {}",
                bootstrap_file,
                backup_path.display()
            );
        }
    }

    // Note about adapter directory (.claude/, .gemini/, etc.)
    let adapter_dir = project_root.join(format!(".{}", name));
    if adapter_dir.exists() {
        // For directories, we just note they exist - full backup would be complex
        println!("  ⚠ Adapter directory .{}/ exists (not backed up)", name);
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
