//! Internal implementation for launch command
//!
//! Handles the launch flow: workspace check → mothership → project check → bootstrap → launch

use anyhow::{bail, Context, Result};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use patina::adapters::launch as frontend;
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
    let project_path = resolve_project_path(&options.path)?;

    // Step 3: Determine frontend
    let frontend_name = options
        .frontend
        .unwrap_or_else(|| frontend::default_name().unwrap_or_else(|_| "claude".to_string()));

    // Step 4: Check if frontend is available
    let frontend_info = frontend::get(&frontend_name)?;
    if !frontend_info.detected {
        bail!(
            "Frontend '{}' ({}) is not installed.\n\
             Install it and try again, or use a different frontend:\n\
             patina launch <frontend>",
            frontend_name,
            frontend_info.display
        );
    }

    println!(
        "🚀 Launching {} in {}",
        frontend_info.display,
        project_path.display()
    );

    // Step 5: Check/start mothership
    if options.auto_start_mothership {
        ensure_mothership_running()?;
    }

    // Step 6: Check if this is a patina project
    let patina_dir = project_path.join(".patina");
    if !patina_dir.exists() {
        if options.auto_init {
            if prompt_init(&project_path)? {
                println!("\n💡 Run `patina init .` first to initialize this project.");
                return Ok(());
            }
        } else {
            bail!(
                "Not a patina project (no .patina/ directory).\n\
                 Run `patina init .` first."
            );
        }
    }

    // Step 7: Check if frontend is in allowed list
    let project_config = project::load_with_migration(&project_path)?;
    if !project_config.frontends.allowed.contains(&frontend_name) {
        bail!(
            "Frontend '{}' is not in allowed frontends for this project.\n\
             Allowed: {:?}\n\n\
             To add it, run: patina adapter add {}",
            frontend_name,
            project_config.frontends.allowed,
            frontend_name
        );
    }

    // Step 8: Ensure bootstrap file exists
    let bootstrap_file = match frontend_name.as_str() {
        "claude" => "CLAUDE.md",
        "gemini" => "GEMINI.md",
        "codex" => "CODEX.md",
        _ => "CLAUDE.md",
    };
    let bootstrap_path = project_path.join(bootstrap_file);
    if !bootstrap_path.exists() {
        println!("  ✓ Generating {} bootstrap", bootstrap_file);
        frontend::generate_bootstrap(&frontend_name, &project_path)?;
    }

    // Step 9: Launch frontend
    launch_frontend_cli(&frontend_name, &project_path)?;

    Ok(())
}

/// Resolve project path from options or current directory
fn resolve_project_path(path_opt: &Option<String>) -> Result<PathBuf> {
    let path = match path_opt {
        Some(p) => {
            let expanded = shellexpand::tilde(p);
            PathBuf::from(expanded.as_ref())
        }
        None => env::current_dir()?,
    };

    // Canonicalize to resolve symlinks
    let canonical = fs::canonicalize(&path)
        .with_context(|| format!("Project path does not exist: {}", path.display()))?;

    Ok(canonical)
}

/// Check if mothership is running via health endpoint
pub fn check_mothership_health() -> bool {
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

/// Ensure mothership is running, start if needed
fn ensure_mothership_running() -> Result<()> {
    if check_mothership_health() {
        println!("  ✓ Mothership running");
        return Ok(());
    }

    println!("  ⏳ Starting mothership...");
    start_mothership_daemon()?;

    // Wait for it to come up
    for _ in 0..10 {
        thread::sleep(Duration::from_millis(500));
        if check_mothership_health() {
            println!("  ✓ Mothership started");
            return Ok(());
        }
    }

    bail!("Failed to start mothership daemon")
}

/// Start mothership as background daemon
pub fn start_mothership_daemon() -> Result<()> {
    // Get the path to the patina binary
    let patina_bin = env::current_exe()?;

    // Spawn serve in background
    Command::new(&patina_bin)
        .args(["serve"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to start mothership daemon")?;

    Ok(())
}

/// Prompt user to initialize project
fn prompt_init(_project_path: &Path) -> Result<bool> {
    print!(
        "This directory is not a patina project.\n\
         Initialize it? [y/N]: "
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let should_init = input.trim().to_lowercase() == "y";

    if should_init {
        // Could call init here, but for now just tell user to do it
        // This keeps the flow simple and avoids circular dependencies
        return Ok(true);
    }

    Ok(false)
}

/// Launch the frontend CLI
fn launch_frontend_cli(frontend_name: &str, project_path: &Path) -> Result<()> {
    println!("\nLaunching {}...\n", frontend_name);

    // Use exec to replace current process (Unix-style)
    // On Windows, we'd spawn and wait instead
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = Command::new(frontend_name).current_dir(project_path).exec();
        // exec only returns on error
        bail!("Failed to exec {}: {}", frontend_name, err);
    }

    #[cfg(not(unix))]
    {
        let status = Command::new(frontend_name)
            .current_dir(project_path)
            .status()
            .with_context(|| format!("Failed to run {}", frontend_name))?;

        if !status.success() {
            bail!("{} exited with status: {}", frontend_name, status);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_current_dir() {
        let path = resolve_project_path(&None);
        assert!(path.is_ok());
        assert!(path.unwrap().is_absolute());
    }

    #[test]
    fn test_resolve_tilde_path() {
        let path = resolve_project_path(&Some("~".to_string()));
        // This should work if home dir exists
        if let Ok(p) = path {
            assert!(p.is_absolute());
        }
    }
}
