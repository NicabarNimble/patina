// ============================================================================
// C LANGUAGE IMPLEMENTATION
// ============================================================================
//! C-specific code extraction and analysis.
//!
//! Handles C's features:
//! - Header files vs implementation files
//! - Preprocessor directives
//! - Function pointers
//! - Structs, unions, and enums
//! - No built-in visibility (header exposure = public)

use crate::commands::scrape::recode_v2::types::{c_nodes::*, SymbolKind};
use crate::commands::scrape::recode_v2::LanguageSpec;

/// C language specification
pub static SPEC: LanguageSpec = LanguageSpec {
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
        FUNCTION_DEFINITION => SymbolKind::Function,
        STRUCT_SPECIFIER => SymbolKind::Struct,
        UNION_SPECIFIER => SymbolKind::Struct,
        ENUM_SPECIFIER => SymbolKind::Enum,
        TYPE_DEFINITION => SymbolKind::TypeAlias,
        DECLARATION => SymbolKind::Const,
        PREPROC_INCLUDE => SymbolKind::Import,
        _ => SymbolKind::Unknown,
    },

    get_symbol_kind_complex: |_node, _source| None,

    clean_doc_comment: |raw| {
        raw.lines()
            .map(|line| {
                let trimmed = line.trim();
                let cleaned = trimmed
                    .strip_prefix("///")
                    .or_else(|| trimmed.strip_prefix("//!"))
                    .or_else(|| trimmed.strip_prefix("//"))
                    .or_else(|| trimmed.strip_prefix("/**"))
                    .or_else(|| trimmed.strip_prefix("/*"))
                    .or_else(|| trimmed.strip_prefix("*"))
                    .unwrap_or(trimmed);

                cleaned.strip_suffix("*/").unwrap_or(cleaned).trim()
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

    extract_calls: Some(|node, source, context| {
        let line_number = (node.start_position().row + 1) as i32;

        match node.kind() {
            CALL_EXPRESSION => {
                // C function calls
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        context.add_call(callee.to_string(), "direct".to_string(), line_number);
                    }
                }
            }
            "macro_invocation" => {
                // C preprocessor macros
                if let Some(macro_node) = node.child_by_field_name("macro") {
                    if let Ok(macro_name) = macro_node.utf8_text(source) {
                        context.add_call(macro_name.to_string(), "macro".to_string(), line_number);
                    }
                }
            }
            _ => {}
        }
    }),
};
