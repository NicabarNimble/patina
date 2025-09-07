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

use super::ScrapeConfig;

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
    
    // Extract and index
    let items_processed = extract_and_index(&config.db_path, &work_dir, config.force)?;
    
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

/// Initialize DuckDB database with lean schema and optimal settings for small size
fn initialize_database(db_path: &str) -> Result<()> {
    use std::process::Command;
    
    // Create parent directory if needed
    if let Some(parent) = Path::new(db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Remove old database if exists
    if Path::new(db_path).exists() {
        std::fs::remove_file(db_path)?;
    }

    // Create with 16KB block size for minimal overhead
    let init_script = format!(
        r#"
-- Attach with minimal block size (16KB instead of default 256KB)
ATTACH '{db_path}' AS knowledge (BLOCK_SIZE 16384);
USE knowledge;

{}

"#,
        generate_schema(),
        db_path = db_path
    );

    // Execute via stdin to avoid command line escaping issues
    let mut child = Command::new("duckdb")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start DuckDB. Is duckdb installed?")?;

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(init_script.as_bytes())?;
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
struct ParseContext {
    current_function: Option<String>,
    call_graph_entries: Vec<(String, String, String, String, i32)>, // (caller, callee, file, call_type, line)
}

impl ParseContext {
    fn add_call(&mut self, callee: String, call_type: String, line_number: i32) {
        if let Some(ref caller) = self.current_function {
            self.call_graph_entries.push((
                caller.clone(),
                callee,
                String::new(), // file will be filled in later
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

    // Find all supported language files
    let mut all_files = Vec::new();

    for entry in WalkBuilder::new(work_dir)
        .hidden(false)
        .git_ignore(true)
        .build()
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy();
                // Check if this is a supported language file
                if Language::from_extension(&ext_str).is_some() {
                    all_files.push(path.to_path_buf());
                }
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

    for file_path in &all_files {
        let relative_path = if let Ok(stripped) = file_path.strip_prefix(work_dir) {
            format!("./{}", stripped.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Read file content
        let source = match std::fs::read(file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ⚠️  Failed to read {}: {}", relative_path, e);
                files_with_errors += 1;
                continue;
            }
        };

        // Detect language
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let language = match Language::from_extension(ext) {
            Some(lang) => lang,
            None => continue,
        };

        // Create parser for this language
        let mut parser = match languages::create_parser_for_language(language) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("  ⚠️  Failed to create parser for {}: {}", relative_path, e);
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
        let mtime = std::fs::metadata(file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        sql_statements.push_str(&format!(
            "INSERT OR REPLACE INTO index_state (path, mtime) VALUES ('{}', {});\n",
            escape_sql(&relative_path),
            mtime
        ));

        // Extract symbols from AST
        let mut context = ParseContext::default();
        let (funcs, types, imps) = extract_symbols_from_tree(
            tree.root_node(),
            &source,
            &relative_path,
            language,
            &mut sql_statements,
            &mut context,
        );
        functions_count += funcs;
        types_count += types;
        imports_count += imps;

        // Add call graph entries with proper file path
        for (caller, callee, _file, call_type, line) in &context.call_graph_entries {
            sql_statements.push_str(&format!(
                "INSERT INTO call_graph (caller, callee, file, call_type, line_number) VALUES ('{}', '{}', '{}', '{}', {});\n",
                escape_sql(caller),
                escape_sql(callee),
                escape_sql(&relative_path),
                call_type,
                line
            ));
        }
    }

    sql_statements.push_str("COMMIT;\n");

    // Execute all SQL in one batch
    let total_stored = functions_count + types_count + imports_count;
    if total_stored > 0 {
        println!("  💾 Writing {} functions, {} types, {} imports to database...", 
                 functions_count, types_count, imports_count);
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
    file_path: &str,
    language: Language,
    sql: &mut String,
    context: &mut ParseContext,
) -> (usize, usize, usize) { // (functions, types, imports)
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
    if symbol_kind == "import" {
        extract_import_fact(node, source, file_path, sql, language);
        import_count += 1;
        // Still need to recurse into children
    } else if symbol_kind == "impl" || symbol_kind == "module" {
        // Skip impl blocks and modules - they don't get stored in tables
        // but still recurse into their children
    } else if symbol_kind == "unknown" {
        // Try complex symbol detection
        if let Some(kind) = (spec.get_symbol_kind_complex)(&node, source) {
            // Process this symbol - handle different name extraction strategies
            let name = extract_symbol_name(&node, source, language)
                .or_else(|| {
                    node.child_by_field_name("name")
                        .and_then(|n| n.utf8_text(source).ok())
                        .map(|s| s.to_string())
                });
                
            if let Some(name) = name {
                let (is_func, is_type) = process_symbol(
                    &node,
                    &name,
                    kind,
                    source,
                    file_path,
                    language,
                    sql,
                    context,
                );
                if is_func { function_count += 1; }
                if is_type { type_count += 1; }
            }
        }
    } else if symbol_kind != "unknown" {
        // Process regular symbol
        if let Some(name) = extract_symbol_name(&node, source, language) {
            let (is_func, is_type) = process_symbol(
                &node,
                &name,
                symbol_kind,
                source,
                file_path,
                language,
                sql,
                context,
            );
            if is_func { function_count += 1; }
            if is_type { type_count += 1; }
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let (f, t, i) = extract_symbols_from_tree(
            child,
            source,
            file_path,
            language,
            sql,
            context,
        );
        function_count += f;
        type_count += t;
        import_count += i;
    }

    (function_count, type_count, import_count)
}

fn extract_symbol_name(node: &Node, source: &[u8], language: Language) -> Option<String> {
    // Special handling for C/C++ function declarators
    if language == Language::C || language == Language::Cpp {
        if node.kind() == "function_definition" {
            if let Some(declarator) = node.child_by_field_name("declarator") {
                if let Some(name_node) = extract_c_function_name(declarator) {
                    return name_node.utf8_text(source).ok().map(|s| s.to_string());
                }
            }
        }
    }

    // Standard name extraction
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.to_string())
}

/// Extract C function name from declarator (iterative to avoid stack overflow)
fn extract_c_function_name(declarator: Node) -> Option<Node> {
    let mut current = declarator;

    loop {
        // C function declarators can be nested (function pointers, etc.)
        // Look for the identifier
        if current.kind() == "identifier" {
            return Some(current);
        }

        // For function_declarator, check the declarator field
        if current.kind() == "function_declarator" {
            if let Some(inner) = current.child_by_field_name("declarator") {
                current = inner;
                continue;
            }
        }

        // For pointer_declarator, check the declarator field
        if current.kind() == "pointer_declarator" {
            if let Some(inner) = current.child_by_field_name("declarator") {
                current = inner;
                continue;
            }
        }

        // Check children
        let mut found = None;
        let mut cursor = current.walk();
        for child in current.children(&mut cursor) {
            if child.kind() == "identifier" {
                found = Some(child);
                break;
            }
        }

        return found;
    }
}

fn process_symbol(
    node: &Node,
    name: &str,
    kind: &str,
    source: &[u8],
    file_path: &str,
    language: Language,
    sql: &mut String,
    context: &mut ParseContext,
) -> (bool, bool) { // (is_function, is_type)
    // Extract documentation first (applies to all symbol types)
    if let Some((doc_raw, doc_clean, keywords)) = extract_doc_comment(*node, source, language) {
        let summary = extract_summary(&doc_clean);
        let has_examples = doc_raw.contains("```") || doc_raw.contains("Example:");

        sql.push_str(&format!(
            "INSERT OR REPLACE INTO documentation (file, symbol_name, symbol_type, line_number, doc_raw, doc_clean, doc_summary, keywords, doc_length, has_examples) VALUES ('{}', '{}', '{}', {}, '{}', '{}', '{}', {}, {}, {});\n",
            escape_sql(file_path),
            escape_sql(name),
            kind,
            node.start_position().row + 1,
            escape_sql(&doc_raw),
            escape_sql(&doc_clean),
            escape_sql(&summary),
            format_string_array(&keywords),
            doc_clean.len(),
            has_examples
        ));
    }
    
    match kind {
        "function" => {
            // Update context with current function
            context.current_function = Some(name.to_string());
            
            extract_function_facts(node, source, file_path, name, sql, language);
            
            // Add to code_search table
            let signature = node.utf8_text(source)
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("");
            
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO code_search (path, name, signature) VALUES ('{}', '{}', '{}');\n",
                escape_sql(file_path),
                escape_sql(name),
                escape_sql(signature)
            ));
            (true, false)
        }
        "struct" | "class" | "enum" | "interface" | "type_alias" | "const" => {
            extract_type_definition(node, source, file_path, name, kind, sql, language);
            (false, true)
        }
        "import" => {
            // This shouldn't be reached anymore as imports are handled specially
            // in extract_symbols_from_tree, but keep for safety
            extract_import_fact(*node, source, file_path, sql, language);
            (false, false)
        }
        "impl" | "module" => {
            // These don't get stored, just used for context/recursion
            (false, false)
        }
        _ => {
            (false, false)
        }
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
    let takes_mut_params = params.iter().any(|p| p.contains("&mut ") && !p.contains("self"));

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
        (spec.extract_generics)(node, source)
            .unwrap_or_else(|| String::new())
    } else {
        String::new()
    };
    
    let params_str = params.join(", ");
    let generic_count = if generics.is_empty() { 0 } else { generics.matches(',').count() + 1 };
    let return_type_str = return_type.unwrap_or_else(|| String::new());

    sql.push_str(&format!(
        "INSERT OR REPLACE INTO function_facts (file, name, takes_mut_self, takes_mut_params, returns_result, returns_option, is_async, is_unsafe, is_public, parameter_count, generic_count, parameters, return_type) VALUES ('{}', '{}', {}, {}, {}, {}, {}, {}, {}, {}, {}, '{}', '{}');\n",
        escape_sql(file_path),
        escape_sql(name),
        takes_mut_self,
        takes_mut_params,
        returns_result,
        returns_option,
        is_async,
        is_unsafe,
        is_public,
        param_count,
        generic_count,
        escape_sql(&params_str),
        escape_sql(&return_type_str)
    ));
}

fn extract_type_definition(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: &str,
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

    sql.push_str(&format!(
        "INSERT OR REPLACE INTO type_vocabulary (file, name, definition, kind, visibility) VALUES ('{}', '{}', '{}', '{}', '{}');\n",
        escape_sql(file_path),
        escape_sql(name),
        escape_sql(&definition),
        kind,
        visibility
    ));
}

fn extract_import_fact(
    node: Node,
    source: &[u8],
    file_path: &str,
    sql: &mut String,
    language: Language,
) {
    if let Some(spec) = get_language_spec(language) {
        let (imported_item, imported_from, is_external) =
            (spec.extract_import_details)(&node, source);

        sql.push_str(&format!(
            "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, 'use');\n",
            escape_sql(file_path),
            escape_sql(&imported_item),
            escape_sql(&imported_from),
            is_external
        ));
    }
}

fn extract_call_expressions(
    node: Node,
    source: &[u8],
    language: Language,
    context: &mut ParseContext,
) {
    let line_number = (node.start_position().row + 1) as i32;
    
    match (language, node.kind()) {
        // Rust call expressions
        (Language::Rust, "call_expression") => {
            if let Some(func_node) = node.child_by_field_name("function") {
                if let Ok(callee) = func_node.utf8_text(source) {
                    context.add_call(callee.to_string(), "direct".to_string(), line_number);
                }
            }
        }
        (Language::Rust, "method_call_expression") => {
            if let Some(method_node) = node.child_by_field_name("name") {
                if let Ok(callee) = method_node.utf8_text(source) {
                    context.add_call(callee.to_string(), "method".to_string(), line_number);
                }
            }
        }
        
        // Go call expressions
        (Language::Go, "call_expression") => {
            if let Some(func_node) = node.child_by_field_name("function") {
                if let Ok(callee) = func_node.utf8_text(source) {
                    let call_type = if callee.contains("go ") {
                        "async"
                    } else {
                        "direct"
                    };
                    context.add_call(
                        callee.replace("go ", ""),
                        call_type.to_string(),
                        line_number,
                    );
                }
            }
        }
        (Language::Go, "selector_expression") => {
            // Go method calls are selector expressions followed by call_expression
            if let Some(parent) = node.parent() {
                if parent.kind() == "call_expression" {
                    if let Some(field_node) = node.child_by_field_name("field") {
                        if let Ok(callee) = field_node.utf8_text(source) {
                            context.add_call(callee.to_string(), "method".to_string(), line_number);
                        }
                    }
                }
            }
        }
        
        // Python call expressions
        (Language::Python, "call") => {
            if let Some(func_node) = node.child_by_field_name("function") {
                if let Ok(callee) = func_node.utf8_text(source) {
                    let call_type = if callee.starts_with("await ") {
                        "async"
                    } else {
                        "direct"
                    };
                    context.add_call(
                        callee.replace("await ", ""),
                        call_type.to_string(),
                        line_number,
                    );
                }
            }
        }
        
        // JavaScript/TypeScript call expressions
        (Language::JavaScript | Language::JavaScriptJSX | Language::TypeScript | Language::TypeScriptTSX, "call_expression") => {
            if let Some(func_node) = node.child_by_field_name("function") {
                if let Ok(callee) = func_node.utf8_text(source) {
                    let call_type = if callee.starts_with("await ") {
                        "async"
                    } else {
                        "direct"
                    };
                    context.add_call(
                        callee.replace("await ", ""),
                        call_type.to_string(),
                        line_number,
                    );
                }
            }
        }
        
        // C/C++ call expressions
        (Language::C | Language::Cpp, "call_expression") => {
            if let Some(func_node) = node.child_by_field_name("function") {
                if let Ok(callee) = func_node.utf8_text(source) {
                    context.add_call(callee.to_string(), "direct".to_string(), line_number);
                }
            }
        }
        
        // Solidity call expressions
        (Language::Solidity, "call_expression") => {
            if let Some(func_node) = node.child_by_field_name("function") {
                if let Ok(callee) = func_node.utf8_text(source) {
                    context.add_call(callee.to_string(), "direct".to_string(), line_number);
                }
            }
        }
        (Language::Solidity, "member_expression") => {
            // Handle contract.method() calls
            if let Some(parent) = node.parent() {
                if parent.kind() == "call_expression" {
                    if let Some(property) = node.child_by_field_name("property") {
                        if let Ok(callee) = property.utf8_text(source) {
                            context.add_call(callee.to_string(), "method".to_string(), line_number);
                        }
                    }
                }
            }
        }
        (Language::Solidity, "new_expression") => {
            // Handle "new Type[]" array constructors
            if let Ok(text) = node.utf8_text(source) {
                context.add_call(text.to_string(), "constructor".to_string(), line_number);
            }
        }
        
        _ => {}
    }
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

fn escape_sql(s: &str) -> String {
    s.replace('\'', "''")
}

fn format_string_array(items: &[String]) -> String {
    if items.is_empty() {
        "ARRAY[]::VARCHAR[]".to_string()
    } else {
        format!(
            "ARRAY[{}]",
            items
                .iter()
                .map(|s| format!("'{}'", escape_sql(s)))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}