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
        "function_declaration" => "function",
        "method_declaration" => "function",
        "const_declaration" => "const",
        "import_declaration" => "import",
        _ => "unknown",
    },
    
    get_symbol_kind_complex: |node, _source| {
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