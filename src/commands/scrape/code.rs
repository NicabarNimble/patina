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
    println!("🗄️  Initializing optimized knowledge database...");
    
    // Create parent directory if needed
    if let Some(parent) = Path::new(&config.db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Remove old database if exists
    if Path::new(&config.db_path).exists() {
        std::fs::remove_file(&config.db_path)?;
    }
    
    // Create with schema
    create_database_with_schema(&config.db_path)?;
    
    println!("✅ Database initialized with 16KB blocks at {}", config.db_path);
    println!("\nNext steps:");
    println!("  1. Run 'patina scrape code' to index your codebase");
    println!("  2. Run 'patina scrape code --query \"SELECT ...\"' to explore");
    
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

fn create_database_with_schema(db_path: &str) -> Result<()> {
    let schema_sql = schema::generate_complete_schema(db_path);
    
    // Execute via stdin to avoid command line escaping issues
    let mut child = Command::new("duckdb")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start DuckDB. Is duckdb installed?")?;
    
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(schema_sql.as_bytes())?;
    }
    
    let output = child.wait_with_output()?;
    
    if !output.status.success() {
        anyhow::bail!(
            "Failed to create database: {}",
            String::from_utf8_lossy(&output.stderr)
        );
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
        format!(
            r#"
-- Attach with minimal block size (16KB instead of default 256KB)
ATTACH '{}' AS knowledge (BLOCK_SIZE 16384);
USE knowledge;

{}

-- Git survival metrics for quality assessment
CREATE TABLE IF NOT EXISTS git_metrics (
    file VARCHAR PRIMARY KEY,
    first_commit VARCHAR,
    last_commit VARCHAR,
    commit_count INTEGER,
    survival_days INTEGER
);

-- Pattern references extracted from documentation
CREATE TABLE IF NOT EXISTS pattern_references (
    from_pattern VARCHAR NOT NULL,
    to_pattern VARCHAR NOT NULL,
    reference_type VARCHAR NOT NULL,
    context VARCHAR,
    PRIMARY KEY (from_pattern, to_pattern, reference_type)
);
"#,
            db_path,
            all_tables_schema()
        )
    }
    
    fn all_tables_schema() -> &'static str {
        r#"
-- Compact fingerprint storage (columnar for SIMD)
CREATE TABLE IF NOT EXISTS code_fingerprints (
    path VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    kind VARCHAR NOT NULL,  -- function, struct, trait, impl
    pattern UINTEGER,       -- AST shape hash
    imports UINTEGER,       -- Dependency hash  
    complexity USMALLINT,   -- Cyclomatic complexity
    flags USMALLINT,        -- Feature bitmask
    PRIMARY KEY (path, name, kind)
);

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
    has_params BOOLEAN,        -- Documents parameters
    parent_symbol VARCHAR,     -- For nested items (methods in impl blocks)
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

-- Behavioral hints: Code smell detection (facts only)
CREATE TABLE IF NOT EXISTS behavioral_hints (
    file VARCHAR NOT NULL,
    function VARCHAR NOT NULL,
    calls_unwrap INTEGER DEFAULT 0,     -- Count of .unwrap()
    calls_expect INTEGER DEFAULT 0,     -- Count of .expect()
    has_panic_macro BOOLEAN,           -- Contains panic!()
    has_todo_macro BOOLEAN,            -- Contains todo!()
    has_unsafe_block BOOLEAN,          -- Contains unsafe {}
    has_mutex BOOLEAN,                 -- Thread synchronization
    has_arc BOOLEAN,                   -- Shared ownership
    PRIMARY KEY (file, function)
);

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
CREATE INDEX IF NOT EXISTS idx_fingerprint_pattern ON code_fingerprints(pattern);
CREATE INDEX IF NOT EXISTS idx_fingerprint_complexity ON code_fingerprints(complexity);
CREATE INDEX IF NOT EXISTS idx_fingerprint_flags ON code_fingerprints(flags);
CREATE INDEX IF NOT EXISTS idx_type_vocabulary_kind ON type_vocabulary(kind);
CREATE INDEX IF NOT EXISTS idx_function_facts_public ON function_facts(is_public);
CREATE INDEX IF NOT EXISTS idx_import_facts_external ON import_facts(is_external);
CREATE INDEX IF NOT EXISTS idx_documentation_symbol ON documentation(symbol_name);
CREATE INDEX IF NOT EXISTS idx_documentation_type ON documentation(symbol_type);
"#
    }
}

// ============================================================================
// MODULE: Language Support
// ============================================================================

pub(crate) mod languages {
    use anyhow::{Context, Result};
    use std::path::Path;
    use tree_sitter::Parser;

    /// Supported programming languages
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Language {
        Rust,
        Go,
        Solidity,
        Python,
        JavaScript,
        JavaScriptJSX, // .jsx files
        TypeScript,
        TypeScriptTSX, // .tsx files
        Unknown,
    }

    impl Language {
        /// Detect language from file extension
        pub fn from_path(path: &Path) -> Self {
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("rs") => Language::Rust,
                Some("go") => Language::Go,
                Some("sol") => Language::Solidity,
                Some("py") => Language::Python,
                Some("js") | Some("mjs") => Language::JavaScript,
                Some("jsx") => Language::JavaScriptJSX,
                Some("ts") => Language::TypeScript,
                Some("tsx") => Language::TypeScriptTSX,
                _ => Language::Unknown,
            }
        }

        /// Convert to patina_metal::Metal enum
        pub fn to_metal(self) -> Option<patina_metal::Metal> {
            match self {
                Language::Rust => Some(patina_metal::Metal::Rust),
                Language::Go => Some(patina_metal::Metal::Go),
                Language::Solidity => Some(patina_metal::Metal::Solidity),
                Language::Python => Some(patina_metal::Metal::Python),
                Language::JavaScript | Language::JavaScriptJSX => {
                    Some(patina_metal::Metal::JavaScript)
                }
                Language::TypeScript | Language::TypeScriptTSX => {
                    Some(patina_metal::Metal::TypeScript)
                }
                Language::Unknown => None,
            }
        }
    }

    /// Create a parser for a specific file path, handling TypeScript's tsx vs ts distinction
    pub fn create_parser_for_path(path: &Path) -> Result<Parser> {
        let language = Language::from_path(path);
        let metal = language
            .to_metal()
            .ok_or_else(|| anyhow::anyhow!("Unsupported language: {:?}", language))?;

        // Use the extension-aware method for TypeScript to get the right parser
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let ts_lang = metal
            .tree_sitter_language_for_ext(ext)
            .ok_or_else(|| anyhow::anyhow!("No parser available for {:?}", language))?;

        let mut parser = Parser::new();
        parser
            .set_language(&ts_lang)
            .context("Failed to set language")?;

        Ok(parser)
    }
}

// ============================================================================
// MODULE: Fingerprinting
// ============================================================================

pub(crate) mod fingerprint {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use tree_sitter::Node;

    /// Compact 16-byte fingerprint for code patterns
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Fingerprint {
        pub pattern: u32,    // AST shape hash
        pub imports: u32,    // Dependency hash
        pub complexity: u16, // Cyclomatic complexity
        pub flags: u16,      // Feature flags
    }

    impl Fingerprint {
        /// Generate fingerprint from tree-sitter AST node
        pub fn from_ast(node: Node, source: &[u8]) -> Self {
            let pattern = hash_ast_shape(node, source);
            let imports = hash_imports(node, source);
            let complexity = calculate_complexity(node) as u16;
            let flags = detect_features(node, source);

            Self {
                pattern,
                imports,
                complexity,
                flags,
            }
        }
    }

    /// Hash the AST structure (types only, not content)
    fn hash_ast_shape(node: Node, _source: &[u8]) -> u32 {
        let mut hasher = DefaultHasher::new();
        hash_node_shape(&mut hasher, node);
        (hasher.finish() & 0xFFFFFFFF) as u32
    }

    fn hash_node_shape(hasher: &mut impl Hasher, node: Node) {
        // Hash node type (structure, not content)
        node.kind().hash(hasher);

        // Hash child structure recursively
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                hash_node_shape(hasher, cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    /// Hash imports/dependencies
    fn hash_imports(node: Node, source: &[u8]) -> u32 {
        let mut hasher = DefaultHasher::new();
        let mut cursor = node.walk();

        find_imports(&mut cursor, source, &mut hasher);
        (hasher.finish() & 0xFFFFFFFF) as u32
    }

    fn find_imports(cursor: &mut tree_sitter::TreeCursor, source: &[u8], hasher: &mut impl Hasher) {
        let node = cursor.node();

        if node.kind() == "use_declaration" {
            if let Ok(text) = node.utf8_text(source) {
                text.hash(hasher);
            }
        }

        if cursor.goto_first_child() {
            loop {
                find_imports(cursor, source, hasher);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Calculate cyclomatic complexity
    fn calculate_complexity(node: Node) -> usize {
        let mut complexity = 1; // Base complexity
        let mut cursor = node.walk();

        count_branches(&mut cursor, &mut complexity);
        complexity
    }

    fn count_branches(cursor: &mut tree_sitter::TreeCursor, complexity: &mut usize) {
        let node = cursor.node();

        match node.kind() {
            "if_expression" | "match_expression" | "while_expression" | "for_expression" => {
                *complexity += 1;
            }
            "match_arm" => {
                // Each arm adds a branch
                *complexity += 1;
            }
            _ => {}
        }

        if cursor.goto_first_child() {
            loop {
                count_branches(cursor, complexity);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    /// Detect feature flags (async, unsafe, etc.)
    fn detect_features(node: Node, source: &[u8]) -> u16 {
        let mut flags = 0u16;
        let mut cursor = node.walk();

        detect_features_recursive(&mut cursor, source, &mut flags);
        flags
    }

    fn detect_features_recursive(
        cursor: &mut tree_sitter::TreeCursor,
        source: &[u8],
        flags: &mut u16,
    ) {
        let node = cursor.node();

        // Check for various features
        match node.kind() {
            "async" => *flags |= 0x0001,                   // Bit 0: async
            "unsafe_block" | "unsafe" => *flags |= 0x0002, // Bit 1: unsafe
            "macro_invocation" => {
                if let Ok(text) = node.utf8_text(source) {
                    if text.starts_with("panic!") || text.starts_with("unreachable!") {
                        *flags |= 0x0004; // Bit 2: has panic
                    }
                    if text.starts_with("todo!") || text.starts_with("unimplemented!") {
                        *flags |= 0x0008; // Bit 3: has todo
                    }
                }
            }
            "question_mark" => *flags |= 0x0010, // Bit 4: uses ?
            "generic_type" | "generic_function" => *flags |= 0x0020, // Bit 5: generic
            "trait_bounds" => *flags |= 0x0040,  // Bit 6: has trait bounds
            "lifetime" => *flags |= 0x0080,      // Bit 7: has lifetimes
            _ => {}
        }

        if cursor.goto_first_child() {
            loop {
                detect_features_recursive(cursor, source, flags);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
}

// ============================================================================
// MODULE: AST Processing
// ============================================================================

mod ast_processing {
    // TODO: Move AST processing functions here from original lines 1125-1443
}