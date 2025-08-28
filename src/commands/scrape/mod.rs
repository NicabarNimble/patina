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

/// Execute scrape command - for now just delegates to code scraper
pub fn execute(init: bool, query: Option<String>, repo: Option<String>, force: bool) -> Result<()> {
    // For backward compatibility, we just run code scraper
    // In future, we'll add subcommand routing here
    let mut config = ScrapeConfig::new(force);
    if let Some(r) = repo.as_ref() {
        config.for_repo(r);
    }

    if init {
        code::initialize(&config)?;
    } else if let Some(_q) = query {
        // Query should move to Ask command eventually
        bail!("Query functionality has moved. Use 'patina ask' instead.");
    } else {
        code::extract(&config)?;
    }
    Ok(())
}
