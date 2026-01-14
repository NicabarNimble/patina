// ============================================================================
// SOLIDITY LANGUAGE PROCESSOR V2 - STRUCT-BASED
// ============================================================================
//! Solidity language processor that returns typed structs instead of SQL strings.
//!
//! This is the refactored version that:
//! - Returns ExtractedData with typed structs
//! - No SQL string generation
//! - Direct data extraction to domain types
//!
//! Handles Solidity's unique features:
//! - Smart contract structure
//! - Visibility modifiers (public, private, internal, external)
//! - State mutability (pure, view, payable)
//! - Events and modifiers
//! - Inheritance and interfaces
//! - Unchecked blocks (similar to unsafe)
//! - Library and contract declarations

use crate::commands::scrape::code::database::{CodeSymbol, FunctionFact, ImportFact, TypeFact};
use crate::commands::scrape::code::extracted_data::ExtractedData;
use crate::commands::scrape::code::types::{CallGraphEntry, CallType, FilePath, SymbolKind};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// Solidity language processor - returns typed structs
pub struct SolidityProcessor;

impl SolidityProcessor {
    /// Process a Solidity file and extract all symbols to typed structs
    pub fn process_file(file_path: FilePath, content: &[u8]) -> Result<ExtractedData> {
        let mut data = ExtractedData::new();

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

        // Walk the AST and extract symbols
        extract_solidity_symbols(
            &tree.root_node(),
            content,
            &file_path.to_string(),
            &mut data,
            None,
            None,
        );

        Ok(data)
    }
}

/// Recursively extract symbols from the Solidity AST
fn extract_solidity_symbols(
    node: &Node,
    source: &[u8],
    file_path: &str,
    data: &mut ExtractedData,
    current_function: Option<String>,
    current_contract: Option<String>,
) {
    // First extract any calls
    extract_solidity_calls(node, source, file_path, current_function.as_deref(), data);

    // Determine symbol kind
    let _symbol_kind = match node.kind() {
        "function_definition" => SymbolKind::Function,
        "modifier_definition" => SymbolKind::Function, // Modifiers are like special functions
        "event_definition" => SymbolKind::Function,    // Events are like special functions
        "contract_declaration" => SymbolKind::Struct,  // Contracts are like structs
        "struct_declaration" => SymbolKind::Struct,
        "interface_declaration" => SymbolKind::Interface,
        "library_declaration" => SymbolKind::Module, // Libraries are like modules
        "enum_declaration" => SymbolKind::Enum,
        "import_directive" => SymbolKind::Import,
        "state_variable_declaration" => SymbolKind::Const, // State variables are like constants
        _ => SymbolKind::Unknown,
    };

    // Process based on symbol kind
    match node.kind() {
        "function_definition" | "modifier_definition" => {
            if let Some(name) = extract_function_name(node, source) {
                let full_name = if let Some(ref contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                process_solidity_function(node, source, file_path, &full_name, data);

                // Recursively process function body
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    extract_solidity_symbols(
                        &child,
                        source,
                        file_path,
                        data,
                        Some(full_name.clone()),
                        current_contract.clone(),
                    );
                }
                return; // Don't recurse again
            }
        }
        "event_definition" => {
            if let Some(name) = extract_event_name(node, source) {
                let full_name = if let Some(ref contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                process_solidity_event(node, source, file_path, &full_name, data);
            }
        }
        "contract_declaration" | "library_declaration" | "interface_declaration" => {
            if let Some(name) = extract_contract_name(node, source) {
                process_solidity_contract(node, source, file_path, &name, data);

                // Process contract body with updated context
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    extract_solidity_symbols(
                        &child,
                        source,
                        file_path,
                        data,
                        current_function.clone(),
                        Some(name.clone()),
                    );
                }
                return; // Don't recurse again
            }
        }
        "struct_declaration" => {
            if let Some(name) = extract_struct_name(node, source) {
                let full_name = if let Some(ref contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                process_solidity_struct(node, source, file_path, &full_name, data);
            }
        }
        "enum_declaration" => {
            if let Some(name) = extract_enum_name(node, source) {
                let full_name = if let Some(ref contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                process_solidity_enum(node, source, file_path, &full_name, data);
            }
        }
        "state_variable_declaration" => {
            if let Some(name) = extract_state_variable_name(node, source) {
                let full_name = if let Some(ref contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                process_solidity_state_variable(node, source, file_path, &full_name, data);
            }
        }
        "import_directive" => {
            process_solidity_import(node, source, file_path, data);
        }
        _ => {}
    }

    // Recurse to children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_solidity_symbols(
            &child,
            source,
            file_path,
            data,
            current_function.clone(),
            current_contract.clone(),
        );
    }
}

/// Process a Solidity function and add to ExtractedData
fn process_solidity_function(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let visibility = extract_visibility(node, source);
    let is_public = visibility != "private" && visibility != "internal";
    let mutability = extract_mutability(node, source);
    let params = extract_params(node, source);
    let return_type = extract_return_type(node, source);
    let is_unsafe = has_unchecked_block(node, source);

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: if node.kind() == "modifier_definition" {
            "modifier"
        } else {
            "function"
        }
        .to_string(),
        line: node.start_position().row + 1,
        context: get_node_context(node, source),
    });

    // Add function fact
    data.add_function(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: false,                     // Solidity doesn't have self
        takes_mut_params: mutability == "payable", // payable functions modify state
        returns_result: false,                     // Solidity uses revert for errors
        returns_option: false,                     // Solidity doesn't have Option
        is_async: false,                           // Solidity doesn't have async
        is_unsafe,                                 // unchecked blocks
        is_public,
        parameter_count: params.len() as i32,
        generic_count: 0, // Solidity doesn't have generics
        parameters: params,
        return_type,
    });
}

/// Process a Solidity event and add to ExtractedData
fn process_solidity_event(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let params = extract_event_params(node, source);

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: format!("event {}", name),
        kind: "event".to_string(),
        line: node.start_position().row + 1,
        context: get_node_context(node, source),
    });

    // Add as function fact with special marker
    data.add_function(FunctionFact {
        file: file_path.to_string(),
        name: format!("event {}", name),
        takes_mut_self: false,
        takes_mut_params: false,
        returns_result: false,
        returns_option: false,
        is_async: false,
        is_unsafe: false,
        is_public: true, // Events are always public
        parameter_count: params.len() as i32,
        generic_count: 0,
        parameters: params,
        return_type: None,
    });
}

/// Process a Solidity contract/library/interface and add to ExtractedData
fn process_solidity_contract(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let type_kind = match node.kind() {
        "contract_declaration" => "contract",
        "library_declaration" => "library",
        "interface_declaration" => "interface",
        _ => "unknown",
    };

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: type_kind.to_string(),
        line: node.start_position().row + 1,
        context: get_node_context(node, source),
    });

    // Add type fact
    data.add_type(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: get_type_definition(node, source),
        kind: type_kind.to_string(),
        visibility: "public".to_string(), // Contracts are public by nature
        usage_count: 0,
    });

    // Extract inheritance relationships
    extract_solidity_inheritance(node, source, file_path, name, data);
}

/// Process a Solidity struct and add to ExtractedData
fn process_solidity_struct(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "struct".to_string(),
        line: node.start_position().row + 1,
        context: get_node_context(node, source),
    });

    // Add type fact
    data.add_type(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: get_type_definition(node, source),
        kind: "struct".to_string(),
        visibility: "public".to_string(), // Structs visibility is contextual
        usage_count: 0,
    });
}

/// Process a Solidity enum and add to ExtractedData
fn process_solidity_enum(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "enum".to_string(),
        line: node.start_position().row + 1,
        context: get_node_context(node, source),
    });

    // Add type fact
    data.add_type(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: get_type_definition(node, source),
        kind: "enum".to_string(),
        visibility: "public".to_string(), // Enums visibility is contextual
        usage_count: 0,
    });
}

/// Process a Solidity state variable and add to ExtractedData
fn process_solidity_state_variable(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let visibility = extract_visibility(node, source);
    let is_public = visibility == "public" || visibility == "external";
    let var_type = extract_variable_type(node, source);

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "state_variable".to_string(),
        line: node.start_position().row + 1,
        context: get_node_context(node, source),
    });

    // Add type fact for state variable
    data.add_type(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: var_type.unwrap_or_else(|| "unknown".to_string()),
        kind: "state_variable".to_string(),
        visibility: if is_public { "public" } else { "private" }.to_string(),
        usage_count: 0,
    });
}

/// Process a Solidity import and add to ExtractedData
fn process_solidity_import(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    if let Some((imported_item, import_path, is_external)) = extract_import_details(node, source) {
        data.add_import(ImportFact {
            file: file_path.to_string(),
            import_path: import_path.clone(),
            imported_names: vec![imported_item],
            import_kind: if is_external { "external" } else { "relative" }.to_string(),
            line_number: (node.start_position().row + 1) as i32,
        });
    }
}

/// Extract call expressions and add to ExtractedData
fn extract_solidity_calls(
    node: &Node,
    source: &[u8],
    file_path: &str,
    current_function: Option<&str>,
    data: &mut ExtractedData,
) {
    let line_number = (node.start_position().row + 1) as i32;

    match node.kind() {
        "call_expression" | "function_call" => {
            if let Some(caller) = current_function {
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        data.add_call_edge(CallGraphEntry::new(
                            caller.to_string(),
                            callee.to_string(),
                            file_path.to_string(),
                            CallType::Direct,
                            line_number,
                        ));
                    }
                }
            }
        }
        "member_expression" => {
            // Handle method calls like contract.method()
            if let Some(parent) = node.parent() {
                if parent.kind() == "call_expression" || parent.kind() == "function_call" {
                    if let Some(caller) = current_function {
                        if let Some(property) = node.child_by_field_name("property") {
                            if let Ok(callee) = property.utf8_text(source) {
                                data.add_call_edge(CallGraphEntry::new(
                                    caller.to_string(),
                                    callee.to_string(),
                                    file_path.to_string(),
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
                    if let Some(contract_name) =
                        text.strip_prefix("new ").and_then(|s| s.split('(').next())
                    {
                        data.add_call_edge(CallGraphEntry::new(
                            caller.to_string(),
                            format!("new {}", contract_name.trim()),
                            file_path.to_string(),
                            CallType::Constructor,
                            line_number,
                        ));
                    }
                }
            }
        }
        _ => {}
    }
}

/// Extract Solidity contract inheritance relationships
fn extract_solidity_inheritance(
    node: &Node,
    source: &[u8],
    file_path: &str,
    contract_name: &str,
    data: &mut ExtractedData,
) {
    use crate::commands::scrape::code::extracted_data::ConstantFact;

    // Look for inheritance_specifier nodes
    // In Solidity: contract Token is ERC20, Ownable { ... }
    // The tree-sitter grammar has "inheritance_specifier" for the "is" clause
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "inheritance_specifier" {
            // Walk through the inheritance specifier to find base contracts
            let mut inherit_cursor = child.walk();
            for inherit_child in child.children(&mut inherit_cursor) {
                // Base contracts appear as type identifiers or user_defined_types
                if matches!(
                    inherit_child.kind(),
                    "type_name" | "user_defined_type" | "identifier"
                ) {
                    if let Ok(base_name) = inherit_child.utf8_text(source) {
                        let base_clean = base_name.trim();

                        // Store inheritance as a special constant fact
                        data.constants.push(ConstantFact {
                            file: file_path.to_string(),
                            name: format!("{}::inherits::{}", contract_name, base_clean),
                            value: None, // Solidity doesn't have access specifiers for inheritance
                            const_type: "inheritance".to_string(),
                            scope: contract_name.to_string(),
                            line: child.start_position().row + 1,
                        });

                        // Also add as a symbol for searchability
                        data.add_symbol(CodeSymbol {
                            path: file_path.to_string(),
                            name: format!("{} : {}", contract_name, base_clean),
                            kind: "inheritance".to_string(),
                            line: child.start_position().row + 1,
                            context: format!(
                                "contract {} inherits from {}",
                                contract_name, base_clean
                            ),
                        });
                    }
                }
            }
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

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
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility" {
            if let Ok(vis) = child.utf8_text(source) {
                return vis.to_string();
            }
        }
    }
    "internal".to_string() // Default visibility in Solidity
}

/// Extract state mutability (pure, view, payable)
fn extract_mutability(node: &Node, source: &[u8]) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "state_mutability" {
            if let Ok(mut_text) = child.utf8_text(source) {
                return mut_text.to_string();
            }
        }
    }
    "nonpayable".to_string() // Default mutability
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
    node.child_by_field_name("return_type")
        .and_then(|r| r.utf8_text(source).ok())
        .map(String::from)
        .or_else(|| {
            // Look for returns keyword
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "return_parameters" {
                    if let Ok(ret_text) = child.utf8_text(source) {
                        return Some(ret_text.to_string());
                    }
                }
            }
            None
        })
}

/// Extract variable type from state variable declaration
fn extract_variable_type(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("type")
        .and_then(|t| t.utf8_text(source).ok())
        .map(String::from)
}

/// Check if function has unchecked blocks (Solidity's unsafe)
fn has_unchecked_block(node: &Node, source: &[u8]) -> bool {
    let mut has_unchecked = false;
    let mut cursor = node.walk();

    fn check_unchecked(node: &Node, _source: &[u8], has_unchecked: &mut bool) {
        if node.kind() == "unchecked_block" {
            *has_unchecked = true;
            return;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            check_unchecked(&child, _source, has_unchecked);
        }
    }

    for child in node.children(&mut cursor) {
        check_unchecked(&child, source, &mut has_unchecked);
    }

    has_unchecked
}

/// Extract import details
fn extract_import_details(node: &Node, source: &[u8]) -> Option<(String, String, bool)> {
    if let Ok(import_text) = node.utf8_text(source) {
        // Simple import parsing - can be improved with proper AST traversal
        let import_clean = import_text
            .trim_start_matches("import ")
            .trim_end_matches(';')
            .trim();

        // Check for various import styles
        if import_clean.contains(" from ") {
            let parts: Vec<&str> = import_clean.split(" from ").collect();
            if parts.len() == 2 {
                let imported = parts[0].trim().trim_matches('{').trim_matches('}');
                let path = parts[1].trim().trim_matches('"').trim_matches('\'');
                let is_external = !path.starts_with('.');
                return Some((imported.to_string(), path.to_string(), is_external));
            }
        } else if import_clean.contains(" as ") {
            let parts: Vec<&str> = import_clean.split(" as ").collect();
            if parts.len() == 2 {
                let path = parts[0].trim().trim_matches('"').trim_matches('\'');
                let alias = parts[1].trim();
                let is_external = !path.starts_with('.');
                return Some((alias.to_string(), path.to_string(), is_external));
            }
        } else {
            // Simple import
            let path = import_clean.trim_matches('"').trim_matches('\'');
            let is_external = !path.starts_with('.');
            let imported = path.split('/').next_back().unwrap_or(path);
            return Some((imported.to_string(), path.to_string(), is_external));
        }
    }
    None
}

/// Extract NatSpec documentation
fn extract_natspec(node: &Node, source: &[u8]) -> String {
    // Look for comment nodes immediately before this node
    if let Some(prev) = node.prev_sibling() {
        if prev.kind() == "comment" {
            if let Ok(text) = prev.utf8_text(source) {
                return clean_natspec_comment(text);
            }
        }
    }
    String::new()
}

/// Clean NatSpec documentation comment
fn clean_natspec_comment(raw: &str) -> String {
    raw.lines()
        .map(|line| {
            line.trim_start()
                .strip_prefix("///")
                .or_else(|| line.strip_prefix("/**"))
                .or_else(|| line.strip_prefix("*/"))
                .or_else(|| line.strip_prefix("*"))
                .unwrap_or(line)
                .trim()
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Get the context around a node (first line of the node, or NatSpec if available)
fn get_node_context(node: &Node, source: &[u8]) -> String {
    // First try to get NatSpec documentation
    let natspec = extract_natspec(node, source);
    if !natspec.is_empty() {
        return natspec;
    }

    // Fall back to first line of the node
    if let Ok(text) = node.utf8_text(source) {
        text.lines().next().unwrap_or("").to_string()
    } else {
        String::new()
    }
}

/// Get the type definition text
fn get_type_definition(node: &Node, source: &[u8]) -> String {
    if let Ok(text) = node.utf8_text(source) {
        // Take first 200 chars or first 3 lines, whichever is shorter
        let lines: Vec<&str> = text.lines().take(3).collect();
        let preview = lines.join("\n");
        if preview.len() > 200 {
            format!("{}...", &preview[..200])
        } else {
            preview
        }
    } else {
        String::new()
    }
}
