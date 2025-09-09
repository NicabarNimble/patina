// ============================================================================
// PYTHON LANGUAGE PROCESSOR V2 - STRUCT-BASED
// ============================================================================
//! Python language processor that returns typed structs instead of SQL strings.
//!
//! This is the refactored version that:
//! - Returns ExtractedData with typed structs
//! - No SQL string generation
//! - Direct data extraction to domain types
//!
//! Handles Python's unique features:
//! - Underscore-based visibility conventions
//! - Docstrings (triple quotes)
//! - Duck typing and dynamic nature
//! - Async/await support
//! - Decorators and class definitions
//! - Import system (from/import statements)

use crate::commands::scrape::recode_v2::database::{
    CallEdge, CodeSymbol, FunctionFact, ImportFact, TypeFact,
};
use crate::commands::scrape::recode_v2::extracted_data::ExtractedData;
use crate::commands::scrape::recode_v2::types::{CallType, FilePath};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// Python language processor - returns typed structs
pub struct PythonProcessor;

impl PythonProcessor {
    /// Process a Python file and extract all symbols to typed structs
    pub fn process_file(file_path: FilePath, content: &[u8]) -> Result<ExtractedData> {
        let mut data = ExtractedData::new();

        // Set up tree-sitter parser for Python
        let mut parser = Parser::new();
        let metal = patina_metal::Metal::Python;
        let language = metal
            .tree_sitter_language_for_ext("py")
            .ok_or_else(|| anyhow::anyhow!("No Python parser available"))?;
        parser
            .set_language(&language)
            .context("Failed to set Python language")?;

        // Parse the file
        let tree = parser
            .parse(content, None)
            .context("Failed to parse Python file")?;

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

    // Handle decorated definitions specially
    if node.kind() == "decorated_definition" {
        process_decorated_definition(&node, source, file_path, data, current_function);
        return;
    }

    // Process based on node kind
    match node.kind() {
        "function_definition" | "async_function_definition" => {
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
        "class_definition" => {
            if let Some(name) = extract_class_name(&node, source) {
                let old_function = current_function.clone();

                process_class(&node, source, file_path, &name, data);

                // Process class methods
                if let Some(body_node) = node.child_by_field_name("body") {
                    let mut cursor = body_node.walk();
                    for child in body_node.children(&mut cursor) {
                        // For methods, include class name in context
                        if matches!(
                            child.kind(),
                            "function_definition" | "async_function_definition"
                        ) {
                            if let Some(method_name) = extract_function_name(&child, source) {
                                *current_function = Some(format!("{}.{}", name, method_name));
                            }
                        }
                        extract_symbols(child, source, file_path, data, current_function);
                    }
                }

                *current_function = old_function;
                return; // Don't recurse again
            }
        }
        "import_statement" | "import_from_statement" => {
            process_import(&node, source, file_path, data);
        }
        _ => {}
    }

    // Recurse to children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_symbols(child, source, file_path, data, current_function);
    }
}

/// Process decorated definitions (functions or classes with decorators)
fn process_decorated_definition(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    data: &mut ExtractedData,
    current_function: &mut Option<String>,
) {
    // Extract decorators for call graph
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "decorator" {
            if let Ok(decorator_text) = child.utf8_text(source) {
                let decorator_name = decorator_text.trim_start_matches('@');

                // Add decorator as a call edge if we're in a function
                if let Some(caller) = current_function {
                    data.add_call_edge(CallEdge {
                        file: file_path.to_string(),
                        caller: caller.clone(),
                        callee: format!("@{}", decorator_name),
                        call_type: CallType::Decorator.as_str().to_string(),
                        line_number: (child.start_position().row + 1) as i32,
                    });
                }
            }
        }
    }

    // Process the actual definition
    if let Some(definition) = node.child_by_field_name("definition") {
        match definition.kind() {
            "function_definition" | "async_function_definition" => {
                if let Some(name) = extract_function_name(&definition, source) {
                    process_function(&definition, source, file_path, &name, data);
                }
            }
            "class_definition" => {
                if let Some(name) = extract_class_name(&definition, source) {
                    process_class(&definition, source, file_path, &name, data);
                }
            }
            _ => {}
        }
    }
}

/// Process a Python function and add to data
fn process_function(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = !name.starts_with('_');
    let is_async = node.kind() == "async_function_definition";
    let params = extract_params(node, source);
    let return_type = extract_return_type(node, source);
    let _docs = extract_docstring(node, source);

    // Check for patterns in parameters
    let takes_mut_self = params.iter().any(|p| p == "self");
    let takes_mut_params = false; // Python doesn't have explicit mutability
    let returns_result = return_type.as_ref().is_some_and(|rt| {
        rt.contains("Result") || rt.contains("Union") || rt.contains("Optional")
    });
    let returns_option = return_type
        .as_ref()
        .is_some_and(|rt| rt.contains("Optional") || rt.contains("None"));

    // Create function fact
    let function = FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self,
        takes_mut_params,
        returns_result,
        returns_option,
        is_async,
        is_unsafe: false, // Python doesn't have unsafe
        is_public,
        parameter_count: params.len() as i32,
        generic_count: 0, // Python doesn't have traditional generics
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

/// Process a Python class and add to data
fn process_class(
    node: &Node,
    source: &[u8],
    file_path: &FilePath,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = !name.starts_with('_');
    let _docs = extract_docstring(node, source);

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
        visibility: if is_public { "public" } else { "private" }.to_string(),
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

/// Process Python imports and add to data
fn process_import(node: &Node, source: &[u8], file_path: &FilePath, data: &mut ExtractedData) {
    if let Some((imported_item, import_path, is_external)) = extract_import_details(node, source) {
        let import = ImportFact {
            file: file_path.to_string(),
            import_path,
            imported_names: vec![imported_item],
            import_kind: if is_external { "external" } else { "internal" }.to_string(),
            line_number: (node.start_position().row + 1) as i32,
        };
        data.add_import(import);

        // Add to code search
        let context = node.utf8_text(source).unwrap_or("").to_string();
        let symbol = CodeSymbol {
            path: file_path.to_string(),
            name: context.clone(),
            kind: "import".to_string(),
            line: node.start_position().row + 1,
            context,
        };
        data.add_symbol(symbol);
    }
}

/// Extract function name
fn extract_function_name(node: &Node, source: &[u8]) -> Option<String> {
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
fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut params = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if matches!(child.kind(), "," | "(" | ")") {
                continue;
            }
            if let Ok(param_text) = child.utf8_text(source) {
                let cleaned = param_text.trim();
                if !cleaned.is_empty() && cleaned != "self" && cleaned != "cls" {
                    params.push(cleaned.to_string());
                }
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
        .map(|s| s.trim_start_matches("->").trim().to_string())
}

/// Extract docstring from function or class
fn extract_docstring(node: &Node, source: &[u8]) -> String {
    // Look for docstring in the body
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "expression_statement" {
                if let Some(string_node) = child.child(0) {
                    if string_node.kind() == "string" {
                        if let Ok(text) = string_node.utf8_text(source) {
                            return clean_docstring(text);
                        }
                    }
                }
            }
        }
    }
    String::new()
}

/// Clean Python docstring
fn clean_docstring(raw: &str) -> String {
    raw.trim()
        .strip_prefix("\"\"\"")
        .and_then(|s| s.strip_suffix("\"\"\""))
        .or_else(|| {
            raw.trim()
                .strip_prefix("'''")
                .and_then(|s| s.strip_suffix("'''"))
        })
        .unwrap_or(raw)
        .trim()
        .to_string()
}

/// Extract import details
fn extract_import_details(node: &Node, source: &[u8]) -> Option<(String, String, bool)> {
    let import_text = node.utf8_text(source).ok()?;

    match node.kind() {
        "import_statement" => {
            // Handle: import module, import module as alias
            let clean = import_text.trim_start_matches("import ").trim();
            let module_name = clean.split(" as ").next().unwrap_or(clean);
            let is_external = !module_name.starts_with('.');
            Some((
                module_name.to_string(),
                module_name.to_string(),
                is_external,
            ))
        }
        "import_from_statement" => {
            // Handle: from module import item, from . import item
            if let Some(module_node) = node.child_by_field_name("module_name") {
                if let Ok(module_name) = module_node.utf8_text(source) {
                    let is_external = !import_text.contains("from .");

                    // Extract imported items
                    let items = if import_text.contains("import *") {
                        "*".to_string()
                    } else if let Some(import_idx) = import_text.find("import ") {
                        import_text[import_idx + 7..].trim().to_string()
                    } else {
                        module_name.to_string()
                    };

                    return Some((items, module_name.to_string(), is_external));
                }
            }

            // Fallback parsing
            let is_external = !import_text.contains("from .");
            Some((
                import_text.to_string(),
                import_text.to_string(),
                is_external,
            ))
        }
        _ => None,
    }
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
        "call" => {
            if let Some(caller) = current_function {
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        // Check for await
                        let (call_type, callee_name) = if callee.starts_with("await ") {
                            (
                                CallType::Async,
                                callee.strip_prefix("await ").unwrap_or(callee),
                            )
                        } else {
                            (CallType::Direct, callee)
                        };

                        data.add_call_edge(CallEdge {
                            file: file_path.to_string(),
                            caller: caller.clone(),
                            callee: callee_name.to_string(),
                            call_type: call_type.as_str().to_string(),
                            line_number,
                        });
                    }
                }
            }
        }
        "await" => {
            // Handle await expressions
            if let Some(caller) = current_function {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "call" {
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
        _ => {}
    }
}
