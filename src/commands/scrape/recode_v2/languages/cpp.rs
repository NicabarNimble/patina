// ============================================================================
// C++ LANGUAGE PROCESSOR V2 - STRUCT-BASED
// ============================================================================
//! C++ language processor that returns typed structs instead of SQL strings.
//!
//! This is the refactored version that:
//! - Returns ExtractedData with typed structs
//! - No SQL string generation
//! - Direct data extraction to domain types
//! - Uses iterative approach for nested declarators to avoid stack overflow
//!
//! Handles C++'s features:
//! - Classes with access modifiers (public/private/protected)
//! - Templates and template specialization
//! - Namespaces
//! - Function overloading
//! - RAII and constructors/destructors
//! - Modern C++ features (auto, lambdas, etc.)

use crate::commands::scrape::recode_v2::database::{
    CallEdge, CodeSymbol, FunctionFact, ImportFact, TypeFact,
};
use crate::commands::scrape::recode_v2::extracted_data::ExtractedData;
use crate::commands::scrape::recode_v2::types::{CallType, FilePath, SymbolKind};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// C++ language processor - returns typed structs
pub struct CppProcessor;

impl CppProcessor {
    /// Process a C++ file and extract all symbols to typed structs
    pub fn process_file(file_path: FilePath, content: &[u8]) -> Result<ExtractedData> {
        let mut data = ExtractedData::new();

        // Set up tree-sitter parser for C++
        let mut parser = Parser::new();
        let metal = patina_metal::Metal::Cpp;
        let language = metal
            .tree_sitter_language_for_ext("cpp")
            .ok_or_else(|| anyhow::anyhow!("No C++ parser available"))?;
        parser
            .set_language(&language)
            .context("Failed to set C++ language")?;

        // Parse the file
        let tree = parser
            .parse(content, None)
            .context("Failed to parse C++ file")?;

        // Walk the AST and extract symbols
        let mut namespace_stack = Vec::new();
        extract_cpp_symbols(
            &tree.root_node(),
            content,
            &file_path.to_string(),
            &mut data,
            None,
            &mut namespace_stack,
        );

        Ok(data)
    }
}

/// Recursively extract symbols from the C++ AST
fn extract_cpp_symbols(
    node: &Node,
    source: &[u8],
    file_path: &str,
    data: &mut ExtractedData,
    current_function: Option<String>,
    namespace_stack: &mut Vec<String>,
) {
    match node.kind() {
        "namespace_definition" => {
            // Enter namespace
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    namespace_stack.push(name.to_string());
                }
            }

            // Process namespace body
            if let Some(body) = node.child_by_field_name("body") {
                let mut cursor = body.walk();
                for child in body.children(&mut cursor) {
                    extract_cpp_symbols(
                        &child,
                        source,
                        file_path,
                        data,
                        current_function.clone(),
                        namespace_stack,
                    );
                }
            }

            // Exit namespace
            namespace_stack.pop();
            return; // Don't recurse again
        }
        "function_definition" => {
            if let Some(name) = extract_function_name(node, source) {
                // Include namespace in function name
                let full_name = if namespace_stack.is_empty() {
                    name.clone()
                } else {
                    format!("{}::{}", namespace_stack.join("::"), name)
                };

                process_cpp_function(node, source, file_path, &full_name, data);

                // Process function body with updated context
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    extract_cpp_symbols(
                        &child,
                        source,
                        file_path,
                        data,
                        Some(full_name.clone()),
                        namespace_stack,
                    );
                }
                return; // Don't recurse again
            }
        }
        "class_specifier" | "struct_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    let kind = if node.kind() == "class_specifier" {
                        SymbolKind::Class
                    } else {
                        SymbolKind::Struct
                    };

                    // Include namespace in type name
                    let full_name = if namespace_stack.is_empty() {
                        name.to_string()
                    } else {
                        format!("{}::{}", namespace_stack.join("::"), name)
                    };

                    process_cpp_class(node, source, file_path, &full_name, kind, data);

                    // Process class body with updated namespace
                    namespace_stack.push(name.to_string());
                    if let Some(body) = node.child_by_field_name("body") {
                        let mut cursor = body.walk();
                        for child in body.children(&mut cursor) {
                            extract_cpp_symbols(
                                &child,
                                source,
                                file_path,
                                data,
                                current_function.clone(),
                                namespace_stack,
                            );
                        }
                    }
                    namespace_stack.pop();
                    return; // Don't recurse again
                }
            }
        }
        "enum_specifier" | "enum_class_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    let full_name = if namespace_stack.is_empty() {
                        name.to_string()
                    } else {
                        format!("{}::{}", namespace_stack.join("::"), name)
                    };
                    process_cpp_enum(node, source, file_path, &full_name, data);
                }
            }
        }
        "type_definition" | "alias_declaration" => {
            if let Some(name) = extract_typedef_name(node, source) {
                let full_name = if namespace_stack.is_empty() {
                    name.clone()
                } else {
                    format!("{}::{}", namespace_stack.join("::"), name)
                };
                process_cpp_typedef(node, source, file_path, &full_name, data);
            }
        }
        "preproc_include" | "preproc_import" => {
            process_cpp_include(node, source, file_path, data);
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
        "new_expression" => {
            // Constructor calls
            if let Some(ref caller) = current_function {
                if let Some(type_node) = node.child_by_field_name("type") {
                    if let Ok(class_name) = type_node.utf8_text(source) {
                        data.add_call_edge(CallEdge {
                            caller: caller.clone(),
                            callee: format!("{}::constructor", class_name),
                            file: file_path.to_string(),
                            call_type: CallType::Constructor.to_string(),
                            line_number: (node.start_position().row + 1) as i32,
                        });
                    }
                }
            }
        }
        "delete_expression" => {
            // Destructor calls
            if let Some(ref caller) = current_function {
                if let Some(arg_node) = node.child_by_field_name("argument") {
                    if let Ok(var_name) = arg_node.utf8_text(source) {
                        data.add_call_edge(CallEdge {
                            caller: caller.clone(),
                            callee: format!("~{}", var_name),
                            file: file_path.to_string(),
                            call_type: CallType::Destructor.to_string(),
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
        extract_cpp_symbols(
            &child,
            source,
            file_path,
            data,
            current_function.clone(),
            namespace_stack,
        );
    }
}

/// Process a C++ function and add to ExtractedData
fn process_cpp_function(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let params = extract_parameters(node, source);
    let return_type = extract_return_type(node, source);
    let _is_template = has_template_parent(node);
    let is_public = is_public_member(node, source);

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
        takes_mut_self: false, // Would need more analysis
        takes_mut_params: params.iter().any(|p| !p.contains("const")),
        returns_result: false, // C++ uses exceptions
        returns_option: return_type.as_ref().is_some_and(|r| r.contains("optional")),
        is_async: false, // C++ doesn't have built-in async
        is_unsafe: true, // All C++ is unsafe
        is_public,
        parameter_count: params.len() as i32,
        generic_count: if _is_template { 1 } else { 0 },
        parameters: params,
        return_type,
    });
}

/// Process a C++ class/struct and add to ExtractedData
fn process_cpp_class(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: SymbolKind,
    data: &mut ExtractedData,
) {
    let _is_template = has_template_parent(node);

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
        definition: get_type_definition(node, source),
        kind: kind.to_string(),
        visibility: "public".to_string(), // Top-level types are public
        usage_count: 0,
    });
}

/// Process a C++ enum and add to ExtractedData
fn process_cpp_enum(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_enum_class = node.kind() == "enum_class_specifier";

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: if is_enum_class { "enum_class" } else { "enum" }.to_string(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });

    // Add type fact
    data.add_type(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: get_type_definition(node, source),
        kind: if is_enum_class { "enum_class" } else { "enum" }.to_string(),
        visibility: "public".to_string(),
        usage_count: 0,
    });
}

/// Process a C++ typedef/using and add to ExtractedData
fn process_cpp_typedef(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_using = node.kind() == "alias_declaration";

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: if is_using { "using" } else { "typedef" }.to_string(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });

    // Add type fact
    data.add_type(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: get_type_definition(node, source),
        kind: if is_using { "using" } else { "typedef" }.to_string(),
        visibility: "public".to_string(),
        usage_count: 0,
    });
}

/// Process a C++ include directive and add to ExtractedData
fn process_cpp_include(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    if let Ok(include_text) = node.utf8_text(source) {
        let header = include_text
            .trim_start_matches("#include")
            .trim_start_matches("#import")
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

/// Extract function name from C++ function_definition node
fn extract_function_name(node: &Node, source: &[u8]) -> Option<String> {
    // First check for simple declarator with name
    if let Some(declarator) = node.child_by_field_name("declarator") {
        return extract_cpp_function_name(declarator)
            .and_then(|n| n.utf8_text(source).ok())
            .map(|s| s.to_string());
    }

    // Fallback to standard name field
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.to_string())
}

/// Extract C++ function name from declarator (iterative to avoid stack overflow)
fn extract_cpp_function_name(declarator: Node) -> Option<Node> {
    let mut current = declarator;

    loop {
        match current.kind() {
            "identifier" | "field_identifier" | "destructor_name" | "operator_name" => {
                return Some(current);
            }
            "qualified_identifier" => {
                // For qualified names like Class::method
                if let Some(name) = current.child_by_field_name("name") {
                    return Some(name);
                }
            }
            "function_declarator" => {
                if let Some(inner) = current.child_by_field_name("declarator") {
                    current = inner;
                    continue;
                }
            }
            "pointer_declarator" | "reference_declarator" => {
                if let Some(inner) = current.child_by_field_name("declarator") {
                    current = inner;
                    continue;
                }
            }
            _ => {}
        }

        // Check children
        let mut cursor = current.walk();
        for child in current.children(&mut cursor) {
            if matches!(
                child.kind(),
                "identifier" | "field_identifier" | "destructor_name" | "operator_name"
            ) {
                return Some(child);
            }
        }

        return None;
    }
}

/// Extract typedef/using name
fn extract_typedef_name(node: &Node, source: &[u8]) -> Option<String> {
    // For using declarations
    if node.kind() == "alias_declaration" {
        return node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok())
            .map(|s| s.to_string());
    }

    // For typedef
    if let Some(declarator) = node.child_by_field_name("declarator") {
        return extract_declarator_name(&declarator, source);
    }

    None
}

/// Extract name from a declarator
fn extract_declarator_name(declarator: &Node, source: &[u8]) -> Option<String> {
    if matches!(declarator.kind(), "type_identifier" | "identifier") {
        return declarator.utf8_text(source).ok().map(|s| s.to_string());
    }

    if declarator.kind() == "pointer_declarator" || declarator.kind() == "reference_declarator" {
        if let Some(inner) = declarator.child_by_field_name("declarator") {
            return extract_declarator_name(&inner, source);
        }
    }

    let mut cursor = declarator.walk();
    for child in declarator.children(&mut cursor) {
        if matches!(child.kind(), "type_identifier" | "identifier") {
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
                if matches!(
                    child.kind(),
                    "parameter_declaration" | "optional_parameter_declaration"
                ) {
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
    // Check for trailing return type (C++11)
    if let Some(trailing) = node.child_by_field_name("trailing_return_type") {
        if let Ok(text) = trailing.utf8_text(source) {
            return Some(text.trim_start_matches("->").trim().to_string());
        }
    }

    // Standard return type
    node.child_by_field_name("type")
        .and_then(|t| t.utf8_text(source).ok())
        .map(String::from)
}

/// Check if node has a template parent
fn has_template_parent(node: &Node) -> bool {
    let mut current = Some(*node);
    while let Some(n) = current {
        if n.kind() == "template_declaration" {
            return true;
        }
        current = n.parent();
    }
    false
}

/// Check if a member is public
fn is_public_member(node: &Node, source: &[u8]) -> bool {
    // Check parent for class/struct context
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "struct_specifier" => return true, // Struct members are public by default
            "class_specifier" => {
                // Class members are private by default
                // Look for access specifier before this node
                let mut is_public = false;
                let mut cursor = parent.walk();
                for sibling in parent.children(&mut cursor) {
                    if sibling.kind() == "access_specifier" {
                        if let Ok(text) = sibling.utf8_text(source) {
                            is_public = text.contains("public");
                        }
                    }
                    if sibling.start_byte() >= node.start_byte() {
                        return is_public;
                    }
                }
                return false;
            }
            _ => current = parent.parent(),
        }
    }

    // Not in a class/struct, so it's public
    true
}

/// Extract context around a symbol
fn extract_context(node: &Node, source: &[u8]) -> String {
    let start_byte = node.start_byte();
    let end_byte = node.end_byte().min(start_byte + 200);

    if let Ok(context) = std::str::from_utf8(&source[start_byte..end_byte]) {
        context.lines().take(3).collect::<Vec<_>>().join(" ")
    } else {
        String::new()
    }
}

/// Get type definition text
fn get_type_definition(node: &Node, source: &[u8]) -> String {
    if let Ok(text) = node.utf8_text(source) {
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
