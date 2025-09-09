// ============================================================================
// C LANGUAGE PROCESSOR V2 - STRUCT-BASED
// ============================================================================
//! C language processor that returns typed structs instead of SQL strings.
//!
//! This is the refactored version that:
//! - Returns ExtractedData with typed structs
//! - No SQL string generation
//! - Direct data extraction to domain types
//! - Uses iterative approach for nested declarators to avoid stack overflow
//!
//! Handles C's features:
//! - Header files vs implementation files
//! - Preprocessor directives  
//! - Function pointers and nested declarators
//! - Structs, unions, and enums
//! - No built-in visibility (header exposure = public)

use crate::commands::scrape::recode_v2::database::{
    CallEdge, CodeSymbol, FunctionFact, ImportFact, TypeFact,
};
use crate::commands::scrape::recode_v2::extracted_data::ExtractedData;
use crate::commands::scrape::recode_v2::types::{CallType, FilePath, SymbolKind};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// C language processor - returns typed structs
pub struct CProcessor;

impl CProcessor {
    /// Process a C file and extract all symbols to typed structs
    pub fn process_file(file_path: FilePath, content: &[u8]) -> Result<ExtractedData> {
        let mut data = ExtractedData::new();

        // Set up tree-sitter parser for C
        let mut parser = Parser::new();
        let metal = patina_metal::Metal::C;
        let language = metal
            .tree_sitter_language_for_ext("c")
            .ok_or_else(|| anyhow::anyhow!("No C parser available"))?;
        parser
            .set_language(&language)
            .context("Failed to set C language")?;

        // Parse the file
        let tree = parser
            .parse(content, None)
            .context("Failed to parse C file")?;

        // Walk the AST and extract symbols
        extract_c_symbols(
            &tree.root_node(),
            content,
            &file_path.to_string(),
            &mut data,
            None,
        );

        Ok(data)
    }
}

/// Recursively extract symbols from the C AST
fn extract_c_symbols(
    node: &Node,
    source: &[u8],
    file_path: &str,
    data: &mut ExtractedData,
    current_function: Option<String>,
) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = extract_function_name(node, source) {
                process_c_function(node, source, file_path, &name, data);

                // Process function body with updated context
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    extract_c_symbols(&child, source, file_path, data, Some(name.clone()));
                }
                return; // Don't recurse again
            }
        }
        "struct_specifier" | "union_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    let kind = if node.kind() == "struct_specifier" {
                        SymbolKind::Struct
                    } else {
                        SymbolKind::Union
                    };
                    process_c_type(node, source, file_path, name, kind, data);
                }
            }
        }
        "enum_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    process_c_type(node, source, file_path, name, SymbolKind::Enum, data);
                }
            }
        }
        "type_definition" => {
            // typedef handling
            if let Some(declarator) = node.child_by_field_name("declarator") {
                if let Some(name) = extract_typedef_name(&declarator, source) {
                    process_c_typedef(node, source, file_path, &name, data);
                }
            }
        }
        "preproc_include" => {
            process_c_include(node, source, file_path, data);
        }
        "call_expression" => {
            // Track function calls for call graph
            if let Some(ref caller) = current_function {
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        data.add_call_edge(CallEdge {
                            caller: caller.clone(),
                            callee: callee.to_string(),
                            file: file_path.to_string(),
                            call_type: CallType::Direct.to_string(),
                            line_number: (node.start_position().row + 1) as i32,
                        });
                    }
                }
            }
        }
        _ => {}
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_c_symbols(&child, source, file_path, data, current_function.clone());
    }
}

/// Process a C function and add to ExtractedData
fn process_c_function(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let params = extract_parameters(node, source);
    let return_type = extract_return_type(node, source);
    let is_public = file_path.ends_with(".h"); // Headers are public

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "function".to_string(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });

    // Add function fact
    data.add_function(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: false, // C doesn't have self
        takes_mut_params: params.iter().any(|p| p.contains('*')),
        returns_result: false, // C uses error codes
        returns_option: false, // C uses NULL
        is_async: false,       // C doesn't have async
        is_unsafe: true,       // All C is unsafe
        is_public,
        parameter_count: params.len() as i32,
        generic_count: 0, // C doesn't have generics
        parameters: params,
        return_type,
    });
}

/// Process a C type (struct/union/enum) and add to ExtractedData
fn process_c_type(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: SymbolKind,
    data: &mut ExtractedData,
) {
    let is_public = file_path.ends_with(".h");

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: kind.to_string(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });

    // Add type fact
    data.add_type(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: format!("{} {}", kind, name),
        kind: kind.to_string(),
        visibility: if is_public { "public" } else { "private" }.to_string(),
        usage_count: 0,
    });
}

/// Process a C typedef and add to ExtractedData
fn process_c_typedef(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = file_path.ends_with(".h");

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "typedef".to_string(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });

    // Add type fact
    data.add_type(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: format!("typedef {}", name),
        kind: "typedef".to_string(),
        visibility: if is_public { "public" } else { "private" }.to_string(),
        usage_count: 0,
    });
}

/// Process a C include directive and add to ExtractedData
fn process_c_include(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    if let Ok(include_text) = node.utf8_text(source) {
        let header = include_text
            .trim_start_matches("#include")
            .trim()
            .trim_start_matches('<')
            .trim_start_matches('"')
            .trim_end_matches('>')
            .trim_end_matches('"');
        let is_external = include_text.contains('<');

        data.add_import(ImportFact {
            file: file_path.to_string(),
            import_path: header.to_string(),
            imported_names: vec![header.to_string()],
            import_kind: if is_external { "system" } else { "local" }.to_string(),
            line_number: (node.start_position().row + 1) as i32,
        });
    }
}

/// Extract function name from C function_definition node, handling nested declarators
fn extract_function_name(node: &Node, source: &[u8]) -> Option<String> {
    let declarator = node.child_by_field_name("declarator")?;
    extract_c_function_name(declarator)
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.to_string())
}

/// Extract C function name from declarator (iterative to avoid stack overflow)
/// Handles function pointers, pointer declarators, and other nested structures
fn extract_c_function_name(declarator: Node) -> Option<Node> {
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

        // Check children
        let mut found = None;
        let mut cursor = current.walk();
        for child in current.children(&mut cursor) {
            if child.kind() == "identifier" {
                found = Some(child);
                break;
            }
        }

        return found;
    }
}

/// Extract typedef name from declarator
fn extract_typedef_name(declarator: &Node, source: &[u8]) -> Option<String> {
    // For typedef, the name is often directly in the declarator
    if declarator.kind() == "type_identifier" || declarator.kind() == "identifier" {
        return declarator.utf8_text(source).ok().map(|s| s.to_string());
    }

    // For pointer typedefs, drill down
    if declarator.kind() == "pointer_declarator" {
        if let Some(inner) = declarator.child_by_field_name("declarator") {
            return extract_typedef_name(&inner, source);
        }
    }

    // Check children
    let mut cursor = declarator.walk();
    for child in declarator.children(&mut cursor) {
        if child.kind() == "type_identifier" || child.kind() == "identifier" {
            return child.utf8_text(source).ok().map(|s| s.to_string());
        }
    }

    None
}

/// Extract function parameters
fn extract_parameters(node: &Node, source: &[u8]) -> Vec<String> {
    if let Some(declarator) = node.child_by_field_name("declarator") {
        if let Some(params_node) = declarator.child_by_field_name("parameters") {
            let mut params = Vec::new();
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
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

/// Extract return type
fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("type")
        .and_then(|t| t.utf8_text(source).ok())
        .map(String::from)
}

/// Extract context around a symbol
fn extract_context(node: &Node, source: &[u8]) -> String {
    // Get a few lines around the symbol for context
    let start_byte = node.start_byte();
    let end_byte = node.end_byte().min(start_byte + 200); // Limit context size

    if let Ok(context) = std::str::from_utf8(&source[start_byte..end_byte]) {
        context.lines().take(3).collect::<Vec<_>>().join(" ")
    } else {
        String::new()
    }
}
