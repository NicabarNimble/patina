//! Session scraper — DEPRECATED, use `scrape layer` instead
//!
//! Sessions are part of layer/ and are now scraped by the unified layer scraper.
//! This module delegates to layer::sessions for backwards compatibility.

use anyhow::Result;
use std::path::Path;
use std::time::Instant;

use super::database;
use super::ScrapeStats;

const SESSIONS_DIR: &str = "layer/sessions";

/// Main entry point for standalone sessions scraping (deprecated path)
pub fn run(full: bool) -> Result<ScrapeStats> {
    let start = Instant::now();
    let db_path = database::patina_db_path()?;
    let sessions_dir = Path::new(SESSIONS_DIR);

    if !sessions_dir.exists() {
        anyhow::bail!("Sessions directory not found: {}", SESSIONS_DIR);
    }

    // Initialize unified database with eventlog
    let conn = database::initialize(&db_path)?;

    // Collect session files
    let mut session_files: Vec<_> = std::fs::read_dir(sessions_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .map(|e| e.path())
        .collect();

    session_files.sort();

    // Delegate to the layer session sub-scraper
    let (processed_count, skipped) =
        super::layer::sessions::scrape_sessions(&conn, &session_files, full)?;

    if full {
        println!("📚 Full session scrape...");
    } else {
        println!("📚 Incremental session scrape ({} skipped)...", skipped);
    }

    println!(
        "  Processed {} sessions ({} skipped)",
        processed_count, skipped
    );

    let elapsed = start.elapsed();
    let db_size = std::fs::metadata(db_path)
        .map(|m| m.len() / 1024)
        .unwrap_or(0);

    Ok(ScrapeStats {
        items_processed: processed_count,
        time_elapsed: elapsed,
        database_size_kb: db_size,
    })
}
