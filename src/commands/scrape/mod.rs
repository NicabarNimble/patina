// Shared utilities for all scrape subcommands

pub mod code;
pub mod database;
pub mod forge;
pub mod git;
pub mod layer;
pub mod sessions;

use anyhow::Result;

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

/// Execute forge scraper (issues and PRs from GitHub/Gitea)
pub fn execute_forge(full: bool) -> Result<()> {
    let config = forge::ForgeScrapeConfig {
        force: full,
        ..Default::default()
    };
    let stats = forge::run(config)?;
    println!("\n📊 Forge Scrape Summary:");
    println!("  • Items processed: {}", stats.items_processed);
    println!("  • Time elapsed: {:?}", stats.time_elapsed);
    println!("  • Database size: {} KB", stats.database_size_kb);
    Ok(())
}
