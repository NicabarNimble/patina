// ============================================================================
// RUST LANGUAGE PROCESSOR V2 - STRUCT-BASED
// ============================================================================
//! Rust language processor that returns typed structs instead of SQL strings.
//!
//! This is the refactored version that:
//! - Returns ExtractedData with typed structs
//! - No SQL string generation
//! - Direct data extraction to domain types

use crate::commands::scrape::code::database::{CodeSymbol, FunctionFact, ImportFact, TypeFact};
use crate::commands::scrape::code::extracted_data::{ConstantFact, ExtractedData, MemberFact};
use crate::commands::scrape::code::types::{CallGraphEntry, CallType, FilePath, SymbolKind};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// Rust language processor - returns typed structs
pub struct RustProcessor;

impl RustProcessor {
    /// Process a Rust file and extract all symbols to typed structs
    pub fn process_file(file_path: FilePath, content: &[u8]) -> Result<ExtractedData> {
        let mut data = ExtractedData::new();

        // Set up tree-sitter parser for Rust
        let mut parser = Parser::new();
        let metal = patina_metal::Metal::Rust;
        let language = metal
            .tree_sitter_language_for_ext("rs")
            .ok_or_else(|| anyhow::anyhow!("No Rust parser available"))?;
        parser
            .set_language(&language)
            .context("Failed to set Rust language")?;

        // Parse the file
        let tree = parser
            .parse(content, None)
            .context("Failed to parse Rust file")?;

        // Walk the AST and extract symbols
        extract_rust_symbols(
            &tree.root_node(),
            content,
            &file_path.to_string(),
            &mut data,
            None,
        );

        Ok(data)
    }
}

/// Recursively extract symbols from the Rust AST
fn extract_rust_symbols(
    node: &Node,
    source: &[u8],
    file_path: &str,
    data: &mut ExtractedData,
    current_function: Option<String>,
) {
    // Determine symbol type
    let symbol_kind = match node.kind() {
        "function_item" => SymbolKind::Function,
        "struct_item" => SymbolKind::Struct,
        "enum_item" => SymbolKind::Enum,
        "trait_item" => SymbolKind::Trait,
        "type_alias" => SymbolKind::TypeAlias,
        "const_item" => SymbolKind::Const,
        "static_item" => SymbolKind::Static,
        "impl_item" => SymbolKind::Impl,
        "mod_item" => SymbolKind::Module,
        "use_declaration" => SymbolKind::Import,
        "macro_definition" => SymbolKind::Macro,
        _ => SymbolKind::Unknown,
    };

    // Process based on symbol type
    if symbol_kind == SymbolKind::Function {
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(name) = name_node.utf8_text(source) {
                process_rust_function(node, source, file_path, name, data);

                // Extract calls within this function
                extract_rust_calls(node, source, file_path, &Some(name.to_string()), data);
            }
        }
    } else if matches!(symbol_kind, SymbolKind::Const | SymbolKind::Static) {
        // Process constants and statics separately for ConstantFact
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(name) = name_node.utf8_text(source) {
                process_rust_constant(node, source, file_path, name, symbol_kind, data);
            }
        }
    } else if matches!(symbol_kind, SymbolKind::Struct | SymbolKind::Enum) {
        // Process structs and enums with member extraction
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(name) = name_node.utf8_text(source) {
                process_rust_type_with_members(node, source, file_path, name, symbol_kind, data);
            }
        }
    } else if matches!(
        symbol_kind,
        SymbolKind::Trait | SymbolKind::TypeAlias | SymbolKind::Impl
    ) {
        if symbol_kind == SymbolKind::Impl {
            // Special handling for impl blocks
            process_rust_impl(node, source, file_path, data);
        } else if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(name) = name_node.utf8_text(source) {
                process_rust_type(node, source, file_path, name, symbol_kind, data);
            }
        }
    } else if symbol_kind == SymbolKind::Macro {
        // Process macro definitions
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(name) = name_node.utf8_text(source) {
                process_rust_macro(node, source, file_path, name, data);
            }
        }
    } else if symbol_kind == SymbolKind::Import {
        process_rust_import(node, source, file_path, data);
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_rust_symbols(&child, source, file_path, data, current_function.clone());
    }
}

/// Process a Rust function and add to data
fn process_rust_function(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    // Extract function details
    let params = extract_params(node, source);
    let return_type = extract_return_type(node, source);
    let is_public = has_visibility_modifier(node);
    let is_async = has_async(node);
    let is_unsafe = has_unsafe(node);

    // Check for specific patterns
    let takes_mut_self = params.iter().any(|p| p.contains("&mut self"));
    let takes_mut_params = params
        .iter()
        .any(|p| p.contains("&mut ") && !p.contains("self"));
    let returns_result = return_type.as_deref().unwrap_or("").contains("Result");
    let returns_option = return_type.as_deref().unwrap_or("").contains("Option");

    // Count generics
    let generic_count = node
        .child_by_field_name("type_parameters")
        .map(|n| n.named_child_count() as i32)
        .unwrap_or(0);

    // Create function fact
    let function = FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self,
        takes_mut_params,
        returns_result,
        returns_option,
        is_async,
        is_unsafe,
        is_public,
        parameter_count: params.len() as i32,
        generic_count,
        parameters: params,
        return_type,
    };
    data.add_function(function);

    // Add to code search
    let context = node
        .utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .to_string();

    let symbol = CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "function".to_string(),
        line: node.start_position().row + 1,
        context,
    };
    data.add_symbol(symbol);
}

/// Process a Rust type and add to data
fn process_rust_type(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: SymbolKind,
    data: &mut ExtractedData,
) {
    let is_public = has_visibility_modifier(node);

    // Get the node text for definition
    let definition = node
        .utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .to_string();

    let kind_str = match kind {
        SymbolKind::Struct => "struct",
        SymbolKind::Enum => "enum",
        SymbolKind::Trait => "trait",
        SymbolKind::TypeAlias => "type_alias",
        SymbolKind::Const => {
            if node.kind() == "static_item" {
                "static"
            } else {
                "const"
            }
        }
        _ => "unknown",
    };

    // Create type fact
    let type_fact = TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: definition.clone(),
        kind: kind_str.to_string(),
        visibility: if is_public { "pub" } else { "private" }.to_string(),
        usage_count: 0,
    };
    data.add_type(type_fact);

    // Add to code search
    let symbol = CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: kind_str.to_string(),
        line: node.start_position().row + 1,
        context: definition,
    };
    data.add_symbol(symbol);
}

/// Process Rust constants and statics
fn process_rust_constant(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: SymbolKind,
    data: &mut ExtractedData,
) {
    // Extract value if present
    let value = node
        .child_by_field_name("value")
        .and_then(|v| v.utf8_text(source).ok())
        .map(|s| s.to_string());

    // Determine const type
    let const_type = if kind == SymbolKind::Static {
        "static"
    } else {
        "const"
    };

    // Check visibility
    let is_public = has_visibility_modifier(node);
    let visibility = if is_public { "pub" } else { "private" };

    // Create constant fact
    let constant = ConstantFact {
        file: file_path.to_string(),
        name: name.to_string(),
        value,
        const_type: const_type.to_string(),
        scope: "module".to_string(), // Will be enhanced with module tracking
        line: node.start_position().row + 1,
    };
    data.constants.push(constant);

    // Also add as type fact for compatibility
    let definition = node
        .utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .to_string();

    let type_fact = TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: definition.clone(),
        kind: const_type.to_string(),
        visibility: visibility.to_string(),
        usage_count: 0,
    };
    data.add_type(type_fact);

    // Add to code search
    let symbol = CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: const_type.to_string(),
        line: node.start_position().row + 1,
        context: definition,
    };
    data.add_symbol(symbol);
}

/// Process structs and enums with member extraction
fn process_rust_type_with_members(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: SymbolKind,
    data: &mut ExtractedData,
) {
    // First process as regular type
    process_rust_type(node, source, file_path, name, kind, data);

    // Then extract members
    if kind == SymbolKind::Struct {
        // Look for field_declaration_list or ordered_field_declaration_list
        if let Some(body) = node.child_by_field_name("body") {
            extract_struct_fields(&body, source, file_path, name, data);
        }
    } else if kind == SymbolKind::Enum {
        // Look for enum_variant_list
        if let Some(body) = node.child_by_field_name("body") {
            extract_enum_variants(&body, source, file_path, name, data);
        }
    }
}

/// Extract struct fields
fn extract_struct_fields(
    body_node: &Node,
    source: &[u8],
    file_path: &str,
    struct_name: &str,
    data: &mut ExtractedData,
) {
    let mut cursor = body_node.walk();
    for child in body_node.children(&mut cursor) {
        if child.kind() == "field_declaration" {
            if let Some(name_node) = child.child_by_field_name("name") {
                if let Ok(field_name) = name_node.utf8_text(source) {
                    // Check for visibility modifier
                    let visibility = if has_visibility_modifier(&child) {
                        "pub"
                    } else {
                        "private"
                    };

                    // Get field type if available
                    let field_type = child
                        .child_by_field_name("type")
                        .and_then(|t| t.utf8_text(source).ok())
                        .map(|s| s.to_string());

                    let mut modifiers = Vec::new();
                    if let Some(ref ft) = field_type {
                        modifiers.push(ft.clone());
                    }

                    let member = MemberFact {
                        file: file_path.to_string(),
                        container: struct_name.to_string(),
                        name: field_name.to_string(),
                        member_type: "field".to_string(),
                        visibility: visibility.to_string(),
                        modifiers,
                        line: child.start_position().row + 1,
                    };
                    data.members.push(member);
                }
            }
        } else if child.kind() == "shorthand_field_identifier" {
            // Tuple struct fields (ordered_field_declaration_list)
            // These don't have names, just types in order
            if let Ok(field_type) = child.utf8_text(source) {
                let member = MemberFact {
                    file: file_path.to_string(),
                    container: struct_name.to_string(),
                    name: format!("_{}", data.members.len()), // Unnamed field
                    member_type: "field".to_string(),
                    visibility: "pub".to_string(), // Tuple fields are typically public
                    modifiers: vec![field_type.to_string()],
                    line: child.start_position().row + 1,
                };
                data.members.push(member);
            }
        }
    }
}

/// Extract enum variants
fn extract_enum_variants(
    body_node: &Node,
    source: &[u8],
    file_path: &str,
    enum_name: &str,
    data: &mut ExtractedData,
) {
    let mut cursor = body_node.walk();
    for child in body_node.children(&mut cursor) {
        if child.kind() == "enum_variant" {
            if let Some(name_node) = child.child_by_field_name("name") {
                if let Ok(variant_name) = name_node.utf8_text(source) {
                    // Check for discriminant value
                    let value = child
                        .child_by_field_name("value")
                        .and_then(|v| v.utf8_text(source).ok())
                        .map(|s| s.to_string());

                    // Store as ConstantFact
                    let constant = ConstantFact {
                        file: file_path.to_string(),
                        name: format!("{}::{}", enum_name, variant_name),
                        value,
                        const_type: "enum_variant".to_string(),
                        scope: enum_name.to_string(),
                        line: child.start_position().row + 1,
                    };
                    data.constants.push(constant);

                    // Also store as MemberFact for consistency
                    let mut modifiers = Vec::new();

                    // Check if variant has fields (struct-like or tuple-like)
                    if let Some(body) = child.child_by_field_name("body") {
                        if body.kind() == "field_declaration_list" {
                            modifiers.push("struct_variant".to_string());
                        } else if body.kind() == "ordered_field_declaration_list" {
                            modifiers.push("tuple_variant".to_string());
                        }
                    }

                    let member = MemberFact {
                        file: file_path.to_string(),
                        container: enum_name.to_string(),
                        name: variant_name.to_string(),
                        member_type: "variant".to_string(),
                        visibility: "pub".to_string(), // Enum variants are always public
                        modifiers,
                        line: child.start_position().row + 1,
                    };
                    data.members.push(member);
                }
            }
        }
    }
}

/// Process macro definitions
fn process_rust_macro(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    // Get macro definition (first line for context)
    let definition = node
        .utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .to_string();

    // Store as ConstantFact
    let constant = ConstantFact {
        file: file_path.to_string(),
        name: format!("{}!", name), // Add ! to indicate it's a macro
        value: Some(definition.clone()),
        const_type: "macro_definition".to_string(),
        scope: "module".to_string(),
        line: node.start_position().row + 1,
    };
    data.constants.push(constant);

    // Also add as type fact for compatibility
    let type_fact = TypeFact {
        file: file_path.to_string(),
        name: format!("{}!", name),
        definition: definition.clone(),
        kind: "macro".to_string(),
        visibility: "pub".to_string(), // Most macros are public
        usage_count: 0,
    };
    data.add_type(type_fact);

    // Add to code search
    let symbol = CodeSymbol {
        path: file_path.to_string(),
        name: format!("{}!", name),
        kind: "macro".to_string(),
        line: node.start_position().row + 1,
        context: definition,
    };
    data.add_symbol(symbol);
}

/// Process impl blocks for trait implementations
fn process_rust_impl(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    // Check if this is a trait implementation
    if let Some(trait_node) = node.child_by_field_name("trait") {
        if let Ok(trait_name) = trait_node.utf8_text(source) {
            // Get the type being implemented for
            if let Some(type_node) = node.child_by_field_name("type") {
                if let Ok(type_name) = type_node.utf8_text(source) {
                    // Store trait implementation as a ConstantFact
                    let constant = ConstantFact {
                        file: file_path.to_string(),
                        name: format!("impl {} for {}", trait_name, type_name),
                        value: None,
                        const_type: "trait_impl".to_string(),
                        scope: "module".to_string(),
                        line: node.start_position().row + 1,
                    };
                    data.constants.push(constant);
                }
            }
        }
    }

    // Process methods within the impl block
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            extract_rust_symbols(&child, source, file_path, data, None);
        }
    }
}

/// Process Rust imports and add to data
fn process_rust_import(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    // Find the use_tree to extract the import path
    if let Some(use_tree) = node.child_by_field_name("argument") {
        if let Ok(import_text) = use_tree.utf8_text(source) {
            // Clean up the import path
            let import_path = import_text.trim();

            // Extract imported names
            let imported_names = if import_path.contains('{') {
                // Multiple imports: use foo::{Bar, Baz}
                if let Some(start) = import_path.find('{') {
                    if let Some(end) = import_path.find('}') {
                        import_path[start + 1..end]
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect()
                    } else {
                        vec![import_path.to_string()]
                    }
                } else {
                    vec![import_path.to_string()]
                }
            } else if let Some(last_part) = import_path.split("::").last() {
                // Single import: use foo::Bar
                vec![last_part.to_string()]
            } else {
                vec![import_path.to_string()]
            };

            // Create import fact
            let import = ImportFact {
                file: file_path.to_string(),
                import_path: import_path.to_string(),
                imported_names,
                import_kind: "use".to_string(),
                line_number: (node.start_position().row + 1) as i32,
            };
            data.add_import(import);

            // Add to code search
            let symbol = CodeSymbol {
                path: file_path.to_string(),
                name: import_path.to_string(),
                kind: "import".to_string(),
                line: node.start_position().row + 1,
                context: format!("use {};", import_path),
            };
            data.add_symbol(symbol);
        }
    }
}

/// Extract function calls for call graph
fn extract_rust_calls(
    node: &Node,
    source: &[u8],
    file_path: &str,
    current_function: &Option<String>,
    data: &mut ExtractedData,
) {
    if let Some(ref caller) = current_function {
        // Look for different types of calls
        match node.kind() {
            "call_expression" => {
                if let Some(function_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = function_node.utf8_text(source) {
                        let call_edge = CallGraphEntry::new(
                            caller.clone(),
                            callee.to_string(),
                            file_path.to_string(),
                            CallType::Direct,
                            (node.start_position().row + 1) as i32,
                        );
                        data.add_call_edge(call_edge);
                    }
                }
            }
            "macro_invocation" => {
                if let Some(macro_node) = node.child_by_field_name("macro") {
                    if let Ok(macro_name) = macro_node.utf8_text(source) {
                        let call_edge = CallGraphEntry::new(
                            caller.clone(),
                            format!("{}!", macro_name),
                            file_path.to_string(),
                            CallType::Macro,
                            (node.start_position().row + 1) as i32,
                        );
                        data.add_call_edge(call_edge);
                    }
                }
            }
            _ => {}
        }
    }

    // Recursively look for calls in children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_rust_calls(&child, source, file_path, current_function, data);
    }
}

// Helper functions (same as original but simplified)

fn has_visibility_modifier(node: &Node) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            return true;
        }
    }
    false
}

fn has_async(node: &Node) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "async" {
            return true;
        }
    }
    false
}

fn has_unsafe(node: &Node) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "unsafe" {
            return true;
        }
    }
    false
}

fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut params = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if child.kind() == "parameter" || child.kind() == "self_parameter" {
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

fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("return_type")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.trim_start_matches("->").trim().to_string())
}
