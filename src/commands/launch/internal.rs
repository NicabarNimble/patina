//! Internal implementation for launch command
//!
//! Handles the launch flow: workspace check → mother → project check → bootstrap → launch

use anyhow::{bail, Context, Result};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use patina::adapters::launch as adapters;
use patina::git;
use patina::project;
use patina::workspace;

use super::LaunchOptions;

/// Main launch entry point
pub fn launch(options: LaunchOptions) -> Result<()> {
    // Step 1: Ensure workspace exists (first-run setup)
    if workspace::is_first_run() {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!(" Welcome to Patina!");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        workspace::setup()?;
        println!();
    }

    // Step 2: Determine project path
    let project_path = resolve_project_path(options.path.as_deref())?;

    // Step 3: Handle explicit vs implicit adapter (per spec-adapter-selection.md)
    let explicit_adapter: Option<String> = options.adapter.clone();

    // If explicit adapter specified, validate it's installed NOW (fail fast)
    if let Some(ref name) = explicit_adapter {
        let adapter_info = adapters::get(name)?;
        if !adapter_info.detected {
            bail!(
                "Adapter '{}' ({}) is not installed.\n\
                 Install it and try again, or use a different adapter.",
                name,
                adapter_info.display
            );
        }
    }

    // Step 4: Check/start mother
    if options.auto_start_mother {
        ensure_mother_running()?;
    }

    // Step 5: Check if this is a patina project
    let patina_dir = project_path.join(".patina");
    let adapter_name: String;

    if !patina_dir.exists() {
        if options.auto_init {
            // Pass explicit_adapter - if Some, skip selection prompt
            match prompt_are_you_lost(&project_path, explicit_adapter.as_deref())? {
                Some(selected) => {
                    // Update adapter_name to what user selected (or explicit)
                    adapter_name = selected;
                }
                None => {
                    // User declined
                    return Ok(());
                }
            }
        } else {
            bail!(
                "Not a patina project (no .patina/ directory).\n\
                 Run `patina init .` first."
            );
        }
    } else {
        // Existing project - resolve adapter name
        // Priority: explicit flag > project default > global default
        let project_config = project::load_with_migration(&project_path)?;
        adapter_name = explicit_adapter.unwrap_or_else(|| {
            // Use project default if set, otherwise fall back to global
            if !project_config.adapters.default.is_empty() {
                project_config.adapters.default.clone()
            } else {
                adapters::default_name().unwrap_or_else(|_| "claude".to_string())
            }
        });

        // Validate adapter is installed
        let adapter_info = adapters::get(&adapter_name)?;
        if !adapter_info.detected {
            bail!(
                "Adapter '{}' ({}) is not installed.\n\
                 Install it and try again, or use a different adapter.",
                adapter_name,
                adapter_info.display
            );
        }

        println!(
            "🚀 Launching {} in {}",
            adapter_info.display,
            project_path.display()
        );
    }

    // Step 6.5: Branch safety - ensure we're on patina branch
    match ensure_on_patina_branch()? {
        BranchAction::AlreadyOnPatina => {
            // Good, already there
        }
        BranchAction::Switched { .. } | BranchAction::StashedAndSwitched { .. } => {
            // Successfully switched, messages already printed
        }
        BranchAction::Rebased { .. } => {
            // Successfully rebased, messages already printed
        }
        BranchAction::RebaseConflicts => {
            // Cannot continue with conflicts
            bail!("Please resolve rebase conflicts before launching.");
        }
        BranchAction::NotGitRepo => {
            // Not a git repo but has .patina/ - unusual but allow
            println!("⚠️  Not a git repository (patina branch model disabled)");
        }
        BranchAction::NoPatinaExists => {
            // Has .patina/ but no patina branch - legacy project or manual setup
            // Allow but warn
            println!("⚠️  No 'patina' branch found (working on current branch)");
        }
    }

    // Step 7: Check if adapter is in allowed list
    let project_config = project::load_with_migration(&project_path)?;
    if !project_config.adapters.allowed.contains(&adapter_name) {
        bail!(
            "Adapter '{}' is not in allowed adapters for this project.\n\
             Allowed: {:?}\n\n\
             To add it, run: patina adapter add {}",
            adapter_name,
            project_config.adapters.allowed,
            adapter_name
        );
    }

    // Step 7.5: Silent MCP auto-configuration (self-healing)
    // If MCP isn't configured, silently fix it. Errors are ignored - if it fails,
    // user will notice when MCP tools don't work, but we don't block launch.
    if !adapters::is_mcp_configured(&adapter_name).unwrap_or(true) {
        let _ = adapters::configure_mcp(&adapter_name);
    }

    // Step 8: Ensure bootstrap file exists
    let bootstrap_file = match adapter_name.as_str() {
        "claude" => "CLAUDE.md",
        "gemini" => "GEMINI.md",
        "opencode" => "OPENCODE.md",
        _ => "CLAUDE.md",
    };
    let bootstrap_path = project_path.join(bootstrap_file);
    if !bootstrap_path.exists() {
        println!("  ✓ Generating {} bootstrap", bootstrap_file);
        adapters::generate_bootstrap(&adapter_name, &project_path)?;
    }

    // Step 9: Launch adapter
    launch_adapter_cli(&adapter_name, &project_path)?;

    Ok(())
}

/// Resolve project path from options or current directory
fn resolve_project_path(path_opt: Option<&str>) -> Result<PathBuf> {
    let path = match path_opt {
        Some(p) => PathBuf::from(shellexpand::tilde(p).as_ref()),
        None => env::current_dir().context("Failed to get current directory")?,
    };

    // Canonicalize to resolve symlinks
    let canonical = fs::canonicalize(&path)
        .with_context(|| format!("Project path does not exist: {}", path.display()))?;

    Ok(canonical)
}

/// Check if mother is running via health endpoint
pub fn check_mother_health() -> bool {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok();

    if let Some(client) = client {
        client
            .get("http://127.0.0.1:50051/health")
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    } else {
        false
    }
}

/// Ensure mother is running, start if needed
fn ensure_mother_running() -> Result<()> {
    if check_mother_health() {
        println!("  ✓ Mother running");
        return Ok(());
    }

    println!("  ⏳ Starting mother...");
    start_mother_daemon()?;

    // Wait for it to come up
    for _ in 0..10 {
        thread::sleep(Duration::from_millis(500));
        if check_mother_health() {
            println!("  ✓ Mother started");
            return Ok(());
        }
    }

    bail!("Failed to start mother daemon")
}

/// Start mother as background daemon
pub fn start_mother_daemon() -> Result<()> {
    // Get the path to the patina binary
    let patina_bin = env::current_exe()?;

    // Spawn serve in background
    Command::new(&patina_bin)
        .args(["serve"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to start mother daemon")?;

    Ok(())
}

/// "Are you lost?" prompt - show git context and offer to initialize.
///
/// Returns:
/// - Ok(None) - user declined to init
/// - Ok(Some(adapter_name)) - user accepted, project initialized with this adapter
///
/// If `explicit_adapter` is Some, uses that adapter without prompting for selection.
/// If None, detects available adapters and prompts user to choose.
fn prompt_are_you_lost(
    project_path: &Path,
    explicit_adapter: Option<&str>,
) -> Result<Option<String>> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Are you lost?");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("This is not a patina project.\n");

    // Show path
    println!("📁 Path: {}", project_path.display());

    // Show git context if available
    if git::is_git_repo().unwrap_or(false) {
        let branch = git::current_branch().unwrap_or_else(|_| "unknown".to_string());
        let clean = git::is_clean().unwrap_or(true);
        let status = if clean {
            "clean".to_string()
        } else {
            let count = git::status_count().unwrap_or(0);
            format!("{} files modified", count)
        };
        println!("🔀 Git:  {} ({})", branch, status);

        // Show remote if available
        if let Ok(url) = git::remote_url("origin") {
            let display_url = format_remote_url(&url);
            println!("🌐 Remote: {}", display_url);
        }
    } else {
        println!("🔀 Git:  not a git repository");
    }

    println!();
    print!("Initialize as patina project? [y/N]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let should_init = input.trim().to_lowercase() == "y";

    if !should_init {
        return Ok(None);
    }

    // User wants to init - determine which adapter to use
    let adapter_name = if let Some(explicit) = explicit_adapter {
        // Flow A: explicit adapter from --adapter flag
        explicit.to_string()
    } else {
        // Flow B: detect available adapters and let user choose
        let all_adapters = adapters::list()?;
        let available: Vec<_> = all_adapters.into_iter().filter(|a| a.detected).collect();

        // Get global default as preference
        let preference = adapters::default_name().ok();

        adapters::select_adapter(&available, preference.as_deref())?
    };

    // Initialize the project with selected adapter
    println!();
    if initialize_project(project_path, &adapter_name)? {
        Ok(Some(adapter_name))
    } else {
        Ok(None)
    }
}

/// Format remote URL for display (strip git@/https://, .git suffix)
fn format_remote_url(url: &str) -> String {
    url.trim()
        .strip_prefix("git@")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(url)
        .replace(":", "/")
        .strip_suffix(".git")
        .unwrap_or(url)
        .to_string()
}

/// Branch safety result indicating what action was taken
#[derive(Debug)]
pub enum BranchAction {
    /// Already on patina, no action needed
    AlreadyOnPatina,
    /// Switched to patina (was clean)
    Switched { _from: String },
    /// Stashed and switched to patina
    StashedAndSwitched { _from: String, _stash_name: String },
    /// Rebased patina onto main
    Rebased { _commits: usize },
    /// Conflicts during rebase - user must resolve
    RebaseConflicts,
    /// Not a git repo
    NotGitRepo,
    /// No patina branch exists
    NoPatinaExists,
}

/// Ensure we're on patina branch using "Do and Inform" model
/// Returns the action taken so caller can display appropriate message
fn ensure_on_patina_branch() -> Result<BranchAction> {
    // Check if this is a git repo
    if !git::is_git_repo()? {
        return Ok(BranchAction::NotGitRepo);
    }

    let current = git::current_branch()?;

    // Check if patina branch exists
    if !git::branch_exists("patina")? {
        return Ok(BranchAction::NoPatinaExists);
    }

    // Already on patina?
    if current == "patina" {
        // Try to fetch to get latest
        let _ = git::fetch("origin"); // Ignore fetch errors (might be offline)

        // Check if behind origin/patina (not main!) and auto-rebase
        // Rebasing onto main was wrong - it linearizes history and breaks merges
        // We only want to sync local patina with remote patina
        let behind = git::commits_behind("patina", "origin/patina").unwrap_or(0);

        if behind > 0 {
            println!(
                "\n📥 Patina branch is {} commits behind origin/patina",
                behind
            );
            println!("   Rebasing onto origin/patina...");

            if git::rebase("origin/patina")? {
                println!("   ✓ Rebased ({} commits)", behind);
                return Ok(BranchAction::Rebased { _commits: behind });
            } else {
                println!("   ✗ Rebase failed (conflicts)");
                println!();
                println!("   To resolve:");
                println!("   1. Fix conflicts");
                println!("   2. git add <files>");
                println!("   3. git rebase --continue");
                println!();
                println!("   Or abort: git rebase --abort");
                return Ok(BranchAction::RebaseConflicts);
            }
        }

        return Ok(BranchAction::AlreadyOnPatina);
    }

    // On another branch, patina exists - need to switch
    let clean = git::is_clean()?;

    if clean {
        // Clean working tree - just switch
        println!("\n🔀 Switching to patina branch...");
        git::checkout("patina")?;
        println!("   ✓ Switched to patina");
        return Ok(BranchAction::Switched { _from: current });
    }

    // Dirty working tree - stash first
    let timestamp = git::timestamp();
    let stash_name = format!("patina-autostash-{}", timestamp);

    println!("\n📦 Stashing changes on '{}'...", current);
    git::stash_push(&stash_name)?;
    println!("   ✓ Stashed: \"{}\"", stash_name);

    println!("🔀 Switching to patina branch...");
    git::checkout("patina")?;
    println!("   ✓ Switched to patina");

    println!();
    println!("────────────────────────────────────────────────");
    println!("💡 Your changes on '{}' are stashed.", current);
    println!("   To restore: git checkout {} && git stash pop", current);
    println!("────────────────────────────────────────────────");

    Ok(BranchAction::StashedAndSwitched {
        _from: current,
        _stash_name: stash_name,
    })
}

/// Initialize project from the "Are you lost?" prompt
fn initialize_project(project_path: &Path, adapter_name: &str) -> Result<bool> {
    // Change to project directory for init
    let original_dir = env::current_dir()?;
    env::set_current_dir(project_path)?;

    // Step 1: Create skeleton
    let init_result = crate::commands::init::execute(
        ".".to_string(), // Use "." to trigger commit step in init
        false,           // force
        true,            // local (skip GitHub integration for quick init)
        false,           // no_commit (allow auto-commit)
    );

    if let Err(e) = init_result {
        env::set_current_dir(original_dir)?;
        eprintln!("\n❌ Failed to initialize: {}", e);
        return Ok(false);
    }

    // Step 2: Add the adapter
    let adapter_result =
        crate::commands::adapter::execute(Some(crate::commands::adapter::AdapterCommands::Add {
            name: adapter_name.to_string(),
            no_commit: false, // Allow auto-commit during launch init
        }));

    if let Err(e) = adapter_result {
        env::set_current_dir(original_dir)?;
        eprintln!("\n❌ Failed to add adapter: {}", e);
        eprintln!(
            "   Run 'patina adapter add {}' to add it manually",
            adapter_name
        );
        return Ok(false);
    }

    // Step 3: Set as project default (so `patina` uses this adapter next time)
    let mut config = project::load_with_migration(project_path)?;
    config.adapters.default = adapter_name.to_string();
    project::save(project_path, &config)?;

    // Restore original directory
    env::set_current_dir(original_dir)?;

    println!(
        "\n✓ Initialized as patina project with {} adapter",
        adapter_name
    );
    Ok(true) // Continue to launch
}

/// Launch the adapter CLI
fn launch_adapter_cli(adapter_name: &str, project_path: &Path) -> Result<()> {
    println!("\nLaunching {}...\n", adapter_name);

    // Use exec to replace current process (Unix-style)
    // On Windows, we'd spawn and wait instead
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = Command::new(adapter_name).current_dir(project_path).exec();
        // exec only returns on error
        bail!("Failed to exec {}: {}", adapter_name, err);
    }

    #[cfg(not(unix))]
    {
        let status = Command::new(adapter_name)
            .current_dir(project_path)
            .status()
            .with_context(|| format!("Failed to run {}", adapter_name))?;

        if !status.success() {
            bail!("{} exited with status: {}", adapter_name, status);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_current_dir() {
        let path = resolve_project_path(None);
        assert!(path.is_ok());
        assert!(path.unwrap().is_absolute());
    }

    #[test]
    fn test_resolve_tilde_path() {
        let path = resolve_project_path(Some("~"));
        // This should work if home dir exists
        if let Ok(p) = path {
            assert!(p.is_absolute());
        }
    }
}
