// ============================================================================
// JAVASCRIPT LANGUAGE PROCESSOR V2 - STRUCT-BASED
// ============================================================================
//! JavaScript language processor that returns typed structs instead of SQL strings.
//!
//! This is the refactored version that:
//! - Returns ExtractedData with typed structs
//! - No SQL string generation
//! - Direct data extraction to domain types
//!
//! Handles JavaScript's unique features:
//! - Prototype-based inheritance
//! - Dynamic typing and duck typing
//! - Multiple function declaration styles (function, arrow, expression)
//! - ES6+ modules and CommonJS
//! - Async/await and promises
//! - Flexible parameter patterns (destructuring, rest)
//! - JSX support (same parser handles .js and .jsx)

use crate::commands::scrape::recode_v2::database::{
    CallEdge, CodeSymbol, FunctionFact, ImportFact, TypeFact,
};
use crate::commands::scrape::recode_v2::extracted_data::ExtractedData;
use crate::commands::scrape::recode_v2::types::{CallType, FilePath};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// JavaScript language processor - returns typed structs
pub struct JavaScriptProcessor;

impl JavaScriptProcessor {
    /// Process a JavaScript file and extract all symbols to typed structs
    pub fn process_file(file_path: FilePath, content: &[u8]) -> Result<ExtractedData> {
        let mut data = ExtractedData::new();

        // Set up tree-sitter parser for JavaScript
        // Note: The same parser handles both .js and .jsx files
        let mut parser = Parser::new();
        let metal = patina_metal::Metal::JavaScript;
        let language = metal
            .tree_sitter_language_for_ext("js") // Same parser for .js and .jsx
            .ok_or_else(|| anyhow::anyhow!("No JavaScript parser available"))?;
        parser
            .set_language(&language)
            .context("Failed to set JavaScript language")?;

        // Parse the file
        let tree = parser
            .parse(content, None)
            .context("Failed to parse JavaScript file")?;

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
        | "function"
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

        // Method definitions in classes
        "method_definition" => {
            if let Some(name) = extract_method_name(&node, source) {
                process_method(&node, source, file_path, &name, data);

                let old_function = current_function.clone();
                *current_function = Some(name);

                // Process method body
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
        "class_declaration" => {
            if let Some(name) = extract_class_name(&node, source) {
                process_class(&node, source, file_path, &name, data);

                // Process class body
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        // For methods, include class name in context
                        if child.kind() == "method_definition" {
                            if let Some(method_name) = extract_method_name(&child, source) {
                                let full_name = format!("{}.{}", name, method_name);
                                let old_function = current_function.clone();
                                *current_function = Some(full_name.clone());

                                process_method(&child, source, file_path, &full_name, data);

                                // Process method body
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

        // Variable declarations that might be functions
        "variable_declarator" => {
            process_variable_declarator(&node, source, file_path, data, current_function);
        }

        // Import statements
        "import_statement" => {
            process_es6_import(&node, source, file_path, data);
        }

        // CommonJS require
        "call_expression" => {
            if is_require_call(&node, source) {
                process_commonjs_require(&node, source, file_path, data);
            }
        }

        // Export statements
        "export_statement" => {
            // Process the declaration inside the export
            if let Some(decl) = node.child_by_field_name("declaration") {
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

/// Process a JavaScript function and add to data
fn process_function(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_async = has_async_keyword(node, source);
    let is_generator = node.kind() == "generator_function_declaration";
    let params = extract_parameters(node, source);
    let return_type = None; // JavaScript doesn't have static types

    // Create function fact
    let function = FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: false,   // Not applicable in JS
        takes_mut_params: false, // JS doesn't have explicit mutability
        returns_result: false,   // No Result type in JS
        returns_option: false,   // No Option type in JS
        is_async,
        is_unsafe: false, // No unsafe in JS
        is_public: true,  // JS doesn't have visibility modifiers at function level
        parameter_count: params.len() as i32,
        generic_count: 0, // JS doesn't have generics
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

/// Process a method definition
fn process_method(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_async = has_async_keyword(node, source);
    let is_static = has_static_keyword(node, source);
    let is_getter = node
        .child_by_field_name("kind")
        .and_then(|n| n.utf8_text(source).ok()) == Some("get");
    let is_setter = node
        .child_by_field_name("kind")
        .and_then(|n| n.utf8_text(source).ok()) == Some("set");

    let params = extract_parameters(node, source);

    // Create function fact for method
    let function = FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: !is_static,
        takes_mut_params: false,
        returns_result: false,
        returns_option: false,
        is_async,
        is_unsafe: false,
        is_public: true,
        parameter_count: params.len() as i32,
        generic_count: 0,
        parameters: params,
        return_type: None,
    };
    data.add_function(function);

    // Add to code search
    let kind = if is_getter {
        "getter"
    } else if is_setter {
        "setter"
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
        kind: "class".to_string(),
        visibility: "public".to_string(), // JS classes are always public
        usage_count: 0,
    };
    data.add_type(type_fact);

    // Add to code search
    let symbol = CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "class".to_string(),
        line: node.start_position().row + 1,
        context: definition,
    };
    data.add_symbol(symbol);
}

/// Process variable declarators (const/let/var)
fn process_variable_declarator(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    data: &mut ExtractedData,
    current_function: &mut Option<String>,
) {
    if let Some(name_node) = node.child_by_field_name("name") {
        if let Ok(name) = name_node.utf8_text(source) {
            // Check if it's a function assignment
            if let Some(value_node) = node.child_by_field_name("value") {
                match value_node.kind() {
                    "arrow_function" | "function" => {
                        let old_function = current_function.clone();
                        *current_function = Some(name.to_string());

                        process_function(&value_node, source, file_path, name, data);

                        // Process function body
                        if let Some(body) = value_node.child_by_field_name("body") {
                            let mut cursor = body.walk();
                            for child in body.children(&mut cursor) {
                                extract_symbols(child, source, file_path, data, current_function);
                            }
                        }

                        *current_function = old_function;
                    }
                    "class" => {
                        process_class(&value_node, source, file_path, name, data);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Process ES6 import statements
fn process_es6_import(node: &Node, source: &[u8], file_path: &FilePath, data: &mut ExtractedData) {
    let import_text = node.utf8_text(source).unwrap_or("");

    // Extract module path
    let module_path = node
        .children(&mut node.walk())
        .find(|n| n.kind() == "string")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.trim_matches(|c| c == '"' || c == '\'' || c == '`'))
        .unwrap_or("");

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
                            if let Some(name_node) = import_child.child_by_field_name("name") {
                                if let Ok(name) = name_node.utf8_text(source) {
                                    imported_names.push(name.to_string());
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

    if imported_names.is_empty() && !module_path.is_empty() {
        // Side-effect import: import 'module'
        imported_names.push("*".to_string());
    }

    let is_external = !module_path.starts_with('.') && !module_path.starts_with('/');

    let import = ImportFact {
        file: file_path.to_string(),
        import_path: module_path.to_string(),
        imported_names,
        import_kind: if is_external { "external" } else { "internal" }.to_string(),
        line_number: (node.start_position().row + 1) as i32,
    };
    data.add_import(import);

    // Add to code search
    let symbol = CodeSymbol {
        path: file_path.to_string(),
        name: import_text.to_string(),
        kind: "import".to_string(),
        line: node.start_position().row + 1,
        context: import_text.to_string(),
    };
    data.add_symbol(symbol);
}

/// Process CommonJS require statements
fn process_commonjs_require(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    data: &mut ExtractedData,
) {
    // Extract module path from require('module')
    if let Some(args_node) = node.child_by_field_name("arguments") {
        let mut cursor = args_node.walk();
        for child in args_node.children(&mut cursor) {
            if child.kind() == "string" {
                if let Ok(module_path_raw) = child.utf8_text(source) {
                    let module_path =
                        module_path_raw.trim_matches(|c| c == '"' || c == '\'' || c == '`');
                    let is_external =
                        !module_path.starts_with('.') && !module_path.starts_with('/');

                    let import = ImportFact {
                        file: file_path.to_string(),
                        import_path: module_path.to_string(),
                        imported_names: vec!["*".to_string()], // CommonJS imports everything
                        import_kind: if is_external { "external" } else { "internal" }.to_string(),
                        line_number: (node.start_position().row + 1) as i32,
                    };
                    data.add_import(import);

                    // Add to code search
                    let context = node.utf8_text(source).unwrap_or("").to_string();
                    let symbol = CodeSymbol {
                        path: file_path.to_string(),
                        name: format!("require('{}')", module_path),
                        kind: "require".to_string(),
                        line: node.start_position().row + 1,
                        context,
                    };
                    data.add_symbol(symbol);
                    break;
                }
            }
        }
    }
}

/// Extract function name from various function nodes
fn extract_function_name(node: &Node, source: &[u8]) -> Option<String> {
    // Try to get name from name field
    if let Some(name_node) = node.child_by_field_name("name") {
        return name_node.utf8_text(source).ok().map(String::from);
    }

    // For anonymous functions, check if it's being assigned
    if let Some(parent) = node.parent() {
        if parent.kind() == "variable_declarator" {
            if let Some(name_node) = parent.child_by_field_name("name") {
                return name_node.utf8_text(source).ok().map(String::from);
            }
        }
    }

    None
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

/// Extract function parameters
fn extract_parameters(node: &Node, source: &[u8]) -> Vec<String> {
    let params_node = node
        .child_by_field_name("parameters")
        .or_else(|| node.child_by_field_name("parameter"));

    if let Some(params) = params_node {
        let mut result = Vec::new();
        let mut cursor = params.walk();
        for child in params.children(&mut cursor) {
            match child.kind() {
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

/// Check if node has async keyword
fn has_async_keyword(node: &Node, source: &[u8]) -> bool {
    // Check for async keyword as a child
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "async" {
            return true;
        }
    }

    // Also check in the node's text
    node.utf8_text(source)
        .is_ok_and(|text| text.starts_with("async "))
}

/// Check if method has static keyword
fn has_static_keyword(node: &Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "static" {
            return true;
        }
    }
    false
}

/// Check if a call expression is a require() call
fn is_require_call(node: &Node, source: &[u8]) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    if let Some(func_node) = node.child_by_field_name("function") {
        if let Ok(func_name) = func_node.utf8_text(source) {
            return func_name == "require";
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
                        // Skip require calls as they're imports
                        if callee != "require" {
                            // Check for await
                            let call_type = if node
                                .parent()
                                .is_some_and(|p| p.kind() == "await_expression")
                            {
                                CallType::Async
                            } else {
                                CallType::Direct
                            };

                            data.add_call_edge(CallEdge {
                                file: file_path.to_string(),
                                caller: caller.clone(),
                                callee: callee.to_string(),
                                call_type: call_type.as_str().to_string(),
                                line_number,
                            });
                        }
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
                                data.add_call_edge(CallEdge {
                                    file: file_path.to_string(),
                                    caller: caller.clone(),
                                    callee: callee.to_string(),
                                    call_type: CallType::Async.as_str().to_string(),
                                    line_number,
                                });
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
                        data.add_call_edge(CallEdge {
                            file: file_path.to_string(),
                            caller: caller.clone(),
                            callee: format!("new {}", callee),
                            call_type: CallType::Constructor.as_str().to_string(),
                            line_number,
                        });
                    }
                }
            }
        }
        _ => {}
    }
}
