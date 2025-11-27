// Shared utilities for all scrape subcommands

pub mod code;
pub mod database;
pub mod git;
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
