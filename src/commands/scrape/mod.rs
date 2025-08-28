// Shared utilities for all scrape subcommands

pub mod code;

use anyhow::{bail, Result};

/// Common configuration for all scrapers
pub struct ScrapeConfig {
    pub db_path: String,
    pub force: bool,
}

impl ScrapeConfig {
    pub fn new(force: bool) -> Self {
        Self {
            db_path: ".patina/knowledge.db".to_string(),
            force,
        }
    }

    pub fn for_repo(&mut self, repo: &str) -> &mut Self {
        self.db_path = format!("layer/dust/repos/{}.db", repo);
        self
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

/// Execute scrape command with optional subcommand support
pub fn execute(init: bool, query: Option<String>, repo: Option<String>, force: bool) -> Result<()> {
    // For backward compatibility, we default to code scraper
    let mut config = ScrapeConfig::new(force);
    if let Some(r) = repo.as_ref() {
        config.for_repo(r);
    }

    if init {
        code::initialize(&config)?;
    } else if let Some(_q) = query {
        // Query functionality has moved to 'patina ask'
        bail!("Query functionality has moved. Use 'patina ask' instead.");
    } else {
        // Extract and show stats
        let stats = code::extract(&config)?;

        // Display summary
        println!("\n📊 Extraction Summary:");
        println!("  • Processed {} items", stats.items_processed);
        println!("  • Time elapsed: {:?}", stats.time_elapsed);
        println!("  • Database size: {} KB", stats.database_size_kb);
    }
    Ok(())
}
