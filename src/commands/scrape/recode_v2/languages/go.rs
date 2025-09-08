// ============================================================================
// GO LANGUAGE IMPLEMENTATION
// ============================================================================
//! Go-specific code extraction and analysis.
//!
//! Handles Go's unique features:
//! - Exported vs unexported (capitalization-based visibility)
//! - Interfaces and struct embedding
//! - Goroutines and channels
//! - Multiple return values
//! - Package-level declarations

use crate::commands::scrape::recode_v2::types::{go_nodes::*, CallType, SymbolKind};
use crate::commands::scrape::recode_v2::LanguageSpec;

/// Go language specification
pub static SPEC: LanguageSpec = LanguageSpec {
    is_doc_comment: |text| {
        // Go uses // for doc comments
        text.starts_with("//")
    },

    parse_visibility: |_node, name, _source| {
        // Go uses capitalization for visibility
        name.chars().next().is_some_and(|c| c.is_uppercase())
    },

    has_async: |_node, _source| {
        // Go doesn't have async/await, it uses goroutines
        false
    },

    has_unsafe: |_node, _source| {
        // Go doesn't have an unsafe keyword
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
        // Go 1.18+ has generics via type parameters
        node.child_by_field_name("type_parameters")
            .and_then(|tp| tp.utf8_text(source).ok())
            .map(String::from)
    },

    get_symbol_kind: |node_kind| match node_kind {
        FUNCTION_DECLARATION => SymbolKind::Function,
        METHOD_DECLARATION => SymbolKind::Function,
        TYPE_DECLARATION => SymbolKind::TypeAlias,
        CONST_DECLARATION => SymbolKind::Const,
        VAR_DECLARATION => SymbolKind::Const,
        IMPORT_DECLARATION => SymbolKind::Import,
        _ => SymbolKind::Unknown,
    },

    get_symbol_kind_complex: |node, _source| {
        if node.kind() == TYPE_SPEC {
            if node
                .child_by_field_name("type")
                .is_some_and(|n| n.kind() == STRUCT_TYPE)
            {
                Some(SymbolKind::Struct)
            } else if node
                .child_by_field_name("type")
                .is_some_and(|n| n.kind() == INTERFACE_TYPE)
            {
                Some(SymbolKind::Trait)
            } else {
                Some(SymbolKind::TypeAlias)
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

    extract_calls: Some(|node, source, context| {
        let line_number = (node.start_position().row + 1) as i32;

        match node.kind() {
            CALL_EXPRESSION => {
                // Regular function calls and goroutines
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        let call_type = if callee.contains("go ") {
                            CallType::Goroutine
                        } else {
                            CallType::Direct
                        };
                        context.add_call(callee.replace("go ", ""), call_type, line_number);
                    }
                }
            }
            "selector_expression" => {
                // Go method calls are selector expressions followed by call_expression
                if let Some(parent) = node.parent() {
                    if parent.kind() == CALL_EXPRESSION {
                        if let Some(field_node) = node.child_by_field_name("field") {
                            if let Ok(callee) = field_node.utf8_text(source) {
                                context.add_call(callee.to_string(), CallType::Method, line_number);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }),
};
