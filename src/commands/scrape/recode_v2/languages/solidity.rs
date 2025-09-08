// ============================================================================
// SOLIDITY LANGUAGE IMPLEMENTATION
// ============================================================================
//! Solidity-specific code extraction and analysis.
//!
//! Handles Solidity's unique features:
//! - Smart contract structure
//! - Visibility modifiers (public, private, internal, external)
//! - State mutability (pure, view, payable)
//! - Events and modifiers
//! - Inheritance and interfaces
//! - Unchecked blocks (similar to unsafe)
//! - Library and contract declarations

use crate::commands::scrape::recode_v2::types::{solidity_nodes::*, SymbolKind};
use crate::commands::scrape::recode_v2::LanguageSpec;

/// Solidity language specification
pub static SPEC: LanguageSpec = LanguageSpec {
    is_doc_comment: |text| text.starts_with("///") || text.starts_with("/**"),

    parse_visibility: |node, _name, source| {
        let text = node.utf8_text(source).unwrap_or("");
        // Solidity has explicit visibility modifiers
        // private and internal are non-public
        !text.contains("private") && !text.contains("internal")
    },

    has_async: |_node, _source| {
        // Solidity doesn't have async/await
        false
    },

    has_unsafe: |node, source| {
        // Solidity has "unchecked" blocks which are similar to unsafe
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
        node.child_by_field_name("return_parameters")
            .and_then(|rp| rp.utf8_text(source).ok())
            .map(String::from)
    },

    extract_generics: |_node, _source| {
        // Solidity doesn't have generics
        None
    },

    get_symbol_kind: |node_kind| match node_kind {
        FUNCTION_DEFINITION => SymbolKind::Function,
        MODIFIER_DEFINITION => SymbolKind::Function,
        EVENT_DEFINITION => SymbolKind::Function,
        CONTRACT_DECLARATION => SymbolKind::Struct,
        STRUCT_DECLARATION => SymbolKind::Struct,
        INTERFACE_DECLARATION => SymbolKind::Trait,
        LIBRARY_DECLARATION => SymbolKind::Impl,
        IMPORT_DIRECTIVE => SymbolKind::Import,
        STATE_VARIABLE_DECLARATION => SymbolKind::Const,
        _ => SymbolKind::Unknown,
    },

    get_symbol_kind_complex: |_node, _source| {
        // Solidity's AST is generally straightforward with node kinds
        None
    },

    clean_doc_comment: |raw| {
        if raw.starts_with("/**") {
            raw.strip_prefix("/**")
                .and_then(|s| s.strip_suffix("*/"))
                .map(|s| {
                    s.lines()
                        .map(|line| line.trim().strip_prefix('*').unwrap_or(line).trim())
                        .filter(|line| !line.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_else(|| raw.to_string())
        } else {
            raw.lines()
                .map(|line| line.trim_start().strip_prefix("//").unwrap_or(line).trim())
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        }
    },

    extract_import_details: |node, source| {
        let import_text = node.utf8_text(source).unwrap_or("");
        if let Some(path_match) = import_text.split('"').nth(1) {
            // External imports often start with '@' (npm packages) or 'http' (URLs)
            let is_external = path_match.starts_with('@') || path_match.starts_with("http");
            (path_match.to_string(), path_match.to_string(), is_external)
        } else {
            (String::new(), String::new(), false)
        }
    },

    extract_calls: Some(|node, source, context| {
        let line_number = (node.start_position().row + 1) as i32;

        match node.kind() {
            CALL_EXPRESSION => {
                // Regular function calls
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        context.add_call(callee.to_string(), "direct".to_string(), line_number);
                    }
                }
            }
            "member_expression" => {
                // Handle contract.method() calls
                if let Some(parent) = node.parent() {
                    if parent.kind() == "call_expression" {
                        if let Some(property) = node.child_by_field_name("property") {
                            if let Ok(callee) = property.utf8_text(source) {
                                context.add_call(
                                    callee.to_string(),
                                    "method".to_string(),
                                    line_number,
                                );
                            }
                        }
                    }
                }
            }
            "new_expression" => {
                // Handle "new Type[]" array constructors - unique to Solidity!
                if let Ok(text) = node.utf8_text(source) {
                    context.add_call(text.to_string(), "constructor".to_string(), line_number);
                }
            }
            "emit_statement" => {
                // Solidity events - other languages don't have this!
                if let Some(event_node) = node.child_by_field_name("name") {
                    if let Ok(event_name) = event_node.utf8_text(source) {
                        context.add_call(
                            format!("emit {}", event_name),
                            "event".to_string(),
                            line_number,
                        );
                    }
                }
            }
            _ => {}
        }
    }),
};
