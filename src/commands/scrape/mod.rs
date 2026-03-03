// Shared utilities for all scrape subcommands

pub mod beliefs;
pub mod code;
pub mod database;
pub mod delta;
pub mod forge;
pub mod git;
pub mod layer;
pub mod sessions;

use anyhow::{bail, Result};
use std::collections::HashSet;
use std::path::PathBuf;

use patina::paths;

/// Common configuration for all scrapers
pub struct ScrapeConfig {
    pub db_path: String,
    pub force: bool,
    /// Optional extension filter for lazy plugin loading.
    /// When Some, only load pipeline plugins claiming these extensions.
    /// When None, load all plugins (default for individual scraper commands).
    pub extension_filter: Option<HashSet<String>>,
    /// Changed code file paths from delta (for incremental FTS5 updates).
    /// When Some, only rebuild FTS5 rows for these files.
    /// When None, do full FTS5 rebuild. See [[scrape-diff-driven]] EC6.
    pub changed_code_files: Option<Vec<String>>,
}

impl ScrapeConfig {
    pub fn new(force: bool) -> Self {
        Self {
            db_path: database::PATINA_DB.to_string(),
            force,
            extension_filter: None,
            changed_code_files: None,
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

/// Run all scrapers in sequence (code, git, layer, beliefs)
///
/// This is the default when running `patina scrape` with no subcommand.
/// Layer scraper handles both patterns and sessions (unified in v0.12.0).
///
/// Delta-driven dispatch: computes what changed since last scrape, then
/// only invokes scrapers with work to do. See [[scrape-diff-driven]].
pub fn execute_all() -> Result<()> {
    let total_start = std::time::Instant::now();

    // Ensure UID exists (migration for projects without one)
    patina::project::create_uid_if_missing(&std::env::current_dir()?)?;

    // Compute delta: what changed since last scrape?
    let scrape_delta = delta::compute_delta()?;
    scrape_delta.log_summary();

    // Empty delta = nothing changed = skip everything (EC1: < 500ms)
    if scrape_delta.is_empty() {
        println!(
            "\n✅ Nothing changed — scrape skipped ({:.0?})",
            total_start.elapsed()
        );
        return Ok(());
    }

    println!("\n🔄 Running scrapers (delta-driven)...\n");

    // Route by source kind — only invoke scrapers with work
    if !scrape_delta.new_commits.is_empty() {
        println!(
            "📊 Scraping git ({} new commits)...",
            scrape_delta.new_commits.len()
        );
        let git_stats = git::run(false)?;
        println!("  • {} commits", git_stats.items_processed);
    } else {
        println!("📊 Scraping git... skipped (no new commits)");
    }

    if !scrape_delta.changed_code_files().is_empty() {
        let extensions = scrape_delta.changed_extensions();
        let changed_paths: Vec<String> = scrape_delta
            .changed_code_files()
            .iter()
            .map(|f| {
                // Normalize to ./ prefix to match eventlog source_id format
                if f.path.starts_with("./") {
                    f.path.clone()
                } else {
                    format!("./{}", f.path)
                }
            })
            .collect();
        println!(
            "\n📊 Scraping code ({} changed files, extensions: {:?})...",
            changed_paths.len(),
            extensions
        );
        execute_code_incremental(false, Some(extensions), changed_paths)?;
    } else {
        println!("\n📊 Scraping code... skipped (no changed code files)");
    }

    if scrape_delta.layer_changed {
        println!("\n📜 Scraping layer (patterns + sessions)...");
        let layer_stats = layer::run(false)?;
        println!("  • {} items", layer_stats.items_processed);
    } else {
        println!("\n📜 Scraping layer... skipped (no layer changes)");
    }

    if scrape_delta.beliefs_affected {
        println!("\n🧠 Scraping beliefs...");
        let belief_stats = beliefs::run(false)?;
        println!("  • {} beliefs", belief_stats.items_processed);
    } else {
        println!("\n🧠 Scraping beliefs... skipped (no affected beliefs)");
    }

    println!(
        "\n✅ All scrapers complete! ({:.1?})",
        total_start.elapsed()
    );
    Ok(())
}

/// Rebuild database from scratch.
///
/// Deletes patina.db (rebuildable projections) and recreates from source.
/// events.db is NEVER touched — it contains irreplaceable runtime events.
///
/// For ref repos: removes old eventlog bloat (git/code events) and rebuilds
/// with lean storage pattern. Includes forge data re-fetch.
///
/// See: layer/surface/build/spec-ref-repo-storage.md
pub fn execute_rebuild() -> Result<()> {
    // Ensure UID exists (migration for projects without one)
    patina::project::create_uid_if_missing(&std::env::current_dir()?)?;

    let db_path = PathBuf::from(database::PATINA_DB);
    let is_ref = database::is_ref_repo(&db_path);

    // Get old size if exists
    let old_size_kb = std::fs::metadata(&db_path)
        .map(|m| m.len() / 1024)
        .unwrap_or(0);

    if is_ref {
        println!("🔧 Rebuilding ref repo database (lean storage)...");
        println!("   Old size: {} KB", old_size_kb);
    } else {
        println!("🔧 Rebuilding project database...");
    }

    // Delete patina.db only — events.db is irreplaceable and never touched by rebuild
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
        println!("   Deleted patina.db (rebuildable projections)");
    }

    // Ensure events.db exists (migration if needed, no-op if already present)
    patina::eventlog::ensure_events_db()?;

    // Run all scrapers fresh (they will use lean storage for ref repos)
    println!("\n🔄 Running all scrapers...\n");

    println!("📊 [1/5] Scraping code...");
    execute_code(false, false)?;

    println!("\n📊 [2/5] Scraping git...");
    let git_stats = git::run(false)?;
    println!("  • {} commits", git_stats.items_processed);

    println!("\n📜 [3/5] Scraping layer (patterns + sessions)...");
    let layer_stats = layer::run(false)?;
    println!("  • {} items", layer_stats.items_processed);

    println!("\n🧠 [4/5] Scraping beliefs...");
    let belief_stats = beliefs::run(false)?;
    println!("  • {} beliefs", belief_stats.items_processed);

    // For ref repos, also rebuild forge data (this is the expensive cached data we preserve)
    if is_ref {
        println!("\n🔗 [5/5] Scraping forge (issues/PRs)...");
        // Use full=true to force complete re-fetch since we deleted the database
        execute_forge(true, false, false, false, None, None)?;
    } else {
        println!("\n📝 [5/5] Skipping forge (run 'patina scrape forge' separately)");
    }

    // Report new size
    let new_size_kb = std::fs::metadata(&db_path)
        .map(|m| m.len() / 1024)
        .unwrap_or(0);

    println!("\n✅ Rebuild complete!");
    println!("   New size: {} KB", new_size_kb);

    if is_ref && old_size_kb > 0 {
        let reduction = if old_size_kb > new_size_kb {
            ((old_size_kb - new_size_kb) * 100) / old_size_kb
        } else {
            0
        };
        println!(
            "   Reduction: {} KB → {} KB ({}% smaller)",
            old_size_kb, new_size_kb, reduction
        );
    }

    Ok(())
}

/// Execute code scraper for current directory
///
/// For external repos, use `patina repo update <name>` instead.
pub fn execute_code(init: bool, force: bool) -> Result<()> {
    execute_code_with_filter(init, force, None)
}

/// Execute code scraper with optional extension filter for lazy plugin loading.
///
/// When `extensions` is Some, only pipeline plugins claiming those extensions
/// are loaded. This avoids compiling WASM for plugins that have no work.
fn execute_code_with_filter(
    init: bool,
    force: bool,
    extensions: Option<HashSet<String>>,
) -> Result<()> {
    let config = ScrapeConfig {
        db_path: database::PATINA_DB.to_string(),
        force,
        extension_filter: extensions,
        changed_code_files: None,
    };

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

/// Execute code scraper with delta-driven incremental FTS5 updates.
///
/// Combines extension filter (for lazy plugin loading) with changed file paths
/// (for incremental FTS5 rebuild). Used by `execute_all()` delta-driven dispatch.
fn execute_code_incremental(
    force: bool,
    extensions: Option<HashSet<String>>,
    changed_files: Vec<String>,
) -> Result<()> {
    let config = ScrapeConfig {
        db_path: database::PATINA_DB.to_string(),
        force,
        extension_filter: extensions,
        changed_code_files: Some(changed_files),
    };

    let stats = code::run(config)?;

    println!("\n📊 Code Extraction Summary:");
    println!("  • Items processed: {}", stats.items_processed);
    println!("  • Time elapsed: {:?}", stats.time_elapsed);
    println!("  • Database size: {} KB", stats.database_size_kb);

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

/// Execute sessions scraper with summary output (deprecated)
pub fn execute_sessions(full: bool) -> Result<()> {
    eprintln!("WARNING: `scrape sessions` is deprecated. Use `scrape layer` instead.");
    eprintln!("         Sessions are part of layer/ and scraped automatically.\n");
    let stats = sessions::run(full)?;
    println!("\n📊 Sessions Scrape Summary:");
    println!("  • Sessions processed: {}", stats.items_processed);
    println!("  • Time elapsed: {:?}", stats.time_elapsed);
    println!("  • Database size: {} KB", stats.database_size_kb);
    Ok(())
}

/// Execute unified layer scraper (patterns + sessions)
pub fn execute_layer(full: bool) -> Result<()> {
    let stats = layer::run(full)?;
    println!("\n📊 Layer Scrape Summary:");
    println!(
        "  • Items processed: {} (patterns + sessions)",
        stats.items_processed
    );
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

    // Compute staging directory for resolved refs
    let work_dir = working_dir
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let staging_dir = patina::paths::project::data_dir(&work_dir)
        .join("forge")
        .join(format!("{}-{}", detected.owner, detected.repo));

    // Start background sync
    let pid = patina::forge::sync::start_background(&db_path, repo_spec, &detected, &staging_dir)?;

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

    // Compute staging directory for resolved refs
    let work_dir = working_dir
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let staging_dir = patina::paths::project::data_dir(&work_dir)
        .join("forge")
        .join(format!("{}-{}", detected.owner, detected.repo));

    println!("🔄 Syncing up to {} refs in foreground...", limit);

    let conn = database::initialize(&db_path)?;
    let reader = patina::forge::reader(&detected);
    let stats =
        patina::forge::sync::sync_limited(&conn, reader.as_ref(), repo_spec, limit, &staging_dir)?;

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
