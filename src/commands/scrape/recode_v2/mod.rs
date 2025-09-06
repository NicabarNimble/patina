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
//! - Common LanguageSpec trait defines the interface
//! - Registry pattern for language lookup
//! - Clean separation of concerns
//!
//! ## Usage
//! ```bash
//! patina scrape recode          # Index using modular architecture
//! patina scrape recode --force  # Rebuild from scratch
//! ```

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;
use tree_sitter::Node;

use crate::commands::incremental;
use super::{ScrapeConfig, ScrapeStats};

// ============================================================================
// LANGUAGE MODULES
// ============================================================================
pub mod languages;

// Re-export the Language enum for convenience
pub use languages::Language;

// ============================================================================
// LANGUAGE SPECIFICATION TRAIT
// ============================================================================
/// Common interface that each language module must implement
pub struct LanguageSpec {
    /// Check if a comment is a documentation comment
    pub is_doc_comment: fn(&str) -> bool,
    
    /// Parse visibility from node and name
    pub parse_visibility: fn(&Node, &str, &[u8]) -> bool,
    
    /// Check if function is async
    pub has_async: fn(&Node, &[u8]) -> bool,
    
    /// Check if function is unsafe
    pub has_unsafe: fn(&Node, &[u8]) -> bool,
    
    /// Extract function parameters
    pub extract_params: fn(&Node, &[u8]) -> Vec<String>,
    
    /// Extract return type
    pub extract_return_type: fn(&Node, &[u8]) -> Option<String>,
    
    /// Extract generic parameters
    pub extract_generics: fn(&Node, &[u8]) -> Option<String>,
    
    /// Map node kind to symbol kind (simple mapping)
    pub get_symbol_kind: fn(&str) -> &'static str,
    
    /// Map node to symbol kind (complex cases that need node inspection)
    pub get_symbol_kind_complex: fn(&Node, &[u8]) -> Option<&'static str>,
    
    /// Clean documentation text for a language
    pub clean_doc_comment: fn(&str) -> String,
    
    /// Extract import details from an import node
    pub extract_import_details: fn(&Node, &[u8]) -> (String, String, bool),
}

// ============================================================================
// LANGUAGE REGISTRY
// ============================================================================
/// Central registry of all language specifications
static LANGUAGE_REGISTRY: LazyLock<HashMap<Language, &'static LanguageSpec>> =
    LazyLock::new(|| {
        let mut registry = HashMap::new();
        
        // Register each language from its module
        registry.insert(Language::Rust, &languages::rust::SPEC);
        registry.insert(Language::Go, &languages::go::SPEC);
        registry.insert(Language::Python, &languages::python::SPEC);
        registry.insert(Language::JavaScript, &languages::javascript::SPEC);
        registry.insert(Language::JavaScriptJSX, &languages::javascript::SPEC); // JSX uses JS spec
        registry.insert(Language::TypeScript, &languages::typescript::SPEC);
        registry.insert(Language::TypeScriptTSX, &languages::typescript::SPEC); // TSX uses TS spec
        registry.insert(Language::Solidity, &languages::solidity::SPEC);
        registry.insert(Language::C, &languages::c::SPEC);
        registry.insert(Language::Cpp, &languages::cpp::SPEC);
        // Note: Cairo is not registered here as it uses cairo-lang-parser instead of tree-sitter
        
        registry
    });

/// Get language specification from registry
pub fn get_language_spec(language: Language) -> Option<&'static LanguageSpec> {
    LANGUAGE_REGISTRY.get(&language).copied()
}

// ============================================================================
// PUBLIC INTERFACE
// ============================================================================

/// Initialize a new knowledge database
pub fn initialize(config: &ScrapeConfig) -> Result<()> {
    println!("🗄️  Initializing optimized knowledge database (recode v2)...");
    
    // Implementation will follow the same pattern as code.rs/recode.rs
    // but with cleaner separation
    todo!("Port initialization logic from recode.rs")
}

/// Main entry point for the recode command
pub fn run(config: ScrapeConfig) -> Result<super::ScrapeStats> {
    println!("🔄 Running recode v2 with modular language architecture...");
    
    let start = std::time::Instant::now();
    
    // This will be the main processing pipeline
    // For now, return placeholder stats
    Ok(super::ScrapeStats {
        items_processed: 0,
        time_elapsed: start.elapsed(),
        database_size_kb: 0,
    })
}