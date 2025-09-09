// Shared utilities for all scrape subcommands

pub mod docs;
pub mod pdf;
pub mod recode_v2;

use anyhow::{bail, Result};
use std::path::Path;

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
        // Find the actual directory name with correct case
        let actual_name = find_repo_actual_name(repo).unwrap_or_else(|| repo.to_string());
        self.db_path = format!("layer/dust/repos/{}.db", actual_name);
        self
    }
}

/// Find the actual directory name for a repository (case-insensitive lookup)
fn find_repo_actual_name(repo_name: &str) -> Option<String> {
    let repos_dir = Path::new("layer/dust/repos");
    if !repos_dir.exists() {
        return None;
    }

    std::fs::read_dir(repos_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .find(|entry| {
            entry.file_name().to_string_lossy().to_lowercase() == repo_name.to_lowercase()
                && entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
        })
        .map(|entry| entry.file_name().to_string_lossy().to_string())
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

/// Execute docs scraper
pub fn execute_docs(
    init: bool,
    query: Option<String>,
    repo: Option<String>,
    force: bool,
) -> Result<()> {
    let mut config = ScrapeConfig::new(force);
    if let Some(r) = repo.as_ref() {
        config.for_repo(r);
    }

    if init {
        docs::initialize(&config)?;
    } else if let Some(_q) = query {
        bail!("Query functionality has moved. Use 'patina ask' instead.");
    } else {
        let stats = docs::extract(&config)?;

        println!("\n📊 Document Extraction Summary:");
        println!("  • Documents found: {}", stats.items_processed);
        println!("  • Time elapsed: {:?}", stats.time_elapsed);
        println!("  • Total size: {} KB", stats.database_size_kb);
    }
    Ok(())
}

/// Execute PDF scraper
pub fn execute_pdf(
    init: bool,
    query: Option<String>,
    repo: Option<String>,
    force: bool,
) -> Result<()> {
    let mut config = ScrapeConfig::new(force);
    if let Some(r) = repo.as_ref() {
        config.for_repo(r);
    }

    if init {
        pdf::initialize(&config)?;
    } else if let Some(_q) = query {
        bail!("Query functionality has moved. Use 'patina ask' instead.");
    } else {
        let stats = pdf::extract(&config)?;

        println!("\n📊 PDF Extraction Summary:");
        println!("  • PDFs found: {}", stats.items_processed);
        println!("  • Time elapsed: {:?}", stats.time_elapsed);
        println!("  • Total size: {} KB", stats.database_size_kb);
    }
    Ok(())
}

/// Execute recode scraper (modular v2 architecture)
pub fn execute_recode(
    init: bool,
    query: Option<String>,
    repo: Option<String>,
    force: bool,
) -> Result<()> {
    let mut config = ScrapeConfig::new(force);
    if let Some(r) = repo.as_ref() {
        config.for_repo(r);
    }

    if init {
        recode_v2::initialize(&config)?;
    } else if let Some(_q) = query {
        bail!("Query functionality has moved. Use 'patina ask' instead.");
    } else {
        let stats = recode_v2::run(config)?;

        println!("\n📊 Recode Extraction Summary:");
        println!("  • Items processed: {}", stats.items_processed);
        println!("  • Time elapsed: {:?}", stats.time_elapsed);
        println!("  • Database size: {} KB", stats.database_size_kb);
    }
    Ok(())
}
