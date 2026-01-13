// Shared utilities for all scrape subcommands

pub mod code;
pub mod database;
pub mod forge;
pub mod git;
pub mod layer;
pub mod sessions;

use anyhow::{bail, Result};
use std::path::PathBuf;

use patina::paths;

/// Common configuration for all scrapers
pub struct ScrapeConfig {
    pub db_path: String,
    pub force: bool,
}

impl ScrapeConfig {
    pub fn new(force: bool) -> Self {
        Self {
            db_path: database::PATINA_DB.to_string(),
            force,
        }
    }
}

/// Common stats that all scrapers return
#[derive(Debug)]
pub struct ScrapeStats {
    pub items_processed: usize,
    pub time_elapsed: std::time::Duration,
    pub database_size_kb: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_scrape_stats_creation() {
        let stats = ScrapeStats {
            items_processed: 100,
            time_elapsed: Duration::from_secs(5),
            database_size_kb: 1024,
        };
        assert_eq!(stats.items_processed, 100);
        assert_eq!(stats.time_elapsed.as_secs(), 5);
        assert_eq!(stats.database_size_kb, 1024);
    }
}

/// Run all scrapers in sequence (code, git, sessions, layer)
///
/// This is the default when running `patina scrape` with no subcommand.
pub fn execute_all() -> Result<()> {
    println!("🔄 Running all scrapers...\n");

    println!("📊 [1/4] Scraping code...");
    execute_code(false, false)?;

    println!("\n📊 [2/4] Scraping git...");
    let git_stats = git::run(false)?;
    println!("  • {} commits", git_stats.items_processed);

    println!("\n📚 [3/4] Scraping sessions...");
    let session_stats = sessions::run(false)?;
    println!("  • {} sessions", session_stats.items_processed);

    println!("\n📜 [4/4] Scraping layer patterns...");
    let layer_stats = layer::run(false)?;
    println!("  • {} patterns", layer_stats.items_processed);

    println!("\n✅ All scrapers complete!");
    Ok(())
}

/// Execute code scraper for current directory
///
/// For external repos, use `patina repo update <name>` instead.
pub fn execute_code(init: bool, force: bool) -> Result<()> {
    let config = ScrapeConfig::new(force);

    if init {
        code::initialize(&config)?;
    } else {
        let stats = code::run(config)?;

        println!("\n📊 Code Extraction Summary:");
        println!("  • Items processed: {}", stats.items_processed);
        println!("  • Time elapsed: {:?}", stats.time_elapsed);
        println!("  • Database size: {} KB", stats.database_size_kb);
    }

    Ok(())
}

/// Execute git scraper with summary output
pub fn execute_git(full: bool) -> Result<()> {
    let stats = git::run(full)?;
    println!("\n📊 Git Scrape Summary:");
    println!("  • Commits processed: {}", stats.items_processed);
    println!("  • Time elapsed: {:?}", stats.time_elapsed);
    println!("  • Database size: {} KB", stats.database_size_kb);
    Ok(())
}

/// Execute sessions scraper with summary output
pub fn execute_sessions(full: bool) -> Result<()> {
    let stats = sessions::run(full)?;
    println!("\n📊 Sessions Scrape Summary:");
    println!("  • Sessions processed: {}", stats.items_processed);
    println!("  • Time elapsed: {:?}", stats.time_elapsed);
    println!("  • Database size: {} KB", stats.database_size_kb);
    Ok(())
}

/// Execute layer pattern scraper with summary output
pub fn execute_layer(full: bool) -> Result<()> {
    let stats = layer::run(full)?;
    println!("\n📊 Layer Scrape Summary:");
    println!("  • Patterns processed: {}", stats.items_processed);
    println!("  • Time elapsed: {:?}", stats.time_elapsed);
    println!("  • Database size: {} KB", stats.database_size_kb);
    Ok(())
}

/// Resolve ref repo name to path.
fn resolve_repo_path(name: &str) -> Result<PathBuf> {
    let repo_path = paths::repos::cache_dir().join(name);
    if !repo_path.exists() {
        bail!(
            "Repository '{}' not found. Use 'patina repo list' to see registered repos.",
            name
        );
    }
    Ok(repo_path)
}

/// Execute forge scraper (issues and PRs from GitHub/Gitea)
pub fn execute_forge(
    full: bool,
    status: bool,
    sync: bool,
    log: bool,
    limit: Option<usize>,
    repo: Option<String>,
) -> Result<()> {
    // Resolve working directory if --repo provided
    let working_dir = match &repo {
        Some(name) => Some(resolve_repo_path(name)?),
        None => None,
    };

    // Get repo spec early - needed for status, sync, log
    let repo_spec = get_repo_spec(working_dir.as_ref())?;

    // Handle --log: tail the sync log file
    if log {
        return execute_forge_log(&repo_spec);
    }

    // Handle --status: show sync status
    if status {
        return execute_forge_status(working_dir.as_ref(), &repo_spec);
    }

    // Handle --sync: fork to background
    if sync {
        return execute_forge_background(working_dir.as_ref(), &repo_spec);
    }

    // Handle --limit: foreground sync with cap
    if let Some(limit_val) = limit {
        return execute_forge_limited(working_dir.as_ref(), &repo_spec, limit_val);
    }

    // Default: discovery only (instant)
    let config = forge::ForgeScrapeConfig {
        force: full,
        working_dir,
        ..Default::default()
    };
    let stats = forge::run(config)?;
    println!("\n📊 Forge Scrape Summary:");
    println!("  • Items processed: {}", stats.items_processed);
    println!("  • Time elapsed: {:?}", stats.time_elapsed);
    println!("  • Database size: {} KB", stats.database_size_kb);
    Ok(())
}

/// Get repo spec (owner/repo) from git remote.
fn get_repo_spec(working_dir: Option<&PathBuf>) -> Result<String> {
    use std::process::Command;

    let mut cmd = Command::new("git");
    cmd.args(["remote", "get-url", "origin"]);
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }
    let output = cmd.output()?;

    if !output.status.success() {
        bail!("No git remote configured.");
    }

    let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detected = patina::forge::detect(&remote_url);

    if detected.owner.is_empty() {
        bail!("Could not detect forge from remote URL.");
    }

    Ok(format!("{}/{}", detected.owner, detected.repo))
}

/// Get db path based on working directory.
fn get_db_path(working_dir: Option<&PathBuf>) -> PathBuf {
    match working_dir {
        Some(dir) => dir.join(".patina/local/data/patina.db"),
        None => PathBuf::from(database::PATINA_DB),
    }
}

/// Show forge sync status without making changes.
fn execute_forge_status(working_dir: Option<&PathBuf>, repo_spec: &str) -> Result<()> {
    let db_path = get_db_path(working_dir);

    if !db_path.exists() {
        println!("No patina.db found. Run `patina scrape` first.");
        return Ok(());
    }

    // Check if sync is running
    let running_pid = patina::forge::sync::is_running(repo_spec);

    let conn = database::initialize(&db_path)?;
    let stats = patina::forge::sync::status(&conn, repo_spec)?;

    println!("📊 Forge Sync Status for {}:", repo_spec);

    if let Some(pid) = running_pid {
        println!("  • Status: Syncing (PID {})", pid);
    } else {
        println!("  • Status: Idle");
    }

    println!("  • Resolved: {}", stats.resolved);
    println!("  • Pending: {}", stats.pending);
    println!("  • Errors: {}", stats.errors);

    if stats.pending > 0 {
        // At 750ms per ref, 50 refs/batch = ~37.5 seconds per batch
        let total_time_secs = (stats.pending as f64) * 0.75;
        let hours = (total_time_secs / 3600.0).floor() as usize;
        let minutes = ((total_time_secs % 3600.0) / 60.0).ceil() as usize;

        if hours > 0 {
            println!("\n  ETA: ~{}h {}m remaining", hours, minutes);
        } else {
            println!("\n  ETA: ~{}m remaining", minutes);
        }

        println!("  Rate: ~48 refs/min (750ms pacing)");
    }

    Ok(())
}

/// Tail the sync log file.
fn execute_forge_log(repo_spec: &str) -> Result<()> {
    use std::process::Command;

    let log_path = patina::forge::sync::log_path(repo_spec);

    if !log_path.exists() {
        println!("No log file found at: {}", log_path.display());
        println!("Run `patina scrape forge --sync` first.");
        return Ok(());
    }

    println!("📄 Tailing: {}", log_path.display());
    println!("   (Ctrl+C to stop)\n");

    // Use tail -f to follow the log
    let status = Command::new("tail")
        .args(["-f", log_path.to_str().unwrap_or("")])
        .status()?;

    if !status.success() {
        bail!("tail command failed");
    }

    Ok(())
}

/// Start background sync.
fn execute_forge_background(working_dir: Option<&PathBuf>, repo_spec: &str) -> Result<()> {
    use std::process::Command;

    let db_path = get_db_path(working_dir);

    if !db_path.exists() {
        bail!("No patina.db found. Run `patina scrape` first.");
    }

    // Get detected forge info
    let mut cmd = Command::new("git");
    cmd.args(["remote", "get-url", "origin"]);
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }
    let output = cmd.output()?;
    let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detected = patina::forge::detect(&remote_url);

    // Start background sync
    let pid = patina::forge::sync::start_background(&db_path, repo_spec, &detected)?;

    let log_path = patina::forge::sync::log_path(repo_spec);

    println!("🔄 Syncing in background (PID {})", pid);
    println!("   Log: {}", log_path.display());
    println!("   Check: patina scrape forge --status");

    Ok(())
}

/// Foreground sync with limit.
fn execute_forge_limited(
    working_dir: Option<&PathBuf>,
    repo_spec: &str,
    limit: usize,
) -> Result<()> {
    use std::process::Command;

    let db_path = get_db_path(working_dir);

    if !db_path.exists() {
        bail!("No patina.db found. Run `patina scrape` first.");
    }

    // Get detected forge info
    let mut cmd = Command::new("git");
    cmd.args(["remote", "get-url", "origin"]);
    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }
    let output = cmd.output()?;
    let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detected = patina::forge::detect(&remote_url);

    println!("🔄 Syncing up to {} refs in foreground...", limit);

    let conn = database::initialize(&db_path)?;
    let reader = patina::forge::reader(&detected);
    let stats = patina::forge::sync::sync_limited(&conn, reader.as_ref(), repo_spec, limit)?;

    println!("\n📊 Forge Sync Summary:");
    println!("  • Discovered: {}", stats.discovered);
    println!("  • Resolved: {}", stats.resolved);
    println!("  • Pending: {}", stats.pending);
    if stats.cache_hits > 0 {
        println!("  • Cache hits: {}", stats.cache_hits);
    }
    if stats.errors > 0 {
        println!("  • Errors: {}", stats.errors);
    }

    Ok(())
}
