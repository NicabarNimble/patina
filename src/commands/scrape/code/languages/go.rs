// ============================================================================
// GO LANGUAGE PROCESSOR V2 - STRUCT-BASED
// ============================================================================
//! Go language processor that returns typed structs instead of SQL strings.
//!
//! This is the refactored version that:
//! - Returns ExtractedData with typed structs
//! - No SQL string generation
//! - Direct data extraction to domain types
//!
//! Handles Go's unique features:
//! - Exported vs unexported (capitalization-based visibility)
//! - Interfaces and struct embedding
//! - Goroutines and channels
//! - Multiple return values
//! - Package-level declarations

use crate::commands::scrape::code::database::{
    CodeSymbol, FunctionFact, ImportFact, TypeFact,
};
use crate::commands::scrape::code::extracted_data::{ConstantFact, ExtractedData, MemberFact};
use crate::commands::scrape::code::types::{CallGraphEntry, CallType, FilePath, SymbolKind};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// Go language processor - returns typed structs
pub struct GoProcessor;

impl GoProcessor {
    /// Process a Go file and extract all symbols to typed structs
    pub fn process_file(file_path: FilePath, content: &[u8]) -> Result<ExtractedData> {
        let mut data = ExtractedData::new();

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

        // Walk the AST and extract symbols
        extract_go_symbols(
            &tree.root_node(),
            content,
            &file_path.to_string(),
            &mut data,
            None,
        );

        Ok(data)
    }
}

/// Recursively extract symbols from the Go AST
fn extract_go_symbols(
    node: &Node,
    source: &[u8],
    file_path: &str,
    data: &mut ExtractedData,
    current_function: Option<String>,
) {
    // First extract any calls
    extract_go_calls(node, source, file_path, &current_function, data);

    // Handle package declaration
    if node.kind() == "package_clause" {
        // Package identifier is a direct child, not a field
        let mut pkg_cursor = node.walk();
        for child in node.children(&mut pkg_cursor) {
            if child.kind() == "package_identifier" {
                if let Ok(package_name) = child.utf8_text(source) {
                    // Store package as a special constant
                    data.add_constant(ConstantFact {
                        file: file_path.to_string(),
                        name: format!("package:{}", package_name),
                        value: Some(package_name.to_string()),
                        const_type: "package".to_string(),
                        scope: "file".to_string(),
                        line: node.start_position().row + 1,
                    });
                    break;
                }
            }
        }
        return; // Package clause is fully processed
    }

    // Handle const declarations
    if node.kind() == "const_declaration" {
        process_go_constants(node, source, file_path, data);
        // Don't recurse further for const declarations
        return;
    }

    // Handle var declarations (check if global)
    if node.kind() == "var_declaration" {
        // Check if this is at package level (global)
        if current_function.is_none() {
            process_go_globals(node, source, file_path, data);
        }
        // Don't recurse further for var declarations
        return;
    }

    // Determine symbol kind for other nodes
    let symbol_kind = match node.kind() {
        "function_declaration" | "method_declaration" => SymbolKind::Function,
        "type_declaration" => SymbolKind::TypeAlias,
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
    if symbol_kind == SymbolKind::Function {
        if let Some(name) = extract_function_name(node, source) {
            process_go_function(node, source, file_path, &name, data);

            // Extract calls within this function
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                extract_go_symbols(&child, source, file_path, data, Some(name.clone()));
            }
            return; // Don't recurse again
        }
    } else if matches!(
        symbol_kind,
        SymbolKind::Struct | SymbolKind::Trait | SymbolKind::TypeAlias | SymbolKind::Enum
    ) {
        if let Some((name, kind)) = extract_type_info(node, source) {
            process_go_type(node, source, file_path, &name, kind, data);
        }
    } else if symbol_kind == SymbolKind::Import {
        process_go_import(node, source, file_path, data);
    }

    // Recurse to children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_go_symbols(&child, source, file_path, data, current_function.clone());
    }
}

/// Process a Go function and add to ExtractedData
fn process_go_function(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = is_exported(name);
    let params = extract_params(node, source);
    let return_type = extract_return_type(node, source);
    let _generics = extract_generics(node, source);

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "function".to_string(),
        line: node.start_position().row + 1,
        context: get_node_context(node, source),
    });

    // Add function fact
    data.add_function(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: false,   // Go doesn't have self
        takes_mut_params: false, // Go passes by value by default
        returns_result: false,   // Go uses multiple returns for errors
        returns_option: false,   // Go uses nil
        is_async: false,         // Go uses goroutines
        is_unsafe: false,        // Go doesn't have unsafe keyword
        is_public,
        parameter_count: params.len() as i32,
        generic_count: if _generics.is_some() { 1 } else { 0 },
        parameters: params,
        return_type,
    });
}

/// Process a Go type and add to ExtractedData
fn process_go_type(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: SymbolKind,
    data: &mut ExtractedData,
) {
    let is_public = is_exported(name);

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: kind.to_string(),
        line: node.start_position().row + 1,
        context: get_node_context(node, source),
    });

    // Add type fact
    data.add_type(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: get_type_definition(node, source),
        kind: kind.to_string(),
        visibility: if is_public { "public" } else { "private" }.to_string(),
        usage_count: 0, // Will be populated later
    });

    // Extract struct fields or interface methods
    if kind == SymbolKind::Struct {
        extract_struct_fields(node, source, file_path, name, data);
    } else if kind == SymbolKind::Trait {
        // In Go, Trait represents interfaces
        extract_interface_methods(node, source, file_path, name, data);
    }
}

/// Process a Go import and add to ExtractedData
fn process_go_import(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    // Handle both single imports and import blocks
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "import_spec" {
            if let Some((import_path, alias)) = extract_import_spec(&child, source) {
                let imported_names = if let Some(alias) = alias {
                    vec![alias]
                } else {
                    // Extract package name from path
                    vec![import_path
                        .split('/')
                        .next_back()
                        .unwrap_or(&import_path)
                        .to_string()]
                };

                data.add_import(ImportFact {
                    file: file_path.to_string(),
                    import_path: import_path.clone(),
                    imported_names,
                    import_kind: if import_path.starts_with('.') {
                        "relative"
                    } else {
                        "external"
                    }
                    .to_string(),
                    line_number: (node.start_position().row + 1) as i32,
                });
            }
        } else if child.kind() == "import_spec_list" {
            // Handle import blocks
            let mut list_cursor = child.walk();
            for spec in child.children(&mut list_cursor) {
                if spec.kind() == "import_spec" {
                    if let Some((import_path, alias)) = extract_import_spec(&spec, source) {
                        let imported_names = if let Some(alias) = alias {
                            vec![alias]
                        } else {
                            vec![import_path
                                .split('/')
                                .next_back()
                                .unwrap_or(&import_path)
                                .to_string()]
                        };

                        data.add_import(ImportFact {
                            file: file_path.to_string(),
                            import_path: import_path.clone(),
                            imported_names,
                            import_kind: if import_path.starts_with('.') {
                                "relative"
                            } else {
                                "external"
                            }
                            .to_string(),
                            line_number: (spec.start_position().row + 1) as i32,
                        });
                    }
                }
            }
        }
    }
}

/// Extract import spec details
fn extract_import_spec(node: &Node, source: &[u8]) -> Option<(String, Option<String>)> {
    let mut alias = None;
    let mut path = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "package_identifier" | "dot" | "blank_identifier" => {
                if let Ok(text) = child.utf8_text(source) {
                    alias = Some(text.to_string());
                }
            }
            "interpreted_string_literal" => {
                if let Ok(text) = child.utf8_text(source) {
                    path = Some(text.trim_matches('"').to_string());
                }
            }
            _ => {}
        }
    }

    path.map(|p| (p, alias))
}

/// Extract call expressions and add to ExtractedData
fn extract_go_calls(
    node: &Node,
    source: &[u8],
    file_path: &str,
    current_function: &Option<String>,
    data: &mut ExtractedData,
) {
    let line_number = (node.start_position().row + 1) as i32;

    match node.kind() {
        "call_expression" => {
            if let Some(caller) = current_function {
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        data.add_call_edge(CallGraphEntry::new(
                            caller.clone(),
                            callee.to_string(),
                            file_path.to_string(),
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
                                data.add_call_edge(CallGraphEntry::new(
                                    caller.clone(),
                                    callee.to_string(),
                                    file_path.to_string(),
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
                                data.add_call_edge(CallGraphEntry::new(
                                    caller.clone(),
                                    callee.to_string(),
                                    file_path.to_string(),
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
                                data.add_call_edge(CallGraphEntry::new(
                                    caller.clone(),
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
        _ => {}
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

/// Get the context around a node (first line of the node)
fn get_node_context(node: &Node, source: &[u8]) -> String {
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

/// Process Go constants (both single and blocks with iota)
fn process_go_constants(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    let mut cursor = node.walk();
    let mut iota_counter = 0;
    
    for child in node.children(&mut cursor) {
        if child.kind() == "const_spec" {
            // Extract constant name and value
            let mut const_name = None;
            let mut const_value = None;
            let mut const_type = None;
            
            let mut spec_cursor = child.walk();
            for spec_child in child.children(&mut spec_cursor) {
                match spec_child.kind() {
                    "identifier" => {
                        if const_name.is_none() {
                            const_name = spec_child.utf8_text(source).ok().map(String::from);
                        }
                    }
                    "type_identifier" | "qualified_type" => {
                        const_type = spec_child.utf8_text(source).ok().map(String::from);
                    }
                    "expression_list" => {
                        if let Ok(text) = spec_child.utf8_text(source) {
                            const_value = Some(text.to_string());
                            // Check for iota
                            if text.contains("iota") {
                                const_value = Some(format!("{} (={})", text, iota_counter));
                            }
                        }
                    }
                    _ => {
                        // Check for direct expression
                        if spec_child.kind().ends_with("_literal") || spec_child.kind() == "identifier" {
                            if const_value.is_none() {
                                const_value = spec_child.utf8_text(source).ok().map(String::from);
                            }
                        }
                    }
                }
            }
            
            if let Some(name) = const_name {
                // If no value is specified in a const block, use iota
                if const_value.is_none() && node.child_count() > 1 {
                    const_value = Some(iota_counter.to_string());
                }
                
                data.add_constant(ConstantFact {
                    file: file_path.to_string(),
                    name: name.clone(),
                    value: const_value,
                    const_type: "const".to_string(),
                    scope: "global".to_string(),
                    line: child.start_position().row + 1,
                });
                
                // Also add as symbol for search
                data.add_symbol(CodeSymbol {
                    path: file_path.to_string(),
                    name,
                    kind: "constant".to_string(),
                    line: child.start_position().row + 1,
                    context: get_node_context(&child, source),
                });
                
                iota_counter += 1;
            }
        }
    }
}

/// Process Go global variables
fn process_go_globals(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    let mut cursor = node.walk();
    
    for child in node.children(&mut cursor) {
        if child.kind() == "var_spec" {
            // Extract variable name and value
            let mut var_name = None;
            let mut var_value = None;
            let mut var_type = None;
            
            let mut spec_cursor = child.walk();
            for spec_child in child.children(&mut spec_cursor) {
                match spec_child.kind() {
                    "identifier" => {
                        if var_name.is_none() {
                            var_name = spec_child.utf8_text(source).ok().map(String::from);
                        }
                    }
                    "type_identifier" | "qualified_type" | "pointer_type" | "slice_type" | "map_type" => {
                        var_type = spec_child.utf8_text(source).ok().map(String::from);
                    }
                    "expression_list" => {
                        var_value = spec_child.utf8_text(source).ok().map(String::from);
                    }
                    _ => {}
                }
            }
            
            if let Some(name) = var_name {
                data.add_constant(ConstantFact {
                    file: file_path.to_string(),
                    name: name.clone(),
                    value: var_value.or(var_type),
                    const_type: "global".to_string(),
                    scope: "global".to_string(),
                    line: child.start_position().row + 1,
                });
                
                // Also add as symbol for search
                data.add_symbol(CodeSymbol {
                    path: file_path.to_string(),
                    name,
                    kind: "variable".to_string(),
                    line: child.start_position().row + 1,
                    context: get_node_context(&child, source),
                });
            }
        }
    }
}

/// Extract struct fields
fn extract_struct_fields(node: &Node, source: &[u8], file_path: &str, struct_name: &str, data: &mut ExtractedData) {
    // Find the struct_type node
    let struct_node = if node.kind() == "type_spec" {
        node.child_by_field_name("type")
    } else {
        Some(*node)
    };
    
    if let Some(struct_node) = struct_node {
        if struct_node.kind() == "struct_type" {
            let mut cursor = struct_node.walk();
            for child in struct_node.children(&mut cursor) {
                if child.kind() == "field_declaration_list" {
                    let mut field_cursor = child.walk();
                    for field in child.children(&mut field_cursor) {
                        if field.kind() == "field_declaration" {
                            extract_field_declaration(&field, source, file_path, struct_name, data);
                        }
                    }
                }
            }
        }
    }
}

/// Extract a single field declaration
fn extract_field_declaration(node: &Node, source: &[u8], file_path: &str, struct_name: &str, data: &mut ExtractedData) {
    let mut field_names = Vec::new();
    let mut field_type = None;
    let mut field_tag = None;
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "field_identifier" => {
                if let Ok(name) = child.utf8_text(source) {
                    field_names.push(name.to_string());
                }
            }
            "type_identifier" | "pointer_type" | "slice_type" | "map_type" | "qualified_type" => {
                field_type = child.utf8_text(source).ok().map(String::from);
            }
            "tag" => {
                field_tag = child.utf8_text(source).ok().map(String::from);
            }
            _ => {}
        }
    }
    
    // Add each field name as a member
    for name in field_names {
        let visibility = if is_exported(&name) { "public" } else { "private" };
        let mut modifiers = Vec::new();
        if field_tag.is_some() {
            modifiers.push("tagged".to_string());
        }
        
        data.add_member(MemberFact {
            file: file_path.to_string(),
            container: struct_name.to_string(),
            name: name.clone(),
            member_type: "field".to_string(),
            visibility: visibility.to_string(),
            modifiers,
            line: node.start_position().row + 1,
        });
    }
}

/// Extract interface methods
fn extract_interface_methods(node: &Node, source: &[u8], file_path: &str, interface_name: &str, data: &mut ExtractedData) {
    // Find the interface_type node
    let interface_node = if node.kind() == "type_spec" {
        node.child_by_field_name("type")
    } else {
        Some(*node)
    };
    
    if let Some(interface_node) = interface_node {
        if interface_node.kind() == "interface_type" {
            // Interface methods and embedded types are direct children of interface_type
            let mut cursor = interface_node.walk();
            for child in interface_node.children(&mut cursor) {
                match child.kind() {
                    "method_elem" => {
                        // Extract method name using the name field
                        if let Some(name_node) = child.child_by_field_name("name") {
                            if let Ok(method_name) = name_node.utf8_text(source) {
                                let visibility = if is_exported(method_name) { "public" } else { "private" };
                                
                                data.add_member(MemberFact {
                                    file: file_path.to_string(),
                                    container: interface_name.to_string(),
                                    name: method_name.to_string(),
                                    member_type: "method".to_string(),
                                    visibility: visibility.to_string(),
                                    modifiers: vec!["abstract".to_string()], // Interface methods are abstract
                                    line: child.start_position().row + 1,
                                });
                            }
                        }
                    }
                    "type_elem" => {
                        // Handle embedded interfaces - type_elem contains the embedded type
                        let mut type_cursor = child.walk();
                        for type_child in child.children(&mut type_cursor) {
                            if matches!(type_child.kind(), "type_identifier" | "qualified_type") {
                                if let Ok(embedded_name) = type_child.utf8_text(source) {
                                    data.add_member(MemberFact {
                                        file: file_path.to_string(),
                                        container: interface_name.to_string(),
                                        name: embedded_name.to_string(),
                                        member_type: "embedded".to_string(),
                                        visibility: "public".to_string(),
                                        modifiers: vec!["embedded".to_string()],
                                        line: type_child.start_position().row + 1,
                                    });
                                }
                            }
                        }
                    }
                    _ => {} // Skip punctuation nodes like "{", "}", etc.
                }
            }
        }
    }
}
