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

use crate::commands::incremental;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{ScrapeConfig, ScrapeStats};

// ============================================================================
// LANGUAGE REGISTRY - Centralized language configuration
// ============================================================================
// All language-specific logic consolidated in ONE place.
// To add a new language:
// 1. Create a LanguageSpec constant
// 2. Add it to LANGUAGE_REGISTRY
// That's it! No scattered match statements to update.

use std::sync::LazyLock;
use tree_sitter::Node;

// Import Language enum from the languages module at the end of this file
use self::languages::Language;

/// Specification for how to parse and extract information from a language
struct LanguageSpec {
    /// File extensions for this language
    extensions: &'static [&'static str],

    /// AST node types that represent functions
    function_nodes: &'static [&'static str],

    /// AST node types that represent structs/classes
    struct_nodes: &'static [&'static str],

    /// AST node types that represent traits/interfaces
    trait_nodes: &'static [&'static str],

    /// AST node types that represent imports
    import_nodes: &'static [&'static str],

    /// Check if a comment is a documentation comment
    is_doc_comment: fn(&str) -> bool,

    /// Parse visibility from node and name
    parse_visibility: fn(&Node, &str, &[u8]) -> bool,

    /// Check if function is async
    has_async: fn(&Node, &[u8]) -> bool,

    /// Check if function is unsafe
    has_unsafe: fn(&Node, &[u8]) -> bool,

    /// Extract function parameters
    extract_params: fn(&Node, &[u8]) -> Vec<String>,

    /// Extract return type
    extract_return_type: fn(&Node, &[u8]) -> Option<String>,

    /// Extract generic parameters
    extract_generics: fn(&Node, &[u8]) -> Option<String>,

    /// Map node kind to symbol kind (simple mapping)
    get_symbol_kind: fn(&str) -> &'static str,

    /// Map node to symbol kind (complex cases that need node inspection)
    get_symbol_kind_complex: fn(&Node, &[u8]) -> Option<&'static str>,

    /// Extract call target from call expression
    extract_call_target: fn(&Node, &[u8]) -> Option<String>,
}

// ============================================================================
// LANGUAGE SPECIFICATIONS
// ============================================================================

/// Rust language specification
static RUST_SPEC: LanguageSpec = LanguageSpec {
    extensions: &["rs"],

    function_nodes: &["function_item", "impl_item"],
    struct_nodes: &["struct_item"],
    trait_nodes: &["trait_item"],
    import_nodes: &["use_declaration"],

    is_doc_comment: |text| text.starts_with("///") || text.starts_with("//!"),

    parse_visibility: |node, _name, _source| {
        // Check for pub keyword via visibility_modifier node
        node.children(&mut node.walk())
            .any(|child| child.kind() == "visibility_modifier")
    },

    has_async: |node, _source| {
        node.children(&mut node.walk())
            .any(|child| child.kind() == "async")
    },

    has_unsafe: |node, _source| {
        node.children(&mut node.walk())
            .any(|child| child.kind() == "unsafe")
    },

    extract_params: |node, source| {
        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut params = Vec::new();
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "parameter" || child.kind() == "self_parameter" {
                    if let Ok(param_text) = child.utf8_text(source) {
                        params.push(param_text.to_string());
                    }
                }
            }
            params
        } else {
            Vec::new()
        }
    },

    extract_return_type: |node, source| {
        node.child_by_field_name("return_type")
            .and_then(|rt| rt.utf8_text(source).ok())
            .map(|s| s.trim_start_matches("->").trim().to_string())
    },

    extract_generics: |node, source| {
        node.child_by_field_name("type_parameters")
            .and_then(|tp| tp.utf8_text(source).ok())
            .map(String::from)
    },

    get_symbol_kind: |node_kind| match node_kind {
        "function_item" => "function",
        "struct_item" => "struct",
        "trait_item" => "trait",
        "impl_item" => "impl",
        "type_alias" => "type_alias",
        "const_item" => "const",
        "use_declaration" => "import",
        _ => "unknown",
    },

    get_symbol_kind_complex: |_node, _source| {
        // Rust doesn't need complex symbol kind detection
        None
    },

    extract_call_target: |node, source| match node.kind() {
        "call_expression" => node
            .child_by_field_name("function")
            .and_then(|f| f.utf8_text(source).ok())
            .map(String::from),
        "method_call_expression" => node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok())
            .map(String::from),
        _ => None,
    },
};

/// Go language specification
static GO_SPEC: LanguageSpec = LanguageSpec {
    extensions: &["go"],

    function_nodes: &["function_declaration", "method_declaration"],
    struct_nodes: &["type_spec"],
    trait_nodes: &[], // Go has interfaces but handled via type_spec
    import_nodes: &["import_declaration"],

    is_doc_comment: |text| {
        // Go uses // for doc comments (before declarations)
        text.starts_with("//")
    },

    parse_visibility: |_node, name, _source| {
        // In Go, uppercase first letter = public
        name.chars().next().is_some_and(|c| c.is_uppercase())
    },

    has_async: |_node, _source| {
        // Go doesn't have async keyword, uses goroutines
        false
    },

    has_unsafe: |_node, _source| {
        // Go doesn't have unsafe keyword in function declarations
        false
    },

    extract_params: |node, source| {
        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut params = Vec::new();
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "parameter_declaration" {
                    if let Ok(param_text) = child.utf8_text(source) {
                        params.push(param_text.to_string());
                    }
                }
            }
            params
        } else {
            Vec::new()
        }
    },

    extract_return_type: |node, source| {
        node.child_by_field_name("result")
            .and_then(|r| r.utf8_text(source).ok())
            .map(String::from)
    },

    extract_generics: |node, source| {
        // Go uses type parameters (generics added in Go 1.18)
        node.child_by_field_name("type_parameters")
            .and_then(|tp| tp.utf8_text(source).ok())
            .map(String::from)
    },

    get_symbol_kind: |node_kind| match node_kind {
        "function_declaration" => "function",
        "method_declaration" => "function",
        "const_declaration" => "const",
        "import_declaration" => "import",
        _ => "unknown",
    },

    get_symbol_kind_complex: |node, _source| {
        // Special handling for type_spec
        if node.kind() == "type_spec" {
            if node
                .child_by_field_name("type")
                .is_some_and(|n| n.kind() == "struct_type")
            {
                Some("struct")
            } else if node
                .child_by_field_name("type")
                .is_some_and(|n| n.kind() == "interface_type")
            {
                Some("trait")
            } else {
                Some("type_alias")
            }
        } else {
            None
        }
    },

    extract_call_target: |node, source| {
        match node.kind() {
            "call_expression" => node
                .child_by_field_name("function")
                .and_then(|f| f.utf8_text(source).ok())
                .map(String::from),
            "selector_expression" => {
                // For method calls like obj.Method()
                node.child_by_field_name("field")
                    .and_then(|f| f.utf8_text(source).ok())
                    .map(String::from)
            }
            _ => None,
        }
    },
};

/// Python language specification
static PYTHON_SPEC: LanguageSpec = LanguageSpec {
    extensions: &["py"],

    function_nodes: &["function_definition", "async_function_definition"],
    struct_nodes: &["class_definition"],
    trait_nodes: &[], // Python doesn't have traits
    import_nodes: &["import_statement", "import_from_statement"],

    is_doc_comment: |text| {
        // Python uses docstrings (triple quotes)
        text.starts_with("\"\"\"") || text.starts_with("'''")
    },

    parse_visibility: |_node, name, _source| {
        // Python convention: _ prefix = private
        !name.starts_with('_')
    },

    has_async: |node, source| {
        // Python uses async def
        node.kind() == "async_function_definition"
            || node.utf8_text(source).unwrap_or("").starts_with("async ")
    },

    has_unsafe: |_node, _source| {
        // Python doesn't have unsafe
        false
    },

    extract_params: |node, source| {
        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut params = Vec::new();
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                // Skip punctuation
                if matches!(child.kind(), "," | "(" | ")") {
                    continue;
                }
                if let Ok(param_text) = child.utf8_text(source) {
                    if !param_text.trim().is_empty() {
                        params.push(param_text.to_string());
                    }
                }
            }
            params
        } else {
            Vec::new()
        }
    },

    extract_return_type: |node, source| {
        // Python has optional type hints -> ReturnType
        node.child_by_field_name("return_type")
            .and_then(|rt| rt.utf8_text(source).ok())
            .map(|s| s.trim_start_matches("->").trim().to_string())
    },

    extract_generics: |_node, _source| {
        // Python doesn't have explicit generics in function definitions
        None
    },

    get_symbol_kind: |node_kind| match node_kind {
        "function_definition" | "async_function_definition" => "function",
        "class_definition" => "struct",
        "import_statement" | "import_from_statement" => "import",
        _ => "unknown",
    },

    get_symbol_kind_complex: |node, _source| {
        // Special handling for decorated_definition
        if node.kind() == "decorated_definition" {
            if node.child_by_field_name("definition").is_some_and(|n| {
                n.kind() == "function_definition" || n.kind() == "async_function_definition"
            }) {
                Some("function")
            } else if node
                .child_by_field_name("definition")
                .is_some_and(|n| n.kind() == "class_definition")
            {
                Some("struct")
            } else {
                None
            }
        } else {
            None
        }
    },

    extract_call_target: |node, source| {
        match node.kind() {
            "call" => node
                .child_by_field_name("function")
                .and_then(|f| f.utf8_text(source).ok())
                .map(String::from),
            "attribute" => {
                // For method calls like obj.method()
                node.child_by_field_name("attribute")
                    .and_then(|a| a.utf8_text(source).ok())
                    .map(String::from)
            }
            _ => None,
        }
    },
};

/// JavaScript language specification (shared base for JS/JSX)
static JS_SPEC: LanguageSpec = LanguageSpec {
    extensions: &["js", "mjs"],

    function_nodes: &[
        "function_declaration",
        "arrow_function",
        "function_expression",
    ],
    struct_nodes: &["class_declaration"],
    trait_nodes: &[], // JS doesn't have traits
    import_nodes: &["import_statement"],

    is_doc_comment: |text| {
        // JSDoc comments
        text.starts_with("/**") || text.starts_with("///")
    },

    parse_visibility: |_node, _name, _source| {
        // JavaScript doesn't have explicit visibility modifiers
        // Everything is public unless using closures/modules
        true
    },

    has_async: |node, source| {
        // Check for async keyword
        node.utf8_text(source).unwrap_or("").contains("async")
    },

    has_unsafe: |_node, _source| {
        // JavaScript doesn't have unsafe
        false
    },

    extract_params: |node, source| {
        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut params = Vec::new();
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if matches!(
                    child.kind(),
                    "identifier" | "rest_pattern" | "object_pattern" | "array_pattern"
                ) {
                    if let Ok(param_text) = child.utf8_text(source) {
                        params.push(param_text.to_string());
                    }
                }
            }
            params
        } else {
            Vec::new()
        }
    },

    extract_return_type: |_node, _source| {
        // JavaScript doesn't have return type annotations
        None
    },

    extract_generics: |_node, _source| {
        // JavaScript doesn't have generics
        None
    },

    get_symbol_kind: |node_kind| match node_kind {
        "function_declaration" | "arrow_function" | "function_expression" => "function",
        "class_declaration" => "struct",
        "import_statement" => "import",
        "const_declaration" | "let_declaration" => "const",
        _ => "unknown",
    },

    get_symbol_kind_complex: |node, _source| {
        // Special handling for variable_declarator
        if node.kind() == "variable_declarator" {
            if node
                .child_by_field_name("value")
                .is_some_and(|n| n.kind() == "arrow_function" || n.kind() == "function_expression")
            {
                Some("function")
            } else if node
                .child_by_field_name("value")
                .is_some_and(|n| n.kind() == "class_expression")
            {
                Some("struct")
            } else {
                None
            }
        } else {
            None
        }
    },

    extract_call_target: |node, source| {
        match node.kind() {
            "call_expression" => node
                .child_by_field_name("function")
                .and_then(|f| f.utf8_text(source).ok())
                .map(String::from),
            "member_expression" => {
                // For method calls like obj.method()
                node.child_by_field_name("property")
                    .and_then(|p| p.utf8_text(source).ok())
                    .map(String::from)
            }
            _ => None,
        }
    },
};

/// TypeScript language specification
static TS_SPEC: LanguageSpec = LanguageSpec {
    extensions: &["ts"],

    function_nodes: &[
        "function_declaration",
        "arrow_function",
        "function_expression",
        "method_definition",
    ],
    struct_nodes: &[
        "class_declaration",
        "interface_declaration",
        "type_alias_declaration",
    ],
    trait_nodes: &["interface_declaration"],
    import_nodes: &["import_statement"],

    is_doc_comment: |text| {
        // TSDoc/JSDoc comments
        text.starts_with("/**") || text.starts_with("///")
    },

    parse_visibility: |node, _name, source| {
        // TypeScript has explicit visibility modifiers
        let text = node.utf8_text(source).unwrap_or("");
        !text.contains("private") && !text.contains("protected")
    },

    has_async: |node, source| {
        // Check for async keyword
        node.utf8_text(source).unwrap_or("").contains("async")
    },

    has_unsafe: |_node, _source| {
        // TypeScript doesn't have unsafe
        false
    },

    extract_params: |node, source| {
        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut params = Vec::new();
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if matches!(
                    child.kind(),
                    "required_parameter" | "optional_parameter" | "rest_parameter"
                ) {
                    if let Ok(param_text) = child.utf8_text(source) {
                        params.push(param_text.to_string());
                    }
                }
            }
            params
        } else {
            Vec::new()
        }
    },

    extract_return_type: |node, source| {
        // TypeScript has return type annotations
        node.child_by_field_name("return_type")
            .and_then(|rt| rt.utf8_text(source).ok())
            .map(|s| s.trim_start_matches(":").trim().to_string())
    },

    extract_generics: |node, source| {
        // TypeScript has generics
        node.child_by_field_name("type_parameters")
            .and_then(|tp| tp.utf8_text(source).ok())
            .map(String::from)
    },

    get_symbol_kind: |node_kind| match node_kind {
        "function_declaration" | "arrow_function" | "function_expression" | "method_definition" => {
            "function"
        }
        "class_declaration" => "struct",
        "interface_declaration" => "trait",
        "type_alias_declaration" => "type_alias",
        "import_statement" => "import",
        "const_statement" | "let_statement" => "const",
        "enum_declaration" => "struct",
        _ => "unknown",
    },

    get_symbol_kind_complex: |node, _source| {
        // Special handling for variable_declarator (same as JS)
        if node.kind() == "variable_declarator" {
            if node
                .child_by_field_name("value")
                .is_some_and(|n| n.kind() == "arrow_function" || n.kind() == "function_expression")
            {
                Some("function")
            } else if node
                .child_by_field_name("value")
                .is_some_and(|n| n.kind() == "class_expression")
            {
                Some("struct")
            } else {
                None
            }
        } else {
            None
        }
    },

    extract_call_target: |node, source| {
        match node.kind() {
            "call_expression" => node
                .child_by_field_name("function")
                .and_then(|f| f.utf8_text(source).ok())
                .map(String::from),
            "member_expression" => {
                // For method calls like obj.method()
                node.child_by_field_name("property")
                    .and_then(|p| p.utf8_text(source).ok())
                    .map(String::from)
            }
            _ => None,
        }
    },
};

/// Solidity language specification
static SOLIDITY_SPEC: LanguageSpec = LanguageSpec {
    extensions: &["sol"],

    function_nodes: &["function_definition", "modifier_definition"],
    struct_nodes: &["contract_declaration", "struct_declaration"],
    trait_nodes: &["interface_declaration"],
    import_nodes: &["import_directive"],

    is_doc_comment: |text| {
        // Solidity uses NatSpec comments
        text.starts_with("///") || text.starts_with("/**")
    },

    parse_visibility: |node, _name, source| {
        // Solidity defaults to public, check for private/internal
        let text = node.utf8_text(source).unwrap_or("");
        !text.contains("private") && !text.contains("internal")
    },

    has_async: |_node, _source| {
        // Solidity doesn't have async
        false
    },

    has_unsafe: |node, source| {
        // In Solidity, unchecked blocks are similar to unsafe
        node.utf8_text(source).unwrap_or("").contains("unchecked")
    },

    extract_params: |node, source| {
        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut params = Vec::new();
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "parameter" {
                    if let Ok(param_text) = child.utf8_text(source) {
                        params.push(param_text.to_string());
                    }
                }
            }
            params
        } else {
            Vec::new()
        }
    },

    extract_return_type: |node, source| {
        // Solidity has return parameters
        node.child_by_field_name("return_parameters")
            .and_then(|rp| rp.utf8_text(source).ok())
            .map(String::from)
    },

    extract_generics: |_node, _source| {
        // Solidity doesn't have generics
        None
    },

    get_symbol_kind: |node_kind| match node_kind {
        "function_definition" => "function",
        "modifier_definition" => "function",
        "event_definition" => "function",
        "contract_declaration" => "struct",
        "struct_declaration" => "struct",
        "interface_declaration" => "trait",
        "library_declaration" => "impl",
        "import_directive" => "import",
        "state_variable_declaration" => "const",
        _ => "unknown",
    },

    get_symbol_kind_complex: |_node, _source| {
        // Solidity doesn't need complex symbol kind detection
        None
    },

    extract_call_target: |node, source| {
        if node.kind() == "call_expression" {
            node.child_by_field_name("function")
                .and_then(|f| f.utf8_text(source).ok())
                .map(String::from)
        } else {
            None
        }
    },
};

/// Central registry of all language specifications
static LANGUAGE_REGISTRY: LazyLock<HashMap<Language, &'static LanguageSpec>> =
    LazyLock::new(|| {
        let mut registry = HashMap::new();

        // Register all language specifications
        registry.insert(Language::Rust, &RUST_SPEC);
        registry.insert(Language::Go, &GO_SPEC);
        registry.insert(Language::Python, &PYTHON_SPEC);
        registry.insert(Language::JavaScript, &JS_SPEC);
        registry.insert(Language::JavaScriptJSX, &JS_SPEC); // JSX uses same spec as JS
        registry.insert(Language::TypeScript, &TS_SPEC);
        registry.insert(Language::TypeScriptTSX, &TS_SPEC); // TSX uses same spec as TS
        registry.insert(Language::Solidity, &SOLIDITY_SPEC);

        registry
    });

/// Get language specification from registry
fn get_language_spec(language: Language) -> Option<&'static LanguageSpec> {
    LANGUAGE_REGISTRY.get(&language).copied()
}

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
    initialize_database(&config.db_path)?;

    println!(
        "✅ Database initialized with 16KB blocks at {}",
        config.db_path
    );
    println!("\nNext steps:");
    println!("  1. Run 'patina scrape code' to index your codebase");
    println!("  2. Run 'patina scrape code --query \"SELECT ...\"' to explore");

    Ok(())
}

/// Extract semantic information from codebase
pub fn extract(config: &ScrapeConfig) -> Result<ScrapeStats> {
    println!("🔍 Starting semantic extraction...\n");

    let start = std::time::Instant::now();

    let work_dir = determine_work_directory(config)?;

    // Print repo info if scraping a repository
    if config.db_path.contains("layer/dust/repos/") {
        if let Some(repo_name) = config
            .db_path
            .strip_prefix("layer/dust/repos/")
            .and_then(|s| s.strip_suffix(".db"))
        {
            println!("📦 Scraping repository: {}", repo_name);
            println!("📁 Source: {}", work_dir.display());
            println!("💾 Database: {}", config.db_path);
        }
    }

    // Run the ETL pipeline (it handles initialization if force=true)
    let items_processed = extract_and_index(&config.db_path, &work_dir, config.force)?;

    // Get database size
    let db_size_kb = std::fs::metadata(&config.db_path)
        .map(|m| m.len() / 1024)
        .unwrap_or(0);

    Ok(ScrapeStats {
        items_processed,
        time_elapsed: start.elapsed(),
        database_size_kb: db_size_kb,
    })
}

// ============================================================================
// CHAPTER 2: ETL PIPELINE ORCHESTRATION
// ============================================================================

fn determine_work_directory(config: &ScrapeConfig) -> Result<PathBuf> {
    // Extract repo name from db_path if it's in layer/dust/repos/
    if config.db_path.contains("layer/dust/repos/") {
        let repo_name = config
            .db_path
            .strip_prefix("layer/dust/repos/")
            .and_then(|s| s.strip_suffix(".db"))
            .context("Invalid repo database path")?;

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
    println!("🗄️  Initializing optimized knowledge database...");

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
        fingerprint::generate_schema(),
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

    println!("✅ Database initialized with 16KB blocks at {}", db_path);
    println!("\nNext steps:");
    println!("  1. Run 'patina scrape' to index your codebase");
    println!("  2. Run 'patina scrape --query \"SELECT ...\"' to explore");

    Ok(())
}

fn extract_and_index(db_path: &str, work_dir: &Path, force: bool) -> Result<usize> {
    println!("🔍 Indexing codebase...\n");

    // If force flag is set, reinitialize database to ensure clean state
    if force {
        initialize_database(db_path)?;
    }

    // Step 1: Git metrics for quality signals
    extract_git_metrics(db_path, work_dir)?;

    // Step 2: Pattern references from docs (only for main repo)
    if db_path.contains(".patina/") {
        extract_pattern_references(db_path, work_dir)?;
    }

    // Step 3: Semantic fingerprints with tree-sitter
    let symbol_count = extract_fingerprints(db_path, work_dir, force)?;

    // Step 4: Show summary
    show_summary(db_path)?;

    Ok(symbol_count)
}

// ============================================================================
// CHAPTER 3: EXTRACTION - Git Metrics
// ============================================================================

/// Check if the git repository has been updated recently
fn check_git_freshness(work_dir: &Path) -> Result<()> {
    // Get the last commit date
    let last_commit = Command::new("git")
        .current_dir(work_dir)
        .args(["log", "-1", "--format=%at"])
        .output()
        .context("Failed to get last commit date")?;

    if !last_commit.status.success() {
        // Not a git repo or no commits
        return Ok(());
    }

    let timestamp_str = String::from_utf8_lossy(&last_commit.stdout)
        .trim()
        .to_string();
    if let Ok(last_commit_timestamp) = timestamp_str.parse::<i64>() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let days_old = (now - last_commit_timestamp) / 86400;

        // Alert thresholds
        if days_old > 30 {
            println!(
                "  ⚠️  WARNING: Repository hasn't been updated in {} days!",
                days_old
            );
            println!("     Consider pulling latest changes before scraping.");
        } else if days_old > 7 {
            println!("  ℹ️  Note: Repository last updated {} days ago", days_old);
        }

        // Also show the last commit info
        let last_commit_info = Command::new("git")
            .current_dir(work_dir)
            .args(["log", "-1", "--format=%h %s (%ar)"])
            .output()?;

        if last_commit_info.status.success() {
            let info = String::from_utf8_lossy(&last_commit_info.stdout)
                .trim()
                .to_string();
            println!("  📝 Last commit: {}", info);
        }
    }

    Ok(())
}

fn extract_git_metrics(db_path: &str, work_dir: &Path) -> Result<()> {
    println!("📊 Analyzing Git history...");

    // Check repository freshness
    check_git_freshness(work_dir)?;

    let rust_files = Command::new("git")
        .current_dir(work_dir)
        .args(["ls-files", "*.rs", "src/**/*.rs"])
        .output()
        .context("Failed to list Git files")?;

    if !rust_files.status.success() {
        anyhow::bail!("Failed to get file list from Git");
    }

    let files = String::from_utf8_lossy(&rust_files.stdout);
    let file_count = files.lines().count();

    let mut metrics_sql = String::from("BEGIN TRANSACTION;\n");
    metrics_sql.push_str("DELETE FROM git_metrics;\n");

    for file in files.lines() {
        if file.is_empty() {
            continue;
        }

        // Get commit history for this file
        let log_output = Command::new("git")
            .current_dir(work_dir)
            .args(["log", "--format=%H %ai", "--follow", "--", file])
            .output()?;

        if log_output.status.success() {
            let log = String::from_utf8_lossy(&log_output.stdout);
            let commits: Vec<&str> = log.lines().collect();

            if !commits.is_empty() {
                let first = commits
                    .last()
                    .unwrap_or(&"")
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                let last = commits
                    .first()
                    .unwrap_or(&"")
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                let count = commits.len();

                // Calculate survival days
                let first_date = Command::new("git")
                    .current_dir(work_dir)
                    .args(["show", "-s", "--format=%at", first])
                    .output()?;

                if first_date.status.success() {
                    let timestamp = String::from_utf8_lossy(&first_date.stdout)
                        .trim()
                        .parse::<i64>()
                        .unwrap_or(0);

                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs() as i64;

                    let survival_days = (now - timestamp) / 86400;

                    metrics_sql.push_str(&format!(
                        "INSERT INTO git_metrics (file, first_commit, last_commit, commit_count, survival_days) VALUES ('{}', '{}', '{}', {}, {});\n",
                        file, first, last, count, survival_days
                    ));
                }
            }
        }
    }

    metrics_sql.push_str("COMMIT;\n");

    Command::new("duckdb")
        .arg(db_path)
        .arg("-c")
        .arg(&metrics_sql)
        .output()
        .context("Failed to insert Git metrics")?;

    println!("  ✓ Analyzed {} files", file_count);
    Ok(())
}

// ============================================================================
// CHAPTER 4: EXTRACTION - Pattern References
// ============================================================================

fn extract_pattern_references(db_path: &str, work_dir: &Path) -> Result<()> {
    println!("🔗 Extracting pattern references...");

    let pattern_files = Command::new("find")
        .current_dir(work_dir)
        .args(["layer", "-name", "*.md", "-type", "f"])
        .output()
        .context("Failed to find pattern files")?;

    if !pattern_files.status.success() {
        anyhow::bail!("Failed to list pattern files");
    }

    let files = String::from_utf8_lossy(&pattern_files.stdout);
    let mut references_sql = String::from("BEGIN TRANSACTION;\n");
    references_sql.push_str("DELETE FROM pattern_references;\n");

    for file in files.lines() {
        if file.is_empty() {
            continue;
        }

        let pattern_id = Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let file_path = work_dir.join(file);
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            // Look for references in YAML frontmatter
            if let Some(refs_line) = content.lines().find(|l| l.starts_with("references:")) {
                if let Some(refs) = refs_line.strip_prefix("references:") {
                    let refs = refs.trim().trim_start_matches('[').trim_end_matches(']');
                    for reference in refs.split(',') {
                        let reference = reference.trim().trim_matches('"').trim_matches('\'');
                        if !reference.is_empty() {
                            references_sql.push_str(&format!(
                                "INSERT INTO pattern_references (from_pattern, to_pattern, reference_type, context) VALUES ('{}', '{}', 'references', 'frontmatter');\n",
                                pattern_id, reference
                            ));
                        }
                    }
                }
            }
        }
    }

    references_sql.push_str("COMMIT;\n");

    Command::new("duckdb")
        .arg(db_path)
        .arg("-c")
        .arg(&references_sql)
        .output()
        .context("Failed to insert pattern references")?;

    println!(
        "  ✓ Extracted references from {} patterns",
        files.lines().count()
    );
    Ok(())
}

// ============================================================================
// CHAPTER 5: EXTRACTION - Semantic Data
// ============================================================================

fn extract_fingerprints(db_path: &str, work_dir: &Path, force: bool) -> Result<usize> {
    println!("🧠 Generating semantic fingerprints and extracting truth data...");

    use ignore::WalkBuilder;
    use languages::{create_parser_for_path, Language};
    use std::collections::HashMap;
    use std::time::SystemTime;

    // Find all supported language files
    let mut all_files = Vec::new();

    // Track skipped files by extension
    let mut skipped_files: HashMap<String, (usize, usize, String)> = HashMap::new(); // ext -> (count, bytes, example_path)

    // Use ignore crate to walk files, respecting .gitignore
    let walker = WalkBuilder::new(work_dir)
        .hidden(false) // Don't process hidden files
        .git_ignore(true) // Respect .gitignore
        .git_global(true) // Respect global gitignore
        .git_exclude(true) // Respect .git/info/exclude
        .ignore(true) // Respect .ignore files
        .build();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        // Skip directories
        if entry.file_type().is_some_and(|ft| ft.is_dir()) {
            continue;
        }

        // Get relative path for storage
        let relative_path = path.strip_prefix(work_dir).unwrap_or(path);
        let relative_path_str = relative_path.to_string_lossy();

        // Skip if path starts with dot (hidden)
        if relative_path_str.starts_with('.') {
            continue;
        }

        // Determine language from extension
        let language = Language::from_path(path);

        match language {
            Language::Rust
            | Language::Go
            | Language::Solidity
            | Language::Python
            | Language::JavaScript
            | Language::JavaScriptJSX
            | Language::TypeScript
            | Language::TypeScriptTSX
            | Language::Cairo => {
                // Supported language - add to processing list with relative path
                all_files.push((format!("./{}", relative_path_str), language));
            }
            Language::Unknown => {
                // Track skipped file
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    // Get file size
                    let file_size = entry.metadata().ok().map(|m| m.len() as usize).unwrap_or(0);

                    let entry = skipped_files.entry(ext.to_string()).or_insert((
                        0,
                        0,
                        relative_path_str.to_string(),
                    ));
                    entry.0 += 1; // count
                    entry.1 += file_size; // bytes
                                          // Keep first example path
                }
            }
        }
    }

    if all_files.is_empty() {
        println!("  ⚠️  No supported language files found");
        return Ok(0);
    }

    println!(
        "  📂 Found {} files ({} Rust, {} Go, {} Solidity, {} Python, {} JS, {} JSX, {} TS, {} TSX, {} Cairo)",
        all_files.len(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::Rust)
            .count(),
        all_files.iter().filter(|(_, l)| *l == Language::Go).count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::Solidity)
            .count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::Python)
            .count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::JavaScript)
            .count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::JavaScriptJSX)
            .count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::TypeScript)
            .count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::TypeScriptTSX)
            .count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::Cairo)
            .count()
    );

    // Build map of current files with mtimes
    let mut current_files = HashMap::new();
    for (file_str, _) in &all_files {
        let file_path = work_dir.join(file_str);
        if let Ok(metadata) = std::fs::metadata(&file_path) {
            if let Ok(modified) = metadata.modified() {
                let mtime = modified
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                current_files.insert(PathBuf::from(file_str), mtime);
            }
        }
    }

    // Handle incremental vs full index
    let files_to_process = if force {
        println!("  ⚡ Force flag set - performing full re-index");

        // Clear all existing data for full re-index
        Command::new("duckdb")
            .arg(db_path)
            .arg("-c")
            .arg("DELETE FROM code_fingerprints; DELETE FROM code_search; DELETE FROM index_state;")
            .output()?;

        all_files
    } else {
        // Detect changes for incremental update
        let changes = incremental::detect_changes(db_path, &current_files)?;
        incremental::print_change_summary(&changes);

        // If no changes, we're done!
        if changes.is_empty() {
            return Ok(0);
        }

        // Clean up changed files
        incremental::cleanup_changed_files(db_path, &changes)?;

        // Build list of files to process
        let mut files_to_process = Vec::new();
        for path in changes
            .new_files
            .iter()
            .chain(changes.modified_files.iter())
        {
            let path_str = path.to_string_lossy().to_string();
            if let Some((_, lang)) = all_files.iter().find(|(f, _)| f == &path_str) {
                files_to_process.push((path_str, *lang));
            }
        }
        files_to_process
    };

    let mut sql = String::from("BEGIN TRANSACTION;\n");
    let mut symbol_count = 0;
    let mut current_lang = Language::Unknown;
    let mut parser: Option<tree_sitter::Parser> = None;
    let mut batch_count = 0;

    // Process only new and modified files
    for (file, language) in files_to_process {
        // Check if file needs reindexing (mtime-based incremental)
        let file_path = work_dir.join(&file);

        // Cairo needs special handling - use cairo-lang-parser instead of tree-sitter
        if language == Language::Cairo {
            // Parse Cairo file using our custom parser
            let metadata = std::fs::metadata(&file_path)?;
            let mtime = metadata
                .modified()?
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs() as i64;

            let content = std::fs::read_to_string(&file_path)?;

            // Parse Cairo code
            if let Ok(symbols) = patina_metal::cairo::parse_cairo(&content, &file) {
                // Convert Cairo symbols to SQL inserts
                for func in symbols.functions {
                    let fingerprint = fingerprint::Fingerprint {
                        pattern: 0,    // TODO: compute pattern hash
                        imports: 0,    // TODO: compute imports hash
                        complexity: 1, // TODO: compute complexity
                        flags: if func.is_public { 1 } else { 0 },
                    };
                    sql.push_str(&format!(
                        "INSERT OR REPLACE INTO code_fingerprints (path, name, kind, pattern, imports, complexity, flags) VALUES ('{}', '{}', 'function', {}, {}, {}, {});\n",
                        file, func.name,
                        fingerprint.pattern, fingerprint.imports,
                        fingerprint.complexity, fingerprint.flags
                    ));
                    symbol_count += 1;
                }

                for s in symbols.structs {
                    let fingerprint = fingerprint::Fingerprint {
                        pattern: 0,
                        imports: 0,
                        complexity: 1,
                        flags: if s.is_public { 1 } else { 0 },
                    };
                    sql.push_str(&format!(
                        "INSERT OR REPLACE INTO code_fingerprints (path, name, kind, pattern, imports, complexity, flags) VALUES ('{}', '{}', 'struct', {}, {}, {}, {});\n",
                        file, s.name,
                        fingerprint.pattern, fingerprint.imports,
                        fingerprint.complexity, fingerprint.flags
                    ));
                    symbol_count += 1;
                }

                for t in symbols.traits {
                    let fingerprint = fingerprint::Fingerprint {
                        pattern: 0,
                        imports: 0,
                        complexity: 1,
                        flags: if t.is_public { 1 } else { 0 },
                    };
                    sql.push_str(&format!(
                        "INSERT OR REPLACE INTO code_fingerprints (path, name, kind, pattern, imports, complexity, flags) VALUES ('{}', '{}', 'trait', {}, {}, {}, {});\n",
                        file, t.name,
                        fingerprint.pattern, fingerprint.imports,
                        fingerprint.complexity, fingerprint.flags
                    ));
                    symbol_count += 1;
                }

                // Record index state
                sql.push_str(&format!(
                    "INSERT INTO index_state (path, mtime) VALUES ('{}', {});\n",
                    file, mtime
                ));
            }
        } else {
            // Use tree-sitter for other languages
            // Create parser for this specific file path
            // This correctly handles TSX vs TS and JSX vs JS distinctions
            // We need to use create_parser_for_path because create_parser loses the TSX/JSX distinction
            if language != current_lang {
                parser = Some(create_parser_for_path(&file_path)?);
                current_lang = language;
            }
            let metadata = std::fs::metadata(&file_path)?;
            let mtime = metadata
                .modified()?
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs() as i64;

            // TODO: Check index_state to skip unchanged files

            // Parse and fingerprint
            let content = std::fs::read_to_string(&file_path)?;
            if let Some(ref mut p) = parser {
                if let Some(tree) = p.parse(&content, None) {
                    let mut cursor = tree.walk();
                    let mut context = ParseContext::new();
                    symbol_count += process_ast_node(
                        &mut cursor,
                        content.as_bytes(),
                        &file,
                        &mut sql,
                        language,
                        &mut context,
                    );

                    // Flush call graph entries for this file
                    context.flush_to_sql(&file, &mut sql);

                    // Record index state
                    sql.push_str(&format!(
                        "INSERT INTO index_state (path, mtime) VALUES ('{}', {});\n",
                        file, mtime
                    ));
                }
            }
        }

        // Batch execute every 10 files to avoid command line limits
        batch_count += 1;
        if batch_count >= 10 {
            sql.push_str("COMMIT;\n");

            // Use stdin to avoid command line length limits
            let mut child = Command::new("duckdb")
                .arg(db_path)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("Failed to start DuckDB")?;

            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin
                    .write_all(sql.as_bytes())
                    .context("Failed to write SQL")?;
            }

            let output = child
                .wait_with_output()
                .context("Failed to execute batch")?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("DuckDB error: {}", stderr);
            }
            sql = String::from("BEGIN TRANSACTION;\n");
            batch_count = 0;
        }
    }

    // Execute final batch
    if batch_count > 0 {
        sql.push_str("COMMIT;\n");

        // Use stdin to avoid command line length limits
        let mut child = Command::new("duckdb")
            .arg(db_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to start DuckDB")?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin
                .write_all(sql.as_bytes())
                .context("Failed to write SQL")?;
        }

        let output = child
            .wait_with_output()
            .context("Failed to insert final batch")?;

        if !output.status.success() {
            eprintln!("DuckDB error: {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    println!("  ✓ Fingerprinted {} symbols", symbol_count);

    // Save and report skipped files
    if !skipped_files.is_empty() {
        save_skipped_files(db_path, &skipped_files)?;
        report_skipped_files(&skipped_files);
    }

    Ok(symbol_count)
}

// ============================================================================
// CHAPTER 6: DATABASE OPERATIONS
// ============================================================================

/// Save skipped files to database
fn save_skipped_files(
    db_path: &str,
    skipped: &HashMap<String, (usize, usize, String)>,
) -> Result<()> {
    use std::process::Command;

    let mut sql = String::from("BEGIN TRANSACTION;\n");
    sql.push_str("DELETE FROM skipped_files;\n");

    for (ext, (count, bytes, example)) in skipped {
        // Map common extensions to language names
        let lang_name = match ext.as_str() {
            "py" => "Python",
            "js" => "JavaScript",
            "ts" => "TypeScript",
            "jsx" => "React JSX",
            "tsx" => "React TSX",
            "java" => "Java",
            "c" => "C",
            "cpp" | "cc" | "cxx" => "C++",
            "h" | "hpp" => "C/C++ Header",
            "cs" => "C#",
            "rb" => "Ruby",
            "php" => "PHP",
            "swift" => "Swift",
            "kt" => "Kotlin",
            "scala" => "Scala",
            "ml" => "OCaml",
            "hs" => "Haskell",
            "ex" | "exs" => "Elixir",
            "clj" => "Clojure",
            "vue" => "Vue",
            "svelte" => "Svelte",
            "lua" => "Lua",
            "r" => "R",
            "jl" => "Julia",
            "zig" => "Zig",
            "nim" => "Nim",
            "dart" => "Dart",
            "sh" | "bash" => "Shell",
            "yaml" | "yml" => "YAML",
            "json" => "JSON",
            "toml" => "TOML",
            "xml" => "XML",
            "md" => "Markdown",
            _ => "",
        };

        sql.push_str(&format!(
            "INSERT INTO skipped_files (extension, file_count, total_bytes, example_path, common_name) VALUES ('{}', {}, {}, '{}', '{}');\n",
            ext, count, bytes, example.replace('\'', "''"), lang_name
        ));
    }

    sql.push_str("COMMIT;\n");

    // Execute via stdin
    let mut child = Command::new("duckdb")
        .arg(db_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start DuckDB")?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(sql.as_bytes())
            .context("Failed to write SQL")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to save skipped files")?;
    if !output.status.success() {
        eprintln!("Warning: Failed to save skipped files stats");
    }

    Ok(())
}

/// Report skipped files to user
fn report_skipped_files(skipped: &HashMap<String, (usize, usize, String)>) {
    // Sort by file count descending
    let mut sorted: Vec<_> = skipped.iter().collect();
    sorted.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

    println!("\n⚠️  Skipped files (no parser available):");

    // Show top 5 most common extensions
    for (ext, (count, bytes, _)) in sorted.iter().take(5) {
        let size_mb = *bytes as f64 / 1_048_576.0;
        println!("   {} .{} files ({:.1} MB)", count, ext, size_mb);
    }

    if sorted.len() > 5 {
        let remaining: usize = sorted.iter().skip(5).map(|(_, (c, _, _))| c).sum();
        println!("   {} files with other extensions", remaining);
    }

    // Suggest adding parsers for common languages
    let suggestions: Vec<&str> = sorted
        .iter()
        .filter_map(|(ext, (count, _, _))| {
            if *count > 10 {
                match ext.as_str() {
                    "py" => Some("Python"),
                    "js" | "ts" | "jsx" | "tsx" => Some("JavaScript/TypeScript"),
                    "java" => Some("Java"),
                    "c" | "cpp" | "h" => Some("C/C++"),
                    _ => None,
                }
            } else {
                None
            }
        })
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    if !suggestions.is_empty() {
        println!(
            "\n💡 Consider adding parsers for: {}",
            suggestions.join(", ")
        );
    }
}

/// Extract documentation comment for a node
fn extract_doc_comment(
    node: tree_sitter::Node,
    source: &[u8],
    language: languages::Language,
) -> Option<(String, String, Vec<String>)> {
    use languages::Language;

    // Look for doc comment in previous sibling
    if let Some(prev) = node.prev_sibling() {
        let is_doc = match language {
            Language::Rust => {
                prev.kind() == "line_comment" && {
                    prev.utf8_text(source)
                        .unwrap_or("")
                        .trim_start()
                        .starts_with("///")
                }
            }
            Language::Go => {
                prev.kind() == "comment" && {
                    prev.utf8_text(source)
                        .unwrap_or("")
                        .trim_start()
                        .starts_with("//")
                }
            }
            Language::Python => {
                // Python docstrings are the first string in the function body
                if node.kind() == "function_definition" || node.kind() == "class_definition" {
                    if let Some(body) = node.child_by_field_name("body") {
                        if let Some(first_stmt) = body.children(&mut body.walk()).nth(1) {
                            if first_stmt.kind() == "expression_statement" {
                                if let Some(string) = first_stmt.child(0) {
                                    return if string.kind() == "string" {
                                        let raw = string.utf8_text(source).unwrap_or("");
                                        let clean = clean_doc_text(raw, language);
                                        let keywords = extract_keywords(&clean);
                                        Some((raw.to_string(), clean, keywords))
                                    } else {
                                        None
                                    };
                                }
                            }
                        }
                    }
                }
                false
            }
            Language::JavaScript
            | Language::JavaScriptJSX
            | Language::TypeScript
            | Language::TypeScriptTSX => {
                prev.kind() == "comment" && {
                    let text = prev.utf8_text(source).unwrap_or("");
                    text.starts_with("/**") || text.starts_with("//")
                }
            }
            Language::Solidity => {
                prev.kind() == "comment" && {
                    let text = prev.utf8_text(source).unwrap_or("");
                    text.starts_with("///") || text.starts_with("/**")
                }
            }
            _ => false,
        };

        if is_doc {
            let raw = prev.utf8_text(source).unwrap_or("").to_string();
            let clean = clean_doc_text(&raw, language);
            let keywords = extract_keywords(&clean);
            return Some((raw, clean, keywords));
        }
    }

    // For languages other than Python, also check block comments above
    if language != Language::Python {
        // Walk up to find doc comments that might be separated by whitespace
        let mut cursor = node.walk();
        if let Some(parent) = node.parent() {
            for child in parent.children(&mut cursor) {
                if child.end_byte() > node.start_byte() {
                    break;
                }
                if child.kind() == "comment"
                    || child.kind() == "line_comment"
                    || child.kind() == "block_comment"
                {
                    let text = child.utf8_text(source).unwrap_or("");
                    let is_doc = if let Some(spec) = get_language_spec(language) {
                        (spec.is_doc_comment)(text)
                    } else {
                        false
                    };
                    if is_doc {
                        let raw = text.to_string();
                        let clean = clean_doc_text(&raw, language);
                        let keywords = extract_keywords(&clean);
                        return Some((raw, clean, keywords));
                    }
                }
            }
        }
    }

    None
}

/// Clean doc text by removing comment markers
fn clean_doc_text(raw: &str, language: languages::Language) -> String {
    use languages::Language;

    match language {
        Language::Rust => raw
            .lines()
            .map(|line| {
                line.trim_start()
                    .strip_prefix("///")
                    .or_else(|| line.strip_prefix("//!"))
                    .unwrap_or(line)
                    .trim()
            })
            .collect::<Vec<_>>()
            .join(" "),
        Language::Go | Language::Solidity => raw
            .lines()
            .map(|line| line.trim_start().strip_prefix("//").unwrap_or(line).trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
        Language::Python => {
            // Remove triple quotes
            raw.trim()
                .strip_prefix("\"\"\"")
                .and_then(|s| s.strip_suffix("\"\"\""))
                .or_else(|| {
                    raw.trim()
                        .strip_prefix("'''")
                        .and_then(|s| s.strip_suffix("'''"))
                })
                .unwrap_or(raw)
                .trim()
                .to_string()
        }
        Language::JavaScript
        | Language::JavaScriptJSX
        | Language::TypeScript
        | Language::TypeScriptTSX => {
            if raw.starts_with("/**") {
                raw.strip_prefix("/**")
                    .and_then(|s| s.strip_suffix("*/"))
                    .map(|s| {
                        s.lines()
                            .map(|line| line.trim().strip_prefix("*").unwrap_or(line).trim())
                            .filter(|line| !line.is_empty())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_else(|| raw.to_string())
            } else {
                raw.lines()
                    .map(|line| line.trim_start().strip_prefix("//").unwrap_or(line).trim())
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        }
        _ => raw.to_string(),
    }
}

/// Extract first sentence as summary
fn extract_summary(doc: &str) -> String {
    doc.split('.').next().unwrap_or(doc).trim().to_string()
}

/// Extract keywords from documentation
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

    words.into_iter().collect()
}

/// Process AST nodes and generate fingerprints
/// Extract call expressions from AST nodes
fn extract_call_expressions(
    node: tree_sitter::Node,
    source: &[u8],
    language: languages::Language,
    context: &mut ParseContext,
) {
    use languages::Language;

    let line_number = (node.start_position().row + 1) as i32;

    match (language, node.kind()) {
        // Rust call expressions
        (Language::Rust, "call_expression") => {
            if let Some(func_node) = node.child_by_field_name("function") {
                let callee = func_node.utf8_text(source).unwrap_or("").to_string();
                context.add_call(callee, "direct".to_string(), line_number);
            }
        }
        (Language::Rust, "method_call_expression") => {
            if let Some(method_node) = node.child_by_field_name("name") {
                let callee = method_node.utf8_text(source).unwrap_or("").to_string();
                context.add_call(callee, "method".to_string(), line_number);
            }
        }

        // Go call expressions
        (Language::Go, "call_expression") => {
            if let Some(func_node) = node.child_by_field_name("function") {
                let callee = func_node.utf8_text(source).unwrap_or("").to_string();
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
        (Language::Go, "selector_expression") => {
            // Go method calls are selector expressions followed by call_expression
            if let Some(parent) = node.parent() {
                if parent.kind() == "call_expression" {
                    if let Some(field_node) = node.child_by_field_name("field") {
                        let callee = field_node.utf8_text(source).unwrap_or("").to_string();
                        context.add_call(callee, "method".to_string(), line_number);
                    }
                }
            }
        }

        // Python call expressions
        (Language::Python, "call") => {
            if let Some(func_node) = node.child_by_field_name("function") {
                let callee = func_node.utf8_text(source).unwrap_or("").to_string();
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
        (Language::Python, "attribute") => {
            // Python method calls via attribute access
            if let Some(parent) = node.parent() {
                if parent.kind() == "call" {
                    if let Some(attr_node) = node.child_by_field_name("attribute") {
                        let callee = attr_node.utf8_text(source).unwrap_or("").to_string();
                        context.add_call(callee, "method".to_string(), line_number);
                    }
                }
            }
        }

        // JavaScript/TypeScript call expressions
        (
            Language::JavaScript
            | Language::JavaScriptJSX
            | Language::TypeScript
            | Language::TypeScriptTSX,
            "call_expression",
        ) => {
            if let Some(func_node) = node.child_by_field_name("function") {
                let callee_text = func_node.utf8_text(source).unwrap_or("");

                // Check if it's an async call (await keyword)
                let call_type = if node
                    .parent()
                    .is_some_and(|p| p.kind() == "await_expression")
                {
                    "async"
                } else if callee_text.contains('.') {
                    "method"
                } else {
                    "direct"
                };

                // Extract just the function name
                let callee = if let Some(last_part) = callee_text.split('.').next_back() {
                    last_part.to_string()
                } else {
                    callee_text.to_string()
                };

                context.add_call(callee, call_type.to_string(), line_number);
            }
        }
        (
            Language::JavaScript
            | Language::JavaScriptJSX
            | Language::TypeScript
            | Language::TypeScriptTSX,
            "new_expression",
        ) => {
            // Constructor calls
            if let Some(constructor_node) = node.child_by_field_name("constructor") {
                let callee = constructor_node.utf8_text(source).unwrap_or("").to_string();
                context.add_call(callee, "constructor".to_string(), line_number);
            }
        }

        // Solidity call expressions
        (Language::Solidity, "call_expression") => {
            if let Some(func_node) = node.child_by_field_name("function") {
                let callee = func_node.utf8_text(source).unwrap_or("").to_string();
                context.add_call(callee, "direct".to_string(), line_number);
            }
        }
        (Language::Solidity, "member_access") => {
            // Solidity method calls
            if let Some(parent) = node.parent() {
                if parent.kind() == "call_expression" {
                    if let Some(member_node) = node.child_by_field_name("property") {
                        let callee = member_node.utf8_text(source).unwrap_or("").to_string();
                        context.add_call(callee, "method".to_string(), line_number);
                    }
                }
            }
        }

        _ => {}
    }
}

// ============================================================================
// CHAPTER 7: AST PROCESSING
// ============================================================================

/// Context for tracking state during AST traversal
struct ParseContext {
    current_function: Option<String>,
    call_graph_entries: Vec<(String, String, String, i32)>, // (caller, callee, call_type, line)
}

impl ParseContext {
    fn new() -> Self {
        Self {
            current_function: None,
            call_graph_entries: Vec::new(),
        }
    }

    fn enter_function(&mut self, name: String) {
        self.current_function = Some(name);
    }

    fn exit_function(&mut self) {
        self.current_function = None;
    }

    fn add_call(&mut self, callee: String, call_type: String, line: i32) {
        if let Some(ref caller) = self.current_function {
            self.call_graph_entries
                .push((caller.clone(), callee, call_type, line));
        }
    }

    fn flush_to_sql(&mut self, file_path: &str, sql: &mut String) {
        for (caller, callee, call_type, line) in &self.call_graph_entries {
            sql.push_str(&format!(
                "INSERT INTO call_graph (caller, callee, file, call_type, line_number) VALUES ('{}', '{}', '{}', '{}', {});\n",
                caller.replace('\'', "''"),
                callee.replace('\'', "''"),
                file_path,
                call_type,
                line
            ));
        }
        self.call_graph_entries.clear();
    }
}

fn process_ast_node(
    cursor: &mut tree_sitter::TreeCursor,
    source: &[u8],
    file_path: &str,
    sql: &mut String,
    language: languages::Language,
    context: &mut ParseContext,
) -> usize {
    use fingerprint::Fingerprint;
    use languages::Language;

    let node = cursor.node();
    let mut count = 0;

    // Check if this is a symbol we want to fingerprint
    let kind = if let Some(spec) = get_language_spec(language) {
        // First try the simple mapping
        let basic_kind = (spec.get_symbol_kind)(node.kind());

        if basic_kind != "unknown" {
            basic_kind
        } else {
            // Try complex mapping that needs node inspection
            if let Some(complex_kind) = (spec.get_symbol_kind_complex)(&node, source) {
                complex_kind
            } else {
                // Not a symbol we care about - recurse into children
                extract_call_expressions(node, source, language, context);

                if cursor.goto_first_child() {
                    loop {
                        count +=
                            process_ast_node(cursor, source, file_path, sql, language, context);
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                    cursor.goto_parent();
                }
                return count;
            }
        }
    } else {
        // Unknown language - just recurse
        if cursor.goto_first_child() {
            loop {
                count += process_ast_node(cursor, source, file_path, sql, language, context);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        return count;
    };

    // Skip empty kinds
    if kind.is_empty() {
        return count;
    }

    // Handle imports separately (they don't have a name field)
    if kind == "import" {
        extract_import_fact(node, source, file_path, sql, language);
        count += 1;
        return count;
    }

    // Extract name for other symbols
    let name_node = match language {
        Language::Go if node.kind() == "type_spec" => {
            // Go type specs have name directly in the node
            node.child_by_field_name("name")
        }
        Language::Solidity => {
            // Solidity has name or identifier field
            node.child_by_field_name("name")
                .or_else(|| node.child_by_field_name("identifier"))
        }
        Language::Python => {
            // Python uses name field for functions/classes
            if node.kind() == "decorated_definition" {
                // For decorated definitions, get the name from the inner definition
                node.child_by_field_name("definition")
                    .and_then(|def| def.child_by_field_name("name"))
            } else {
                node.child_by_field_name("name")
            }
        }
        Language::JavaScript
        | Language::JavaScriptJSX
        | Language::TypeScript
        | Language::TypeScriptTSX => {
            // JS/TS uses different field names depending on context
            if node.kind() == "variable_declarator" {
                // For const/let/var declarations, name is in the 'name' field
                node.child_by_field_name("name")
            } else if node.kind() == "method_definition" {
                // Methods have name in 'name' field
                node.child_by_field_name("name")
            } else {
                // Functions, classes use 'name' field
                node.child_by_field_name("name")
            }
        }
        _ => node.child_by_field_name("name"),
    };

    if let Some(name_node) = name_node {
        let name = name_node.utf8_text(source).unwrap_or("<unknown>");

        // Track function context for call graph
        if kind == "function" {
            context.enter_function(name.to_string());

            // Process function body to extract calls
            if cursor.goto_first_child() {
                loop {
                    count += process_ast_node(cursor, source, file_path, sql, language, context);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }

            // Exit function context
            context.exit_function();
        }

        // Extract documentation if present
        let doc_info = extract_doc_comment(node, source, language);
        if let Some((doc_raw, doc_clean, keywords)) = doc_info {
            // Store documentation in database
            let line_number = node.start_position().row + 1;
            let doc_summary = extract_summary(&doc_clean);
            let doc_length = doc_clean.len() as i32;
            let has_examples = doc_raw.contains("```")
                || doc_raw.contains("Example:")
                || doc_raw.contains("example:");
            let has_params = doc_raw.contains("@param")
                || doc_raw.contains("Args:")
                || doc_raw.contains("Parameters:");

            // Format keywords as DuckDB array
            let keywords_str = if keywords.is_empty() {
                "ARRAY[]::VARCHAR[]".to_string()
            } else {
                format!(
                    "ARRAY[{}]",
                    keywords
                        .iter()
                        .map(|k| format!("'{}'", k.replace('\'', "''")))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };

            sql.push_str(&format!(
                "INSERT OR REPLACE INTO documentation (file, symbol_name, symbol_type, line_number, doc_raw, doc_clean, doc_summary, keywords, doc_length, has_examples, has_params) VALUES ('{}', '{}', '{}', {}, '{}', '{}', '{}', {}, {}, {}, {});\n",
                file_path,
                name.replace('\'', "''"),
                kind,
                line_number,
                doc_raw.replace('\'', "''"),
                doc_clean.replace('\'', "''"),
                doc_summary.replace('\'', "''"),
                keywords_str,
                doc_length,
                has_examples,
                has_params
            ));
        }

        // Extract based on kind
        match kind {
            "function" => {
                // Extract function facts
                extract_function_facts(node, source, file_path, name, sql, language);

                // Also generate fingerprint for functions
                let fingerprint = Fingerprint::from_ast(node, source);
                let signature = node
                    .utf8_text(source)
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .replace('\'', "''");

                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO code_fingerprints (path, name, kind, pattern, imports, complexity, flags) VALUES ('{}', '{}', '{}', {}, {}, {}, {});\n",
                    file_path, name, kind,
                    fingerprint.pattern, fingerprint.imports,
                    fingerprint.complexity, fingerprint.flags
                ));

                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO code_search (path, name, signature) VALUES ('{}', '{}', '{}');\n",
                    file_path, name, signature
                ));
            }
            "type_alias" | "struct" | "trait" | "const" => {
                // Extract type vocabulary
                extract_type_definition(node, source, file_path, name, kind, sql, language);

                // Also generate fingerprint for structs/traits
                if kind == "struct" || kind == "trait" {
                    let fingerprint = Fingerprint::from_ast(node, source);
                    sql.push_str(&format!(
                        "INSERT OR REPLACE INTO code_fingerprints (path, name, kind, pattern, imports, complexity, flags) VALUES ('{}', '{}', '{}', {}, {}, {}, {});\n",
                        file_path, name, kind,
                        fingerprint.pattern, fingerprint.imports,
                        fingerprint.complexity, fingerprint.flags
                    ));
                }
            }
            "impl" => {
                // Keep fingerprinting for impl blocks
                let fingerprint = Fingerprint::from_ast(node, source);
                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO code_fingerprints (path, name, kind, pattern, imports, complexity, flags) VALUES ('{}', '{}', '{}', {}, {}, {}, {});\n",
                    file_path, name, kind,
                    fingerprint.pattern, fingerprint.imports,
                    fingerprint.complexity, fingerprint.flags
                ));
            }
            _ => {}
        }

        count += 1;
    }

    count
}

/// Show extraction summary
fn show_summary(db_path: &str) -> Result<()> {
    println!("\n📈 Summary:");

    let summary_query = r#"
SELECT 
    'Functions indexed' as metric,
    COUNT(*) as value
FROM code_fingerprints
WHERE kind = 'function'
UNION ALL
SELECT 
    'Average complexity' as metric,
    CAST(AVG(complexity) AS INTEGER) as value
FROM code_fingerprints
WHERE kind = 'function'
UNION ALL
SELECT 
    'Unique patterns' as metric,
    COUNT(DISTINCT pattern) as value
FROM code_fingerprints
UNION ALL
SELECT 
    'Files with 10+ commits' as metric,
    COUNT(*) as value
FROM git_metrics
WHERE commit_count >= 10
UNION ALL
SELECT
    'Languages skipped' as metric,
    COUNT(*) as value
FROM skipped_files
WHERE file_count > 0;
"#;

    let output = Command::new("duckdb")
        .arg(db_path)
        .arg("-c")
        .arg(summary_query)
        .output()
        .context("Failed to query summary")?;

    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }

    // Show database size and block info
    let size_query = "PRAGMA database_size;";
    let size_output = Command::new("duckdb")
        .arg(db_path)
        .arg("-c")
        .arg(size_query)
        .output()?;

    if size_output.status.success() {
        println!("\n💾 Database info:");
        println!("{}", String::from_utf8_lossy(&size_output.stdout));
    }

    // Also show file size
    if let Ok(metadata) = std::fs::metadata(db_path) {
        let size_kb = metadata.len() / 1024;
        println!("📁 File size: {}KB", size_kb);
    }

    Ok(())
}

/// Run a custom query
/// Extract function facts (truth data only)
fn extract_function_facts(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    sql: &mut String,
    language: languages::Language,
) {
    use languages::Language;

    // Extract visibility
    let is_public = if let Some(spec) = get_language_spec(language) {
        (spec.parse_visibility)(&node, name, source)
    } else {
        false // Unknown language defaults to private
    };

    // Extract async
    let is_async = if let Some(spec) = get_language_spec(language) {
        (spec.has_async)(&node, source)
    } else {
        false
    };

    // Extract unsafe
    let is_unsafe = if let Some(spec) = get_language_spec(language) {
        (spec.has_unsafe)(&node, source)
    } else {
        false
    };

    // Extract parameters with details
    // Note: Solidity doesn't have a "parameters" field, parameters are direct children
    let params_node = if language != Language::Solidity {
        node.child_by_field_name("parameters")
    } else {
        None
    };

    let (takes_mut_self, takes_mut_params, parameter_count, parameter_list) = if language
        == Language::Solidity
    {
        // Special handling for Solidity - parameters are direct children of type "parameter"
        let mut param_count = 0;
        let mut param_details = Vec::new();

        for child in node.children(&mut node.walk()) {
            if child.kind() == "parameter" {
                let param_text = child.utf8_text(source).unwrap_or("").to_string();
                param_details.push(param_text);
                param_count += 1;
            }
        }

        let param_list = if param_details.is_empty() {
            "[]".to_string()
        } else {
            serde_json::to_string(&param_details).unwrap_or_else(|_| "[]".to_string())
        };

        (false, false, param_count, param_list)
    } else if let Some(params) = params_node {
        let mut has_mut_self = false;
        let mut has_mut_params = false;
        let mut param_count = 0;
        let mut param_details = Vec::new();

        let params_text = params.utf8_text(source).unwrap_or("");

        match language {
            Language::Rust => {
                // Check for &mut self
                if params_text.contains("&mut self") {
                    has_mut_self = true;
                }
                // Check for other mut params
                if params_text.contains("mut ") && !params_text.contains("&mut self") {
                    has_mut_params = true;
                }
                // Extract each parameter
                for child in params.children(&mut params.walk()) {
                    if child.kind() == "parameter" {
                        // Get parameter name and type
                        if let Some(pattern) = child.child_by_field_name("pattern") {
                            let param_name = pattern.utf8_text(source).unwrap_or("").to_string();
                            let param_type = child
                                .child_by_field_name("type")
                                .and_then(|t| t.utf8_text(source).ok())
                                .unwrap_or("")
                                .to_string();
                            param_details.push(format!("{}:{}", param_name, param_type));
                        }
                        param_count += 1;
                    } else if child.kind() == "self_parameter" {
                        param_details.push("self".to_string());
                        param_count += 1;
                    }
                }
            }
            Language::Go => {
                // Extract Go parameters
                for child in params.children(&mut params.walk()) {
                    if child.kind() == "parameter_declaration" {
                        let param_text = child.utf8_text(source).unwrap_or("").to_string();
                        param_details.push(param_text);
                        param_count += 1;
                    }
                }
            }
            Language::Python => {
                // Extract Python parameters - simpler approach
                for child in params.children(&mut params.walk()) {
                    // Skip punctuation
                    if child.kind() == "," || child.kind() == "(" || child.kind() == ")" {
                        continue;
                    }

                    // Get any parameter-like text
                    if child.kind().contains("parameter") || child.kind() == "identifier" {
                        let param_text = child.utf8_text(source).unwrap_or("").trim().to_string();
                        if !param_text.is_empty() {
                            param_count += 1;
                            if param_text != "self" {
                                // Skip 'self' in param list but count it
                                param_details.push(param_text);
                            }
                        }
                    }
                }
            }
            Language::JavaScript
            | Language::JavaScriptJSX
            | Language::TypeScript
            | Language::TypeScriptTSX => {
                // Extract JS/TS parameters - they can be formal_parameter, required_parameter, optional_parameter, or just identifier
                for child in params.children(&mut params.walk()) {
                    // Skip punctuation like commas and parentheses
                    if child.kind() == "," || child.kind() == "(" || child.kind() == ")" {
                        continue;
                    }

                    // Get the parameter text for any parameter-like node
                    if child.kind().contains("parameter") || child.kind() == "identifier" {
                        let param_text = child.utf8_text(source).unwrap_or("").trim().to_string();
                        if !param_text.is_empty() {
                            param_details.push(param_text);
                            param_count += 1;
                        }
                    }
                }
            }
            Language::Solidity | Language::Cairo | Language::Unknown => {} // Solidity handled earlier, Cairo uses different parser, Unknown skipped
        }

        // Create parameter list string (escape for SQL)
        let param_list = if !param_details.is_empty() {
            param_details.join(", ").replace('\'', "''")
        } else {
            String::new()
        };

        (has_mut_self, has_mut_params, param_count, param_list)
    } else {
        (false, false, 0, String::new())
    };

    // Extract return type with full details
    let (returns_result, returns_option, return_type) = match language {
        Language::Rust => {
            if let Some(return_type_node) = node.child_by_field_name("return_type") {
                let ret_text = return_type_node.utf8_text(source).unwrap_or("");
                let ret_clean = ret_text.replace('\'', "''");
                (
                    ret_text.contains("Result"),
                    ret_text.contains("Option"),
                    ret_clean,
                )
            } else {
                (false, false, String::new())
            }
        }
        Language::Go => {
            if let Some(result) = node.child_by_field_name("result") {
                let ret_text = result.utf8_text(source).unwrap_or("");
                let ret_clean = ret_text.replace('\'', "''");
                (ret_text.contains("error"), false, ret_clean) // Go uses error, not Result/Option
            } else {
                (false, false, String::new())
            }
        }
        Language::TypeScript | Language::TypeScriptTSX => {
            if let Some(return_type_node) = node.child_by_field_name("return_type") {
                let ret_text = return_type_node.utf8_text(source).unwrap_or("");
                let ret_clean = ret_text.replace('\'', "''");
                (false, false, ret_clean) // TypeScript has explicit return types
            } else {
                (false, false, String::new())
            }
        }
        _ => (false, false, String::new()),
    };

    // Count generics
    let generic_count = match language {
        Language::Rust => node
            .child_by_field_name("type_parameters")
            .map(|tp| {
                tp.children(&mut tp.walk())
                    .filter(|c| c.kind() == "type_identifier" || c.kind() == "lifetime")
                    .count()
            })
            .unwrap_or(0),
        _ => 0, // Go doesn't have generics (until recently), Solidity doesn't
    };

    // Insert function facts with parameter and return type details
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
        parameter_count,
        generic_count,
        parameter_list,  // Already escaped with '' replacement
        return_type      // Already escaped with '' replacement
    ));

    // Extract behavioral hints
    if language == Language::Rust {
        extract_behavioral_hints(node, source, file_path, name, sql);
    }
}

/// Extract type definitions for vocabulary
fn extract_type_definition(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: &str,
    sql: &mut String,
    language: languages::Language,
) {
    use languages::Language;

    // Get the full definition (first line for brevity)
    let definition = node
        .utf8_text(source)
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .replace('\'', "''");

    // Determine visibility
    let visibility = match language {
        Language::Rust => {
            if node.children(&mut node.walk()).any(|child| {
                if child.kind() == "visibility_modifier" {
                    let vis_text = child.utf8_text(source).unwrap_or("");
                    vis_text.contains("pub(crate)")
                } else {
                    false
                }
            }) {
                "pub(crate)"
            } else if node
                .children(&mut node.walk())
                .any(|child| child.kind() == "visibility_modifier")
            {
                "pub"
            } else {
                "private"
            }
        }
        Language::Go => {
            // In Go, uppercase = public
            if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                "pub"
            } else {
                "private"
            }
        }
        Language::Solidity => "pub", // Most things in Solidity are public by default
        Language::Python => {
            // Python convention: _ prefix = private
            if name.starts_with('_') {
                "private"
            } else {
                "pub"
            }
        }
        Language::JavaScript
        | Language::JavaScriptJSX
        | Language::TypeScript
        | Language::TypeScriptTSX => {
            // JS/TS: look for export keyword
            let text = node.utf8_text(source).unwrap_or("");
            if text.contains("export") {
                "pub"
            } else {
                "private"
            }
        }
        Language::Cairo => "pub", // Cairo defaults to public
        Language::Unknown => "private",
    };

    // Insert type vocabulary
    sql.push_str(&format!(
        "INSERT OR REPLACE INTO type_vocabulary (file, name, definition, kind, visibility) VALUES ('{}', '{}', '{}', '{}', '{}');\n",
        escape_sql(file_path),
        escape_sql(name),
        escape_sql(&definition),
        kind,
        visibility
    ));
}

/// Extract import facts
fn extract_import_fact(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    sql: &mut String,
    language: languages::Language,
) {
    use languages::Language;

    let import_text = node.utf8_text(source).unwrap_or("");

    match language {
        Language::Rust => {
            // Parse Rust use statements
            let import_clean = import_text.trim_start_matches("use ").trim_end_matches(';');

            // Determine if external
            let is_external = !import_clean.starts_with("crate::")
                && !import_clean.starts_with("super::")
                && !import_clean.starts_with("self::");

            // Extract the imported item (last part after ::)
            let imported_item = import_clean.split("::").last().unwrap_or(import_clean);

            // Extract the source module
            let imported_from = if import_clean.contains("::") {
                import_clean
                    .rsplit_once("::")
                    .map(|(from, _)| from)
                    .unwrap_or(import_clean)
            } else {
                import_clean
            };

            sql.push_str(&format!(
                "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, 'use');\n",
                escape_sql(file_path),
                escape_sql(imported_item),
                escape_sql(imported_from),
                is_external
            ));
        }
        Language::Go => {
            // Parse Go imports
            let import_clean = import_text
                .trim_start_matches("import ")
                .trim()
                .trim_matches('"');

            let is_external = !import_clean.starts_with(".");

            let imported_item = import_clean.split('/').next_back().unwrap_or(import_clean);

            sql.push_str(&format!(
                "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, 'import');\n",
                escape_sql(file_path),
                escape_sql(imported_item),
                escape_sql(import_clean),
                is_external
            ));
        }
        Language::Solidity => {
            // Parse Solidity imports
            if let Some(path_match) = import_text.split('"').nth(1) {
                let is_external = path_match.starts_with('@') || path_match.starts_with("http");

                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, 'import');\n",
                    escape_sql(file_path),
                    escape_sql(path_match),
                    escape_sql(path_match),
                    is_external
                ));
            }
        }
        Language::Python => {
            // Python imports: import x or from x import y
            // Simple extraction - just store the whole import for now
            let import_clean = import_text.trim();
            let is_external = !import_clean.contains("from .");

            sql.push_str(&format!(
                "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, 'import');\n",
                escape_sql(file_path),
                escape_sql(import_clean),
                escape_sql(import_clean),
                is_external
            ));
        }
        Language::JavaScript
        | Language::JavaScriptJSX
        | Language::TypeScript
        | Language::TypeScriptTSX => {
            // JS/TS imports: import x from 'y'
            // Simple extraction - just store the module path
            if let Some(module_match) = import_text
                .split('\'')
                .nth(1)
                .or_else(|| import_text.split('"').nth(1))
            {
                let is_external = !module_match.starts_with('.');

                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, 'import');\n",
                    escape_sql(file_path),
                    escape_sql(module_match),
                    escape_sql(module_match),
                    is_external
                ));
            }
        }
        Language::Cairo => {
            // Parse Cairo use statements
            let import_clean = import_text.trim_start_matches("use ").trim_end_matches(';');

            // Cairo imports are typically external unless they're relative
            let is_external =
                !import_clean.starts_with("super::") && !import_clean.starts_with("self::");

            let imported_item = import_clean.split("::").last().unwrap_or(import_clean);

            sql.push_str(&format!(
                "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, 'use');\n",
                escape_sql(file_path),
                escape_sql(imported_item),
                escape_sql(import_clean),
                is_external
            ));
        }
        Language::Unknown => {} // Skip unknown languages
    }
}

/// Extract behavioral hints (code smells as facts)
fn extract_behavioral_hints(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    function_name: &str,
    sql: &mut String,
) {
    // Only extract for function bodies
    if let Some(body) = node.child_by_field_name("body") {
        let body_text = body.utf8_text(source).unwrap_or("");

        // Count unwrap calls
        let calls_unwrap = body_text.matches(".unwrap()").count();

        // Count expect calls
        let calls_expect = body_text.matches(".expect(").count();

        // Check for panic! macro
        let has_panic_macro = body_text.contains("panic!");

        // Check for todo! macro
        let has_todo_macro = body_text.contains("todo!");

        // Check for unsafe blocks
        let has_unsafe_block = body_text.contains("unsafe {");

        // Check for Mutex usage
        let has_mutex = body_text.contains("Mutex");

        // Check for Arc usage
        let has_arc = body_text.contains("Arc<") || body_text.contains("Arc::");

        // Only insert if there are any behavioral hints
        if calls_unwrap > 0
            || calls_expect > 0
            || has_panic_macro
            || has_todo_macro
            || has_unsafe_block
            || has_mutex
            || has_arc
        {
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO behavioral_hints (file, function, calls_unwrap, calls_expect, has_panic_macro, has_todo_macro, has_unsafe_block, has_mutex, has_arc) VALUES ('{}', '{}', {}, {}, {}, {}, {}, {}, {});\n",
                escape_sql(file_path),
                escape_sql(function_name),
                calls_unwrap,
                calls_expect,
                has_panic_macro,
                has_todo_macro,
                has_unsafe_block,
                has_mutex,
                has_arc
            ));
        }
    }
}

// ============================================================================
// CHAPTER 8: UTILITIES
// ============================================================================

/// Escape SQL strings
fn escape_sql(s: &str) -> String {
    s.replace('\'', "''")
}

// ============================================================================
// CHAPTER 9: MODULES
// ============================================================================

// ============================================================================
// FINGERPRINT MODULE
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

    /// Generate DuckDB schema for fingerprint storage
    pub fn generate_schema() -> &'static str {
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
// LANGUAGES MODULE
// ============================================================================
pub(crate) mod languages {
    use anyhow::{Context, Result};
    use std::path::Path;
    use tree_sitter::Parser;

    /// Supported programming languages
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Language {
        Rust,
        Go,
        Solidity,
        Python,
        JavaScript,
        JavaScriptJSX, // .jsx files
        TypeScript,
        TypeScriptTSX, // .tsx files
        Cairo,
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
                Some("cairo") => Language::Cairo,
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
                Language::Cairo => Some(patina_metal::Metal::Cairo),
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
