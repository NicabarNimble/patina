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

use crate::commands::scrape::code::database::{
    CodeSymbol, FunctionFact, ImportFact, TypeFact,
};
use crate::commands::scrape::code::extracted_data::{ExtractedData, ConstantFact, MemberFact};
use crate::commands::scrape::code::types::{CallGraphEntry, CallType, FilePath, SymbolKind};
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
                    // Extract enum values
                    process_c_enum_values(node, source, file_path, name, data);
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
        "preproc_def" => {
            // Extract #define macros
            process_c_macro(node, source, file_path, data);
        }
        "declaration" => {
            // Extract global variables and constants
            // Only process if we're at file scope (not inside a function)
            if current_function.is_none() {
                process_c_declaration(node, source, file_path, data);
            }
        }
        "call_expression" => {
            // Track function calls for call graph
            if let Some(ref caller) = current_function {
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        data.add_call_edge(CallGraphEntry::new(
                            caller.clone(),
                            callee.to_string(),
                            file_path.to_string(),
                            CallType::Direct,
                            (node.start_position().row + 1) as i32,
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
    
    // Extract struct/union fields
    if matches!(kind, SymbolKind::Struct | SymbolKind::Union) {
        process_c_struct_fields(node, source, file_path, name, data);
    }
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

/// Process enum values and add to ExtractedData
fn process_c_enum_values(
    node: &Node, 
    source: &[u8], 
    file_path: &str, 
    enum_name: &str,
    data: &mut ExtractedData,
) {
    // Look for enumerator_list child
    if let Some(list_node) = node.child_by_field_name("body") {
        let mut cursor = list_node.walk();
        for child in list_node.children(&mut cursor) {
            if child.kind() == "enumerator" {
                // Get the enumerator name
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(value_name) = name_node.utf8_text(source) {
                        // Try to get explicit value if present
                        let value = child
                            .child_by_field_name("value")
                            .and_then(|v| v.utf8_text(source).ok())
                            .map(|s| s.to_string());
                        
                        // Add as symbol for backwards compatibility
                        let full_name = format!("{}::{}", enum_name, value_name);
                        let context = if let Some(val) = &value {
                            format!("{} = {}", value_name, val)
                        } else {
                            value_name.to_string()
                        };
                        
                        data.add_symbol(CodeSymbol {
                            path: file_path.to_string(),
                            name: full_name.clone(),
                            kind: "enum_value".to_string(),
                            line: child.start_position().row + 1,
                            context,
                        });
                        
                        // Add as ConstantFact for better organization
                        data.add_constant(ConstantFact {
                            file: file_path.to_string(),
                            name: format!("{}::{}", enum_name, value_name),
                            value: value.clone(),
                            const_type: "enum_value".to_string(),
                            scope: enum_name.to_string(),
                            line: child.start_position().row + 1,
                        });
                    }
                }
            }
        }
    }
}

/// Process global variable/constant declarations
fn process_c_declaration(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    // Check for storage class specifiers (static, extern, const)
    let mut is_static = false;
    let mut is_const = false;
    let mut is_extern = false;
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "storage_class_specifier" => {
                if let Ok(text) = child.utf8_text(source) {
                    match text {
                        "static" => is_static = true,
                        "extern" => is_extern = true,
                        _ => {}
                    }
                }
            }
            "type_qualifier" => {
                if let Ok(text) = child.utf8_text(source) {
                    if text == "const" {
                        is_const = true;
                    }
                }
            }
            "init_declarator" | "declarator" => {
                // Extract variable name
                if let Some(name) = extract_declarator_name(&child, source) {
                    let kind = if is_const {
                        "const"
                    } else if is_static {
                        "static"
                    } else if is_extern {
                        "extern"
                    } else {
                        "global"
                    };
                    
                    // Get the full declaration as context
                    let context = node
                        .utf8_text(source)
                        .ok()
                        .and_then(|s| s.lines().next())
                        .unwrap_or("")
                        .to_string();
                    
                    // Add as symbol for backwards compatibility
                    data.add_symbol(CodeSymbol {
                        path: file_path.to_string(),
                        name: name.clone(),
                        kind: kind.to_string(),
                        line: node.start_position().row + 1,
                        context: context.clone(),
                    });
                    
                    // Add as ConstantFact for better organization
                    let const_type = if is_const {
                        "const"
                    } else if is_static {
                        "static" 
                    } else if is_extern {
                        "extern"
                    } else {
                        "global"
                    }.to_string();
                    
                    data.add_constant(ConstantFact {
                        file: file_path.to_string(),
                        name: name.clone(),
                        value: None, // Could extract initializer value here
                        const_type,
                        scope: "global".to_string(),
                        line: node.start_position().row + 1,
                    });
                }
            }
            _ => {}
        }
    }
}

/// Extract name from a declarator node
fn extract_declarator_name(node: &Node, source: &[u8]) -> Option<String> {
    // Handle simple identifiers
    if node.kind() == "identifier" {
        return node.utf8_text(source).ok().map(|s| s.to_string());
    }
    
    // Handle nested declarators (pointers, arrays, etc.)
    let mut current = Some(*node);
    while let Some(n) = current {
        if n.kind() == "identifier" {
            return n.utf8_text(source).ok().map(|s| s.to_string());
        }
        // Try to find identifier in children
        let mut cursor = n.walk();
        current = n.children(&mut cursor).find(|c| 
            c.kind() == "identifier" || 
            c.kind() == "declarator" || 
            c.kind() == "pointer_declarator" ||
            c.kind() == "array_declarator"
        );
    }
    None
}

/// Process struct/union fields and add them as MemberFacts
fn process_c_struct_fields(
    node: &Node,
    source: &[u8],
    file_path: &str,
    struct_name: &str,
    data: &mut ExtractedData,
) {
    // Look for field_declaration_list child
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "field_declaration" {
                // Extract field type
                let field_type = if let Some(type_node) = child.child_by_field_name("type") {
                    type_node.utf8_text(source).ok()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                } else {
                    "unknown".to_string()
                };
                
                // Extract field declarators (there can be multiple in one declaration)
                if let Some(declarator) = child.child_by_field_name("declarator") {
                    if let Some(field_name) = extract_declarator_name(&declarator, source) {
                        // Add as MemberFact
                        data.members.push(MemberFact {
                            file: file_path.to_string(),
                            container: struct_name.to_string(),
                            name: field_name.clone(),
                            member_type: "field".to_string(),
                            visibility: "public".to_string(), // C struct fields are always public
                            modifiers: vec![],
                            line: child.start_position().row + 1,
                        });
                        
                        // Also add as symbol for searchability with type info
                        data.add_symbol(CodeSymbol {
                            path: file_path.to_string(),
                            name: format!("{}::{}", struct_name, field_name),
                            kind: "field".to_string(),
                            line: child.start_position().row + 1,
                            context: format!("{} {}", field_type, field_name),
                        });
                    }
                }
            }
        }
    }
}

/// Process a #define macro and add to ExtractedData  
fn process_c_macro(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    // Get the macro name
    if let Some(name_node) = node.child_by_field_name("name") {
        if let Ok(name) = name_node.utf8_text(source) {
            // Try to get the value if present
            let value = node
                .child_by_field_name("value")
                .and_then(|v| v.utf8_text(source).ok())
                .map(|s| s.to_string());
            
            // Get the full macro text for context
            let context = node
                .utf8_text(source)
                .ok()
                .and_then(|s| s.lines().next())
                .unwrap_or("")
                .to_string();
            
            // Add as a symbol for backwards compatibility
            data.add_symbol(CodeSymbol {
                path: file_path.to_string(),
                name: name.to_string(),
                kind: "macro".to_string(),
                line: node.start_position().row + 1,
                context,
            });
            
            // Add as a ConstantFact for better organization
            data.add_constant(ConstantFact {
                file: file_path.to_string(),
                name: name.to_string(),
                value: value.clone(),
                const_type: "macro".to_string(),
                scope: "global".to_string(),
                line: node.start_position().row + 1,
            });
        }
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
