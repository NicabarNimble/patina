// Shared utilities for all scrape subcommands

pub mod code;

use anyhow::Result;

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