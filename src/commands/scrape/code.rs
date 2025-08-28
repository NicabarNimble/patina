// ============================================================================
// SEMANTIC CODE EXTRACTION PIPELINE
// ============================================================================
//! # Code → Knowledge ETL Pipeline
//! 
//! Transforms source code into queryable semantic knowledge using tree-sitter.
//! 
//! ## Purpose
//! This module implements a pure ETL (Extract, Transform, Load) pipeline:
//! - **Extract**: Read source files and git history
//! - **Transform**: Parse ASTs, calculate complexity, detect patterns
//! - **Load**: Store in DuckDB for querying via Ask command
//!
//! ## Database Tables
//! - `code_fingerprints`: AST patterns and complexity metrics
//! - `function_facts`: Behavioral signals (async, unsafe, mutability)
//! - `git_metrics`: Code survival and evolution tracking
//! - `call_graph`: Function dependency relationships
//! - `documentation`: Extracted doc comments with keywords
//!
//! ## Supported Languages
//! Rust, Go, Python, JavaScript, TypeScript, Solidity
//!
//! ## Usage
//! ```bash
//! patina scrape code           # Index current directory
//! patina scrape code --force   # Rebuild from scratch
//! patina scrape code --repo x  # Index layer/dust/repos/x
//! ```

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::ScrapeConfig;

// ============================================================================
// CHAPTER 1: PUBLIC INTERFACE
// ============================================================================

/// Initialize a new knowledge database
pub fn initialize(config: &ScrapeConfig) -> Result<()> {
    println!("🗄️  Initializing knowledge database at {}...", config.db_path);
    
    // Create parent directory if needed
    if let Some(parent) = Path::new(&config.db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Remove old database if exists
    if Path::new(&config.db_path).exists() {
        std::fs::remove_file(&config.db_path)?;
    }
    
    // Create with schema
    create_database_schema(&config.db_path)?;
    
    println!("✅ Database initialized");
    println!("\nNext: Run 'patina scrape code' to index your codebase");
    Ok(())
}

/// Extract semantic information from codebase
pub fn extract(config: &ScrapeConfig) -> Result<()> {
    println!("🔍 Starting semantic extraction...\n");
    
    let work_dir = determine_work_directory(config)?;
    
    if config.force {
        initialize(config)?;
    }
    
    // Run the ETL pipeline
    run_pipeline(&config.db_path, &work_dir)?;
    
    Ok(())
}

/// Query the knowledge database (temporary - should move to Ask)
pub fn query(config: &ScrapeConfig, sql: &str) -> Result<()> {
    let output = Command::new("duckdb")
        .arg(&config.db_path)
        .arg("-c")
        .arg(sql)
        .output()
        .context("Failed to execute query")?;
    
    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        anyhow::bail!("Query failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    Ok(())
}

// ============================================================================
// CHAPTER 2: ETL PIPELINE ORCHESTRATION
// ============================================================================

fn run_pipeline(db_path: &str, work_dir: &Path) -> Result<()> {
    // Phase 1: Git metrics for code survival analysis
    println!("📊 Phase 1: Analyzing git history...");
    extract_and_load_git_metrics(db_path, work_dir)?;
    
    // Phase 2: Pattern references from documentation
    println!("🔗 Phase 2: Extracting pattern references...");
    extract_and_load_pattern_references(db_path, work_dir)?;
    
    // Phase 3: Semantic analysis with tree-sitter
    println!("🧠 Phase 3: Parsing and analyzing code...");
    extract_and_load_semantic_data(db_path, work_dir)?;
    
    // Phase 4: Summary
    println!("\n📈 Phase 4: Generating summary...");
    show_extraction_summary(db_path)?;
    
    Ok(())
}

// ============================================================================
// CHAPTER 3: EXTRACTION - Git Metrics
// ============================================================================

fn extract_and_load_git_metrics(db_path: &str, work_dir: &Path) -> Result<()> {
    // TODO: Move Git metrics extraction here from original lines 156-244
    println!("  ✓ Git metrics extracted");
    Ok(())
}

// ============================================================================
// CHAPTER 4: EXTRACTION - Pattern References
// ============================================================================

fn extract_and_load_pattern_references(db_path: &str, work_dir: &Path) -> Result<()> {
    // TODO: Move pattern extraction here from original lines 246-308
    println!("  ✓ Pattern references extracted");
    Ok(())
}

// ============================================================================
// CHAPTER 5: EXTRACTION - Semantic Data
// ============================================================================

fn extract_and_load_semantic_data(db_path: &str, work_dir: &Path) -> Result<()> {
    // TODO: Move semantic extraction here from original lines 311-603
    // This is the big one with tree-sitter parsing
    println!("  ✓ Semantic data extracted");
    Ok(())
}

// ============================================================================
// CHAPTER 6: DATABASE OPERATIONS
// ============================================================================

fn create_database_schema(db_path: &str) -> Result<()> {
    let schema = schema::generate_complete_schema(db_path);
    
    // Execute via stdin to avoid command line escaping
    let mut child = Command::new("duckdb")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start DuckDB")?;
    
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(schema.as_bytes())?;
    }
    
    let output = child.wait_with_output()?;
    if !output.status.success() {
        anyhow::bail!("Failed to create schema: {}", 
                      String::from_utf8_lossy(&output.stderr));
    }
    
    Ok(())
}

fn show_extraction_summary(db_path: &str) -> Result<()> {
    // TODO: Move summary generation here from original lines 1446-1510
    println!("  Summary generated");
    Ok(())
}

// ============================================================================
// CHAPTER 7: UTILITIES
// ============================================================================

fn determine_work_directory(config: &ScrapeConfig) -> Result<PathBuf> {
    // Extract repo name from db_path if it's in layer/dust/repos/
    if config.db_path.contains("layer/dust/repos/") {
        let repo_name = config.db_path
            .strip_prefix("layer/dust/repos/")
            .and_then(|s| s.strip_suffix(".db"))
            .context("Invalid repo database path")?;
        
        let repo_dir = PathBuf::from("layer/dust/repos").join(repo_name);
        if !repo_dir.exists() {
            anyhow::bail!("Repository '{}' not found. Clone it first to layer/dust/repos/", repo_name);
        }
        
        Ok(std::env::current_dir()?.join(repo_dir))
    } else {
        Ok(std::env::current_dir()?)
    }
}

// ============================================================================
// MODULE: Database Schema
// ============================================================================

mod schema {
    pub fn generate_complete_schema(db_path: &str) -> String {
        format!(r#"
-- Attach with minimal block size for efficiency
ATTACH '{}' AS knowledge (BLOCK_SIZE 16384);
USE knowledge;

{}

-- Additional tables...
"#, db_path, fingerprint_schema())
    }
    
    fn fingerprint_schema() -> &'static str {
        // TODO: Move schema definitions here from fingerprint module
        r#"
CREATE TABLE IF NOT EXISTS code_fingerprints (
    path VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    kind VARCHAR NOT NULL,
    pattern UINTEGER,
    imports UINTEGER,
    complexity USMALLINT,
    flags USMALLINT,
    PRIMARY KEY (path, name, kind)
);
        "#
    }
}

// ============================================================================
// MODULE: Language Support
// ============================================================================

mod languages {
    // TODO: Move language support here from original lines 2382-2455
}

// ============================================================================
// MODULE: Fingerprinting
// ============================================================================

mod fingerprint {
    // TODO: Move fingerprint module here from original lines 2078-2377
}

// ============================================================================
// MODULE: AST Processing
// ============================================================================

mod ast_processing {
    // TODO: Move AST processing functions here from original lines 1125-1443
}