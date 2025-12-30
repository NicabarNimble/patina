// ============================================================================
// SEMANTIC CODE EXTRACTION PIPELINE V2 - MODULAR ARCHITECTURE
// ============================================================================
//! # Code → Knowledge ETL Pipeline
//!
//! A clean, modular refactor of the code extraction pipeline where each
//! language is fully self-contained in its own module file.
//!
//! ## Architecture
//! - Each language gets its own file (rust.rs, go.rs, etc.)
//! - Each language processor returns ExtractedData structs
//! - Database module uses transactions for bulk insert performance
//! - Clean separation of concerns
//!
//! ## Usage
//! ```bash
//! patina scrape code          # Index using modular architecture
//! patina scrape code --force  # Rebuild from scratch
//! ```

use anyhow::Result;
use std::path::Path;

use super::ScrapeConfig;

// ============================================================================
// LANGUAGE MODULES
// ============================================================================
pub mod database;
pub mod extract_v2;
pub mod extracted_data;
pub mod languages;
pub mod types;

// ============================================================================
// PUBLIC INTERFACE
// ============================================================================

/// Initialize a new knowledge database
pub fn initialize(config: &ScrapeConfig) -> Result<()> {
    println!("🗄️  Initializing optimized knowledge database...");

    // Create parent directory if needed
    if let Some(parent) = Path::new(&config.db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Remove old database if exists
    if Path::new(&config.db_path).exists() {
        std::fs::remove_file(&config.db_path)?;
    }

    // Initialize with schema
    initialize_database(&config.db_path)?;

    println!("✅ Database initialized at {}", config.db_path);
    Ok(())
}

/// Main entry point for the code command
pub fn run(config: ScrapeConfig) -> Result<super::ScrapeStats> {
    println!("🔄 Extracting semantic code information...");

    let start = std::time::Instant::now();

    // Determine work directory
    let work_dir = std::env::current_dir()?;
    println!("📂 Working directory: {}", work_dir.display());

    // Initialize database if needed
    if !Path::new(&config.db_path).exists() || config.force {
        initialize_database(&config.db_path)?;
    }

    // Always use the new embedded SQLite implementation
    let items_processed =
        extract_v2::extract_code_metadata_v2(&config.db_path, &work_dir, config.force)?;

    // Populate FTS5 index for lexical search
    println!("📝 Building FTS5 lexical index...");
    let conn = rusqlite::Connection::open(&config.db_path)?;
    let fts_count = super::database::populate_fts5(&conn)?;
    println!("   Indexed {} symbols", fts_count);

    // Get database size
    let metadata = std::fs::metadata(&config.db_path)?;
    let database_size_kb = metadata.len() / 1024;

    Ok(super::ScrapeStats {
        items_processed,
        time_elapsed: start.elapsed(),
        database_size_kb,
    })
}

// ============================================================================
// INTERNAL IMPLEMENTATION
// ============================================================================

/// Initialize SQLite database with embedded library
fn initialize_database(db_path: &str) -> Result<()> {
    // Create parent directory if needed
    if let Some(parent) = Path::new(db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Remove old database if exists (when force flag is used)
    if Path::new(db_path).exists() {
        std::fs::remove_file(db_path)?;
    }

    // Initialize unified eventlog (git, sessions, code events)
    super::database::initialize(Path::new(db_path))?;

    // Create code-specific materialized views
    let mut db = database::Database::open(db_path)?;
    db.init_schema()?;
    Ok(())
}
