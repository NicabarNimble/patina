// Shared utilities for all scrape subcommands

pub mod docs;
pub mod pdf;
pub mod code;

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

/// Discover all repository directories in layer/dust/repos/
fn discover_all_repos() -> Result<Vec<String>> {
    let repos_dir = Path::new("layer/dust/repos");
    if !repos_dir.exists() {
        return Ok(Vec::new());
    }

    let mut repos = Vec::new();
    for entry in std::fs::read_dir(repos_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            // Check if it's a git repository
            let git_dir = entry.path().join(".git");
            if git_dir.exists() {
                repos.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    repos.sort();
    Ok(repos)
}

/// Parse .patina-update.log for STALE database entries
fn parse_stale_repos() -> Result<Vec<String>> {
    let log_path = Path::new("layer/dust/repos/.patina-update.log");
    if !log_path.exists() {
        return Ok(Vec::new());
    }

    let contents = std::fs::read_to_string(log_path)?;
    let mut stale_dbs = std::collections::HashSet::new();

    // Parse log lines for STALE entries
    for line in contents.lines() {
        if line.contains("| STALE") {
            // Format: "2025-10-01T15:52:10Z | STALE | duckdb.db | needs rescrape"
            let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
            if parts.len() >= 3 {
                let db_name = parts[2];
                if let Some(repo_name) = db_name.strip_suffix(".db") {
                    stale_dbs.insert(repo_name.to_string());
                }
            }
        }
    }

    let mut stale_repos: Vec<String> = stale_dbs.into_iter().collect();
    stale_repos.sort();
    Ok(stale_repos)
}

/// Log a RESCRAPE action to .patina-update.log
fn log_rescrape(repo_name: &str, stats: &ScrapeStats) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let log_path = Path::new("layer/dust/repos/.patina-update.log");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let message = format!(
        "completed ({} items, {:.1}s)",
        stats.items_processed,
        stats.time_elapsed.as_secs_f64()
    );

    writeln!(
        file,
        "{} | RESCRAPE | {}.db | {}",
        timestamp, repo_name, message
    )?;

    Ok(())
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

/// Execute code scraper (modular architecture)
pub fn execute_code(
    init: bool,
    query: Option<String>,
    repo: Option<String>,
    force: bool,
) -> Result<()> {
    if let Some(_q) = query {
        bail!("Query functionality has moved. Use 'patina ask' instead.");
    }

    // Handle special repo values: "all" and "doctor"
    let repos_to_scrape = match repo.as_deref() {
        Some("all") => {
            let repos = discover_all_repos()?;
            if repos.is_empty() {
                println!("No repositories found in layer/dust/repos/");
                return Ok(());
            }
            println!("🔍 Found {} repositories to scrape\n", repos.len());
            repos
        }
        Some("doctor") => {
            let stale_repos = parse_stale_repos()?;
            if stale_repos.is_empty() {
                println!("✅ No stale repositories found in update log");
                return Ok(());
            }
            println!(
                "🔍 Found {} stale repositories from doctor log\n",
                stale_repos.len()
            );
            stale_repos
        }
        Some(single_repo) => vec![single_repo.to_string()],
        None => Vec::new(),
    };

    // When scraping repos, force is always true (fresh data)
    let force_flag = if !repos_to_scrape.is_empty() {
        true
    } else {
        force
    };

    // Single repo or current directory
    if repos_to_scrape.is_empty() || repos_to_scrape.len() == 1 {
        let mut config = ScrapeConfig::new(force_flag);
        if let Some(r) = repos_to_scrape.first() {
            config.for_repo(r);
        }

        if init {
            code::initialize(&config)?;
        } else {
            let stats = code::run(config)?;

            println!("\n📊 Code Extraction Summary:");
            println!("  • Items processed: {}", stats.items_processed);
            println!("  • Time elapsed: {:?}", stats.time_elapsed);
            println!("  • Database size: {} KB", stats.database_size_kb);

            // Log RESCRAPE if this was a repo scrape
            if let Some(repo_name) = repos_to_scrape.first() {
                log_rescrape(repo_name, &stats)?;
            }
        }
        return Ok(());
    }

    // Batch scraping multiple repos
    println!("🚀 Starting batch scrape...\n");
    let mut total_items = 0;
    let mut successful = 0;
    let start_time = std::time::Instant::now();

    for (i, repo_name) in repos_to_scrape.iter().enumerate() {
        println!(
            "[{}/{}] Scraping {}...",
            i + 1,
            repos_to_scrape.len(),
            repo_name
        );

        let mut config = ScrapeConfig::new(force_flag);
        config.for_repo(repo_name);

        match code::run(config) {
            Ok(stats) => {
                println!(
                    "  ✓ {} items in {:.1}s\n",
                    stats.items_processed,
                    stats.time_elapsed.as_secs_f64()
                );
                total_items += stats.items_processed;
                successful += 1;

                // Log each successful scrape
                if let Err(e) = log_rescrape(repo_name, &stats) {
                    eprintln!("  ⚠ Failed to log rescrape: {}", e);
                }
            }
            Err(e) => {
                eprintln!("  ✗ Error: {}\n", e);
            }
        }
    }

    let total_time = start_time.elapsed();
    println!("📊 Batch Scrape Summary:");
    println!("  • Repositories: {}/{}", successful, repos_to_scrape.len());
    println!("  • Total items: {}", total_items);
    println!("  • Total time: {:.1}s", total_time.as_secs_f64());

    Ok(())
}
