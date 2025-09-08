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
use std::sync::LazyLock;
use tree_sitter::Node;

use self::sql_builder::{InsertBuilder, SqlValue, TableName};
use self::types::{CallGraphEntry, CallType, FilePath, SymbolKind, SymbolName};
use super::ScrapeConfig;

// ============================================================================
// LANGUAGE MODULES
// ============================================================================
pub mod database;
pub mod extract_v2;
pub mod extracted_data;
pub mod languages;
pub mod sql_builder;
pub mod types;

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
    pub get_symbol_kind: fn(&str) -> SymbolKind,

    /// Map node to symbol kind (complex cases that need node inspection)
    pub get_symbol_kind_complex: fn(&Node, &[u8]) -> Option<SymbolKind>,

    /// Clean documentation text for a language
    pub clean_doc_comment: fn(&str) -> String,

    /// Extract import details from an import node
    pub extract_import_details: fn(&Node, &[u8]) -> (String, String, bool),

    /// Extract call expressions from a node (language-specific)
    /// This allows each language to handle its unique call patterns
    pub extract_calls: Option<fn(&Node, &[u8], &mut ParseContext)>,
}

// ============================================================================
// LANGUAGE REGISTRY
// ============================================================================
/// Central registry of all language specifications
static LANGUAGE_REGISTRY: LazyLock<HashMap<Language, &'static LanguageSpec>> =
    LazyLock::new(|| {
        let registry = HashMap::new();

        // All languages now use isolated processors instead of LanguageSpec
        // The registry is kept for backward compatibility but is empty

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
    let items_processed = extract_v2::extract_code_metadata_v2(&config.db_path, &work_dir, config.force)?;

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

fn extract_and_index(db_path: &str, work_dir: &Path, force: bool) -> Result<usize> {
    println!("🔍 Indexing codebase...\n");

    // If force flag is set, reinitialize database to ensure clean state
    if force {
        initialize_database(db_path)?;
    }

    // Step 3: Extract code metadata with tree-sitter
    let symbol_count = extract_code_metadata(db_path, work_dir, force)?;

    println!("\n✅ Extraction complete!");
    Ok(symbol_count)
}

// ============================================================================
// PARSING CONTEXT
// ============================================================================
#[derive(Default)]
pub struct ParseContext {
    pub current_function: Option<String>,
    pub call_graph_entries: Vec<CallGraphEntry>,
}

/// Groups parameters for symbol processing to avoid too many arguments
struct SymbolInfo<'a> {
    node: &'a Node<'a>,
    name: SymbolName<'a>,
    kind: SymbolKind,
    source: &'a [u8],
    file_path: FilePath<'a>,
    language: Language,
}

impl ParseContext {
    pub fn add_call(&mut self, callee: String, call_type: CallType, line_number: i32) {
        if let Some(ref caller) = self.current_function {
            self.call_graph_entries.push(CallGraphEntry::new(
                caller.clone(),
                callee,
                call_type,
                line_number,
            ));
        }
    }
}

fn extract_code_metadata(db_path: &str, work_dir: &Path, _force: bool) -> Result<usize> {
    println!("🧠 Extracting code metadata and semantic information...");

    use ignore::WalkBuilder;
    use std::time::SystemTime;

    // Find all supported language files with their detected language
    let mut all_files: Vec<(PathBuf, Language)> = Vec::new();

    for entry in WalkBuilder::new(work_dir)
        .hidden(false)
        .git_ignore(true)
        .build()
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let language = Language::from_path(path);
            if !matches!(language, Language::Unknown) {
                all_files.push((path.to_path_buf(), language));
            }
        }
    }

    println!("  Found {} source files", all_files.len());
    if all_files.is_empty() {
        println!("  No source files found. Is this a code repository?");
        return Ok(0);
    }

    // Process files and build SQL statements
    let mut sql_statements = String::with_capacity(1024 * 1024); // Pre-allocate 1MB
    sql_statements.push_str("BEGIN TRANSACTION;\n");

    let mut functions_count = 0;
    let mut types_count = 0;
    let mut imports_count = 0;
    let mut files_with_errors = 0;
    let mut _files_processed = 0;

    // Separate files by processing type: all languages now use isolated processors
    let mut cairo_files = Vec::new();
    let mut c_files = Vec::new();
    let mut cpp_files = Vec::new();
    let mut rust_files = Vec::new();
    let mut go_files = Vec::new();
    let mut python_files = Vec::new();
    let mut javascript_files = Vec::new();
    let mut typescript_files = Vec::new();
    let mut solidity_files = Vec::new();
    let mut treesitter_files = Vec::new();
    
    for (path, lang) in all_files {
        match lang {
            Language::Cairo => cairo_files.push((path, lang)),
            Language::C => c_files.push((path, lang)),
            Language::Cpp => cpp_files.push((path, lang)),
            Language::Rust => rust_files.push((path, lang)),
            Language::Go => go_files.push((path, lang)),
            Language::Python => python_files.push((path, lang)),
            Language::JavaScript | Language::JavaScriptJSX => javascript_files.push((path, lang)),
            Language::TypeScript | Language::TypeScriptTSX => typescript_files.push((path, lang)),
            Language::Solidity => solidity_files.push((path, lang)),
            _ => treesitter_files.push((path, lang)),
        }
    }

    // Process Cairo files with special parser
    for (file_path, _language) in cairo_files {
        let relative_path = if let Ok(stripped) = file_path.strip_prefix(work_dir) {
            format!("./{}", stripped.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Read file content as string (Cairo parser needs UTF-8 text)
        let content = match std::fs::read_to_string(&file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ⚠️  Failed to read {}: {}", relative_path, e);
                files_with_errors += 1;
                continue;
            }
        };

        // Track file in index_state
        let mtime = std::fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let insert_sql = InsertBuilder::new(TableName::INDEX_STATE)
            .or_replace()
            .value("path", relative_path.as_str())
            .value("mtime", mtime)
            .build();
        sql_statements.push_str(&insert_sql);
        sql_statements.push_str(";\n");

        // Process Cairo file with special parser
        match languages::cairo::CairoProcessor::process_file(
            FilePath::from(relative_path.as_str()),
            &content,
        ) {
            Ok((statements, funcs, types, imps)) => {
                for stmt in statements {
                    sql_statements.push_str(&stmt);
                    sql_statements.push('\n');
                }
                functions_count += funcs;
                types_count += types;
                imports_count += imps;
                _files_processed += 1;
            }
            Err(e) => {
                eprintln!("  ⚠️  Cairo parsing error in {}: {}", relative_path, e);
                files_with_errors += 1;
            }
        }
    }

    // Process C files with isolated processor
    for (file_path, _language) in c_files {
        let relative_path = if let Ok(stripped) = file_path.strip_prefix(work_dir) {
            format!("./{}", stripped.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Read file content
        let content = match std::fs::read(&file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ⚠️  Failed to read {}: {}", relative_path, e);
                files_with_errors += 1;
                continue;
            }
        };

        // Track file in index_state
        let mtime = std::fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let insert_sql = InsertBuilder::new(TableName::INDEX_STATE)
            .or_replace()
            .value("path", relative_path.as_str())
            .value("mtime", mtime)
            .build();
        sql_statements.push_str(&insert_sql);
        sql_statements.push_str(";\n");

        // Process C file with isolated processor
        match languages::c::CProcessor::process_file(
            FilePath::from(relative_path.as_str()),
            &content,
        ) {
            Ok((statements, funcs, types, imps)) => {
                for stmt in statements {
                    sql_statements.push_str(&stmt);
                    sql_statements.push('\n');
                }
                functions_count += funcs;
                types_count += types;
                imports_count += imps;
                _files_processed += 1;
            }
            Err(e) => {
                eprintln!("  ⚠️  C parsing error in {}: {}", relative_path, e);
                files_with_errors += 1;
            }
        }
    }

    // Process C++ files with isolated processor
    for (file_path, _language) in cpp_files {
        let relative_path = if let Ok(stripped) = file_path.strip_prefix(work_dir) {
            format!("./{}", stripped.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Read file content
        let content = match std::fs::read(&file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ⚠️  Failed to read {}: {}", relative_path, e);
                files_with_errors += 1;
                continue;
            }
        };

        // Track file in index_state
        let mtime = std::fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let insert_sql = InsertBuilder::new(TableName::INDEX_STATE)
            .or_replace()
            .value("path", relative_path.as_str())
            .value("mtime", mtime)
            .build();
        sql_statements.push_str(&insert_sql);
        sql_statements.push_str(";\n");

        // Process C++ file with isolated processor
        match languages::cpp::CppProcessor::process_file(
            FilePath::from(relative_path.as_str()),
            &content,
        ) {
            Ok((statements, funcs, types, imps)) => {
                for stmt in statements {
                    sql_statements.push_str(&stmt);
                    sql_statements.push('\n');
                }
                functions_count += funcs;
                types_count += types;
                imports_count += imps;
                _files_processed += 1;
            }
            Err(e) => {
                eprintln!("  ⚠️  C++ parsing error in {}: {}", relative_path, e);
                files_with_errors += 1;
            }
        }
    }

    // Process Rust files with isolated processor
    for (file_path, _language) in rust_files {
        let relative_path = if let Ok(stripped) = file_path.strip_prefix(work_dir) {
            format!("./{}", stripped.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Read file content
        let content = match std::fs::read(&file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ⚠️  Failed to read {}: {}", relative_path, e);
                files_with_errors += 1;
                continue;
            }
        };

        // Track file in index_state
        let mtime = std::fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let insert_sql = InsertBuilder::new(TableName::INDEX_STATE)
            .or_replace()
            .value("path", relative_path.as_str())
            .value("mtime", mtime)
            .build();
        sql_statements.push_str(&insert_sql);
        sql_statements.push_str(";\n");

        // Process Rust file with isolated processor (now returns ExtractedData)
        match languages::rust::RustProcessor::process_file(
            FilePath::from(relative_path.as_str()),
            &content,
        ) {
            Ok(extracted_data) => {
                // For now, just count the items (old SQL path is deprecated)
                functions_count += extracted_data.functions.len();
                types_count += extracted_data.types.len();
                imports_count += extracted_data.imports.len();
                _files_processed += 1;
                
                // TODO: This entire old SQL path should be removed
                // For now, we skip SQL generation for Rust files
            }
            Err(e) => {
                eprintln!("  ⚠️  Rust parsing error in {}: {}", relative_path, e);
                files_with_errors += 1;
            }
        }
    }

    // Process Go files with isolated processor
    for (file_path, _language) in go_files {
        let relative_path = if let Ok(stripped) = file_path.strip_prefix(work_dir) {
            format!("./{}", stripped.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Read file content
        let content = match std::fs::read(&file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ⚠️  Failed to read {}: {}", relative_path, e);
                files_with_errors += 1;
                continue;
            }
        };

        // Track file in index_state
        let mtime = std::fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let insert_sql = InsertBuilder::new(TableName::INDEX_STATE)
            .or_replace()
            .value("path", relative_path.as_str())
            .value("mtime", mtime)
            .build();
        sql_statements.push_str(&insert_sql);
        sql_statements.push_str(";\n");

        // Process Go file with isolated processor
        match languages::go::GoProcessor::process_file(
            FilePath::from(relative_path.as_str()),
            &content,
        ) {
            Ok((statements, funcs, types, imps)) => {
                for stmt in statements {
                    sql_statements.push_str(&stmt);
                    sql_statements.push('\n');
                }
                functions_count += funcs;
                types_count += types;
                imports_count += imps;
                _files_processed += 1;
            }
            Err(e) => {
                eprintln!("  ⚠️  Go parsing error in {}: {}", relative_path, e);
                files_with_errors += 1;
            }
        }
    }

    // Process Python files with isolated processor
    for (file_path, _language) in python_files {
        let relative_path = if let Ok(stripped) = file_path.strip_prefix(work_dir) {
            format!("./{}", stripped.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Read file content
        let content = match std::fs::read(&file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ⚠️  Failed to read {}: {}", relative_path, e);
                files_with_errors += 1;
                continue;
            }
        };

        // Track file in index_state
        let mtime = std::fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let insert_sql = InsertBuilder::new(TableName::INDEX_STATE)
            .or_replace()
            .value("path", relative_path.as_str())
            .value("mtime", mtime)
            .build();
        sql_statements.push_str(&insert_sql);
        sql_statements.push_str(";\n");

        // Process Python file with isolated processor (now returns ExtractedData)
        match languages::python::PythonProcessor::process_file(
            FilePath::from(relative_path.as_str()),
            &content,
        ) {
            Ok(extracted_data) => {
                // For now, just count the items (old SQL path is deprecated)
                functions_count += extracted_data.functions.len();
                types_count += extracted_data.types.len();
                imports_count += extracted_data.imports.len();
                _files_processed += 1;
                
                // TODO: This entire old SQL path should be removed
                // For now, we skip SQL generation for Python files
            }
            Err(e) => {
                eprintln!("  ⚠️  Python parsing error in {}: {}", relative_path, e);
                files_with_errors += 1;
            }
        }
    }

    // Process JavaScript files with isolated processor (handles both .js and .jsx)
    for (file_path, _language) in javascript_files {
        let relative_path = if let Ok(stripped) = file_path.strip_prefix(work_dir) {
            format!("./{}", stripped.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Read file content
        let content = match std::fs::read(&file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ⚠️  Failed to read {}: {}", relative_path, e);
                files_with_errors += 1;
                continue;
            }
        };

        // Track file in index_state
        let mtime = std::fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let insert_sql = InsertBuilder::new(TableName::INDEX_STATE)
            .or_replace()
            .value("path", relative_path.as_str())
            .value("mtime", mtime)
            .build();
        sql_statements.push_str(&insert_sql);
        sql_statements.push_str(";\n");

        // Process JavaScript file with isolated processor (now returns ExtractedData)
        match languages::javascript::JavaScriptProcessor::process_file(
            FilePath::from(relative_path.as_str()),
            &content,
        ) {
            Ok(extracted_data) => {
                // For now, just count the items (old SQL path is deprecated)
                functions_count += extracted_data.functions.len();
                types_count += extracted_data.types.len();
                imports_count += extracted_data.imports.len();
                _files_processed += 1;
                
                // TODO: This entire old SQL path should be removed
                // For now, we skip SQL generation for JavaScript files
            }
            Err(e) => {
                eprintln!("  ⚠️  JavaScript parsing error in {}: {}", relative_path, e);
                files_with_errors += 1;
            }
        }
    }

    // Process TypeScript files with isolated processor (handles both .ts and .tsx)
    for (file_path, _language) in typescript_files {
        let relative_path = if let Ok(stripped) = file_path.strip_prefix(work_dir) {
            format!("./{}", stripped.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Read file content
        let content = match std::fs::read(&file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ⚠️  Failed to read {}: {}", relative_path, e);
                files_with_errors += 1;
                continue;
            }
        };

        // Track file in index_state
        let mtime = std::fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let insert_sql = InsertBuilder::new(TableName::INDEX_STATE)
            .or_replace()
            .value("path", relative_path.as_str())
            .value("mtime", mtime)
            .build();
        sql_statements.push_str(&insert_sql);
        sql_statements.push_str(";\n");

        // Process TypeScript file with isolated processor (now returns ExtractedData)
        match languages::typescript::TypeScriptProcessor::process_file(
            FilePath::from(relative_path.as_str()),
            &content,
        ) {
            Ok(extracted_data) => {
                // For now, just count the items (old SQL path is deprecated)
                functions_count += extracted_data.functions.len();
                types_count += extracted_data.types.len();
                imports_count += extracted_data.imports.len();
                _files_processed += 1;
                
                // TODO: This entire old SQL path should be removed
                // For now, we skip SQL generation for TypeScript files
            }
            Err(e) => {
                eprintln!("  ⚠️  TypeScript parsing error in {}: {}", relative_path, e);
                files_with_errors += 1;
            }
        }
    }

    // Process Solidity files with isolated processor
    for (file_path, _language) in solidity_files {
        let relative_path = if let Ok(stripped) = file_path.strip_prefix(work_dir) {
            format!("./{}", stripped.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Read file content
        let content = match std::fs::read(&file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ⚠️  Failed to read {}: {}", relative_path, e);
                files_with_errors += 1;
                continue;
            }
        };

        // Track file in index_state
        let mtime = std::fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let insert_sql = InsertBuilder::new(TableName::INDEX_STATE)
            .or_replace()
            .value("path", relative_path.as_str())
            .value("mtime", mtime)
            .build();
        sql_statements.push_str(&insert_sql);
        sql_statements.push_str(";\n");

        // Process Solidity file with isolated processor
        match languages::solidity::SolidityProcessor::process_file(
            FilePath::from(relative_path.as_str()),
            &content,
        ) {
            Ok((statements, funcs, types, imps)) => {
                for stmt in statements {
                    sql_statements.push_str(&stmt);
                    sql_statements.push('\n');
                }
                functions_count += funcs;
                types_count += types;
                imports_count += imps;
                _files_processed += 1;
            }
            Err(e) => {
                eprintln!("  ⚠️  Solidity parsing error in {}: {}", relative_path, e);
                files_with_errors += 1;
            }
        }
    }

    // Process tree-sitter files
    for (file_path, language) in treesitter_files {
        let relative_path = if let Ok(stripped) = file_path.strip_prefix(work_dir) {
            format!("./{}", stripped.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Read file content
        let source = match std::fs::read(&file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ⚠️  Failed to read {}: {}", relative_path, e);
                files_with_errors += 1;
                continue;
            }
        };

        // Create parser for this file (handles TypeScript variants correctly)
        let mut parser = match languages::create_parser_for_path(&file_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!(
                    "  ⚠️  Failed to create parser for {} ({}): {}",
                    relative_path,
                    language.name(),
                    e
                );
                files_with_errors += 1;
                continue;
            }
        };

        // Parse the file
        let tree = match parser.parse(&source, None) {
            Some(t) => t,
            None => {
                eprintln!("  ⚠️  Failed to parse {}", relative_path);
                files_with_errors += 1;
                continue;
            }
        };

        // Track file in index_state
        let mtime = std::fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let insert_sql = InsertBuilder::new(TableName::INDEX_STATE)
            .or_replace()
            .value("path", relative_path.as_str())
            .value("mtime", mtime)
            .build();
        sql_statements.push_str(&insert_sql);
        sql_statements.push_str(";\n");

        // Extract symbols from AST
        let mut context = ParseContext::default();
        let (funcs, types, imps) = extract_symbols_from_tree(
            tree.root_node(),
            &source,
            FilePath::from(relative_path.as_str()),
            language,
            &mut sql_statements,
            &mut context,
        );
        functions_count += funcs;
        types_count += types;
        imports_count += imps;
        _files_processed += 1;

        // Add call graph entries with proper file path
        for entry in &context.call_graph_entries {
            let insert_sql = InsertBuilder::new(TableName::CALL_GRAPH)
                .value("caller", entry.caller.as_str())
                .value("callee", entry.callee.as_str())
                .value("file", relative_path.as_str())
                .value("call_type", entry.call_type.as_str())
                .value("line_number", entry.line_number as i64)
                .build();
            sql_statements.push_str(&insert_sql);
            sql_statements.push_str(";\n");
        }
    }

    sql_statements.push_str("COMMIT;\n");

    // Execute all SQL in one batch
    let total_stored = functions_count + types_count + imports_count;
    if total_stored > 0 {
        println!(
            "  💾 Writing {} functions, {} types, {} imports to database...",
            functions_count, types_count, imports_count
        );
        execute_sql_batch(db_path, &sql_statements)?;
    }

    if files_with_errors > 0 {
        println!(
            "  ⚠️  {} files had parsing errors and were skipped",
            files_with_errors
        );
    }

    Ok(total_stored)
}

fn extract_symbols_from_tree(
    node: Node,
    source: &[u8],
    file_path: FilePath,
    language: Language,
    sql: &mut String,
    context: &mut ParseContext,
) -> (usize, usize, usize) {
    // (functions, types, imports)
    let mut function_count = 0;
    let mut type_count = 0;
    let mut import_count = 0;

    // Get language spec
    let spec = match get_language_spec(language) {
        Some(s) => s,
        None => return (0, 0, 0),
    };

    // First, extract any call expressions from this node regardless of whether it's a symbol
    extract_call_expressions(node, source, language, context);

    // Determine symbol kind
    let symbol_kind = (spec.get_symbol_kind)(node.kind());

    // Handle imports specially - they don't have a "name" field
    if symbol_kind == SymbolKind::Import {
        extract_import_fact(node, source, file_path, sql, language);
        import_count += 1;
        // Still need to recurse into children
    } else if symbol_kind == SymbolKind::Impl || symbol_kind == SymbolKind::Module {
        // Skip impl blocks and modules - they don't get stored in tables
        // but still recurse into their children
    } else if symbol_kind == SymbolKind::Unknown {
        // Try complex symbol detection
        if let Some(kind) = (spec.get_symbol_kind_complex)(&node, source) {
            // Process this symbol - handle different name extraction strategies
            let name = extract_symbol_name(&node, source, language).or_else(|| {
                node.child_by_field_name("name")
                    .and_then(|n| n.utf8_text(source).ok())
                    .map(|s| s.to_string())
            });

            if let Some(name) = name {
                let symbol_info = SymbolInfo {
                    node: &node,
                    name: SymbolName::from(name.as_str()),
                    kind,
                    source,
                    file_path: FilePath::from(file_path),
                    language,
                };
                let (is_func, is_type) = process_symbol(symbol_info, sql, context);
                if is_func {
                    function_count += 1;
                }
                if is_type {
                    type_count += 1;
                }
            }
        }
    } else if symbol_kind != SymbolKind::Unknown {
        // Process regular symbol
        if let Some(name) = extract_symbol_name(&node, source, language) {
            let symbol_info = SymbolInfo {
                node: &node,
                name: SymbolName::from(name.as_str()),
                kind: symbol_kind,
                source,
                file_path: FilePath::from(file_path),
                language,
            };
            let (is_func, is_type) = process_symbol(symbol_info, sql, context);
            if is_func {
                function_count += 1;
            }
            if is_type {
                type_count += 1;
            }
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let (f, t, i) = extract_symbols_from_tree(child, source, file_path, language, sql, context);
        function_count += f;
        type_count += t;
        import_count += i;
    }

    (function_count, type_count, import_count)
}

fn extract_symbol_name(node: &Node, source: &[u8], _language: Language) -> Option<String> {
    // Standard name extraction - C/C++ now handled by their isolated processors
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.to_string())
}

fn process_symbol(
    symbol: SymbolInfo,
    sql: &mut String,
    context: &mut ParseContext,
) -> (bool, bool) {
    // (is_function, is_type)
    // Extract documentation first (applies to all symbol types)
    if let Some((doc_raw, doc_clean, keywords)) =
        extract_doc_comment(*symbol.node, symbol.source, symbol.language)
    {
        let summary = extract_summary(&doc_clean);
        let has_examples = doc_raw.contains("```") || doc_raw.contains("Example:");

        let insert_sql = InsertBuilder::new(TableName::DOCUMENTATION)
            .or_replace()
            .value("file", symbol.file_path.as_str())
            .value("symbol_name", symbol.name.as_str())
            .value("symbol_type", symbol.kind.as_str())
            .value("line_number", (symbol.node.start_position().row + 1) as i64)
            .value("doc_raw", doc_raw)
            .value("doc_clean", doc_clean.clone())
            .value("doc_summary", summary)
            .value("keywords", SqlValue::Array(keywords))
            .value("doc_length", doc_clean.len() as i64)
            .value("has_examples", has_examples)
            .build();
        sql.push_str(&insert_sql);
        sql.push_str(";\n");
    }

    match symbol.kind {
        SymbolKind::Function => {
            // Update context with current function
            context.current_function = Some(symbol.name.as_str().to_string());

            extract_function_facts(
                symbol.node,
                symbol.source,
                symbol.file_path.as_str(),
                symbol.name.as_str(),
                sql,
                symbol.language,
            );

            // Add to code_search table
            let signature = symbol
                .node
                .utf8_text(symbol.source)
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("");

            let insert_sql = InsertBuilder::new(TableName::CODE_SEARCH)
                .or_replace()
                .value("path", symbol.file_path.as_str())
                .value("name", symbol.name.as_str())
                .value("signature", signature)
                .build();
            sql.push_str(&insert_sql);
            sql.push_str(";\n");
            (true, false)
        }
        SymbolKind::Struct
        | SymbolKind::Class
        | SymbolKind::Enum
        | SymbolKind::Interface
        | SymbolKind::TypeAlias
        | SymbolKind::Const
        | SymbolKind::Static => {
            extract_type_definition(
                symbol.node,
                symbol.source,
                symbol.file_path.as_str(),
                symbol.name.as_str(),
                symbol.kind,
                sql,
                symbol.language,
            );
            (false, true)
        }
        SymbolKind::Import => {
            // This shouldn't be reached anymore as imports are handled specially
            // in extract_symbols_from_tree, but keep for safety
            extract_import_fact(
                *symbol.node,
                symbol.source,
                symbol.file_path,
                sql,
                symbol.language,
            );
            (false, false)
        }
        SymbolKind::Impl | SymbolKind::Module => {
            // These don't get stored, just used for context/recursion
            (false, false)
        }
        SymbolKind::Trait | SymbolKind::Unknown => (false, false),
    }
}

fn extract_function_facts(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    sql: &mut String,
    language: Language,
) {
    let spec = match get_language_spec(language) {
        Some(s) => s,
        None => return,
    };

    // Extract visibility
    let is_public = (spec.parse_visibility)(node, name, source);

    // Extract async/unsafe
    let is_async = (spec.has_async)(node, source);
    let is_unsafe = (spec.has_unsafe)(node, source);

    // Extract parameters
    let params = (spec.extract_params)(node, source);
    let param_count = params.len() as i32;

    // Analyze parameters for mutability
    let takes_mut_self = params.iter().any(|p| p.contains("&mut self"));
    let takes_mut_params = params
        .iter()
        .any(|p| p.contains("&mut ") && !p.contains("self"));

    // Extract return type
    let return_type = (spec.extract_return_type)(node, source);
    let returns_result = return_type
        .as_ref()
        .map(|rt| rt.contains("Result"))
        .unwrap_or(false);
    let returns_option = return_type
        .as_ref()
        .map(|rt| rt.contains("Option"))
        .unwrap_or(false);

    // Extract generics and format parameters
    let generics = if let Some(spec) = get_language_spec(language) {
        (spec.extract_generics)(node, source).unwrap_or_else(String::new)
    } else {
        String::new()
    };

    let params_str = params.join(", ");
    let generic_count = if generics.is_empty() {
        0
    } else {
        generics.matches(',').count() + 1
    };
    let return_type_str = return_type.unwrap_or_else(String::new);

    let insert_sql = InsertBuilder::new(TableName::FUNCTION_FACTS)
        .or_replace()
        .value("file", file_path)
        .value("name", name)
        .value("takes_mut_self", takes_mut_self)
        .value("takes_mut_params", takes_mut_params)
        .value("returns_result", returns_result)
        .value("returns_option", returns_option)
        .value("is_async", is_async)
        .value("is_unsafe", is_unsafe)
        .value("is_public", is_public)
        .value("parameter_count", param_count as i64)
        .value("generic_count", generic_count as i64)
        .value("parameters", params_str)
        .value("return_type", return_type_str)
        .build();
    sql.push_str(&insert_sql);
    sql.push_str(";\n");
}

fn extract_type_definition(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: SymbolKind,
    sql: &mut String,
    language: Language,
) {
    let spec = match get_language_spec(language) {
        Some(s) => s,
        None => return,
    };

    // Extract visibility
    let visibility = if (spec.parse_visibility)(node, name, source) {
        "pub"
    } else {
        "private"
    };

    // Get the full definition text
    let definition = node
        .utf8_text(source)
        .unwrap_or("")
        .lines()
        .take(5) // Limit to first 5 lines
        .collect::<Vec<_>>()
        .join("\n");

    let insert_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
        .or_replace()
        .value("file", file_path)
        .value("name", name)
        .value("definition", definition)
        .value("kind", kind.as_str())
        .value("visibility", visibility)
        .build();
    sql.push_str(&insert_sql);
    sql.push_str(";\n");
}

fn extract_import_fact(
    node: Node,
    source: &[u8],
    file_path: FilePath,
    sql: &mut String,
    language: Language,
) {
    if let Some(spec) = get_language_spec(language) {
        let (imported_item, imported_from, is_external) =
            (spec.extract_import_details)(&node, source);

        let insert_sql = InsertBuilder::new(TableName::IMPORT_FACTS)
            .or_replace()
            .value("importer_file", file_path.as_str())
            .value("imported_item", imported_item)
            .value("imported_from", imported_from)
            .value("is_external", is_external)
            .value("import_kind", "use")
            .build();
        sql.push_str(&insert_sql);
        sql.push_str(";\n");
    }
}

fn extract_call_expressions(
    node: Node,
    source: &[u8],
    language: Language,
    context: &mut ParseContext,
) {
    // All languages now have their own extract_calls implementation
    if let Some(spec) = get_language_spec(language) {
        if let Some(extract_fn) = spec.extract_calls {
            // Use language-specific implementation
            extract_fn(&node, source, context);
        }
    }
    // No fallback needed - all languages are migrated!
}

fn extract_doc_comment(
    node: Node,
    source: &[u8],
    language: Language,
) -> Option<(String, String, Vec<String>)> {
    let spec = get_language_spec(language)?;

    // Look for doc comment in previous sibling
    if let Some(prev) = node.prev_sibling() {
        let is_doc = match prev.kind() {
            "comment" | "line_comment" | "block_comment" => {
                if let Ok(text) = prev.utf8_text(source) {
                    (spec.is_doc_comment)(text)
                } else {
                    false
                }
            }
            _ => false,
        };

        if is_doc {
            if let Ok(raw) = prev.utf8_text(source) {
                let clean = (spec.clean_doc_comment)(raw);
                let keywords = extract_keywords(&clean);
                return Some((raw.to_string(), clean, keywords));
            }
        }
    }

    None
}

fn extract_summary(doc: &str) -> String {
    doc.split('.').next().unwrap_or(doc).trim().to_string()
}

fn extract_keywords(doc: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "the", "and", "for", "with", "this", "that", "from", "into", "will", "can", "may", "must",
        "should", "would", "could", "has", "have", "had", "does", "did", "are", "was", "were",
        "been", "being", "get", "set", "new", "all", "some", "any", "each", "every",
    ];

    let words: std::collections::HashSet<String> = doc
        .split_whitespace()
        .flat_map(|word| word.split(|c: char| !c.is_alphanumeric()))
        .filter(|w| w.len() > 3)
        .map(|w| w.to_lowercase())
        .filter(|w| !STOP_WORDS.contains(&w.as_str()))
        .collect();

    words.into_iter().take(10).collect()
}

fn execute_sql_batch(db_path: &str, sql: &str) -> Result<()> {
    use std::process::Command;

    let mut child = Command::new("duckdb")
        .arg(db_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start DuckDB")?;

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(sql.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to execute SQL: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}
