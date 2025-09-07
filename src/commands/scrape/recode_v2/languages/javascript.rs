// ============================================================================
// JAVASCRIPT LANGUAGE IMPLEMENTATION
// ============================================================================
//! JavaScript-specific code extraction and analysis.
//!
//! Handles JavaScript's unique features:
//! - Prototype-based inheritance
//! - Dynamic typing and duck typing
//! - Multiple function declaration styles (function, arrow, expression)
//! - ES6+ modules and CommonJS
//! - Async/await and promises
//! - Flexible parameter patterns (destructuring, rest)

use crate::commands::scrape::recode_v2::{LanguageSpec, ParseContext};

/// JavaScript language specification
pub static SPEC: LanguageSpec = LanguageSpec {
    is_doc_comment: |text| {
        text.starts_with("/**") || text.starts_with("///")
    },

    parse_visibility: |_node, _name, _source| {
        // JavaScript doesn't have built-in visibility modifiers
        // Everything is effectively public by default
        true
    },

    has_async: |node, source| {
        node.utf8_text(source).unwrap_or("").contains("async")
    },

    has_unsafe: |_node, _source| {
        // JavaScript doesn't have unsafe keyword
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
        // JavaScript doesn't have explicit return types
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
                        .map(|line| line.trim().strip_prefix('*').unwrap_or(line).trim())
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
    
    extract_calls: Some(|node, source, context| {
        let line_number = (node.start_position().row + 1) as i32;
        
        match node.kind() {
            "call_expression" => {
                // JavaScript function calls
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
            "new_expression" => {
                // JavaScript constructor calls (new Object())
                if let Some(constructor_node) = node.child_by_field_name("constructor") {
                    if let Ok(constructor) = constructor_node.utf8_text(source) {
                        context.add_call(
                            format!("new {}", constructor),
                            "constructor".to_string(),
                            line_number,
                        );
                    }
                }
            }
            _ => {}
        }
    }),
};