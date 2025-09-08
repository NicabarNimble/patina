// ============================================================================
// PYTHON LANGUAGE IMPLEMENTATION
// ============================================================================
//! Python-specific code extraction and analysis.
//!
//! Handles Python's unique features:
//! - Underscore-based visibility conventions
//! - Docstrings (triple quotes)
//! - Duck typing and dynamic nature
//! - Async/await support
//! - Decorators and class definitions
//! - Import system (from/import statements)

use crate::commands::scrape::recode_v2::types::{python_nodes::*, SymbolKind};
use crate::commands::scrape::recode_v2::LanguageSpec;

/// Python language specification
pub static SPEC: LanguageSpec = LanguageSpec {
    is_doc_comment: |text| text.starts_with("\"\"\"") || text.starts_with("'''"),

    parse_visibility: |_node, name, _source| {
        // Python uses underscore convention for visibility
        !name.starts_with('_')
    },

    has_async: |node, source| {
        node.kind() == "async_function_definition"
            || node.utf8_text(source).unwrap_or("").starts_with("async ")
    },

    has_unsafe: |_node, _source| {
        // Python doesn't have unsafe keyword
        false
    },

    extract_params: |node, source| {
        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut params = Vec::new();
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
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
        node.child_by_field_name("return_type")
            .and_then(|rt| rt.utf8_text(source).ok())
            .map(|s| s.trim_start_matches("->").trim().to_string())
    },

    extract_generics: |_node, _source| {
        // Python doesn't have traditional generics (typing is handled at runtime)
        None
    },

    get_symbol_kind: |node_kind| match node_kind {
        FUNCTION_DEFINITION | "async_function_definition" => SymbolKind::Function,
        CLASS_DEFINITION => SymbolKind::Struct,
        IMPORT_STATEMENT | IMPORT_FROM_STATEMENT => SymbolKind::Import,
        _ => SymbolKind::Unknown,
    },

    get_symbol_kind_complex: |node, _source| {
        if node.kind() == DECORATED_DEFINITION {
            if node.child_by_field_name("definition").is_some_and(|n| {
                n.kind() == FUNCTION_DEFINITION || n.kind() == "async_function_definition"
            }) {
                Some(SymbolKind::Function)
            } else if node
                .child_by_field_name("definition")
                .is_some_and(|n| n.kind() == CLASS_DEFINITION)
            {
                Some(SymbolKind::Struct)
            } else {
                None
            }
        } else {
            None
        }
    },

    clean_doc_comment: |raw| {
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

        (
            import_clean.to_string(),
            import_clean.to_string(),
            is_external,
        )
    },

    extract_calls: Some(|node, source, context| {
        let line_number = (node.start_position().row + 1) as i32;

        match node.kind() {
            CALL => {
                // Python function/method calls
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
            "decorator" => {
                // Python decorators are a special kind of call
                if let Some(decorator_node) = node.child(0) {
                    if decorator_node.kind() == "@" {
                        if let Some(name_node) = node.child(1) {
                            if let Ok(decorator_name) = name_node.utf8_text(source) {
                                context.add_call(
                                    format!("@{}", decorator_name),
                                    "decorator".to_string(),
                                    line_number,
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }),
};
