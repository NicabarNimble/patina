// ============================================================================
// RUST LANGUAGE IMPLEMENTATION
// ============================================================================
//! Rust-specific code extraction and analysis.
//!
//! Handles Rust's unique features:
//! - Ownership and borrowing patterns
//! - Trait implementations
//! - Async/await support
//! - Unsafe blocks
//! - Macro usage

use crate::commands::scrape::recode_v2::LanguageSpec;
use tree_sitter::Node;

/// Rust language specification
pub static SPEC: LanguageSpec = LanguageSpec {
    is_doc_comment: |text| {
        text.starts_with("///") || text.starts_with("//!")
    },

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

    get_symbol_kind_complex: |_node, _source| None,

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

        (
            imported_item.to_string(),
            imported_from.to_string(),
            is_external,
        )
    },
};