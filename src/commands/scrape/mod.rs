// Shared utilities for all scrape subcommands

pub mod beliefs;
pub mod code;
pub mod database;
pub mod delta;
pub mod events;
pub mod git;
pub mod layer;
pub mod projection;
pub mod sessions;

use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;

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

    // Trigger on-scrape sources via broker
    trigger_on_scrape_sources();

    println!(
        "\n✅ All scrapers complete! ({:.1?})",
        total_start.elapsed()
    );
    Ok(())
}

/// Trigger on-scrape sources after local scrape completes.
///
/// Finds all sources with schedule = "on-scrape" in this project's
/// sources.toml and runs each one. Errors are logged but don't fail
/// the scrape — the local scrape data is already committed.
fn trigger_on_scrape_sources() {
    let project_root = match std::env::current_dir() {
        Ok(p) => p,
        Err(_) => return,
    };

    let sources = match patina::mother::broker::sources::load_project_sources(&project_root) {
        Ok(Some(ps)) => ps.sources,
        _ => return,
    };

    let on_scrape: Vec<_> = sources
        .iter()
        .filter(|s| s.schedule == "on-scrape")
        .collect();

    if on_scrape.is_empty() {
        return;
    }

    println!("\n🔗 Triggering {} on-scrape source(s)...", on_scrape.len());

    for source in on_scrape {
        match patina::mother::broker::run_source(source, &project_root, false) {
            Ok(result) => {
                println!(
                    "  {} — {} written, {} dedup",
                    source.name, result.inserted, result.dedup_skipped
                );
            }
            Err(e) => {
                eprintln!("  {} — error: {}", source.name, e);
            }
        }
    }
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

    // Forge data now comes from github-connector via broker.
    // Run `patina mother run github` to fetch issues/PRs.
    println!("\n📝 [5/5] Forge data via github-connector (run `patina mother run github`)");

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
