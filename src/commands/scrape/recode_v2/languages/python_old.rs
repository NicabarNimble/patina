// ============================================================================
// PYTHON LANGUAGE MODULE - Self-Contained Processor
// ============================================================================
//! Python language processor with complete isolation.
//!
//! Handles Python's unique features:
//! - Underscore-based visibility conventions
//! - Docstrings (triple quotes)
//! - Duck typing and dynamic nature
//! - Async/await support
//! - Decorators and class definitions
//! - Import system (from/import statements)

use crate::commands::scrape::recode_v2::sql_builder::{InsertBuilder, TableName};
use crate::commands::scrape::recode_v2::types::{CallType, FilePath, SymbolKind};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// Python language processor - completely self-contained
pub struct PythonProcessor;

impl PythonProcessor {
    /// Process a Python file and extract all symbols to SQL statements
    pub fn process_file(
        file_path: FilePath,
        content: &[u8],
    ) -> Result<(Vec<String>, usize, usize, usize)> {
        // Set up tree-sitter parser for Python
        let mut parser = Parser::new();
        let metal = patina_metal::Metal::Python;
        let language = metal
            .tree_sitter_language_for_ext("py")
            .ok_or_else(|| anyhow::anyhow!("No Python parser available"))?;
        parser
            .set_language(&language)
            .context("Failed to set Python language")?;

        // Parse the file
        let tree = parser
            .parse(content, None)
            .context("Failed to parse Python file")?;

        let root = tree.root_node();
        let mut sql_statements = Vec::new();
        let mut functions = 0;
        let mut types = 0;
        let mut imports = 0;

        // Track current function/class for call graph
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
    // First extract any calls
    extract_calls(&node, source, current_function, call_graph);

    // Handle decorated definitions specially
    if node.kind() == "decorated_definition" {
        process_decorated_definition(
            &node,
            source,
            file_path,
            sql,
            functions,
            types,
            current_function,
            call_graph,
        );
        return;
    }

    // Determine symbol kind
    let symbol_kind = match node.kind() {
        "function_definition" | "async_function_definition" => SymbolKind::Function,
        "class_definition" => SymbolKind::Class,
        "import_statement" | "import_from_statement" => SymbolKind::Import,
        _ => SymbolKind::Unknown,
    };

    // Process based on symbol kind
    match symbol_kind {
        SymbolKind::Function => {
            if let Some(name) = extract_function_name(&node, source) {
                let old_function = current_function.clone();
                *current_function = Some(name.clone());

                let is_public = !name.starts_with('_');
                let is_async = node.kind() == "async_function_definition";
                let params = extract_params(&node, source);
                let return_type = extract_return_type(&node, source);
                let docs = extract_docstring(&node, source);

                let insert_sql = InsertBuilder::new(TableName::FUNCTION_FACTS)
                    .or_replace()
                    .value("name", name.as_str())
                    .value("file", file_path.as_str())
                    .value("is_public", is_public)
                    .value("is_async", is_async)
                    .value("is_unsafe", false) // Python doesn't have unsafe
                    .value("parameters", params.join(", ").as_str())
                    .value("return_type", return_type.as_deref().unwrap_or(""))
                    .value("generics", "") // Python doesn't have traditional generics
                    .value("doc_comment", docs.as_str())
                    .value("line_number", (node.start_position().row + 1) as i64)
                    .build();
                sql.push(format!("{};\n", insert_sql));
                *functions += 1;

                // Recursively process function body
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

                *current_function = old_function;
                return; // Don't recurse again
            }
        }
        SymbolKind::Class => {
            if let Some(name) = extract_class_name(&node, source) {
                let old_function = current_function.clone();
                
                let is_public = !name.starts_with('_');
                let docs = extract_docstring(&node, source);

                let insert_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                    .or_replace()
                    .value("symbol", name.as_str())
                    .value("symbol_type", "class")
                    .value("file", file_path.as_str())
                    .value("is_public", is_public)
                    .value("doc_comment", docs.as_str())
                    .value("line_number", (node.start_position().row + 1) as i64)
                    .build();
                sql.push(format!("{};\n", insert_sql));
                *types += 1;

                // Process class methods
                if let Some(body_node) = node.child_by_field_name("body") {
                    let mut cursor = body_node.walk();
                    for child in body_node.children(&mut cursor) {
                        // For methods, include class name in context
                        if matches!(child.kind(), "function_definition" | "async_function_definition") {
                            if let Some(method_name) = extract_function_name(&child, source) {
                                *current_function = Some(format!("{}.{}", name, method_name));
                            }
                        }
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

                *current_function = old_function;
                return; // Don't recurse again
            }
        }
        SymbolKind::Import => {
            if let Some((imported_item, import_path, is_external)) = extract_import_details(&node, source) {
                let insert_sql = InsertBuilder::new(TableName::IMPORT_FACTS)
                    .or_replace()
                    .value("imported_item", imported_item.as_str())
                    .value("import_path", import_path.as_str())
                    .value("file", file_path.as_str())
                    .value("is_external", is_external)
                    .value("line_number", (node.start_position().row + 1) as i64)
                    .build();
                sql.push(format!("{};\n", insert_sql));
                *imports += 1;
            }
        }
        _ => {}
    }

    // Recurse to children
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

/// Process decorated definitions (functions or classes with decorators)
fn process_decorated_definition(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    sql: &mut Vec<String>,
    functions: &mut usize,
    types: &mut usize,
    current_function: &mut Option<String>,
    call_graph: &mut Vec<(String, String, CallType, i32)>,
) {
    // Extract decorators
    let mut decorators = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "decorator" {
            if let Ok(decorator_text) = child.utf8_text(source) {
                let decorator_name = decorator_text.trim_start_matches('@');
                decorators.push(decorator_name.to_string());
                
                // Add decorator as a call
                if let Some(caller) = current_function {
                    call_graph.push((
                        caller.clone(),
                        format!("@{}", decorator_name),
                        CallType::Decorator,
                        (child.start_position().row + 1) as i32,
                    ));
                }
            }
        }
    }

    // Process the actual definition
    if let Some(definition) = node.child_by_field_name("definition") {
        match definition.kind() {
            "function_definition" | "async_function_definition" => {
                if let Some(name) = extract_function_name(&definition, source) {
                    let is_public = !name.starts_with('_');
                    let is_async = definition.kind() == "async_function_definition";
                    let params = extract_params(&definition, source);
                    let return_type = extract_return_type(&definition, source);
                    let docs = extract_docstring(&definition, source);

                    let insert_sql = InsertBuilder::new(TableName::FUNCTION_FACTS)
                        .or_replace()
                        .value("name", name.as_str())
                        .value("file", file_path.as_str())
                        .value("is_public", is_public)
                        .value("is_async", is_async)
                        .value("is_unsafe", false)
                        .value("parameters", params.join(", ").as_str())
                        .value("return_type", return_type.as_deref().unwrap_or(""))
                        .value("generics", "")
                        .value("doc_comment", docs.as_str())
                        .value("line_number", (definition.start_position().row + 1) as i64)
                        .build();
                    sql.push(format!("{};\n", insert_sql));
                    *functions += 1;
                }
            }
            "class_definition" => {
                if let Some(name) = extract_class_name(&definition, source) {
                    let is_public = !name.starts_with('_');
                    let docs = extract_docstring(&definition, source);

                    let insert_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                        .or_replace()
                        .value("symbol", name.as_str())
                        .value("symbol_type", "class")
                        .value("file", file_path.as_str())
                        .value("is_public", is_public)
                        .value("doc_comment", docs.as_str())
                        .value("line_number", (definition.start_position().row + 1) as i64)
                        .build();
                    sql.push(format!("{};\n", insert_sql));
                    *types += 1;
                }
            }
            _ => {}
        }
    }
}

/// Extract function name
fn extract_function_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(String::from)
}

/// Extract class name
fn extract_class_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(String::from)
}

/// Extract function parameters
fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut params = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if matches!(child.kind(), "," | "(" | ")") {
                continue;
            }
            if let Ok(param_text) = child.utf8_text(source) {
                let cleaned = param_text.trim();
                if !cleaned.is_empty() && cleaned != "self" && cleaned != "cls" {
                    params.push(cleaned.to_string());
                }
            }
        }
        params
    } else {
        Vec::new()
    }
}

/// Extract return type annotation
fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("return_type")
        .and_then(|rt| rt.utf8_text(source).ok())
        .map(|s| s.trim_start_matches("->").trim().to_string())
}

/// Extract docstring from function or class
fn extract_docstring(node: &Node, source: &[u8]) -> String {
    // Look for docstring in the body
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "expression_statement" {
                if let Some(string_node) = child.child(0) {
                    if string_node.kind() == "string" {
                        if let Ok(text) = string_node.utf8_text(source) {
                            return clean_docstring(text);
                        }
                    }
                }
            }
        }
    }
    String::new()
}

/// Clean Python docstring
fn clean_docstring(raw: &str) -> String {
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
}

/// Extract import details
fn extract_import_details(node: &Node, source: &[u8]) -> Option<(String, String, bool)> {
    let import_text = node.utf8_text(source).ok()?;
    
    match node.kind() {
        "import_statement" => {
            // Handle: import module, import module as alias
            let clean = import_text.trim_start_matches("import ").trim();
            let module_name = clean.split(" as ").next().unwrap_or(clean);
            let is_external = !module_name.starts_with('.');
            Some((module_name.to_string(), module_name.to_string(), is_external))
        }
        "import_from_statement" => {
            // Handle: from module import item, from . import item
            if let Some(module_node) = node.child_by_field_name("module_name") {
                if let Ok(module_name) = module_node.utf8_text(source) {
                    let is_external = !import_text.contains("from .");
                    
                    // Extract imported items
                    let items = if import_text.contains("import *") {
                        "*".to_string()
                    } else if let Some(import_idx) = import_text.find("import ") {
                        import_text[import_idx + 7..].trim().to_string()
                    } else {
                        module_name.to_string()
                    };
                    
                    return Some((items, module_name.to_string(), is_external));
                }
            }
            
            // Fallback parsing
            let is_external = !import_text.contains("from .");
            Some((import_text.to_string(), import_text.to_string(), is_external))
        }
        _ => None,
    }
}

/// Extract call expressions
fn extract_calls(
    node: &Node,
    source: &[u8],
    current_function: &Option<String>,
    call_graph: &mut Vec<(String, String, CallType, i32)>,
) {
    let line_number = (node.start_position().row + 1) as i32;

    match node.kind() {
        "call" => {
            if let Some(caller) = current_function {
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        // Check for await
                        let (call_type, callee_name) = if callee.starts_with("await ") {
                            (CallType::Async, callee.strip_prefix("await ").unwrap_or(callee))
                        } else {
                            (CallType::Direct, callee)
                        };
                        
                        call_graph.push((
                            caller.clone(),
                            callee_name.to_string(),
                            call_type,
                            line_number,
                        ));
                    }
                }
            }
        }
        "await" => {
            // Handle await expressions
            if let Some(caller) = current_function {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "call" {
                        if let Some(func_node) = child.child_by_field_name("function") {
                            if let Ok(callee) = func_node.utf8_text(source) {
                                call_graph.push((
                                    caller.clone(),
                                    callee.to_string(),
                                    CallType::Async,
                                    line_number,
                                ));
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}