//! Adapter command - compatibility shim for AI interface configuration
//!
//! Interfaces are integrations with AI tools (Claude Code, Gemini CLI, etc.) that
//! interact with a patina project. This command manages:
//! - Global interface availability (detected from system)
//! - Project-level allowed interfaces (configured per-project)
//!
//! # Example
//!
//! ```no_run
//! # fn main() -> anyhow::Result<()> {
//! // List available and allowed interfaces
//! // patina adapter list
//!
//! // Set global default
//! // patina adapter default claude
//!
//! // Set project default
//! // patina adapter default gemini --project
//!
//! // Add interface to project
//! // patina adapter add gemini  # compatibility shim for `patina ai setup`
//!
//! // Remove interface from project (with backup)
//! // patina adapter remove claude
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use patina::interface;
use patina::interface::runtime::launch as interfaces;
use patina::project;

/// Interface subcommands (compatibility re-export for main.rs)
#[derive(Debug, Clone, clap::Subcommand)]
pub enum AdapterCommands {
    /// List available interfaces (global) and allowed interfaces (project)
    List,

    /// Set default interface (global or project with --project)
    Default {
        /// Interface name (claude, gemini, opencode)
        name: String,

        /// Set default for current project (not global)
        #[arg(short, long)]
        project: bool,
    },

    /// Check interface installation status
    Check {
        /// Interface name (optional, checks all if not specified)
        name: Option<String>,
    },

    /// Add an interface to project's allowed list
    Add {
        /// Interface name (claude, gemini, opencode)
        name: String,

        /// Skip automatic git commit
        #[arg(long)]
        no_commit: bool,
    },

    /// Remove an interface from project's allowed list
    Remove {
        /// Interface name (claude, gemini, opencode)
        name: String,

        /// Don't backup files before removing
        #[arg(long)]
        no_backup: bool,

        /// Skip automatic git commit
        #[arg(long)]
        no_commit: bool,
    },

    /// Refresh interface files (backup, update templates, restore sessions)
    Refresh {
        /// Interface name (claude, gemini, opencode)
        name: String,

        /// Skip automatic git commit
        #[arg(long)]
        no_commit: bool,
    },

    /// Health check all configured interfaces
    Doctor,

    /// Configure MCP server for an interface
    Mcp {
        /// Interface name (claude)
        name: String,

        /// Remove MCP configuration instead of adding
        #[arg(long)]
        remove: bool,
    },
}

/// Execute the interface compatibility command (main entry point from CLI)
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

/// List available interfaces (global) and allowed interfaces (project)
fn list() -> Result<()> {
    // Show global interfaces
    let adapter_list = interfaces::list()?;
    println!("📱 Available AI Interfaces (Global)\n");
    println!("{:<12} {:<15} {:<10} VERSION", "NAME", "DISPLAY", "STATUS");
    println!("{}", "─".repeat(50));
    for adapter in adapter_list {
        let status = if adapter.detected {
            "✓ found"
        } else {
            "✗ missing"
        };
        let version = adapter.version.unwrap_or_else(|| "-".to_string());
        println!(
            "{:<12} {:<15} {:<10} {}",
            adapter.name, adapter.display, status, version
        );
    }

    let default = interfaces::default_interface_name()?;
    println!("\nGlobal default: {}", default);

    // Show project interfaces if in a patina project
    let cwd = std::env::current_dir()?;
    if project::is_patina_project(&cwd) {
        let config = project::load_with_migration(&cwd)?;
        println!("\n📁 Project Allowed Interfaces\n");
        println!("Allowed: {:?}", config.adapters.allowed);
        println!("Project default: {}", config.adapters.default);
    }

    Ok(())
}

/// Set default interface (global or project-level)
fn set_default(name: &str, is_project: bool) -> Result<()> {
    if is_project {
        // Set project default
        let cwd = std::env::current_dir()?;
        if !project::is_patina_project(&cwd) {
            anyhow::bail!("Not a patina project. Run `patina init .` first.");
        }
        let mut config = project::load_with_migration(&cwd)?;
        if !config.adapters.allowed.contains(&name.to_string()) {
            anyhow::bail!(
                "Interface '{}' is not in allowed list. Prepare the Patina AI surface first with: patina ai setup",
                name,
            );
        }
        config.adapters.default = name.to_string();
        project::save(&cwd, &config)?;
        println!("✓ Project default interface set to: {}", name);
    } else {
        // Set global default
        interfaces::set_default_interface(name)?;
        println!("✓ Global default interface set to: {}", name);
    }
    Ok(())
}

/// Check interface installation status
fn check(name: Option<&str>) -> Result<()> {
    if let Some(n) = name {
        let adapter = interfaces::get(n)?;
        if adapter.detected {
            println!("✓ {} is installed", adapter.display);
            if let Some(v) = adapter.version {
                println!("  Version: {}", v);
            }
        } else {
            println!("✗ {} is not installed", adapter.display);
        }
    } else {
        // Check all
        for adapter in interfaces::list()? {
            let status = if adapter.detected { "✓" } else { "✗" };
            println!("{} {}", status, adapter.display);
        }
    }
    Ok(())
}

/// Add an interface to project's allowed list
fn add(name: &str, no_commit: bool) -> Result<()> {
    // Verify interface exists
    let _ = interfaces::get(name)?;

    let cwd = std::env::current_dir()?;
    if !project::is_patina_project(&cwd) {
        anyhow::bail!("Not a patina project. Run `patina init .` first.");
    }

    if interface::is_supported_ai_interface(name) {
        let config = project::load_with_migration(&cwd)?;
        let default_interface = if config.adapters.default.is_empty() {
            Some(name)
        } else {
            None
        };

        println!("  Compatibility shim: deploying the Patina AI bundles...");
        interface::ensure_ai_surface(interface::AiSurfaceRequest {
            project_root: &cwd,
            force: false,
            default_interface,
        })?;
        println!("  ✓ Deployed Patina AI bundles");

        if !no_commit {
            println!("\n📦 Committing interface setup...");
            let mut files_to_add = Vec::new();
            collect_ai_surface_paths(&cwd, &mut files_to_add);
            if cwd.join(".patina/config.toml").exists() {
                files_to_add.push(".patina/config.toml".to_string());
            }

            let refs: Vec<&str> = files_to_add.iter().map(|s| s.as_str()).collect();
            patina::git::add_paths(&refs)?;
            if patina::git::has_staged_changes()? {
                patina::git::commit(&format!("chore: prepare patina ai surface via {}", name))?;
                println!("✓ Committed interface files");
            } else {
                println!("  ℹ No trackable changes to commit");
            }
        }

        return Ok(());
    }

    let mut config = project::load_with_migration(&cwd)?;
    let already_allowed = config.adapters.allowed.contains(&name.to_string());

    if !already_allowed {
        config.adapters.allowed.push(name.to_string());
        if config.adapters.default.is_empty() {
            config.adapters.default = name.to_string();
        }
        project::save(&cwd, &config)?;
        println!("✓ Added '{}' to allowed interfaces", name);
        println!("  Allowed: {:?}", config.adapters.allowed);
    } else {
        println!("Interface '{}' is already in allowed list.", name);
    }

    let adapter_dir = cwd.join(format!(".{}", name));
    let bootstrap_file = get_bootstrap_filename(name);
    let bootstrap_path = cwd.join(&bootstrap_file);
    let created_files = !adapter_dir.exists() || !bootstrap_path.exists();

    {
        if !adapter_dir.exists() {
            println!("  Creating .{}/ directory...", name);
            patina::interface::runtime::templates::copy_to_project(name, &cwd)?;
            println!("  ✓ Created interface files");
        }

        // Create bootstrap file (CLAUDE.md, GEMINI.md, etc.) if it doesn't exist
        if !bootstrap_path.exists() {
            println!("  Creating {}...", bootstrap_file);
            interfaces::generate_bootstrap(name, &cwd, false)?;
            println!("  ✓ Created {}", bootstrap_file);
        }
    }

    // Commit if files were created and not in no_commit mode
    if created_files && !no_commit {
        println!("\n📦 Committing interface setup...");
        let mut files_to_add = Vec::new();
        collect_adapter_surface_paths(&cwd, name, &mut files_to_add);
        if cwd.join(".patina/config.toml").exists() {
            files_to_add.push(".patina/config.toml".to_string());
        }

        let refs: Vec<&str> = files_to_add.iter().map(|s| s.as_str()).collect();
        patina::git::add_paths(&refs)?;
        patina::git::commit(&format!("chore: add {} interface", name))?;
        println!("✓ Committed interface files");
    }

    Ok(())
}

/// Remove an interface from project's allowed list
fn remove(name: &str, no_backup: bool, _no_commit: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    if !project::is_patina_project(&cwd) {
        anyhow::bail!("Not a patina project. Run `patina init .` first.");
    }

    let mut config = project::load_with_migration(&cwd)?;
    if !config.adapters.allowed.contains(&name.to_string()) {
        println!("Interface '{}' is not in allowed list.", name);
        return Ok(());
    }

    // Backup files if requested
    if !no_backup {
        backup_adapter_files(&cwd, name)?;
    }

    // Remove from allowed list
    config.adapters.allowed.retain(|a| a != name);

    // Update default if we removed it
    if config.adapters.default == name {
        config.adapters.default = config.adapters.allowed.first().cloned().unwrap_or_default();
        if !config.adapters.default.is_empty() {
            println!("  ✓ Default changed to: {}", config.adapters.default);
        }
    }

    project::save(&cwd, &config)?;

    println!("✓ Removed '{}' from allowed interfaces", name);
    println!("  Allowed: {:?}", config.adapters.allowed);
    println!(
        "\n💡 To also remove files: rm -rf .{}/ {}",
        name,
        get_bootstrap_filename(name)
    );

    // Note: We don't auto-commit removal since files still exist
    // User should manually delete files and commit

    Ok(())
}

/// Get the bootstrap filename for an interface (CLAUDE.md, GEMINI.md, etc.)
fn get_bootstrap_filename(name: &str) -> String {
    match name {
        "claude" => "CLAUDE.md".to_string(),
        "gemini" => "GEMINI.md".to_string(),
        "codex" => "AGENTS.md".to_string(),
        "opencode" => "AGENTS.md".to_string(),
        _ => format!("{}.md", name.to_uppercase()),
    }
}

fn collect_adapter_surface_paths(
    project_root: &std::path::Path,
    name: &str,
    out: &mut Vec<String>,
) {
    let adapter_dir = format!(".{}", name);
    if project_root.join(&adapter_dir).exists() {
        out.push(adapter_dir);
    }

    let bootstrap_file = get_bootstrap_filename(name);
    if project_root.join(&bootstrap_file).exists() {
        out.push(bootstrap_file);
    }

    if interface::is_supported_ai_interface(name)
        && project_root.join("AGENTS.md").exists()
        && !out.iter().any(|path| path == "AGENTS.md")
    {
        out.push("AGENTS.md".to_string());
    }

    if matches!(name, "gemini" | "opencode")
        && project_root.join("AGENTS.md").exists()
        && !out.iter().any(|path| path == "AGENTS.md")
    {
        out.push("AGENTS.md".to_string());
    }
}

fn collect_ai_surface_paths(project_root: &std::path::Path, out: &mut Vec<String>) {
    for interface_name in interface::supported_ai_interfaces() {
        collect_adapter_surface_paths(project_root, interface_name, out);
    }
}

/// Refresh interface files - backup, update templates, restore sessions
fn refresh(name: &str, no_commit: bool) -> Result<()> {
    // Verify interface exists
    let _ = interfaces::get(name)?;

    let cwd = std::env::current_dir()?;
    if !project::is_patina_project(&cwd) {
        anyhow::bail!("Not a patina project. Run `patina init .` first.");
    }

    let config = project::load_with_migration(&cwd)?;
    if !config.adapters.allowed.contains(&name.to_string()) {
        anyhow::bail!(
            "Interface '{}' is not in allowed list. Deploy the Patina AI bundles first with: patina ai setup",
            name,
        );
    }

    println!("🔄 Refreshing {} bundle projection...\n", name);

    if interface::is_supported_ai_interface(name) {
        interface::ensure_ai_surface(interface::AiSurfaceRequest {
            project_root: &cwd,
            force: false,
            default_interface: None,
        })?;

        if !no_commit {
            println!("\n📦 Committing refresh...");
            let mut files_to_add = Vec::new();
            collect_ai_surface_paths(&cwd, &mut files_to_add);
            let refs: Vec<&str> = files_to_add.iter().map(|s| s.as_str()).collect();
            patina::git::add_paths(&refs)?;

            if patina::git::has_staged_changes()? {
                patina::git::commit(&format!("chore: refresh patina ai surface via {}", name))?;
                println!("  ✓ Committed interface refresh");
            } else {
                println!("  ℹ No trackable changes to commit (interface dir may be gitignored)");
            }
        }

        println!("\n✨ Patina AI bundles refreshed successfully!");
        return Ok(());
    }

    // Step 1: Backup existing files (including session files)
    println!("📦 Backing up existing files...");
    backup_adapter_files(&cwd, name)?;

    // Step 2: Preserve user files before removing interface directory
    let adapter_dir = cwd.join(format!(".{}", name));
    let preserved_files = preserve_user_files(&adapter_dir)?;

    // Step 3: Remove old interface directory
    if adapter_dir.exists() {
        std::fs::remove_dir_all(&adapter_dir)?;
        println!("  ✓ Removed old .{}/ directory", name);
    }

    // Step 4: Copy fresh templates
    println!("\n📋 Copying fresh templates...");
    patina::interface::runtime::templates::copy_to_project(name, &cwd)?;
    println!("  ✓ Copied fresh interface files");

    // Create/refresh bootstrap file (CLAUDE.md, GEMINI.md, etc.)
    let bootstrap_file = get_bootstrap_filename(name);
    println!("  Generating {}...", bootstrap_file);
    interfaces::generate_bootstrap(name, &cwd, false)?;
    println!("  ✓ Created {}", bootstrap_file);

    // Step 5: Restore preserved user files
    if !preserved_files.is_empty() {
        println!("\n📁 Restoring user files...");
        restore_user_files(&adapter_dir, &preserved_files)?;
        for (path, _) in &preserved_files {
            println!("  ✓ Restored {}", path);
        }
    }

    // Step 6: Commit if not in no_commit mode
    if !no_commit {
        println!("\n📦 Committing refresh...");
        let mut files_to_add = Vec::new();
        collect_adapter_surface_paths(&cwd, name, &mut files_to_add);

        let refs: Vec<&str> = files_to_add.iter().map(|s| s.as_str()).collect();
        patina::git::add_paths(&refs)?;

        // Only commit if there are staged changes (paths may have been gitignored)
        if patina::git::has_staged_changes()? {
            patina::git::commit(&format!("chore: refresh {} interface", name))?;
            println!("  ✓ Committed interface refresh");
        } else {
            println!("  ℹ No trackable changes to commit (interface dir may be gitignored)");
        }
    }

    println!("\n✨ {} interface refreshed successfully!", name);
    Ok(())
}

/// Template-managed command files (not user-created)
const TEMPLATE_COMMANDS: &[&str] = &[
    "session-start.md",
    "session-update.md",
    "session-note.md",
    "session-end.md",
    "patina-review.md",
    "spec.md",
];

/// Root-level user files to preserve across refresh (explicit allowlist)
const USER_ROOT_FILES: &[&str] = &["settings.local.json"];

/// Template-managed skill directories (not user-created)
const TEMPLATE_SKILLS: &[&str] = &["epistemic-beliefs"];

/// Preserve user files from interface directory (context, custom commands, custom skills)
fn preserve_user_files(adapter_dir: &std::path::Path) -> Result<Vec<(String, Vec<u8>)>> {
    let mut preserved = Vec::new();

    // Preserve context/ files (session state)
    let context_dir = adapter_dir.join("context");
    if context_dir.exists() {
        for entry in std::fs::read_dir(&context_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                if filename.contains("session") || filename.ends_with(".md") {
                    let content = std::fs::read(&path)?;
                    preserved.push((format!("context/{}", filename), content));
                }
            }
        }
    }

    // Preserve custom commands/ (user-created, not template-managed)
    let commands_dir = adapter_dir.join("commands");
    if commands_dir.exists() {
        for entry in std::fs::read_dir(&commands_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                // Only preserve if NOT a template-managed file
                if !TEMPLATE_COMMANDS.contains(&filename.as_str()) {
                    let content = std::fs::read(&path)?;
                    preserved.push((format!("commands/{}", filename), content));
                }
            }
        }
    }

    // Preserve custom skills/ (user-created directories, not template-managed)
    let skills_dir = adapter_dir.join("skills");
    if skills_dir.exists() {
        for entry in std::fs::read_dir(&skills_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let dirname = path.file_name().unwrap().to_string_lossy().to_string();
                // Only preserve if NOT a template-managed skill
                if !TEMPLATE_SKILLS.contains(&dirname.as_str()) {
                    preserve_directory_recursive(
                        &path,
                        &format!("skills/{}", dirname),
                        &mut preserved,
                    )?;
                }
            }
        }
    }

    // Preserve root-level user files (settings, config)
    for filename in USER_ROOT_FILES {
        let path = adapter_dir.join(filename);
        if path.exists() {
            let content = std::fs::read(&path)?;
            preserved.push((filename.to_string(), content));
        }
    }

    Ok(preserved)
}

/// Recursively preserve all files in a directory
fn preserve_directory_recursive(
    dir: &std::path::Path,
    prefix: &str,
    preserved: &mut Vec<(String, Vec<u8>)>,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let relative = format!("{}/{}", prefix, name);

        if path.is_file() {
            let content = std::fs::read(&path)?;
            preserved.push((relative, content));
        } else if path.is_dir() {
            preserve_directory_recursive(&path, &relative, preserved)?;
        }
    }
    Ok(())
}

/// Restore preserved user files to interface directory
fn restore_user_files(adapter_dir: &std::path::Path, files: &[(String, Vec<u8>)]) -> Result<()> {
    for (relative_path, content) in files {
        let full_path = adapter_dir.join(relative_path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full_path, content)?;
    }
    Ok(())
}

/// Health check all configured interfaces
fn doctor() -> Result<()> {
    let cwd = std::env::current_dir()?;
    if !project::is_patina_project(&cwd) {
        anyhow::bail!("Not a patina project. Run `patina init .` first.");
    }

    let config = project::load_with_migration(&cwd)?;

    println!("🩺 Interface Health Check\n");

    if config.adapters.allowed.is_empty() {
        println!("⚠️  No interfaces configured.");
        println!("   Run: patina ai setup");
        return Ok(());
    }

    let mut all_healthy = true;

    for adapter_name in &config.adapters.allowed {
        println!("📱 {} interface:", adapter_name);

        // Check 1: Interface CLI installed on system
        let adapter_info = interfaces::get(adapter_name);
        match adapter_info {
            Ok(a) if a.detected => {
                println!("  ✓ CLI installed: {}", a.version.unwrap_or_default());
            }
            Ok(_) => {
                println!("  ✗ CLI not found on system");
                all_healthy = false;
            }
            Err(_) => {
                println!("  ✗ Unknown interface type");
                all_healthy = false;
            }
        }

        // Check 2: Interface directory exists
        let adapter_dir = cwd.join(format!(".{}", adapter_name));
        if adapter_dir.exists() {
            println!("  ✓ .{}/ directory exists", adapter_name);
        } else {
            println!("  ✗ .{}/ directory missing", adapter_name);
            println!("    Fix: patina ai refresh {}", adapter_name);
            all_healthy = false;
        }

        // Check 3: Bootstrap file exists
        let bootstrap_file = get_bootstrap_filename(adapter_name);
        let bootstrap_path = cwd.join(&bootstrap_file);
        if bootstrap_path.exists() {
            println!("  ✓ {} exists", bootstrap_file);
        } else {
            println!("  ✗ {} missing", bootstrap_file);
            println!("    Fix: patina ai refresh {}", adapter_name);
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
        println!("✅ All interfaces healthy!");
    } else {
        println!("⚠️  Some issues found. See above for fixes.");
    }

    Ok(())
}

/// Check if MCP is configured for Claude
fn check_mcp_configured() -> Result<bool> {
    use std::process::Command;

    let output = Command::new("claude").args(["mcp", "list"]).output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains("patina"))
    } else {
        Ok(false)
    }
}

/// Backup interface-specific files before removal or refresh
fn backup_adapter_files(project_root: &std::path::Path, name: &str) -> Result<()> {
    let bootstrap_file = get_bootstrap_filename(name);
    let file_path = project_root.join(&bootstrap_file);
    if let Some(backup_path) = project::backup_file(project_root, &file_path)? {
        println!(
            "  ✓ Backed up {} to {}",
            bootstrap_file,
            backup_path.display()
        );
    }

    // Backup interface directory (.claude/, .gemini/, etc.) to .patina/local/backups/
    let adapter_dir = project_root.join(format!(".{}", name));
    if adapter_dir.exists() {
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let backup_dir = project::backups_dir(project_root).join(format!("{}-{}", name, timestamp));
        std::fs::create_dir_all(&backup_dir)?;

        // Copy interface directory contents
        copy_dir_recursive(&adapter_dir, &backup_dir)?;
        println!("  ✓ Backed up .{}/ to {}", name, backup_dir.display());
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

/// Configure MCP server for an interface
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
            anyhow::bail!("Unknown interface: {}. Supported: claude", name);
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
            no_commit: false,
        };
        assert!(matches!(add, AdapterCommands::Add { .. }));
    }
}
