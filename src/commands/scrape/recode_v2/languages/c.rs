// ============================================================================
// C LANGUAGE MODULE - Self-Contained Processor
// ============================================================================
//! C language processor with complete isolation.
//!
//! Handles C's features:
//! - Header files vs implementation files
//! - Preprocessor directives  
//! - Function pointers and nested declarators
//! - Structs, unions, and enums
//! - No built-in visibility (header exposure = public)

use crate::commands::scrape::recode_v2::sql_builder::{InsertBuilder, TableName};
use crate::commands::scrape::recode_v2::types::{CallType, FilePath};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// C language processor - completely self-contained
pub struct CProcessor;

impl CProcessor {
    /// Process a C file and extract all symbols to SQL statements
    pub fn process_file(
        file_path: FilePath,
        content: &[u8],
    ) -> Result<(Vec<String>, usize, usize, usize)> {
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

        let root = tree.root_node();
        let mut sql_statements = Vec::new();
        let mut functions = 0;
        let mut types = 0;
        let mut imports = 0;

        // Track current function for call graph
        let mut current_function: Option<String> = None;
        let mut call_graph_entries = Vec::new();

        // Walk the tree and extract symbols
        extract_symbols(
            root,
            content,
            &file_path,
            &mut sql_statements,
            &mut functions,
            &mut types,
            &mut imports,
            &mut current_function,
            &mut call_graph_entries,
        );

        // Add call graph entries
        for (caller, callee, call_type, line) in call_graph_entries {
            let call_sql = InsertBuilder::new(TableName::CALL_GRAPH)
                .or_replace()
                .value("caller", caller.as_str())
                .value("callee", callee.as_str())
                .value("call_type", call_type.as_str())
                .value("file", file_path.as_str())
                .value("line_number", line as i64)
                .build();
            sql_statements.push(format!("{};\n", call_sql));
        }

        Ok((sql_statements, functions, types, imports))
    }
}

/// Recursively extract symbols from the syntax tree
fn extract_symbols(
    node: Node,
    source: &[u8],
    file_path: &FilePath,
    sql: &mut Vec<String>,
    functions: &mut usize,
    types: &mut usize,
    imports: &mut usize,
    current_function: &mut Option<String>,
    call_graph: &mut Vec<(String, String, CallType, i32)>,
) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = extract_function_name(&node, source) {
                // Track for call graph
                let old_func = current_function.clone();
                *current_function = Some(name.clone());

                // Extract function details
                let params = extract_parameters(&node, source);
                let return_type = extract_return_type(&node, source);
                let is_public = file_path.as_str().ends_with(".h"); // Headers are public

                // Build signature
                let signature = if params.is_empty() {
                    format!("{} {}()", return_type.as_deref().unwrap_or("void"), name)
                } else {
                    format!(
                        "{} {}({})",
                        return_type.as_deref().unwrap_or("void"),
                        name,
                        params.join(", ")
                    )
                };

                // Insert into function_facts
                let func_sql = InsertBuilder::new(TableName::FUNCTION_FACTS)
                    .or_replace()
                    .value("file", file_path.as_str())
                    .value("name", name.as_str())
                    .value("takes_mut_self", false)
                    .value("takes_mut_params", params.iter().any(|p| p.contains("*")))
                    .value("returns_result", false)
                    .value("returns_option", false)
                    .value("is_async", false)
                    .value("is_unsafe", true) // All C is unsafe
                    .value("is_public", is_public)
                    .value("parameter_count", params.len() as i64)
                    .value("generic_count", 0i64)
                    .value("parameters", params.join(", "))
                    .value("return_type", return_type.as_deref().unwrap_or("void"))
                    .build();
                sql.push(format!("{};\n", func_sql));

                // Also insert into code_search
                let search_sql = InsertBuilder::new(TableName::CODE_SEARCH)
                    .or_replace()
                    .value("path", file_path.as_str())
                    .value("name", name.as_str())
                    .value("signature", signature)
                    .value("context", extract_context(&node, source))
                    .build();
                sql.push(format!("{};\n", search_sql));

                *functions += 1;

                // Process function body for calls
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    extract_symbols(
                        child,
                        source,
                        file_path,
                        sql,
                        functions,
                        types,
                        imports,
                        current_function,
                        call_graph,
                    );
                }

                // Restore previous function context
                *current_function = old_func;
                return; // Don't recurse again
            }
        }
        "struct_specifier" | "union_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    let kind = if node.kind() == "struct_specifier" {
                        "struct"
                    } else {
                        "union"
                    };
                    let is_public = file_path.as_str().ends_with(".h");

                    let type_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                        .or_replace()
                        .value("file", file_path.as_str())
                        .value("name", name)
                        .value("definition", format!("{} {}", kind, name))
                        .value("kind", kind)
                        .value("visibility", if is_public { "public" } else { "private" })
                        .build();
                    sql.push(format!("{};\n", type_sql));
                    *types += 1;
                }
            }
        }
        "enum_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    let is_public = file_path.as_str().ends_with(".h");

                    let type_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                        .or_replace()
                        .value("file", file_path.as_str())
                        .value("name", name)
                        .value("definition", format!("enum {}", name))
                        .value("kind", "enum")
                        .value("visibility", if is_public { "public" } else { "private" })
                        .build();
                    sql.push(format!("{};\n", type_sql));
                    *types += 1;
                }
            }
        }
        "type_definition" => {
            // typedef handling
            if let Some(declarator) = node.child_by_field_name("declarator") {
                if let Some(name) = extract_typedef_name(&declarator, source) {
                    let is_public = file_path.as_str().ends_with(".h");

                    let type_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                        .or_replace()
                        .value("file", file_path.as_str())
                        .value("name", name.as_str())
                        .value("definition", format!("typedef {}", name))
                        .value("kind", "typedef")
                        .value("visibility", if is_public { "public" } else { "private" })
                        .build();
                    sql.push(format!("{};\n", type_sql));
                    *types += 1;
                }
            }
        }
        "preproc_include" => {
            if let Ok(include_text) = node.utf8_text(source) {
                let header = include_text
                    .trim_start_matches("#include")
                    .trim()
                    .trim_start_matches('<')
                    .trim_start_matches('"')
                    .trim_end_matches('>')
                    .trim_end_matches('"');
                let is_external = include_text.contains('<');

                let import_sql = InsertBuilder::new(TableName::IMPORT_FACTS)
                    .or_replace()
                    .value("importer_file", file_path.as_str())
                    .value("imported_item", header)
                    .value("imported_from", header)
                    .value("is_external", is_external)
                    .value("import_kind", "include")
                    .build();
                sql.push(format!("{};\n", import_sql));
                *imports += 1;
            }
        }
        "call_expression" => {
            // Track function calls for call graph
            if let Some(ref caller) = current_function {
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        let line = (node.start_position().row + 1) as i32;
                        call_graph.push((
                            caller.clone(),
                            callee.to_string(),
                            CallType::Direct,
                            line,
                        ));
                    }
                }
            }
        }
        _ => {}
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_symbols(
            child,
            source,
            file_path,
            sql,
            functions,
            types,
            imports,
            current_function,
            call_graph,
        );
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