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
//! - `function_facts`: Behavioral signals (async, unsafe, mutability)
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

    /// Clean documentation text for a language
    clean_doc_comment: fn(&str) -> String,

    /// Extract import details from an import node
    extract_import_details: fn(&Node, &[u8]) -> (String, String, bool),
}

// ============================================================================
// LANGUAGE SPECIFICATIONS
// ============================================================================

/// Rust language specification
static RUST_SPEC: LanguageSpec = LanguageSpec {
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

    clean_doc_comment: |raw| {
        raw.lines()
            .map(|line| {
                line.trim_start()
                    .strip_prefix("///")
                    .or_else(|| line.strip_prefix("//!"))
                    .unwrap_or(line)
                    .trim()
            })
            .collect::<Vec<_>>()
            .join(" ")
    },

    extract_import_details: |node, source| {
        let import_text = node.utf8_text(source).unwrap_or("");
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

        (
            imported_item.to_string(),
            imported_from.to_string(),
            is_external,
        )
    },
};

/// Go language specification
static GO_SPEC: LanguageSpec = LanguageSpec {
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

    clean_doc_comment: |raw| {
        raw.lines()
            .map(|line| line.trim_start().strip_prefix("//").unwrap_or(line).trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    },

    extract_import_details: |node, source| {
        let import_text = node.utf8_text(source).unwrap_or("");
        let import_clean = import_text
            .trim_start_matches("import ")
            .trim()
            .trim_matches('"');

        let is_external = !import_clean.starts_with(".");
        let imported_item = import_clean.split('/').next_back().unwrap_or(import_clean);

        (
            imported_item.to_string(),
            import_clean.to_string(),
            is_external,
        )
    },
};

/// Python language specification
static PYTHON_SPEC: LanguageSpec = LanguageSpec {
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

    clean_doc_comment: |raw| {
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
    },

    extract_import_details: |node, source| {
        let import_text = node.utf8_text(source).unwrap_or("");
        let import_clean = import_text.trim();
        let is_external = !import_clean.contains("from .");

        // For now, just use the whole import text as both item and from
        (
            import_clean.to_string(),
            import_clean.to_string(),
            is_external,
        )
    },
};

/// JavaScript language specification (shared base for JS/JSX)
static JS_SPEC: LanguageSpec = LanguageSpec {
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

    clean_doc_comment: |raw| {
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
    },

    extract_import_details: |node, source| {
        let import_text = node.utf8_text(source).unwrap_or("");
        if let Some(module_match) = import_text
            .split('\'')
            .nth(1)
            .or_else(|| import_text.split('"').nth(1))
        {
            let is_external = !module_match.starts_with('.');
            (
                module_match.to_string(),
                module_match.to_string(),
                is_external,
            )
        } else {
            (String::new(), String::new(), false)
        }
    },
};

/// TypeScript language specification
static TS_SPEC: LanguageSpec = LanguageSpec {
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

    clean_doc_comment: |raw| {
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
    },

    extract_import_details: |node, source| {
        let import_text = node.utf8_text(source).unwrap_or("");
        if let Some(module_match) = import_text
            .split('\'')
            .nth(1)
            .or_else(|| import_text.split('"').nth(1))
        {
            let is_external = !module_match.starts_with('.');
            (
                module_match.to_string(),
                module_match.to_string(),
                is_external,
            )
        } else {
            (String::new(), String::new(), false)
        }
    },
};

/// Solidity language specification
static SOLIDITY_SPEC: LanguageSpec = LanguageSpec {
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

    clean_doc_comment: |raw| {
        raw.lines()
            .map(|line| line.trim_start().strip_prefix("//").unwrap_or(line).trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    },

    extract_import_details: |node, source| {
        let import_text = node.utf8_text(source).unwrap_or("");
        if let Some(path_match) = import_text.split('"').nth(1) {
            let is_external = path_match.starts_with('@') || path_match.starts_with("http");
            (path_match.to_string(), path_match.to_string(), is_external)
        } else {
            (String::new(), String::new(), false)
        }
    },
};

/// C language specification
static C_SPEC: LanguageSpec = LanguageSpec {
    is_doc_comment: |text| {
        // C uses /** */ for doc comments
        text.starts_with("/**") || text.starts_with("/*!")
    },

    parse_visibility: |_node, _name, _source| {
        // C doesn't have visibility modifiers, everything in headers is public
        true
    },

    has_async: |_node, _source| {
        // C doesn't have async
        false
    },

    has_unsafe: |_node, _source| {
        // All C is technically unsafe from Rust's perspective
        true
    },

    extract_params: |node, source| {
        if let Some(params_node) = node
            .child_by_field_name("declarator")
            .and_then(|d| d.child_by_field_name("parameters"))
        {
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
        node.child_by_field_name("type")
            .and_then(|t| t.utf8_text(source).ok())
            .map(String::from)
    },

    extract_generics: |_node, _source| {
        // C doesn't have generics
        None
    },

    get_symbol_kind: |node_kind| match node_kind {
        "function_definition" => "function",
        "struct_specifier" => "struct",
        "union_specifier" => "union",
        "enum_specifier" => "enum",
        "type_definition" => "type_alias",
        "declaration" => "variable",
        "preproc_include" => "import",
        _ => "unknown",
    },

    get_symbol_kind_complex: |_node, _source| None,

    clean_doc_comment: |raw| {
        raw.lines()
            .map(|line| {
                line.trim()
                    .strip_prefix("///")
                    .or_else(|| line.strip_prefix("//!"))
                    .or_else(|| line.strip_prefix("//"))
                    .or_else(|| line.strip_prefix("/**"))
                    .or_else(|| line.strip_prefix("/*"))
                    .or_else(|| line.strip_prefix("*"))
                    .or_else(|| line.strip_suffix("*/"))
                    .unwrap_or(line)
                    .trim()
            })
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    },

    extract_import_details: |node, source| {
        let import_text = node.utf8_text(source).unwrap_or("");
        let import_clean = import_text
            .trim_start_matches("#include")
            .trim()
            .trim_start_matches('<')
            .trim_start_matches('"')
            .trim_end_matches('>')
            .trim_end_matches('"');

        // System headers use <>, local headers use ""
        let is_external = import_text.contains('<');

        (
            import_clean.to_string(),
            import_clean.to_string(),
            is_external,
        )
    },
};

/// C++ language specification
static CPP_SPEC: LanguageSpec = LanguageSpec {
    is_doc_comment: |text| {
        // C++ uses /** */ or /// for doc comments
        text.starts_with("/**") || text.starts_with("///") || text.starts_with("//!")
    },

    parse_visibility: |node, _name, source| {
        // Check for public/private/protected access specifiers
        // Default is private for class, public for struct
        let mut cursor = node.walk();
        let parent = node.parent();

        // Check if we're in a class (default private) or struct (default public)
        let default_public = parent
            .is_none_or(|p| p.kind() == "struct_specifier" || p.kind() == "namespace_definition");

        // Look for explicit access specifiers
        for child in node.children(&mut cursor) {
            if let Ok(text) = child.utf8_text(source) {
                if text.contains("private") {
                    return false;
                } else if text.contains("public") {
                    return true;
                }
            }
        }

        default_public
    },

    has_async: |_node, _source| {
        // C++ doesn't have async keyword (uses std::async)
        false
    },

    has_unsafe: |_node, _source| {
        // All C++ is technically unsafe from Rust's perspective
        true
    },

    extract_params: |node, source| {
        if let Some(params_node) = node
            .child_by_field_name("declarator")
            .and_then(|d| d.child_by_field_name("parameters"))
        {
            let mut params = Vec::new();
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "parameter_declaration"
                    || child.kind() == "optional_parameter_declaration"
                {
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
        // Look for trailing return type first (C++11 style)
        node.child_by_field_name("trailing_return_type")
            .or_else(|| node.child_by_field_name("type"))
            .and_then(|t| t.utf8_text(source).ok())
            .map(String::from)
    },

    extract_generics: |node, source| {
        // Look for template parameters
        node.parent()
            .and_then(|p| {
                if p.kind() == "template_declaration" {
                    p.child_by_field_name("parameters")
                } else {
                    None
                }
            })
            .and_then(|tp| tp.utf8_text(source).ok())
            .map(String::from)
    },

    get_symbol_kind: |node_kind| match node_kind {
        "function_definition" => "function",
        "class_specifier" => "class",
        "struct_specifier" => "struct",
        "union_specifier" => "union",
        "enum_specifier" => "enum",
        "namespace_definition" => "namespace",
        "template_declaration" => "template",
        "type_alias_declaration" | "using_declaration" => "type_alias",
        "declaration" => "variable",
        "preproc_include" => "import",
        _ => "unknown",
    },

    get_symbol_kind_complex: |node, _source| {
        // Check if template_declaration contains a class/struct/function
        if node.kind() == "template_declaration" {
            if let Some(child) = node.named_child(1) {
                return match child.kind() {
                    "class_specifier" => Some("template_class"),
                    "struct_specifier" => Some("template_struct"),
                    "function_definition" => Some("template_function"),
                    _ => Some("template"),
                };
            }
        }
        None
    },

    clean_doc_comment: |raw| {
        raw.lines()
            .map(|line| {
                line.trim()
                    .strip_prefix("///")
                    .or_else(|| line.strip_prefix("//!"))
                    .or_else(|| line.strip_prefix("//"))
                    .or_else(|| line.strip_prefix("/**"))
                    .or_else(|| line.strip_prefix("/*"))
                    .or_else(|| line.strip_prefix("*"))
                    .or_else(|| line.strip_suffix("*/"))
                    .unwrap_or(line)
                    .trim()
            })
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    },

    extract_import_details: |node, source| {
        let import_text = node.utf8_text(source).unwrap_or("");
        let import_clean = import_text
            .trim_start_matches("#include")
            .trim()
            .trim_start_matches('<')
            .trim_start_matches('"')
            .trim_end_matches('>')
            .trim_end_matches('"');

        // System headers use <>, local headers use ""
        let is_external = import_text.contains('<');

        (
            import_clean.to_string(),
            import_clean.to_string(),
            is_external,
        )
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
        registry.insert(Language::C, &C_SPEC);
        registry.insert(Language::Cpp, &CPP_SPEC);

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


"#,
        schema::generate_schema(),
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


    // Step 3: Extract code metadata with tree-sitter
    let symbol_count = extract_code_metadata(db_path, work_dir, force)?;

    // Step 4: Show summary
    show_summary(db_path)?;

    Ok(symbol_count)
}

// ============================================================================
// CHAPTER 3: EXTRACTION - Git Metrics
// ============================================================================


// ============================================================================
// CHAPTER 4: EXTRACTION - Pattern References
// ============================================================================


// ============================================================================
// CHAPTER 5: EXTRACTION - Semantic Data
// ============================================================================

fn extract_code_metadata(db_path: &str, work_dir: &Path, force: bool) -> Result<usize> {
    println!("🧠 Extracting code metadata and semantic information...");

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
            | Language::Cairo
            | Language::C
            | Language::Cpp => {
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
        "  📂 Found {} files ({} Rust, {} Go, {} Solidity, {} Python, {} JS, {} JSX, {} TS, {} TSX, {} Cairo, {} C, {} C++)",
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
            .count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::C)
            .count(),
        all_files
            .iter()
            .filter(|(_, l)| *l == Language::Cpp)
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
            .arg("DELETE FROM code_search; DELETE FROM index_state;")
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
        if language == Language::C || language == Language::Cpp {
            // Use iterative tree walking for C/C++ to avoid stack overflow on deeply nested code
            let metadata = std::fs::metadata(&file_path)?;
            let mtime = metadata
                .modified()?
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs() as i64;

            let content = std::fs::read_to_string(&file_path)?;

            // Create parser for C/C++
            if language != current_lang {
                parser = Some(create_parser_for_path(&file_path)?);
                current_lang = language;
            }

            if let Some(ref mut p) = parser {
                if let Some(tree) = p.parse(&content, None) {

                    let mut context = ParseContext::new();

                    // Use iterative processing for C/C++ to avoid stack overflow
                    symbol_count += process_c_cpp_iterative(
                        tree,
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
        } else if language == Language::Cairo {
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
                for _func in symbols.functions {
                    symbol_count += 1;
                }

                for _s in symbols.structs {
                    symbol_count += 1;
                }

                for _t in symbols.traits {
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

            // Parse and extract metadata
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

    println!("  ✓ Processed {} symbols", symbol_count);


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
            Language::C | Language::Cpp => {
                // C/C++ doc comments: /** */ or /// or //
                prev.kind() == "comment" && {
                    let text = prev.utf8_text(source).unwrap_or("");
                    text.starts_with("/**")
                        || text.starts_with("///")
                        || text.starts_with("//!")
                        || text.starts_with("//")
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
    if let Some(spec) = get_language_spec(language) {
        (spec.clean_doc_comment)(raw)
    } else {
        raw.to_string()
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

/// Iterative tree processing for C/C++ to avoid stack overflow on deeply nested code
/// Uses a queue-based approach instead of recursion to handle deeply nested AST trees
fn process_c_cpp_iterative(
    tree: tree_sitter::Tree,
    source: &[u8],
    file_path: &str,
    sql: &mut String,
    language: languages::Language,
    context: &mut ParseContext,
) -> usize {
    use std::collections::VecDeque;

    let mut count = 0;
    let mut work_queue = VecDeque::new();

    // Start with root node
    work_queue.push_back(tree.root_node());

    while let Some(node) = work_queue.pop_front() {
        // First, extract any call expressions from this node
        extract_call_expressions(node, source, language, context);

        // Check if this is a symbol we want to extract
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
                    // Not a symbol we care about - just add children to queue
                    for i in 0..node.child_count() {
                        if let Some(child) = node.child(i) {
                            work_queue.push_back(child);
                        }
                    }
                    continue;
                }
            }
        } else {
            // Should not happen for C/C++, but handle gracefully
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    work_queue.push_back(child);
                }
            }
            continue;
        };

        // Skip empty kinds
        if kind.is_empty() {
            // Still need to process children
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    work_queue.push_back(child);
                }
            }
            continue;
        }

        // Handle imports separately
        if kind == "import" {
            extract_import_fact(node, source, file_path, sql, language);
            count += 1;
            // Still process children
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    work_queue.push_back(child);
                }
            }
            continue;
        }

        // Extract name for other symbols - C/C++ specific handling
        let name_node = if node.kind() == "function_definition" {
            node.child_by_field_name("declarator")
                .and_then(|d| extract_c_function_name(d))
        } else {
            node.child_by_field_name("name")
                .or_else(|| node.child_by_field_name("declarator"))
        };

        if let Some(name_node) = name_node {
            let name = String::from_utf8_lossy(&source[name_node.byte_range()]).to_string();

            // Only process if we have a valid name
            if !name.is_empty() && !name.contains('\n') {
                // If this is a function, update the context and extract additional data
                if kind == "function" {
                    context.current_function = Some(name.clone());

                    // Extract function facts for C/C++
                    extract_function_facts(node, source, file_path, &name, sql, language);


                    // Add to code_search
                    let signature = node
                        .utf8_text(source)
                        .unwrap_or("")
                        .lines()
                        .next()
                        .unwrap_or("")
                        .replace('\'', "''");

                    sql.push_str(&format!(
                        "INSERT OR REPLACE INTO code_search (path, name, signature) VALUES ('{}', '{}', '{}');\n",
                        file_path, name.replace('\'', "''"), signature
                    ));
                }

                // Extract documentation if present
                if let Some((doc_raw, doc_clean, keywords)) =
                    extract_doc_comment(node, source, language)
                {
                    let doc_summary = extract_summary(&doc_clean);
                    let _keywords_str = keywords.join(",");
                    let doc_length = doc_clean.len() as i32;
                    let has_examples =
                        doc_clean.contains("example") || doc_clean.contains("Example");
                    let has_params = doc_clean.contains("param") || doc_clean.contains("@param");
                    let line_number = (node.start_position().row + 1) as i32;

                    sql.push_str(&format!(
                        "INSERT OR REPLACE INTO documentation (file, symbol_name, symbol_type, line_number, doc_raw, doc_clean, doc_summary, keywords, doc_length, has_examples, has_params) VALUES ('{}', '{}', '{}', {}, '{}', '{}', '{}', ARRAY[{}], {}, {}, {});\n",
                        file_path,
                        name.replace('\'', "''"),
                        kind,
                        line_number,
                        doc_raw.replace('\'', "''"),
                        doc_clean.replace('\'', "''"),
                        doc_summary.replace('\'', "''"),
                        if keywords.is_empty() {
                            String::new()
                        } else {
                            keywords.iter().map(|k| format!("'{}'", k.replace('\'', "''"))).collect::<Vec<_>>().join(",")
                        },
                        doc_length,
                        has_examples,
                        has_params
                    ));
                }

                count += 1;
            }
        }

        // Add children to work queue for further processing
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                work_queue.push_back(child);
            }
        }
    }

    count
}

/// Extract function name from C/C++ declarator (handles nested declarators)
/// Uses iterative approach to avoid stack overflow with deeply nested declarators
fn extract_c_function_name(declarator: tree_sitter::Node) -> Option<tree_sitter::Node> {
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

        // Try first child as fallback
        if let Some(child) = current.child(0) {
            if child.kind() == "identifier" {
                return Some(child);
            }
        }

        // No more nested declarators to check
        break;
    }

    None
}

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

        // C/C++ call expressions
        (Language::C | Language::Cpp, "call_expression") => {
            // C/C++ call: func() or obj->method() or obj.method()
            if let Some(func_node) = node.child_by_field_name("function") {
                match func_node.kind() {
                    "identifier" => {
                        // Direct function call
                        let callee = func_node.utf8_text(source).unwrap_or("").to_string();
                        context.add_call(callee, "direct".to_string(), line_number);
                    }
                    "field_expression" => {
                        // Method call: obj.method() or obj->method()
                        if let Some(field_node) = func_node.child_by_field_name("field") {
                            let callee = field_node.utf8_text(source).unwrap_or("").to_string();
                            context.add_call(callee, "method".to_string(), line_number);
                        }
                    }
                    "qualified_identifier" | "scoped_identifier" => {
                        // Namespace or class qualified call: std::function() or Class::method()
                        let callee = func_node.utf8_text(source).unwrap_or("").to_string();
                        context.add_call(callee, "direct".to_string(), line_number);
                    }
                    _ => {
                        // Try to get the full text for other cases (function pointers, etc.)
                        let callee = func_node.utf8_text(source).unwrap_or("").to_string();
                        if !callee.is_empty() {
                            context.add_call(callee, "direct".to_string(), line_number);
                        }
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
    use languages::Language;

    let node = cursor.node();
    let mut count = 0;

    // Check if this is a symbol we want to extract
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

                // Detect patterns for LLM code intelligence (if we have access to pattern trackers)
                // Note: This is a local detection - we'll need to pass pattern trackers through
                // For now, we'll add a TODO comment
                // TODO: Add pattern detection here once we pass pattern trackers through

                let signature = node
                    .utf8_text(source)
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .replace('\'', "''");

                sql.push_str(&format!(
                    "INSERT OR REPLACE INTO code_search (path, name, signature) VALUES ('{}', '{}', '{}');\n",
                    file_path, name, signature
                ));
            }
            "type_alias" | "struct" | "trait" | "const" => {
                // Extract type vocabulary
                extract_type_definition(node, source, file_path, name, kind, sql, language);

            }
            "impl" => {}
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
FROM function_facts
UNION ALL
SELECT 
    'Types defined' as metric,
    COUNT(*) as value
FROM type_vocabulary
UNION ALL
SELECT 
    'Imports tracked' as metric,
    COUNT(*) as value
FROM import_facts
UNION ALL
SELECT 
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

    // Extract parameters using language spec
    let (takes_mut_self, takes_mut_params, parameter_count, parameter_list) =
        if language == Language::Solidity {
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
        } else if let Some(spec) = get_language_spec(language) {
            // Use the language spec's extract_params function
            let param_details = (spec.extract_params)(&node, source);
            let param_count = param_details.len();

            // Check for Rust-specific mut patterns
            let (has_mut_self, has_mut_params) = if language == Language::Rust {
                if let Some(params) = params_node {
                    let params_text = params.utf8_text(source).unwrap_or("");
                    let has_mut_self = params_text.contains("&mut self");
                    let has_mut_params =
                        params_text.contains("mut ") && !params_text.contains("&mut self");
                    (has_mut_self, has_mut_params)
                } else {
                    (false, false)
                }
            } else {
                (false, false)
            };

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

    // Extract return type using language spec
    let (returns_result, returns_option, return_type) =
        if let Some(spec) = get_language_spec(language) {
            let ret_type_opt = (spec.extract_return_type)(&node, source);
            if let Some(ret_text) = ret_type_opt {
                let ret_clean = ret_text.replace('\'', "''");
                match language {
                    Language::Rust => (
                        ret_text.contains("Result"),
                        ret_text.contains("Option"),
                        ret_clean,
                    ),
                    Language::Go => (ret_text.contains("error"), false, ret_clean),
                    _ => (false, false, ret_clean),
                }
            } else {
                (false, false, String::new())
            }
        } else {
            (false, false, String::new())
        };

    // Extract generics using language spec
    let generic_count = if let Some(spec) = get_language_spec(language) {
        if let Some(generics_text) = (spec.extract_generics)(&node, source) {
            // For Rust, count the type parameters and lifetimes
            if language == Language::Rust {
                // Parse the generics text to count items
                // This is a simple approximation - count commas + 1
                generics_text.matches(',').count() + 1
            } else {
                // For other languages, just check if generics exist
                if generics_text.is_empty() {
                    0
                } else {
                    1
                }
            }
        } else {
            0
        }
    } else {
        0
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
        Language::Cairo => "pub",   // Cairo defaults to public
        Language::C => "pub",       // C functions in headers are public
        Language::Cpp => "private", // C++ defaults to private
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

    if let Some(spec) = get_language_spec(language) {
        let (imported_item, imported_from, is_external) =
            (spec.extract_import_details)(&node, source);

        if !imported_item.is_empty() {
            sql.push_str(&format!(
                "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, 'import');\n",
                escape_sql(file_path),
                escape_sql(&imported_item),
                escape_sql(&imported_from),
                is_external
            ));
        }
    } else if language == Language::Cairo {
        // Parse Cairo use statements (Cairo doesn't use tree-sitter so handle separately)
        let import_text = node.utf8_text(source).unwrap_or("");
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
// DATABASE SCHEMA MODULE
// ============================================================================
pub(crate) mod schema {
    /// Generate DuckDB schema for code intelligence storage
    pub fn generate_schema() -> &'static str {
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
CREATE INDEX IF NOT EXISTS idx_import_facts_external ON import_facts(is_external);
CREATE INDEX IF NOT EXISTS idx_documentation_symbol ON documentation(symbol_name);
CREATE INDEX IF NOT EXISTS idx_documentation_type ON documentation(symbol_type);
"#
    }
}

// ============================================================================
// REMOVED: Pattern detection moved to ask command
// ============================================================================
// Pattern detection has been moved to src/commands/ask/patterns.rs
// The ask command now discovers patterns from the extracted data





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
        C,
        Cpp,
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
                Some("c") | Some("h") => Language::C,
                Some("cpp") | Some("cc") | Some("cxx") | Some("hpp") | Some("hxx") => Language::Cpp,
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
                Language::C => Some(patina_metal::Metal::C),
                Language::Cpp => Some(patina_metal::Metal::Cpp),
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
// LANGUAGE-SPECIFIC MODULES
// ============================================================================
// Each language gets its own module with complete extraction logic

// ============================================================================
// RUST LANGUAGE MODULE
// ============================================================================
mod rust_lang {
    use super::*;
    use tree_sitter::Node;
    
    pub struct RustExtractor;
    
    impl RustExtractor {
        pub fn is_doc_comment(text: &str) -> bool {
            text.starts_with("///") || text.starts_with("//!")
        }
        
        pub fn parse_visibility(node: &Node, _name: &str, _source: &[u8]) -> bool {
            node.children(&mut node.walk())
                .any(|child| child.kind() == "visibility_modifier")
        }
        
        pub fn has_async(node: &Node, _source: &[u8]) -> bool {
            node.children(&mut node.walk())
                .any(|child| child.kind() == "async")
        }
        
        pub fn has_unsafe(node: &Node, _source: &[u8]) -> bool {
            node.children(&mut node.walk())
                .any(|child| child.kind() == "unsafe")
        }
        
        pub fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
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
        }
        
        pub fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
            node.child_by_field_name("return_type")
                .and_then(|rt| rt.utf8_text(source).ok())
                .map(|s| s.trim_start_matches("->").trim().to_string())
        }
        
        pub fn extract_generics(node: &Node, source: &[u8]) -> Option<String> {
            node.child_by_field_name("type_parameters")
                .and_then(|tp| tp.utf8_text(source).ok())
                .map(String::from)
        }
        
        pub fn get_symbol_kind(node_kind: &str) -> &'static str {
            match node_kind {
                "function_item" => "function",
                "struct_item" => "struct",
                "trait_item" => "trait",
                "impl_item" => "impl",
                "type_alias" => "type_alias",
                "const_item" => "const",
                "use_declaration" => "import",
                _ => "unknown",
            }
        }
        
        pub fn clean_doc_comment(raw: &str) -> String {
            raw.lines()
                .map(|line| {
                    line.trim_start()
                        .strip_prefix("///")
                        .or_else(|| line.strip_prefix("//!"))
                        .unwrap_or(line)
                        .trim()
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
        
        pub fn extract_import_details(node: &Node, source: &[u8]) -> (String, String, bool) {
            let import_text = node.utf8_text(source).unwrap_or("");
            let import_clean = import_text.trim_start_matches("use ").trim_end_matches(';');
            
            let is_external = !import_clean.starts_with("crate::")
                && !import_clean.starts_with("super::")
                && !import_clean.starts_with("self::");
            
            let imported_item = import_clean.split("::").last().unwrap_or(import_clean);
            
            let imported_from = if import_clean.contains("::") {
                import_clean
                    .rsplit_once("::")
                    .map(|(from, _)| from)
                    .unwrap_or(import_clean)
            } else {
                import_clean
            };
            
            (imported_item.to_string(), imported_from.to_string(), is_external)
        }
    }
}

// ============================================================================
// GO LANGUAGE MODULE
// ============================================================================
mod go_lang {
    use super::*;
    use tree_sitter::Node;
    
    pub struct GoExtractor;
    
    impl GoExtractor {
        pub fn is_doc_comment(text: &str) -> bool {
            // Go uses // for doc comments (before declarations)
            text.starts_with("//")
        }
        
        pub fn parse_visibility(_node: &Node, name: &str, _source: &[u8]) -> bool {
            // In Go, uppercase first letter = public
            name.chars().next().is_some_and(|c| c.is_uppercase())
        }
        
        pub fn has_async(_node: &Node, _source: &[u8]) -> bool {
            // Go doesn't have async keyword, uses goroutines
            false
        }
        
        pub fn has_unsafe(_node: &Node, _source: &[u8]) -> bool {
            // Go doesn't have unsafe keyword in function declarations
            false
        }
        
        pub fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
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
        }
        
        pub fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
            node.child_by_field_name("result")
                .and_then(|r| r.utf8_text(source).ok())
                .map(String::from)
        }
        
        pub fn extract_generics(node: &Node, source: &[u8]) -> Option<String> {
            // Go uses type parameters (generics added in Go 1.18)
            node.child_by_field_name("type_parameters")
                .and_then(|tp| tp.utf8_text(source).ok())
                .map(String::from)
        }
        
        pub fn get_symbol_kind(node_kind: &str) -> &'static str {
            match node_kind {
                "function_declaration" | "method_declaration" => "function",
                "type_declaration" => "type_alias",
                "type_spec" => "type_alias",
                "struct_type" => "struct",
                "interface_type" => "trait",
                "const_declaration" | "const_spec" => "const",
                "import_declaration" | "import_spec" => "import",
                _ => "unknown",
            }
        }
        
        pub fn get_symbol_kind_complex(node: &Node, source: &[u8]) -> Option<&'static str> {
            // For type declarations, check what's being declared
            if node.kind() == "type_spec" {
                if let Some(type_node) = node.child_by_field_name("type") {
                    return Some(match type_node.kind() {
                        "struct_type" => "struct",
                        "interface_type" => "trait",
                        _ => "type_alias",
                    });
                }
            }
            None
        }
        
        pub fn clean_doc_comment(raw: &str) -> String {
            raw.lines()
                .map(|line| line.trim_start().strip_prefix("//").unwrap_or(line).trim())
                .collect::<Vec<_>>()
                .join(" ")
        }
        
        pub fn extract_import_details(node: &Node, source: &[u8]) -> (String, String, bool) {
            let import_text = node.utf8_text(source).unwrap_or("");
            let import_clean = import_text.trim_start_matches("import ").trim();
            
            // Handle both single and grouped imports
            let (imported_item, imported_from) = if import_clean.starts_with('"') {
                // Single import: import "fmt"
                let pkg = import_clean.trim_matches('"');
                let item = pkg.split('/').last().unwrap_or(pkg);
                (item.to_string(), pkg.to_string())
            } else {
                // Aliased import: import alias "package"
                let parts: Vec<&str> = import_clean.split_whitespace().collect();
                if parts.len() == 2 {
                    (parts[0].to_string(), parts[1].trim_matches('"').to_string())
                } else {
                    ("".to_string(), import_clean.to_string())
                }
            };
            
            // Standard library packages don't have dots or slashes at the start
            let is_external = imported_from.contains('.')
                || imported_from.starts_with("github.com")
                || imported_from.starts_with("golang.org");
            
            (imported_item, imported_from, is_external)
        }
    }
}
// This file contains all the language-specific modules that will be appended to refactor-code.rs

// ============================================================================
// PYTHON LANGUAGE MODULE
// ============================================================================
mod python_lang {
    use super::*;
    use tree_sitter::Node;
    
    pub struct PythonExtractor;
    
    impl PythonExtractor {
        pub fn is_doc_comment(text: &str) -> bool {
            // Python uses triple quotes for docstrings
            text.starts_with("\"\"\"") || text.starts_with("'''")
        }
        
        pub fn parse_visibility(_node: &Node, name: &str, _source: &[u8]) -> bool {
            // Python convention: underscore prefix = private
            !name.starts_with('_')
        }
        
        pub fn has_async(node: &Node, source: &[u8]) -> bool {
            // Check for async def
            if let Some(parent) = node.parent() {
                if parent.kind() == "decorated_definition" {
                    // Check decorators for async patterns
                    return parent.utf8_text(source).unwrap_or("").contains("async def");
                }
            }
            node.utf8_text(source).unwrap_or("").starts_with("async def")
        }
        
        pub fn has_unsafe(_node: &Node, _source: &[u8]) -> bool {
            // Python doesn't have unsafe
            false
        }
        
        pub fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
            if let Some(params_node) = node.child_by_field_name("parameters") {
                let mut params = Vec::new();
                let mut cursor = params_node.walk();
                for child in params_node.children(&mut cursor) {
                    match child.kind() {
                        "identifier" | "typed_parameter" | "default_parameter" | "list_splat_pattern" | "dictionary_splat_pattern" => {
                            if let Ok(param_text) = child.utf8_text(source) {
                                if param_text != "self" && param_text != "cls" {
                                    params.push(param_text.to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
                params
            } else {
                Vec::new()
            }
        }
        
        pub fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
            // Python 3 type hints: -> ReturnType
            node.child_by_field_name("return_type")
                .and_then(|rt| rt.utf8_text(source).ok())
                .map(|s| s.trim_start_matches("->").trim().to_string())
        }
        
        pub fn extract_generics(_node: &Node, _source: &[u8]) -> Option<String> {
            // Python doesn't have generics in function declarations
            // (though it has type parameters in type hints)
            None
        }
        
        pub fn get_symbol_kind(node_kind: &str) -> &'static str {
            match node_kind {
                "function_definition" => "function",
                "class_definition" => "struct",
                "decorated_definition" => "function", // Usually a function with decorators
                "import_statement" | "import_from_statement" => "import",
                _ => "unknown",
            }
        }
        
        pub fn get_symbol_kind_complex(node: &Node, _source: &[u8]) -> Option<&'static str> {
            // Check if decorated_definition contains a function or class
            if node.kind() == "decorated_definition" {
                if let Some(def) = node.child_by_field_name("definition") {
                    return Some(match def.kind() {
                        "function_definition" => "function",
                        "class_definition" => "struct",
                        _ => "unknown",
                    });
                }
            }
            None
        }
        
        pub fn clean_doc_comment(raw: &str) -> String {
            raw.trim_matches('"')
                .trim_matches('\'')
                .lines()
                .map(|l| l.trim())
                .collect::<Vec<_>>()
                .join(" ")
        }
        
        pub fn extract_import_details(node: &Node, source: &[u8]) -> (String, String, bool) {
            let import_text = node.utf8_text(source).unwrap_or("");
            
            let (imported_item, imported_from) = if import_text.starts_with("from ") {
                // from module import item
                let parts: Vec<&str> = import_text.split(" import ").collect();
                if parts.len() == 2 {
                    let module = parts[0].trim_start_matches("from ").trim();
                    let items = parts[1].trim();
                    (items.to_string(), module.to_string())
                } else {
                    ("".to_string(), import_text.to_string())
                }
            } else {
                // import module
                let module = import_text.trim_start_matches("import ").trim();
                (module.to_string(), module.to_string())
            };
            
            // Standard library vs external (heuristic)
            let is_external = !imported_from.starts_with('.')
                && !["sys", "os", "io", "re", "math", "json", "time", "datetime", "collections", "itertools"]
                    .contains(&imported_from.split('.').next().unwrap_or(""));
            
            (imported_item, imported_from, is_external)
        }
    }
}

// ============================================================================
// SOLIDITY LANGUAGE MODULE
// ============================================================================
mod solidity_lang {
    use super::*;
    use tree_sitter::Node;
    
    pub struct SolidityExtractor;
    
    impl SolidityExtractor {
        pub fn is_doc_comment(text: &str) -> bool {
            // Solidity uses /// for NatSpec comments
            text.starts_with("///") || text.starts_with("/**")
        }
        
        pub fn parse_visibility(node: &Node, _name: &str, source: &[u8]) -> bool {
            // Check for visibility modifiers
            let text = node.utf8_text(source).unwrap_or("");
            text.contains("public") || text.contains("external")
        }
        
        pub fn has_async(_node: &Node, _source: &[u8]) -> bool {
            // Solidity doesn't have async
            false
        }
        
        pub fn has_unsafe(node: &Node, source: &[u8]) -> bool {
            // Consider assembly blocks as unsafe
            node.utf8_text(source).unwrap_or("").contains("assembly")
        }
        
        pub fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
            // Solidity parameters are direct children
            let mut params = Vec::new();
            for child in node.children(&mut node.walk()) {
                if child.kind() == "parameter" {
                    if let Ok(param_text) = child.utf8_text(source) {
                        params.push(param_text.to_string());
                    }
                }
            }
            params
        }
        
        pub fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
            // Look for returns clause
            for child in node.children(&mut node.walk()) {
                if child.kind() == "return_type_definition" {
                    return child.utf8_text(source).ok().map(String::from);
                }
            }
            None
        }
        
        pub fn extract_generics(_node: &Node, _source: &[u8]) -> Option<String> {
            // Solidity doesn't have generics
            None
        }
        
        pub fn get_symbol_kind(node_kind: &str) -> &'static str {
            match node_kind {
                "function_definition" => "function",
                "modifier_definition" => "function",
                "contract_declaration" => "struct",
                "struct_declaration" => "struct",
                "enum_declaration" => "struct",
                "interface_declaration" => "trait",
                "event_definition" => "function",
                "import_directive" => "import",
                _ => "unknown",
            }
        }
        
        pub fn get_symbol_kind_complex(_node: &Node, _source: &[u8]) -> Option<&'static str> {
            None
        }
        
        pub fn clean_doc_comment(raw: &str) -> String {
            raw.lines()
                .map(|line| {
                    line.trim_start()
                        .strip_prefix("///")
                        .or_else(|| line.strip_prefix("/**"))
                        .or_else(|| line.strip_prefix("*/"))
                        .unwrap_or(line)
                        .trim()
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
        
        pub fn extract_import_details(node: &Node, source: &[u8]) -> (String, String, bool) {
            let import_text = node.utf8_text(source).unwrap_or("");
            let import_clean = import_text.trim_start_matches("import ").trim_end_matches(';');
            
            let (imported_item, imported_from) = if import_clean.contains(" as ") {
                let parts: Vec<&str> = import_clean.split(" as ").collect();
                (parts[1].trim_matches('"').to_string(), parts[0].trim_matches('"').to_string())
            } else {
                let path = import_clean.trim_matches('"');
                let item = path.split('/').last().unwrap_or(path);
                (item.to_string(), path.to_string())
            };
            
            // Most Solidity imports are relative or from node_modules
            let is_external = imported_from.starts_with('@') || !imported_from.starts_with('.');
            
            (imported_item, imported_from, is_external)
        }
    }
}

// ============================================================================
// C/C++ LANGUAGE MODULE
// ============================================================================
mod c_cpp_lang {
    use super::*;
    use tree_sitter::Node;
    
    pub struct CCppExtractor;
    
    impl CCppExtractor {
        pub fn is_doc_comment(text: &str) -> bool {
            // C/C++ use /** */ or /// for doc comments
            text.starts_with("/**") || text.starts_with("///") || text.starts_with("//!")
        }
        
        pub fn parse_visibility(node: &Node, _name: &str, source: &[u8]) -> bool {
            // Check if static (which means file-local, so private)
            let text = node.utf8_text(source).unwrap_or("");
            !text.contains("static")
        }
        
        pub fn has_async(_node: &Node, _source: &[u8]) -> bool {
            // C/C++ don't have async keyword
            false
        }
        
        pub fn has_unsafe(_node: &Node, _source: &[u8]) -> bool {
            // All C/C++ could be considered unsafe from Rust perspective
            false
        }
        
        pub fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
            if let Some(params_node) = node.child_by_field_name("declarator") {
                if let Some(param_list) = params_node.child_by_field_name("parameters") {
                    let mut params = Vec::new();
                    for child in param_list.children(&mut param_list.walk()) {
                        if child.kind() == "parameter_declaration" {
                            if let Ok(param_text) = child.utf8_text(source) {
                                params.push(param_text.to_string());
                            }
                        }
                    }
                    return params;
                }
            }
            Vec::new()
        }
        
        pub fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
            node.child_by_field_name("type")
                .and_then(|t| t.utf8_text(source).ok())
                .map(String::from)
        }
        
        pub fn extract_generics(node: &Node, source: &[u8]) -> Option<String> {
            // C++ templates
            if let Some(template) = node.child_by_field_name("template_parameters") {
                return template.utf8_text(source).ok().map(String::from);
            }
            None
        }
        
        pub fn get_symbol_kind(node_kind: &str) -> &'static str {
            match node_kind {
                "function_definition" => "function",
                "struct_specifier" => "struct",
                "class_specifier" => "struct",
                "enum_specifier" => "struct",
                "type_definition" => "type_alias",
                "preproc_include" => "import",
                _ => "unknown",
            }
        }
        
        pub fn get_symbol_kind_complex(_node: &Node, _source: &[u8]) -> Option<&'static str> {
            None
        }
        
        pub fn clean_doc_comment(raw: &str) -> String {
            raw.lines()
                .map(|line| {
                    line.trim_start()
                        .strip_prefix("///")
                        .or_else(|| line.strip_prefix("/**"))
                        .or_else(|| line.strip_prefix("*/"))
                        .or_else(|| line.strip_prefix("*"))
                        .unwrap_or(line)
                        .trim()
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
        
        pub fn extract_import_details(node: &Node, source: &[u8]) -> (String, String, bool) {
            let import_text = node.utf8_text(source).unwrap_or("");
            let import_clean = import_text
                .trim_start_matches("#include")
                .trim()
                .trim_matches('<')
                .trim_matches('>') 
                .trim_matches('"');
            
            let is_system = import_text.contains('<');
            
            (import_clean.to_string(), import_clean.to_string(), is_system)
        }
        
        pub fn extract_c_function_name(node: Node, source: &[u8]) -> Option<Node> {
            // Iteratively handle pointer declarators for C functions
            let mut current = node;
            let mut depth = 0;
            const MAX_DEPTH: usize = 10;
            
            while depth < MAX_DEPTH {
                match current.kind() {
                    "pointer_declarator" => {
                        if let Some(child) = current.child(1) {
                            current = child;
                            depth += 1;
                        } else {
                            break;
                        }
                    }
                    "function_declarator" => {
                        if let Some(declarator) = current.child_by_field_name("declarator") {
                            current = declarator;
                            depth += 1;
                        } else {
                            break;
                        }
                    }
                    "identifier" => return Some(current),
                    _ => break,
                }
            }
            
            if current.kind() == "identifier" {
                Some(current)
            } else {
                None
            }
        }
    }
}

// ============================================================================
// TYPESCRIPT/JAVASCRIPT LANGUAGE MODULE
// ============================================================================
mod typescript_lang {
    use super::*;
    use tree_sitter::Node;
    
    pub struct TypeScriptExtractor;
    
    impl TypeScriptExtractor {
        pub fn is_doc_comment(text: &str) -> bool {
            // JS/TS use /** */ for JSDoc comments
            text.starts_with("/**") || text.starts_with("//")
        }
        
        pub fn parse_visibility(node: &Node, _name: &str, source: &[u8]) -> bool {
            // Check for export keyword
            let text = node.utf8_text(source).unwrap_or("");
            text.contains("export")
        }
        
        pub fn has_async(node: &Node, source: &[u8]) -> bool {
            // Check for async keyword
            node.utf8_text(source).unwrap_or("").contains("async")
        }
        
        pub fn has_unsafe(_node: &Node, _source: &[u8]) -> bool {
            // JS/TS don't have unsafe
            false
        }
        
        pub fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
            if let Some(params_node) = node.child_by_field_name("parameters") {
                let mut params = Vec::new();
                for child in params_node.children(&mut params_node.walk()) {
                    match child.kind() {
                        "required_parameter" | "optional_parameter" | "rest_parameter" => {
                            if let Ok(param_text) = child.utf8_text(source) {
                                params.push(param_text.to_string());
                            }
                        }
                        "identifier" => {
                            if let Ok(param_text) = child.utf8_text(source) {
                                params.push(param_text.to_string());
                            }
                        }
                        _ => {}
                    }
                }
                params
            } else {
                Vec::new()
            }
        }
        
        pub fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
            // TypeScript return type annotation
            node.child_by_field_name("return_type")
                .and_then(|rt| rt.utf8_text(source).ok())
                .map(|s| s.trim_start_matches(':').trim().to_string())
        }
        
        pub fn extract_generics(node: &Node, source: &[u8]) -> Option<String> {
            // TypeScript generics <T, U>
            node.child_by_field_name("type_parameters")
                .and_then(|tp| tp.utf8_text(source).ok())
                .map(String::from)
        }
        
        pub fn get_symbol_kind(node_kind: &str) -> &'static str {
            match node_kind {
                "function_declaration" | "function_expression" | "arrow_function" | "method_definition" => "function",
                "class_declaration" => "struct",
                "interface_declaration" => "trait",
                "type_alias_declaration" => "type_alias",
                "enum_declaration" => "struct",
                "import_statement" => "import",
                "variable_declarator" => "const",
                _ => "unknown",
            }
        }
        
        pub fn get_symbol_kind_complex(node: &Node, source: &[u8]) -> Option<&'static str> {
            // Check if variable contains a function
            if node.kind() == "variable_declarator" {
                if let Some(init) = node.child_by_field_name("value") {
                    return Some(match init.kind() {
                        "arrow_function" | "function_expression" => "function",
                        _ => "const",
                    });
                }
            }
            None
        }
        
        pub fn clean_doc_comment(raw: &str) -> String {
            raw.lines()
                .map(|line| {
                    line.trim_start()
                        .strip_prefix("/**")
                        .or_else(|| line.strip_prefix("*/"))
                        .or_else(|| line.strip_prefix("*"))
                        .or_else(|| line.strip_prefix("//"))
                        .unwrap_or(line)
                        .trim()
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
        
        pub fn extract_import_details(node: &Node, source: &[u8]) -> (String, String, bool) {
            let import_text = node.utf8_text(source).unwrap_or("");
            
            let (imported_item, imported_from) = if import_text.contains(" from ") {
                let parts: Vec<&str> = import_text.split(" from ").collect();
                if parts.len() == 2 {
                    let items = parts[0]
                        .trim_start_matches("import ")
                        .trim_matches('{')
                        .trim_matches('}')
                        .trim();
                    let module = parts[1].trim_matches('\'').trim_matches('"').trim_end_matches(';');
                    (items.to_string(), module.to_string())
                } else {
                    ("".to_string(), import_text.to_string())
                }
            } else {
                // require() or import 'module'
                let module = import_text
                    .trim_start_matches("import ")
                    .trim_matches('\'')
                    .trim_matches('"')
                    .trim_end_matches(';');
                (module.to_string(), module.to_string())
            };
            
            // Node modules vs relative imports
            let is_external = !imported_from.starts_with('.') && !imported_from.starts_with('/');
            
            (imported_item, imported_from, is_external)
        }
    }
}