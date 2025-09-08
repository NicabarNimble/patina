// ============================================================================
// C++ LANGUAGE IMPLEMENTATION
// ============================================================================
//! C++-specific code extraction and analysis.
//!
//! Handles C++'s features:
//! - Classes with access modifiers (public/private/protected)
//! - Templates and template specialization
//! - Namespaces
//! - Function overloading
//! - RAII and constructors/destructors
//! - Modern C++ features (auto, lambdas, etc.)

use crate::commands::scrape::recode_v2::types::{cpp_nodes::*, SymbolKind, CallType};
use crate::commands::scrape::recode_v2::LanguageSpec;

/// C++ language specification
pub static SPEC: LanguageSpec = LanguageSpec {
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
        FUNCTION_DEFINITION => SymbolKind::Function,
        CLASS_SPECIFIER => SymbolKind::Class,
        STRUCT_SPECIFIER => SymbolKind::Struct,
        "union_specifier" => SymbolKind::Struct,
        "enum_specifier" => SymbolKind::Enum,
        NAMESPACE_DEFINITION => SymbolKind::Module,
        TEMPLATE_DECLARATION => SymbolKind::Function,
        "type_alias_declaration" | USING_DECLARATION => SymbolKind::TypeAlias,
        DECLARATION => SymbolKind::Const,
        PREPROC_INCLUDE => SymbolKind::Import,
        _ => SymbolKind::Unknown,
    },

    get_symbol_kind_complex: |node, _source| {
        // Check if template_declaration contains a class/struct/function
        if node.kind() == "template_declaration" {
            if let Some(child) = node.named_child(1) {
                return match child.kind() {
                    "class_specifier" => Some(SymbolKind::Class),
                    "struct_specifier" => Some(SymbolKind::Struct),
                    "function_definition" => Some(SymbolKind::Function),
                    _ => None,
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

    extract_calls: Some(|node, source, context| {
        let line_number = (node.start_position().row + 1) as i32;

        match node.kind() {
            CALL_EXPRESSION => {
                // C++ function/method calls
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        context.add_call(callee.to_string(), CallType::Direct, line_number);
                    }
                }
            }
            "template_function" => {
                // C++ template instantiations
                if let Some(name_node) = node.child_by_field_name("name") {
                    if let Ok(template_name) = name_node.utf8_text(source) {
                        context.add_call(
                            template_name.to_string(),
                            CallType::Template,
                            line_number,
                        );
                    }
                }
            }
            "new_expression" => {
                // C++ new operator
                if let Some(type_node) = node.child_by_field_name("type") {
                    if let Ok(type_name) = type_node.utf8_text(source) {
                        context.add_call(
                            format!("new {}", type_name),
                            CallType::Constructor,
                            line_number,
                        );
                    }
                }
            }
            _ => {}
        }
    }),
};
