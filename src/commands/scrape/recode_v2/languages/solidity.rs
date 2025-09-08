// ============================================================================
// SOLIDITY LANGUAGE MODULE - Self-Contained Processor
// ============================================================================
//! Solidity language processor with complete isolation.
//!
//! Handles Solidity's unique features:
//! - Smart contract structure
//! - Visibility modifiers (public, private, internal, external)
//! - State mutability (pure, view, payable)
//! - Events and modifiers
//! - Inheritance and interfaces
//! - Unchecked blocks (similar to unsafe)
//! - Library and contract declarations

use crate::commands::scrape::recode_v2::sql_builder::{InsertBuilder, TableName};
use crate::commands::scrape::recode_v2::types::{CallType, FilePath, SymbolKind};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// Solidity language processor - completely self-contained
pub struct SolidityProcessor;

impl SolidityProcessor {
    /// Process a Solidity file and extract all symbols to SQL statements
    pub fn process_file(
        file_path: FilePath,
        content: &[u8],
    ) -> Result<(Vec<String>, usize, usize, usize)> {
        // Set up tree-sitter parser for Solidity
        let mut parser = Parser::new();
        let metal = patina_metal::Metal::Solidity;
        let language = metal
            .tree_sitter_language_for_ext("sol")
            .ok_or_else(|| anyhow::anyhow!("No Solidity parser available"))?;
        parser
            .set_language(&language)
            .context("Failed to set Solidity language")?;

        // Parse the file
        let tree = parser
            .parse(content, None)
            .context("Failed to parse Solidity file")?;

        let root = tree.root_node();
        let mut sql_statements = Vec::new();
        let mut functions = 0;
        let mut types = 0;
        let mut imports = 0;

        // Track current function/contract for call graph
        let mut current_function: Option<String> = None;
        let mut current_contract: Option<String> = None;
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
            &mut current_contract,
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
    current_contract: &mut Option<String>,
    call_graph: &mut Vec<(String, String, CallType, i32)>,
) {
    // First extract any calls
    extract_calls(&node, source, current_function, call_graph);

    // Determine symbol kind
    let symbol_kind = match node.kind() {
        "function_definition" => SymbolKind::Function,
        "modifier_definition" => SymbolKind::Function, // Modifiers are like special functions
        "event_definition" => SymbolKind::Function, // Events are like special functions
        "contract_declaration" => SymbolKind::Struct, // Contracts are like structs
        "struct_declaration" => SymbolKind::Struct,
        "interface_declaration" => SymbolKind::Interface,
        "library_declaration" => SymbolKind::Module, // Libraries are like modules
        "enum_declaration" => SymbolKind::Enum,
        "import_directive" => SymbolKind::Import,
        "state_variable_declaration" => SymbolKind::Const, // State variables are like constants
        _ => SymbolKind::Unknown,
    };

    // Process based on symbol kind
    match symbol_kind {
        SymbolKind::Function => {
            if let Some(name) = extract_function_name(&node, source) {
                let old_function = current_function.clone();
                let full_name = if let Some(contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                *current_function = Some(full_name.clone());

                let visibility = extract_visibility(&node, source);
                let is_public = visibility != "private" && visibility != "internal";
                let _mutability = extract_mutability(&node, source);
                let params = extract_params(&node, source);
                let return_type = extract_return_type(&node, source);
                let docs = extract_natspec(&node, source);
                let is_unsafe = has_unchecked_block(&node, source);

                let insert_sql = InsertBuilder::new(TableName::FUNCTION_FACTS)
                    .or_replace()
                    .value("name", full_name.as_str())
                    .value("file", file_path.as_str())
                    .value("is_public", is_public)
                    .value("is_async", false) // Solidity doesn't have async
                    .value("is_unsafe", is_unsafe) // unchecked blocks
                    .value("parameters", params.join(", ").as_str())
                    .value("return_type", return_type.as_deref().unwrap_or(""))
                    .value("generics", "") // Solidity doesn't have generics
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
                        current_contract,
                        call_graph,
                    );
                }

                *current_function = old_function;
                return; // Don't recurse again
            }
        }
        SymbolKind::Function if node.kind() == "event_definition" => {
            if let Some(name) = extract_event_name(&node, source) {
                let full_name = if let Some(contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                
                let params = extract_event_params(&node, source);
                let docs = extract_natspec(&node, source);

                // Store events as functions with special marker
                let insert_sql = InsertBuilder::new(TableName::FUNCTION_FACTS)
                    .or_replace()
                    .value("name", format!("event {}", full_name).as_str())
                    .value("file", file_path.as_str())
                    .value("is_public", true) // Events are always public
                    .value("is_async", false)
                    .value("is_unsafe", false)
                    .value("parameters", params.join(", ").as_str())
                    .value("return_type", "")
                    .value("generics", "")
                    .value("doc_comment", docs.as_str())
                    .value("line_number", (node.start_position().row + 1) as i64)
                    .build();
                sql.push(format!("{};\n", insert_sql));
                *functions += 1;
            }
        }
        SymbolKind::Struct | SymbolKind::Module | SymbolKind::Interface if matches!(node.kind(), "contract_declaration" | "library_declaration" | "interface_declaration") => {
            if let Some(name) = extract_contract_name(&node, source) {
                let old_contract = current_contract.clone();
                *current_contract = Some(name.clone());
                
                let docs = extract_natspec(&node, source);
                let type_kind = match node.kind() {
                    "contract_declaration" => "contract",
                    "library_declaration" => "library",
                    "interface_declaration" => "interface",
                    _ => "unknown",
                };

                let insert_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                    .or_replace()
                    .value("symbol", name.as_str())
                    .value("symbol_type", type_kind)
                    .value("file", file_path.as_str())
                    .value("is_public", true) // Contracts are public by nature
                    .value("doc_comment", docs.as_str())
                    .value("line_number", (node.start_position().row + 1) as i64)
                    .build();
                sql.push(format!("{};\n", insert_sql));
                *types += 1;

                // Process contract body
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
                        current_contract,
                        call_graph,
                    );
                }

                *current_contract = old_contract;
                return; // Don't recurse again
            }
        }
        SymbolKind::Struct => {
            if let Some(name) = extract_struct_name(&node, source) {
                let full_name = if let Some(contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                
                let docs = extract_natspec(&node, source);

                let insert_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                    .or_replace()
                    .value("symbol", full_name.as_str())
                    .value("symbol_type", "struct")
                    .value("file", file_path.as_str())
                    .value("is_public", true) // Structs visibility is contextual
                    .value("doc_comment", docs.as_str())
                    .value("line_number", (node.start_position().row + 1) as i64)
                    .build();
                sql.push(format!("{};\n", insert_sql));
                *types += 1;
            }
        }
        SymbolKind::Enum => {
            if let Some(name) = extract_enum_name(&node, source) {
                let full_name = if let Some(contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                
                let docs = extract_natspec(&node, source);

                let insert_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                    .or_replace()
                    .value("symbol", full_name.as_str())
                    .value("symbol_type", "enum")
                    .value("file", file_path.as_str())
                    .value("is_public", true) // Enums visibility is contextual
                    .value("doc_comment", docs.as_str())
                    .value("line_number", (node.start_position().row + 1) as i64)
                    .build();
                sql.push(format!("{};\n", insert_sql));
                *types += 1;
            }
        }
        SymbolKind::Const if node.kind() == "state_variable_declaration" => {
            if let Some(name) = extract_state_variable_name(&node, source) {
                let full_name = if let Some(contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                
                let visibility = extract_visibility(&node, source);
                let is_public = visibility == "public" || visibility == "external";
                let var_type = extract_variable_type(&node, source);
                let docs = extract_natspec(&node, source);

                // Store state variables as a special type
                let insert_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                    .or_replace()
                    .value("symbol", full_name.as_str())
                    .value("symbol_type", format!("state_var:{}", var_type.as_deref().unwrap_or("unknown")).as_str())
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
            current_contract,
            call_graph,
        );
    }
}

/// Extract function name
fn extract_function_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(String::from)
}

/// Extract event name
fn extract_event_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(String::from)
}

/// Extract contract/library/interface name
fn extract_contract_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(String::from)
}

/// Extract struct name
fn extract_struct_name(node: &Node, source: &[u8]) -> Option<String> {
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

/// Extract state variable name
fn extract_state_variable_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(String::from)
}

/// Extract visibility modifier (public, private, internal, external)
fn extract_visibility(node: &Node, source: &[u8]) -> String {
    let text = node.utf8_text(source).unwrap_or("");
    
    if text.contains("public") {
        "public".to_string()
    } else if text.contains("external") {
        "external".to_string()
    } else if text.contains("internal") {
        "internal".to_string()
    } else if text.contains("private") {
        "private".to_string()
    } else {
        "internal".to_string() // Default in Solidity
    }
}

/// Extract state mutability (pure, view, payable)
fn extract_mutability(node: &Node, source: &[u8]) -> Option<String> {
    let text = node.utf8_text(source).unwrap_or("");
    
    if text.contains("pure") {
        Some("pure".to_string())
    } else if text.contains("view") {
        Some("view".to_string())
    } else if text.contains("payable") {
        Some("payable".to_string())
    } else {
        None
    }
}

/// Check if function contains unchecked blocks (similar to unsafe)
fn has_unchecked_block(node: &Node, source: &[u8]) -> bool {
    node.utf8_text(source)
        .unwrap_or("")
        .contains("unchecked")
}

/// Extract function parameters
fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut params = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if child.kind() == "parameter" {
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

/// Extract event parameters
fn extract_event_params(node: &Node, source: &[u8]) -> Vec<String> {
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut params = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if child.kind() == "event_parameter" {
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

/// Extract return type
fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("return_parameters")
        .and_then(|rp| rp.utf8_text(source).ok())
        .map(|s| s.trim_start_matches("returns").trim().to_string())
}

/// Extract variable type
fn extract_variable_type(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("type")
        .and_then(|t| t.utf8_text(source).ok())
        .map(String::from)
}

/// Extract NatSpec documentation
fn extract_natspec(node: &Node, source: &[u8]) -> String {
    // Look for comment nodes immediately before this node
    if let Some(prev) = node.prev_sibling() {
        if prev.kind() == "comment" {
            if let Ok(text) = prev.utf8_text(source) {
                return clean_natspec(text);
            }
        }
    }
    String::new()
}

/// Clean NatSpec comment
fn clean_natspec(raw: &str) -> String {
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
    let import_text = node.utf8_text(source).ok()?;
    
    // Extract path from quotes
    if let Some(start) = import_text.find('"') {
        if let Some(end) = import_text[start + 1..].find('"') {
            let path = &import_text[start + 1..start + 1 + end];
            
            // Check if it's an external import
            // External imports often start with '@' (npm packages) or 'http' (URLs)
            let is_external = path.starts_with('@') || 
                              path.starts_with("http") ||
                              !path.starts_with('.');
            
            // Extract imported items if specified
            let imported_item = if import_text.contains(" as ") {
                // Handle aliased imports
                import_text.split(" as ")
                    .nth(1)
                    .and_then(|s| s.split_whitespace().next())
                    .unwrap_or(path)
                    .to_string()
            } else if import_text.contains("{") && import_text.contains("}") {
                // Handle selective imports
                if let Some(start) = import_text.find('{') {
                    if let Some(end) = import_text.find('}') {
                        import_text[start + 1..end].trim().to_string()
                    } else {
                        path.to_string()
                    }
                } else {
                    path.to_string()
                }
            } else {
                path.to_string()
            };
            
            return Some((imported_item, path.to_string(), is_external));
        }
    }
    
    None
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
        "member_expression" => {
            // Handle contract.method() calls
            if let Some(parent) = node.parent() {
                if parent.kind() == "call_expression" {
                    if let Some(caller) = current_function {
                        if let Some(property) = node.child_by_field_name("property") {
                            if let Ok(callee) = property.utf8_text(source) {
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
        "new_expression" => {
            // Handle contract creation
            if let Some(caller) = current_function {
                if let Ok(text) = node.utf8_text(source) {
                    call_graph.push((
                        caller.clone(),
                        text.to_string(),
                        CallType::Constructor,
                        line_number,
                    ));
                }
            }
        }
        "emit_statement" => {
            // Solidity events
            if let Some(caller) = current_function {
                if let Some(event_node) = node.child_by_field_name("name") {
                    if let Ok(event_name) = event_node.utf8_text(source) {
                        call_graph.push((
                            caller.clone(),
                            format!("emit {}", event_name),
                            CallType::Event,
                            line_number,
                        ));
                    }
                }
            }
        }
        _ => {}
    }
}