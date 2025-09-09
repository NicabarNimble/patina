// ============================================================================
// SEMANTIC CODE EXTRACTION PIPELINE V2 - MODULAR ARCHITECTURE
// ============================================================================
//! # Code → Knowledge ETL Pipeline (Recode Version)
//!
//! A clean, modular refactor of the code extraction pipeline where each
//! language is fully self-contained in its own module file.
//!
//! ## Architecture
//! - Each language gets its own file (rust.rs, go.rs, etc.)
//! - Each language processor returns ExtractedData structs
//! - Database module uses DuckDB Appender API for 10-100x performance
//! - Clean separation of concerns
//!
//! ## Usage
//! ```bash
//! patina scrape recode          # Index using modular architecture
//! patina scrape recode --force  # Rebuild from scratch
//! ```

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

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
    println!("🗄️  Initializing optimized knowledge database (recode v2)...");

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

/// Main entry point for the recode command
pub fn run(config: ScrapeConfig) -> Result<super::ScrapeStats> {
    println!("🔄 Running recode v2 with modular language architecture...");

    let start = std::time::Instant::now();

    // Determine work directory
    let work_dir = determine_work_directory(&config)?;

    // Print repo info if scraping a repository
    if config.db_path.contains("layer/dust/repos/") {
        if let Some(repo_name) = config
            .db_path
            .split('/')
            .find(|s| s.ends_with(".db"))
            .and_then(|s| s.strip_suffix(".db"))
        {
            println!("📦 Repository: {}", repo_name);
        }
    }
    println!("📂 Working directory: {}", work_dir.display());

    // Initialize database if needed
    if !Path::new(&config.db_path).exists() || config.force {
        initialize_database(&config.db_path)?;
    }

    // Always use the new embedded DuckDB implementation
    let items_processed =
        extract_v2::extract_code_metadata_v2(&config.db_path, &work_dir, config.force)?;

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

fn determine_work_directory(config: &ScrapeConfig) -> Result<PathBuf> {
    // Extract repo name from db_path if it's in layer/dust/repos/
    if config.db_path.contains("layer/dust/repos/") {
        let repo_name = config
            .db_path
            .strip_prefix("layer/dust/repos/")
            .and_then(|s| s.strip_suffix(".db"))
            .context("Invalid repo database path")?;

        // The repo_name in db_path already has the correct case from for_repo()
        let repo_dir = PathBuf::from("layer/dust/repos").join(repo_name);
        if !repo_dir.exists() {
            anyhow::bail!(
                "Repository '{}' not found. Clone it first to layer/dust/repos/",
                repo_name
            );
        }

        Ok(std::env::current_dir()?.join(repo_dir))
    } else {
        Ok(std::env::current_dir()?)
    }
}

/// Initialize DuckDB database with embedded library
fn initialize_database(db_path: &str) -> Result<()> {
    // Create parent directory if needed
    if let Some(parent) = Path::new(db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Remove old database if exists (when force flag is used)
    if Path::new(db_path).exists() {
        std::fs::remove_file(db_path)?;
    }

    let mut db = database::Database::open(db_path)?;
    db.init_schema()?;
    Ok(())
}

/// Generate database schema
fn generate_schema() -> &'static str {
    r#"
-- Full-text search for actual code search
CREATE TABLE IF NOT EXISTS code_search (
    path VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    signature VARCHAR,      -- Function/struct signature
    context VARCHAR,        -- Surrounding code snippet
    PRIMARY KEY (path, name)
);

-- Type vocabulary: The domain language (compiler-verified truth)
CREATE TABLE IF NOT EXISTS type_vocabulary (
    file VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    definition TEXT,        -- 'type NodeId = u32' or 'struct User { ... }'
    kind VARCHAR,          -- 'type_alias', 'struct', 'enum', 'const'
    visibility VARCHAR,     -- 'pub', 'pub(crate)', 'private'
    usage_count INTEGER DEFAULT 0,
    PRIMARY KEY (file, name)
);

-- Function facts: Behavioral signals without interpretation
CREATE TABLE IF NOT EXISTS function_facts (
    file VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    takes_mut_self BOOLEAN,     -- Thread safety signal
    takes_mut_params BOOLEAN,   -- Mutation indicator
    returns_result BOOLEAN,     -- Error handling
    returns_option BOOLEAN,     -- Nullability
    is_async BOOLEAN,          -- Concurrency
    is_unsafe BOOLEAN,         -- Safety requirements
    is_public BOOLEAN,         -- API surface
    parameter_count INTEGER,
    generic_count INTEGER,      -- Complexity indicator
    parameters TEXT,            -- Parameter names and types
    return_type TEXT,           -- Return type signature
    PRIMARY KEY (file, name)
);

-- Import facts: Navigation and dependencies
CREATE TABLE IF NOT EXISTS import_facts (
    importer_file VARCHAR NOT NULL,
    imported_item VARCHAR NOT NULL,
    imported_from VARCHAR,      -- Source module/crate
    is_external BOOLEAN,       -- External crate?
    import_kind VARCHAR,        -- 'use', 'mod', 'extern'
    PRIMARY KEY (importer_file, imported_item)
);

-- Documentation: Searchable docs with keywords for LLM context retrieval
CREATE TABLE IF NOT EXISTS documentation (
    file VARCHAR NOT NULL,
    symbol_name VARCHAR NOT NULL,
    symbol_type VARCHAR,        -- 'function', 'struct', 'module', 'field'
    line_number INTEGER,
    doc_raw TEXT,              -- Original with comment markers
    doc_clean TEXT,            -- Cleaned text for display
    doc_summary VARCHAR,       -- First sentence (fast preview)
    keywords VARCHAR[],        -- Extracted keywords for search
    doc_length INTEGER,        -- Character count
    has_examples BOOLEAN,      -- Contains code blocks
    PRIMARY KEY (file, symbol_name)
);

-- Call graph: Function relationships for context traversal
CREATE TABLE IF NOT EXISTS call_graph (
    caller VARCHAR NOT NULL,
    callee VARCHAR NOT NULL,
    file VARCHAR NOT NULL,
    call_type VARCHAR,         -- 'direct', 'method', 'async', 'callback'
    line_number INTEGER        -- Where the call happens
);

CREATE INDEX IF NOT EXISTS idx_caller ON call_graph(caller);
CREATE INDEX IF NOT EXISTS idx_callee ON call_graph(callee);

-- Index metadata for incremental updates
CREATE TABLE IF NOT EXISTS index_state (
    path VARCHAR PRIMARY KEY,
    mtime BIGINT NOT NULL,
    hash VARCHAR,           -- File content hash
    indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Track files we skipped due to missing language support
CREATE TABLE IF NOT EXISTS skipped_files (
    extension VARCHAR PRIMARY KEY,
    file_count INTEGER DEFAULT 0,
    total_bytes INTEGER DEFAULT 0,
    example_path VARCHAR,
    common_name VARCHAR     -- e.g., "Python", "TypeScript"
);

-- Create indexes for fast lookups
CREATE INDEX IF NOT EXISTS idx_type_vocabulary_kind ON type_vocabulary(kind);
CREATE INDEX IF NOT EXISTS idx_function_facts_public ON function_facts(is_public);
"#
}
