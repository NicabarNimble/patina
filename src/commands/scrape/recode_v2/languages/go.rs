// ============================================================================
// GO LANGUAGE MODULE - Self-Contained Processor
// ============================================================================
//! Go language processor with complete isolation.
//!
//! Handles Go's unique features:
//! - Exported vs unexported (capitalization-based visibility)
//! - Interfaces and struct embedding
//! - Goroutines and channels
//! - Multiple return values
//! - Package-level declarations

use crate::commands::scrape::recode_v2::sql_builder::{InsertBuilder, TableName};
use crate::commands::scrape::recode_v2::types::{CallType, FilePath, SymbolKind};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// Go language processor - completely self-contained
pub struct GoProcessor;

impl GoProcessor {
    /// Process a Go file and extract all symbols to SQL statements
    pub fn process_file(
        file_path: FilePath,
        content: &[u8],
    ) -> Result<(Vec<String>, usize, usize, usize)> {
        // Set up tree-sitter parser for Go
        let mut parser = Parser::new();
        let metal = patina_metal::Metal::Go;
        let language = metal
            .tree_sitter_language_for_ext("go")
            .ok_or_else(|| anyhow::anyhow!("No Go parser available"))?;
        parser
            .set_language(&language)
            .context("Failed to set Go language")?;

        // Parse the file
        let tree = parser
            .parse(content, None)
            .context("Failed to parse Go file")?;

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
    // First extract any calls
    extract_calls(&node, source, current_function, call_graph);

    // Determine symbol kind
    let symbol_kind = match node.kind() {
        "function_declaration" | "method_declaration" => SymbolKind::Function,
        "type_declaration" => SymbolKind::TypeAlias,
        "const_declaration" | "var_declaration" => SymbolKind::Const,
        "import_declaration" => SymbolKind::Import,
        _ => {
            // Check for complex types that need node inspection
            if node.kind() == "type_spec" {
                if let Some(type_node) = node.child_by_field_name("type") {
                    match type_node.kind() {
                        "struct_type" => SymbolKind::Struct,
                        "interface_type" => SymbolKind::Trait,
                        _ => SymbolKind::TypeAlias,
                    }
                } else {
                    SymbolKind::TypeAlias
                }
            } else {
                SymbolKind::Unknown
            }
        }
    };

    // Process based on symbol kind
    match symbol_kind {
        SymbolKind::Function => {
            if let Some(name) = extract_function_name(&node, source) {
                let old_function = current_function.clone();
                *current_function = Some(name.clone());

                let is_public = is_exported(&name);
                let params = extract_params(&node, source);
                let return_type = extract_return_type(&node, source);
                let generics = extract_generics(&node, source);
                let docs = extract_doc_comment(&node, source);

                let insert_sql = InsertBuilder::new(TableName::FUNCTION_FACTS)
                    .or_replace()
                    .value("name", name.as_str())
                    .value("file", file_path.as_str())
                    .value("is_public", is_public)
                    .value("is_async", false) // Go uses goroutines instead
                    .value("is_unsafe", false) // Go doesn't have unsafe
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
        SymbolKind::Struct | SymbolKind::Trait | SymbolKind::TypeAlias | SymbolKind::Enum => {
            if let Some((name, kind)) = extract_type_info(&node, source) {
                let is_public = is_exported(&name);
                let docs = extract_doc_comment(&node, source);

                let insert_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                    .or_replace()
                    .value("symbol", name.as_str())
                    .value("symbol_type", kind.as_str())
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

/// Extract function name from a function/method declaration
fn extract_function_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(String::from)
}

/// Check if a name is exported (public) in Go
fn is_exported(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase())
}

/// Extract function parameters
fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
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
}

/// Extract return type from function
fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("result")
        .and_then(|r| r.utf8_text(source).ok())
        .map(String::from)
}

/// Extract generic parameters (Go 1.18+)
fn extract_generics(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("type_parameters")
        .and_then(|tp| tp.utf8_text(source).ok())
        .map(String::from)
}

/// Extract type information (name and kind)
fn extract_type_info(node: &Node, source: &[u8]) -> Option<(String, SymbolKind)> {
    if node.kind() == "type_declaration" {
        // Type declarations can contain multiple type_spec children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_spec" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(source) {
                        let kind = if let Some(type_node) = child.child_by_field_name("type") {
                            match type_node.kind() {
                                "struct_type" => SymbolKind::Struct,
                                "interface_type" => SymbolKind::Trait,
                                _ => SymbolKind::TypeAlias,
                            }
                        } else {
                            SymbolKind::TypeAlias
                        };
                        return Some((name.to_string(), kind));
                    }
                }
            }
        }
    } else if node.kind() == "type_spec" {
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(name) = name_node.utf8_text(source) {
                let kind = if let Some(type_node) = node.child_by_field_name("type") {
                    match type_node.kind() {
                        "struct_type" => SymbolKind::Struct,
                        "interface_type" => SymbolKind::Trait,
                        _ => SymbolKind::TypeAlias,
                    }
                } else {
                    SymbolKind::TypeAlias
                };
                return Some((name.to_string(), kind));
            }
        }
    }
    None
}

/// Extract import details from an import declaration
fn extract_import_details(node: &Node, source: &[u8]) -> Option<(String, String, bool)> {
    if node.kind() != "import_declaration" {
        return None;
    }

    // Handle both single imports and import blocks
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "import_spec" {
            if let Ok(import_text) = child.utf8_text(source) {
                let import_clean = import_text.trim().trim_matches('"');
                let is_external = !import_clean.starts_with(".");
                let imported_item = import_clean.split('/').last().unwrap_or(import_clean);
                return Some((imported_item.to_string(), import_clean.to_string(), is_external));
            }
        } else if child.kind() == "import_spec_list" {
            // Handle import blocks - process first spec for now
            let mut list_cursor = child.walk();
            for spec in child.children(&mut list_cursor) {
                if spec.kind() == "import_spec" {
                    if let Ok(import_text) = spec.utf8_text(source) {
                        let import_clean = import_text.trim().trim_matches('"');
                        let is_external = !import_clean.starts_with(".");
                        let imported_item = import_clean.split('/').last().unwrap_or(import_clean);
                        return Some((imported_item.to_string(), import_clean.to_string(), is_external));
                    }
                }
            }
        }
    }

    // Fallback to simple parsing
    if let Ok(import_text) = node.utf8_text(source) {
        let import_clean = import_text
            .trim_start_matches("import ")
            .trim()
            .trim_matches('"');
        let is_external = !import_clean.starts_with(".");
        let imported_item = import_clean.split('/').last().unwrap_or(import_clean);
        return Some((imported_item.to_string(), import_clean.to_string(), is_external));
    }

    None
}

/// Extract documentation comment
fn extract_doc_comment(node: &Node, source: &[u8]) -> String {
    // Look for comment nodes immediately before this node
    if let Some(prev) = node.prev_sibling() {
        if prev.kind() == "comment" {
            if let Ok(text) = prev.utf8_text(source) {
                return clean_doc_comment(text);
            }
        }
    }
    String::new()
}

/// Clean Go documentation comment
fn clean_doc_comment(raw: &str) -> String {
    raw.lines()
        .map(|line| line.trim_start().strip_prefix("//").unwrap_or(line).trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract call expressions from a node
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
                        call_graph.push((
                            caller.clone(),
                            callee.to_string(),
                            CallType::Direct,
                            line_number,
                        ));
                    }
                }
            }
        }
        "go_statement" => {
            // Handle goroutines
            if let Some(caller) = current_function {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "call_expression" {
                        if let Some(func_node) = child.child_by_field_name("function") {
                            if let Ok(callee) = func_node.utf8_text(source) {
                                call_graph.push((
                                    caller.clone(),
                                    callee.to_string(),
                                    CallType::Goroutine,
                                    line_number,
                                ));
                            }
                        }
                    }
                }
            }
        }
        "defer_statement" => {
            // Handle defer statements
            if let Some(caller) = current_function {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "call_expression" {
                        if let Some(func_node) = child.child_by_field_name("function") {
                            if let Ok(callee) = func_node.utf8_text(source) {
                                call_graph.push((
                                    caller.clone(),
                                    callee.to_string(),
                                    CallType::Defer,
                                    line_number,
                                ));
                            }
                        }
                    }
                }
            }
        }
        "selector_expression" => {
            // Handle method calls
            if let Some(parent) = node.parent() {
                if parent.kind() == "call_expression" {
                    if let Some(caller) = current_function {
                        if let Some(field_node) = node.child_by_field_name("field") {
                            if let Ok(callee) = field_node.utf8_text(source) {
                                call_graph.push((
                                    caller.clone(),
                                    callee.to_string(),
                                    CallType::Method,
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