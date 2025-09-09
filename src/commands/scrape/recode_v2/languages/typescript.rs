// ============================================================================
// TYPESCRIPT LANGUAGE PROCESSOR V2 - STRUCT-BASED
// ============================================================================
//! TypeScript language processor that returns typed structs instead of SQL strings.
//!
//! This is the refactored version that:
//! - Returns ExtractedData with typed structs
//! - No SQL string generation
//! - Direct data extraction to domain types
//!
//! Handles TypeScript's unique features:
//! - Static typing with type annotations
//! - Access modifiers (public, private, protected)
//! - Interfaces and type aliases
//! - Generics and type parameters
//! - Enums and advanced type features
//! - Decorators and metadata
//! - JSX support (.tsx files with different parser)

use crate::commands::scrape::recode_v2::database::{
    CodeSymbol, FunctionFact, ImportFact, TypeFact,
};
use crate::commands::scrape::recode_v2::extracted_data::ExtractedData;
use crate::commands::scrape::recode_v2::types::{CallGraphEntry, CallType, FilePath};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// TypeScript language processor - returns typed structs
pub struct TypeScriptProcessor;

impl TypeScriptProcessor {
    /// Process a TypeScript file and extract all symbols to typed structs
    pub fn process_file(file_path: FilePath, content: &[u8]) -> Result<ExtractedData> {
        let mut data = ExtractedData::new();

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
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No TypeScript parser available for extension: {}",
                    extension
                )
            })?;
        parser
            .set_language(&language)
            .context("Failed to set TypeScript language")?;

        // Parse the file
        let tree = parser
            .parse(content, None)
            .context("Failed to parse TypeScript file")?;

        // Track current function/class for call graph
        let mut current_function: Option<String> = None;

        // Walk the tree and extract symbols
        extract_symbols(
            tree.root_node(),
            content,
            &file_path,
            &mut data,
            &mut current_function,
        );

        Ok(data)
    }
}

/// Recursively extract symbols from the syntax tree
fn extract_symbols(
    node: Node,
    source: &[u8],
    file_path: &FilePath,
    data: &mut ExtractedData,
    current_function: &mut Option<String>,
) {
    // First extract any calls
    extract_calls(&node, source, file_path, current_function, data);

    // Process based on node kind
    match node.kind() {
        // Function declarations and expressions
        "function_declaration"
        | "function_expression"
        | "arrow_function"
        | "generator_function_declaration" => {
            if let Some(name) = extract_function_name(&node, source) {
                let old_function = current_function.clone();
                *current_function = Some(name.clone());

                process_function(&node, source, file_path, &name, data);

                // Recursively process function body
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    extract_symbols(child, source, file_path, data, current_function);
                }

                *current_function = old_function;
                return; // Don't recurse again
            }
        }

        // Method definitions and signatures
        "method_definition" | "method_signature" => {
            if let Some(name) = extract_method_name(&node, source) {
                process_method(&node, source, file_path, &name, data);

                let old_function = current_function.clone();
                *current_function = Some(name);

                // Process method body if it exists (method_signature has no body)
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        extract_symbols(child, source, file_path, data, current_function);
                    }
                }

                *current_function = old_function;
                return;
            }
        }

        // Class declarations
        "class_declaration" | "class_expression" => {
            if let Some(name) = extract_class_name(&node, source) {
                process_class(&node, source, file_path, &name, data);

                // Process class body
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        // For methods, include class name in context
                        if matches!(child.kind(), "method_definition" | "method_signature") {
                            if let Some(method_name) = extract_method_name(&child, source) {
                                let full_name = format!("{}.{}", name, method_name);
                                let old_function = current_function.clone();
                                *current_function = Some(full_name.clone());

                                process_method(&child, source, file_path, &full_name, data);

                                // Process method body if it exists
                                if let Some(method_body) = child.child_by_field_name("body") {
                                    let mut method_cursor = method_body.walk();
                                    for method_child in method_body.children(&mut method_cursor) {
                                        extract_symbols(
                                            method_child,
                                            source,
                                            file_path,
                                            data,
                                            current_function,
                                        );
                                    }
                                }

                                *current_function = old_function;
                            }
                        } else {
                            extract_symbols(child, source, file_path, data, current_function);
                        }
                    }
                }
                return; // Don't recurse again into class
            }
        }

        // Interface declarations
        "interface_declaration" => {
            if let Some(name) = extract_interface_name(&node, source) {
                process_interface(&node, source, file_path, &name, data);
            }
        }

        // Type alias declarations
        "type_alias_declaration" => {
            if let Some(name) = extract_type_alias_name(&node, source) {
                process_type_alias(&node, source, file_path, &name, data);
            }
        }

        // Enum declarations
        "enum_declaration" => {
            if let Some(name) = extract_enum_name(&node, source) {
                process_enum(&node, source, file_path, &name, data);
            }
        }

        // Variable declarations that might be functions
        "lexical_declaration" | "variable_declaration" => {
            process_variable_declaration(&node, source, file_path, data, current_function);
        }

        // Import statements
        "import_statement" => {
            process_import(&node, source, file_path, data);
        }

        // Export statements
        "export_statement" => {
            // Process the declaration inside the export
            if let Some(decl) = node.child_by_field_name("declaration") {
                extract_symbols(decl, source, file_path, data, current_function);
            } else if let Some(decl) = node.child_by_field_name("value") {
                extract_symbols(decl, source, file_path, data, current_function);
            }
        }

        _ => {}
    }

    // Recurse to children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_symbols(child, source, file_path, data, current_function);
    }
}

/// Process a TypeScript function and add to data
fn process_function(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = extract_visibility(node, source);
    let is_async = is_async_function(node, source);
    let is_generator = node.kind() == "generator_function_declaration";
    let params = extract_params(node, source);
    let return_type = extract_return_type(node, source);
    let generics = extract_generics(node, source);

    // Create function fact
    let function = FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: false,   // Not applicable in TS
        takes_mut_params: false, // TS doesn't have explicit mutability
        returns_result: return_type
            .as_ref()
            .is_some_and(|rt| rt.contains("Promise")),
        returns_option: return_type
            .as_ref()
            .is_some_and(|rt| rt.contains("undefined") || rt.contains("null") || rt.contains("?")),
        is_async,
        is_unsafe: false, // No unsafe in TS
        is_public,
        parameter_count: params.len() as i32,
        generic_count: count_generics(&generics),
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
        kind: if is_generator {
            "generator"
        } else {
            "function"
        }
        .to_string(),
        line: node.start_position().row + 1,
        context,
    };
    data.add_symbol(symbol);
}

/// Process a method definition or signature
fn process_method(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    name: &str,
    data: &mut ExtractedData,
) {
    let visibility = extract_visibility(node, source);
    let is_async = is_async_function(node, source);
    let is_static = has_static_keyword(node, source);
    let is_abstract = has_abstract_keyword(node, source);
    let is_getter = is_getter_method(node, source);
    let is_setter = is_setter_method(node, source);

    let params = extract_params(node, source);
    let return_type = extract_return_type(node, source);
    let generics = extract_generics(node, source);

    // Create function fact for method
    let function = FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: !is_static,
        takes_mut_params: false,
        returns_result: return_type
            .as_ref()
            .is_some_and(|rt| rt.contains("Promise")),
        returns_option: return_type
            .as_ref()
            .is_some_and(|rt| rt.contains("undefined") || rt.contains("null") || rt.contains("?")),
        is_async,
        is_unsafe: false,
        is_public: visibility,
        parameter_count: params.len() as i32,
        generic_count: count_generics(&generics),
        parameters: params,
        return_type,
    };
    data.add_function(function);

    // Add to code search
    let kind = if is_getter {
        "getter"
    } else if is_setter {
        "setter"
    } else if is_abstract {
        "abstract_method"
    } else if is_static {
        "static_method"
    } else {
        "method"
    };

    let context = node
        .utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .to_string();

    let symbol = CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: kind.to_string(),
        line: node.start_position().row + 1,
        context,
    };
    data.add_symbol(symbol);
}

/// Process a class declaration
fn process_class(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = extract_visibility(node, source);
    let is_abstract = has_abstract_keyword(node, source);

    // Get the class definition line
    let definition = node
        .utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .to_string();

    // Create type fact
    let type_fact = TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: definition.clone(),
        kind: if is_abstract {
            "abstract_class"
        } else {
            "class"
        }
        .to_string(),
        visibility: if is_public { "public" } else { "private" }.to_string(),
        usage_count: 0,
    };
    data.add_type(type_fact);

    // Add to code search
    let symbol = CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: if is_abstract {
            "abstract_class"
        } else {
            "class"
        }
        .to_string(),
        line: node.start_position().row + 1,
        context: definition,
    };
    data.add_symbol(symbol);
}

/// Process an interface declaration
fn process_interface(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = extract_visibility(node, source);

    // Get the interface definition line
    let definition = node
        .utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .to_string();

    // Create type fact
    let type_fact = TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: definition.clone(),
        kind: "interface".to_string(),
        visibility: if is_public { "public" } else { "private" }.to_string(),
        usage_count: 0,
    };
    data.add_type(type_fact);

    // Add to code search
    let symbol = CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "interface".to_string(),
        line: node.start_position().row + 1,
        context: definition,
    };
    data.add_symbol(symbol);
}

/// Process a type alias declaration
fn process_type_alias(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = extract_visibility(node, source);

    // Get the type alias definition
    let definition = node
        .utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .to_string();

    // Create type fact
    let type_fact = TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: definition.clone(),
        kind: "type_alias".to_string(),
        visibility: if is_public { "public" } else { "private" }.to_string(),
        usage_count: 0,
    };
    data.add_type(type_fact);

    // Add to code search
    let symbol = CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "type_alias".to_string(),
        line: node.start_position().row + 1,
        context: definition,
    };
    data.add_symbol(symbol);
}

/// Process an enum declaration
fn process_enum(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = extract_visibility(node, source);
    let is_const = has_const_keyword(node, source);

    // Get the enum definition line
    let definition = node
        .utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .to_string();

    // Create type fact
    let type_fact = TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: definition.clone(),
        kind: if is_const { "const_enum" } else { "enum" }.to_string(),
        visibility: if is_public { "public" } else { "private" }.to_string(),
        usage_count: 0,
    };
    data.add_type(type_fact);

    // Add to code search
    let symbol = CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: if is_const { "const_enum" } else { "enum" }.to_string(),
        line: node.start_position().row + 1,
        context: definition,
    };
    data.add_symbol(symbol);
}

/// Process variable declarations (const/let/var)
fn process_variable_declaration(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    data: &mut ExtractedData,
    current_function: &mut Option<String>,
) {
    // Check each declarator in the declaration
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            if let Some(name_node) = child.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    // Check if it's a function assignment
                    if let Some(value_node) = child.child_by_field_name("value") {
                        match value_node.kind() {
                            "arrow_function" | "function_expression" => {
                                let old_function = current_function.clone();
                                *current_function = Some(name.to_string());

                                process_function(&value_node, source, file_path, name, data);

                                // Process function body
                                if let Some(body) = value_node.child_by_field_name("body") {
                                    let mut body_cursor = body.walk();
                                    for body_child in body.children(&mut body_cursor) {
                                        extract_symbols(
                                            body_child,
                                            source,
                                            file_path,
                                            data,
                                            current_function,
                                        );
                                    }
                                }

                                *current_function = old_function;
                            }
                            "class_expression" => {
                                process_class(&value_node, source, file_path, name, data);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

/// Process TypeScript import statements
fn process_import(node: &Node, source: &[u8], file_path: &FilePath, data: &mut ExtractedData) {
    let import_text = node.utf8_text(source).unwrap_or("");

    // Extract module path
    let module_path = extract_import_path(node, source).unwrap_or_default();

    // Extract imported items
    let mut imported_names = Vec::new();

    // Named imports: import { a, b } from 'module'
    if let Some(import_clause) = node.child_by_field_name("import") {
        let mut cursor = import_clause.walk();
        for child in import_clause.children(&mut cursor) {
            match child.kind() {
                "named_imports" => {
                    let mut import_cursor = child.walk();
                    for import_child in child.children(&mut import_cursor) {
                        if import_child.kind() == "import_specifier" {
                            // Handle both name and alias
                            if let Some(name_node) = import_child.child_by_field_name("name") {
                                if let Ok(name) = name_node.utf8_text(source) {
                                    // Check for alias
                                    if let Some(alias_node) =
                                        import_child.child_by_field_name("alias")
                                    {
                                        if let Ok(alias) = alias_node.utf8_text(source) {
                                            imported_names.push(format!("{} as {}", name, alias));
                                        } else {
                                            imported_names.push(name.to_string());
                                        }
                                    } else {
                                        imported_names.push(name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                "identifier" => {
                    // Default import
                    if let Ok(name) = child.utf8_text(source) {
                        imported_names.push(name.to_string());
                    }
                }
                "namespace_import" => {
                    // import * as name from 'module'
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if let Ok(name) = name_node.utf8_text(source) {
                            imported_names.push(format!("* as {}", name));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Type-only imports: import type { ... } from 'module'
    let is_type_import = import_text.starts_with("import type");

    if imported_names.is_empty() && !module_path.is_empty() {
        // Side-effect import: import 'module'
        imported_names.push("*".to_string());
    }

    let is_external = !module_path.starts_with('.') && !module_path.starts_with('/');

    let import = ImportFact {
        file: file_path.to_string(),
        import_path: module_path,
        imported_names,
        import_kind: if is_type_import {
            "type_import"
        } else if is_external {
            "external"
        } else {
            "internal"
        }
        .to_string(),
        line_number: (node.start_position().row + 1) as i32,
    };
    data.add_import(import);

    // Add to code search
    let symbol = CodeSymbol {
        path: file_path.to_string(),
        name: import_text.to_string(),
        kind: if is_type_import {
            "type_import"
        } else {
            "import"
        }
        .to_string(),
        line: node.start_position().row + 1,
        context: import_text.to_string(),
    };
    data.add_symbol(symbol);
}

/// Extract function name
fn extract_function_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(String::from)
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

/// Extract function parameters with types
fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
    let params_node = node
        .child_by_field_name("parameters")
        .or_else(|| node.child_by_field_name("parameter"));

    if let Some(params) = params_node {
        let mut result = Vec::new();
        let mut cursor = params.walk();
        for child in params.children(&mut cursor) {
            match child.kind() {
                "required_parameter" | "optional_parameter" => {
                    if let Ok(param_text) = child.utf8_text(source) {
                        result.push(param_text.to_string());
                    }
                }
                "identifier" | "rest_pattern" | "object_pattern" | "array_pattern" => {
                    if let Ok(param_text) = child.utf8_text(source) {
                        result.push(param_text.to_string());
                    }
                }
                _ => {}
            }
        }
        result
    } else {
        Vec::new()
    }
}

/// Extract return type annotation
fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("return_type")
        .and_then(|rt| rt.child_by_field_name("type"))
        .and_then(|t| t.utf8_text(source).ok())
        .map(String::from)
}

/// Extract generic type parameters
fn extract_generics(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("type_parameters")
        .and_then(|tp| tp.utf8_text(source).ok())
        .map(String::from)
}

/// Count generic parameters
fn count_generics(generics: &Option<String>) -> i32 {
    generics.as_ref().map_or(0, |g| {
        // Simple count of commas + 1, accounting for <T> vs <T, U>
        if g.contains('<') && g.contains('>') {
            g.matches(',').count() as i32 + 1
        } else {
            0
        }
    })
}

/// Extract visibility modifier (public/private/protected)
fn extract_visibility(node: &Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Ok(text) = child.utf8_text(source) {
            match text {
                "public" | "export" => return true,
                "private" | "protected" => return false,
                _ => {}
            }
        }
    }
    // Default to public for top-level declarations without explicit modifier
    true
}

/// Extract import path from import statement
fn extract_import_path(node: &Node, source: &[u8]) -> Option<String> {
    // Look for string node containing the path
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "string" {
            if let Ok(path_text) = child.utf8_text(source) {
                return Some(
                    path_text
                        .trim_matches(|c| c == '"' || c == '\'' || c == '`')
                        .to_string(),
                );
            }
        }
    }
    None
}

/// Check if node has async keyword
fn is_async_function(node: &Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "async" {
            return true;
        }
    }
    // Also check in the node's text for arrow functions
    node.utf8_text(source)
        .is_ok_and(|text| text.starts_with("async "))
}

/// Check if method has static keyword
fn has_static_keyword(node: &Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Ok(text) = child.utf8_text(source) {
            if text == "static" {
                return true;
            }
        }
    }
    false
}

/// Check if has abstract keyword
fn has_abstract_keyword(node: &Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Ok(text) = child.utf8_text(source) {
            if text == "abstract" {
                return true;
            }
        }
    }
    false
}

/// Check if has const keyword (for enums)
fn has_const_keyword(node: &Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Ok(text) = child.utf8_text(source) {
            if text == "const" {
                return true;
            }
        }
    }
    false
}

/// Check if method is a getter
fn is_getter_method(node: &Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Ok(text) = child.utf8_text(source) {
            if text == "get" {
                return true;
            }
        }
    }
    false
}

/// Check if method is a setter
fn is_setter_method(node: &Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Ok(text) = child.utf8_text(source) {
            if text == "set" {
                return true;
            }
        }
    }
    false
}

/// Extract call expressions and add to data
fn extract_calls(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    current_function: &Option<String>,
    data: &mut ExtractedData,
) {
    let line_number = (node.start_position().row + 1) as i32;

    match node.kind() {
        "call_expression" => {
            if let Some(caller) = current_function {
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        // Check for await
                        let call_type = if node
                            .parent()
                            .is_some_and(|p| p.kind() == "await_expression")
                        {
                            CallType::Async
                        } else {
                            CallType::Direct
                        };

                        data.add_call_edge(CallGraphEntry::new(
                            caller.clone(),
                            callee.to_string(),
                            file_path.to_string(),
                            call_type,
                            line_number,
                        ));
                    }
                }
            }
        }
        "await_expression" => {
            // Handle await expressions
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
                                    CallType::Async,
                                    line_number,
                                ));
                            }
                        }
                    }
                }
            }
        }
        "new_expression" => {
            // Handle constructor calls
            if let Some(caller) = current_function {
                if let Some(constructor_node) = node.child_by_field_name("constructor") {
                    if let Ok(callee) = constructor_node.utf8_text(source) {
                        data.add_call_edge(CallGraphEntry::new(
                            caller.clone(),
                            format!("new {}", callee),
                            file_path.to_string(),
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
                    data.add_call_edge(CallGraphEntry::new(
                        caller.clone(),
                        decorator_text.to_string(),
                        file_path.to_string(),
                        CallType::Decorator,
                        line_number,
                    ));
                }
            }
        }
        _ => {}
    }
}
