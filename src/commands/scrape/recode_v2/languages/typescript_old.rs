// ============================================================================
// TYPESCRIPT LANGUAGE MODULE - Self-Contained Processor
// ============================================================================
//! TypeScript language processor with complete isolation.
//!
//! Handles TypeScript's unique features:
//! - Static typing with type annotations
//! - Access modifiers (public, private, protected)
//! - Interfaces and type aliases
//! - Generics and type parameters
//! - Enums and advanced type features
//! - Decorators and metadata
//! - JSX support (.tsx files with different parser)

use crate::commands::scrape::recode_v2::sql_builder::{InsertBuilder, TableName};
use crate::commands::scrape::recode_v2::types::{CallType, FilePath, SymbolKind};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// TypeScript language processor - completely self-contained
pub struct TypeScriptProcessor;

impl TypeScriptProcessor {
    /// Process a TypeScript file and extract all symbols to SQL statements
    pub fn process_file(
        file_path: FilePath,
        content: &[u8],
    ) -> Result<(Vec<String>, usize, usize, usize)> {
        // Set up tree-sitter parser for TypeScript
        // IMPORTANT: TypeScript uses different parsers for .ts and .tsx files
        let mut parser = Parser::new();
        let metal = patina_metal::Metal::TypeScript;
        
        // Extract extension from file path to choose the right parser
        let path_str = file_path.as_str();
        let extension = path_str
            .rfind('.')
            .and_then(|idx| path_str.get(idx + 1..))
            .unwrap_or("ts");
        
        let language = metal
            .tree_sitter_language_for_ext(extension)
            .ok_or_else(|| anyhow::anyhow!("No TypeScript parser available for extension: {}", extension))?;
        parser
            .set_language(&language)
            .context("Failed to set TypeScript language")?;

        // Parse the file
        let tree = parser
            .parse(content, None)
            .context("Failed to parse TypeScript file")?;

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

    // Determine symbol kind
    let symbol_kind = match node.kind() {
        "function_declaration" | "function_expression" | "arrow_function" 
        | "generator_function_declaration" | "method_definition" | "method_signature" => SymbolKind::Function,
        "class_declaration" | "class_expression" => SymbolKind::Class,
        "interface_declaration" => SymbolKind::Interface,
        "type_alias_declaration" => SymbolKind::TypeAlias,
        "enum_declaration" => SymbolKind::Enum,
        "import_statement" => SymbolKind::Import,
        "export_statement" => {
            // Check if it's exporting a function, class, interface, etc.
            if has_child_of_kind(&node, "function_declaration") || 
               has_child_of_kind(&node, "class_declaration") ||
               has_child_of_kind(&node, "interface_declaration") ||
               has_child_of_kind(&node, "type_alias_declaration") {
                SymbolKind::Unknown // Will be handled by the child
            } else {
                SymbolKind::Unknown
            }
        }
        "lexical_declaration" | "variable_declaration" => {
            // Check if it's a function assigned to a variable
            if is_function_assignment(&node) {
                SymbolKind::Function
            } else {
                SymbolKind::Unknown
            }
        }
        _ => SymbolKind::Unknown,
    };

    // Process based on symbol kind
    match symbol_kind {
        SymbolKind::Function => {
            if let Some(name) = extract_function_name(&node, source) {
                let old_function = current_function.clone();
                *current_function = Some(name.clone());

                let is_public = extract_visibility(&node, source);
                let is_async = is_async_function(&node, source);
                let params = extract_params(&node, source);
                let return_type = extract_return_type(&node, source);
                let generics = extract_generics(&node, source);
                let docs = extract_tsdoc(&node, source);

                let insert_sql = InsertBuilder::new(TableName::FUNCTION_FACTS)
                    .or_replace()
                    .value("name", name.as_str())
                    .value("file", file_path.as_str())
                    .value("is_public", is_public)
                    .value("is_async", is_async)
                    .value("is_unsafe", false) // TypeScript doesn't have unsafe
                    .value("parameters", params.join(", ").as_str())
                    .value("return_type", return_type.as_deref().unwrap_or(""))
                    .value("generics", generics.as_deref().unwrap_or(""))
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
                
                let is_public = extract_visibility(&node, source);
                let _generics = extract_generics(&node, source);
                let docs = extract_tsdoc(&node, source);

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
                        if matches!(child.kind(), "method_definition" | "method_signature") {
                            if let Some(method_name) = extract_method_name(&child, source) {
                                *current_function = Some(format!("{}.{}", name, method_name));
                                
                                // Process method as a function
                                let method_visibility = extract_visibility(&child, source);
                                let is_async = is_async_function(&child, source);
                                let params = extract_params(&child, source);
                                let return_type = extract_return_type(&child, source);
                                let method_generics = extract_generics(&child, source);
                                let method_docs = extract_tsdoc(&child, source);
                                
                                let method_sql = InsertBuilder::new(TableName::FUNCTION_FACTS)
                                    .or_replace()
                                    .value("name", format!("{}.{}", name, method_name).as_str())
                                    .value("file", file_path.as_str())
                                    .value("is_public", method_visibility)
                                    .value("is_async", is_async)
                                    .value("is_unsafe", false)
                                    .value("parameters", params.join(", ").as_str())
                                    .value("return_type", return_type.as_deref().unwrap_or(""))
                                    .value("generics", method_generics.as_deref().unwrap_or(""))
                                    .value("doc_comment", method_docs.as_str())
                                    .value("line_number", (child.start_position().row + 1) as i64)
                                    .build();
                                sql.push(format!("{};\n", method_sql));
                                *functions += 1;
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
        SymbolKind::Interface => {
            if let Some(name) = extract_interface_name(&node, source) {
                let is_public = extract_visibility(&node, source);
                let _generics = extract_generics(&node, source);
                let docs = extract_tsdoc(&node, source);

                let insert_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                    .or_replace()
                    .value("symbol", name.as_str())
                    .value("symbol_type", "interface")
                    .value("file", file_path.as_str())
                    .value("is_public", is_public)
                    .value("doc_comment", docs.as_str())
                    .value("line_number", (node.start_position().row + 1) as i64)
                    .build();
                sql.push(format!("{};\n", insert_sql));
                *types += 1;
            }
        }
        SymbolKind::TypeAlias => {
            if let Some(name) = extract_type_alias_name(&node, source) {
                let is_public = extract_visibility(&node, source);
                let _generics = extract_generics(&node, source);
                let docs = extract_tsdoc(&node, source);

                let insert_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                    .or_replace()
                    .value("symbol", name.as_str())
                    .value("symbol_type", "type_alias")
                    .value("file", file_path.as_str())
                    .value("is_public", is_public)
                    .value("doc_comment", docs.as_str())
                    .value("line_number", (node.start_position().row + 1) as i64)
                    .build();
                sql.push(format!("{};\n", insert_sql));
                *types += 1;
            }
        }
        SymbolKind::Enum => {
            if let Some(name) = extract_enum_name(&node, source) {
                let is_public = extract_visibility(&node, source);
                let docs = extract_tsdoc(&node, source);

                let insert_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                    .or_replace()
                    .value("symbol", name.as_str())
                    .value("symbol_type", "enum")
                    .value("file", file_path.as_str())
                    .value("is_public", is_public)
                    .value("doc_comment", docs.as_str())
                    .value("line_number", (node.start_position().row + 1) as i64)
                    .build();
                sql.push(format!("{};\n", insert_sql));
                *types += 1;
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

/// Check if node has a child of specific kind
fn has_child_of_kind(node: &Node, kind: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return true;
        }
    }
    false
}

/// Check if a variable declaration is a function assignment
fn is_function_assignment(node: &Node) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            if let Some(init) = child.child_by_field_name("value") {
                if matches!(init.kind(), "arrow_function" | "function_expression") {
                    return true;
                }
            }
        }
    }
    false
}

/// Extract visibility modifiers (public, private, protected)
fn extract_visibility(node: &Node, source: &[u8]) -> bool {
    let text = node.utf8_text(source).unwrap_or("");
    !text.contains("private") && !text.contains("protected")
}

/// Extract function name
fn extract_function_name(node: &Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "function_declaration" | "generator_function_declaration" | "method_signature" => {
            node.child_by_field_name("name")
                .and_then(|n| n.utf8_text(source).ok())
                .map(String::from)
        }
        "method_definition" => extract_method_name(node, source),
        "arrow_function" | "function_expression" => {
            // Look for parent variable declarator
            if let Some(parent) = node.parent() {
                if parent.kind() == "variable_declarator" {
                    return parent.child_by_field_name("name")
                        .and_then(|n| n.utf8_text(source).ok())
                        .map(String::from);
                }
            }
            None
        }
        "lexical_declaration" | "variable_declaration" => {
            // Find the variable declarator with a function
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "variable_declarator" {
                    if let Some(init) = child.child_by_field_name("value") {
                        if matches!(init.kind(), "arrow_function" | "function_expression") {
                            return child.child_by_field_name("name")
                                .and_then(|n| n.utf8_text(source).ok())
                                .map(String::from);
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Extract method name
fn extract_method_name(node: &Node, source: &[u8]) -> Option<String> {
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

/// Extract interface name
fn extract_interface_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(String::from)
}

/// Extract type alias name
fn extract_type_alias_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(String::from)
}

/// Extract enum name
fn extract_enum_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(String::from)
}

/// Check if function is async
fn is_async_function(node: &Node, source: &[u8]) -> bool {
    node.utf8_text(source)
        .unwrap_or("")
        .contains("async")
}

/// Extract function parameters
fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut params = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            match child.kind() {
                "required_parameter" | "optional_parameter" | "rest_parameter" => {
                    if let Ok(param_text) = child.utf8_text(source) {
                        params.push(param_text.to_string());
                    }
                }
                _ => {}
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
        .map(|s| s.trim_start_matches(':').trim().to_string())
}

/// Extract generic parameters
fn extract_generics(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("type_parameters")
        .and_then(|tp| tp.utf8_text(source).ok())
        .map(String::from)
}

/// Extract TSDoc comment
fn extract_tsdoc(node: &Node, source: &[u8]) -> String {
    // Look for comment nodes immediately before this node
    if let Some(prev) = node.prev_sibling() {
        if prev.kind() == "comment" {
            if let Ok(text) = prev.utf8_text(source) {
                if text.starts_with("/**") || text.starts_with("///") {
                    return clean_tsdoc(text);
                }
            }
        }
    }
    String::new()
}

/// Clean TSDoc comment
fn clean_tsdoc(raw: &str) -> String {
    if raw.starts_with("/**") {
        raw.trim()
            .strip_prefix("/**")
            .and_then(|s| s.strip_suffix("*/"))
            .unwrap_or(raw)
            .lines()
            .map(|line| line.trim_start().strip_prefix("* ").unwrap_or(line.trim()))
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    } else if raw.starts_with("///") {
        raw.lines()
            .map(|line| line.trim_start().strip_prefix("/// ").unwrap_or(line.trim()))
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        raw.to_string()
    }
}

/// Extract import details
fn extract_import_details(node: &Node, source: &[u8]) -> Option<(String, String, bool)> {
    if node.kind() != "import_statement" {
        return None;
    }

    let _import_text = node.utf8_text(source).ok()?;
    
    // Extract the module path
    let mut module_path = String::new();
    let mut imported_items = Vec::new();
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "string" => {
                if let Ok(path) = child.utf8_text(source) {
                    module_path = path.trim_matches(|c| c == '"' || c == '\'' || c == '`').to_string();
                }
            }
            "import_clause" => {
                // Handle various import patterns
                let mut clause_cursor = child.walk();
                for clause_child in child.children(&mut clause_cursor) {
                    match clause_child.kind() {
                        "identifier" => {
                            // Default import: import foo from 'module'
                            if let Ok(name) = clause_child.utf8_text(source) {
                                imported_items.push(name.to_string());
                            }
                        }
                        "named_imports" => {
                            // Named imports: import { foo, bar } from 'module'
                            let mut named_cursor = clause_child.walk();
                            for named_child in clause_child.children(&mut named_cursor) {
                                if named_child.kind() == "import_specifier" {
                                    if let Some(name_node) = named_child.child_by_field_name("name") {
                                        if let Ok(name) = name_node.utf8_text(source) {
                                            imported_items.push(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        "namespace_import" => {
                            // Namespace import: import * as foo from 'module'
                            if let Ok(ns_text) = clause_child.utf8_text(source) {
                                imported_items.push(ns_text.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    
    let is_external = !module_path.starts_with('.') && !module_path.starts_with('/');
    let imported_item = if imported_items.is_empty() {
        module_path.clone()
    } else {
        imported_items.join(", ")
    };
    
    Some((imported_item, module_path, is_external))
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
        "call_expression" => {
            if let Some(caller) = current_function {
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        // Check if it's an async call (preceded by await)
                        let call_type = if let Some(parent) = node.parent() {
                            if parent.kind() == "await_expression" {
                                CallType::Async
                            } else {
                                CallType::Direct
                            }
                        } else {
                            CallType::Direct
                        };
                        
                        call_graph.push((
                            caller.clone(),
                            callee.to_string(),
                            call_type,
                            line_number,
                        ));
                    }
                }
            }
        }
        "new_expression" => {
            // Constructor calls
            if let Some(caller) = current_function {
                if let Some(constructor_node) = node.child_by_field_name("constructor") {
                    if let Ok(constructor) = constructor_node.utf8_text(source) {
                        call_graph.push((
                            caller.clone(),
                            format!("new {}", constructor),
                            CallType::Constructor,
                            line_number,
                        ));
                    }
                }
            }
        }
        "decorator" => {
            // TypeScript decorators
            if let Some(caller) = current_function {
                if let Ok(decorator_text) = node.utf8_text(source) {
                    call_graph.push((
                        caller.clone(),
                        decorator_text.to_string(),
                        CallType::Decorator,
                        line_number,
                    ));
                }
            }
        }
        _ => {}
    }
}