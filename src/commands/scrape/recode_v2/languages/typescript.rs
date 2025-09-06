// ============================================================================
// TYPESCRIPT LANGUAGE IMPLEMENTATION
// ============================================================================
//! TypeScript-specific code extraction and analysis.
//!
//! Handles TypeScript's unique features:
//! - Static typing with type annotations
//! - Access modifiers (public, private, protected)
//! - Interfaces and type aliases
//! - Generics and type parameters
//! - Enums and advanced type features
//! - Decorators and metadata
//! - JSX support (.tsx files)

use crate::commands::scrape::recode_v2::LanguageSpec;

/// TypeScript language specification
pub static SPEC: LanguageSpec = LanguageSpec {
    is_doc_comment: |text| {
        text.starts_with("/**") || text.starts_with("///")
    },

    parse_visibility: |node, _name, source| {
        let text = node.utf8_text(source).unwrap_or("");
        // TypeScript has explicit visibility modifiers
        !text.contains("private") && !text.contains("protected")
    },

    has_async: |node, source| {
        node.utf8_text(source).unwrap_or("").contains("async")
    },

    has_unsafe: |_node, _source| {
        // TypeScript doesn't have unsafe keyword
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
        node.child_by_field_name("return_type")
            .and_then(|rt| rt.utf8_text(source).ok())
            .map(|s| s.trim_start_matches(':').trim().to_string())
    },

    extract_generics: |node, source| {
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
};