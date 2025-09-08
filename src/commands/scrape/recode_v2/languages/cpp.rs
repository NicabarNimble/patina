// ============================================================================
// C++ LANGUAGE MODULE - Self-Contained Processor
// ============================================================================
//! C++ language processor with complete isolation.
//!
//! Handles C++'s features:
//! - Classes with access modifiers (public/private/protected)
//! - Templates and template specialization
//! - Namespaces
//! - Function overloading
//! - RAII and constructors/destructors
//! - Modern C++ features (auto, lambdas, etc.)

use crate::commands::scrape::recode_v2::sql_builder::{InsertBuilder, TableName};
use crate::commands::scrape::recode_v2::types::{CallType, FilePath};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// C++ language processor - completely self-contained
pub struct CppProcessor;

impl CppProcessor {
    /// Process a C++ file and extract all symbols to SQL statements
    pub fn process_file(
        file_path: FilePath,
        content: &[u8],
    ) -> Result<(Vec<String>, usize, usize, usize)> {
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

        let root = tree.root_node();
        let mut sql_statements = Vec::new();
        let mut functions = 0;
        let mut types = 0;
        let mut imports = 0;

        // Track current function and namespace for call graph
        let mut current_function: Option<String> = None;
        let mut current_namespace: Vec<String> = Vec::new();
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
            &mut current_namespace,
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
    current_namespace: &mut Vec<String>,
    call_graph: &mut Vec<(String, String, CallType, i32)>,
) {
    match node.kind() {
        "namespace_definition" => {
            // Enter namespace
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    current_namespace.push(name.to_string());
                }
            }

            // Process namespace body
            if let Some(body) = node.child_by_field_name("body") {
                let mut cursor = body.walk();
                for child in body.children(&mut cursor) {
                    extract_symbols(
                        child,
                        source,
                        file_path,
                        sql,
                        functions,
                        types,
                        imports,
                        current_function,
                        current_namespace,
                        call_graph,
                    );
                }
            }

            // Exit namespace
            current_namespace.pop();
            return; // Don't recurse again
        }
        "function_definition" => {
            if let Some(name) = extract_function_name(&node, source) {
                // Include namespace in function name
                let full_name = if current_namespace.is_empty() {
                    name.clone()
                } else {
                    format!("{}::{}", current_namespace.join("::"), name)
                };

                // Track for call graph
                let old_func = current_function.clone();
                *current_function = Some(full_name.clone());

                // Extract function details
                let params = extract_parameters(&node, source);
                let return_type = extract_return_type(&node, source);
                let is_template = has_template_parent(&node);
                let is_public = is_public_member(&node, source);

                // Build signature
                let signature = if params.is_empty() {
                    format!("{} {}()", return_type.as_deref().unwrap_or("auto"), full_name)
                } else {
                    format!(
                        "{} {}({})",
                        return_type.as_deref().unwrap_or("auto"),
                        full_name,
                        params.join(", ")
                    )
                };

                // Insert into function_facts
                let func_sql = InsertBuilder::new(TableName::FUNCTION_FACTS)
                    .or_replace()
                    .value("file", file_path.as_str())
                    .value("name", full_name.as_str())
                    .value("takes_mut_self", false)
                    .value("takes_mut_params", params.iter().any(|p| !p.contains("const")))
                    .value("returns_result", false)
                    .value("returns_option", false)
                    .value("is_async", false)
                    .value("is_unsafe", true) // All C++ is unsafe
                    .value("is_public", is_public)
                    .value("parameter_count", params.len() as i64)
                    .value("generic_count", if is_template { 1i64 } else { 0i64 })
                    .value("parameters", params.join(", "))
                    .value("return_type", return_type.as_deref().unwrap_or("void"))
                    .build();
                sql.push(format!("{};\n", func_sql));

                // Also insert into code_search
                let search_sql = InsertBuilder::new(TableName::CODE_SEARCH)
                    .or_replace()
                    .value("path", file_path.as_str())
                    .value("name", full_name.as_str())
                    .value("signature", signature)
                    .value("context", extract_context(&node, source))
                    .build();
                sql.push(format!("{};\n", search_sql));

                *functions += 1;

                // Process function body for calls
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
                        current_namespace,
                        call_graph,
                    );
                }

                // Restore previous function context
                *current_function = old_func;
                return; // Don't recurse again
            }
        }
        "class_specifier" | "struct_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    let full_name = if current_namespace.is_empty() {
                        name.to_string()
                    } else {
                        format!("{}::{}", current_namespace.join("::"), name)
                    };

                    let kind = if node.kind() == "class_specifier" {
                        "class"
                    } else {
                        "struct"
                    };
                    let is_template = has_template_parent(&node);

                    let type_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                        .or_replace()
                        .value("file", file_path.as_str())
                        .value("name", full_name.as_str())
                        .value("definition", format!("{} {}", kind, full_name))
                        .value("kind", kind)
                        .value("visibility", "public") // Top-level classes are public
                        .build();
                    sql.push(format!("{};\n", type_sql));
                    *types += 1;

                    // Process class body
                    if let Some(body) = node.child_by_field_name("body") {
                        let mut cursor = body.walk();
                        for child in body.children(&mut cursor) {
                            extract_symbols(
                                child,
                                source,
                                file_path,
                                sql,
                                functions,
                                types,
                                imports,
                                current_function,
                                current_namespace,
                                call_graph,
                            );
                        }
                    }
                }
            }
        }
        "enum_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    let full_name = if current_namespace.is_empty() {
                        name.to_string()
                    } else {
                        format!("{}::{}", current_namespace.join("::"), name)
                    };

                    let is_enum_class = node
                        .utf8_text(source)
                        .map(|t| t.contains("enum class"))
                        .unwrap_or(false);

                    let type_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                        .or_replace()
                        .value("file", file_path.as_str())
                        .value("name", full_name.as_str())
                        .value(
                            "definition",
                            format!(
                                "{} {}",
                                if is_enum_class { "enum class" } else { "enum" },
                                full_name
                            ),
                        )
                        .value("kind", "enum")
                        .value("visibility", "public")
                        .build();
                    sql.push(format!("{};\n", type_sql));
                    *types += 1;
                }
            }
        }
        "type_definition" | "alias_declaration" => {
            // typedef and using declarations
            if let Some(name) = extract_typedef_name(&node, source) {
                let full_name = if current_namespace.is_empty() {
                    name.clone()
                } else {
                    format!("{}::{}", current_namespace.join("::"), name)
                };

                let kind = if node.kind() == "alias_declaration" {
                    "using"
                } else {
                    "typedef"
                };

                let type_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                    .or_replace()
                    .value("file", file_path.as_str())
                    .value("name", full_name.as_str())
                    .value("definition", format!("{} {}", kind, full_name))
                    .value("kind", kind)
                    .value("visibility", "public")
                    .build();
                sql.push(format!("{};\n", type_sql));
                *types += 1;
            }
        }
        "preproc_include" | "using_directive" | "using_declaration" => {
            if let Ok(import_text) = node.utf8_text(source) {
                let (item, from, is_external) = if node.kind() == "preproc_include" {
                    let header = import_text
                        .trim_start_matches("#include")
                        .trim()
                        .trim_start_matches('<')
                        .trim_start_matches('"')
                        .trim_end_matches('>')
                        .trim_end_matches('"');
                    (header, header, import_text.contains('<'))
                } else {
                    // using namespace or using declaration
                    let using_part = import_text
                        .trim_start_matches("using")
                        .trim_start_matches("namespace")
                        .trim()
                        .trim_end_matches(';');
                    (using_part, using_part, true)
                };

                let import_sql = InsertBuilder::new(TableName::IMPORT_FACTS)
                    .or_replace()
                    .value("importer_file", file_path.as_str())
                    .value("imported_item", item)
                    .value("imported_from", from)
                    .value("is_external", is_external)
                    .value(
                        "import_kind",
                        if node.kind() == "preproc_include" {
                            "include"
                        } else {
                            "using"
                        },
                    )
                    .build();
                sql.push(format!("{};\n", import_sql));
                *imports += 1;
            }
        }
        "call_expression" => {
            // Track function calls for call graph
            if let Some(ref caller) = current_function {
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        let line = (node.start_position().row + 1) as i32;
                        call_graph.push((
                            caller.clone(),
                            callee.to_string(),
                            CallType::Direct,
                            line,
                        ));
                    }
                }
            }
        }
        _ => {}
    }

    // Recurse into children (unless we already handled them)
    if !matches!(
        node.kind(),
        "namespace_definition" | "function_definition" | "class_specifier" | "struct_specifier"
    ) {
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
                current_namespace,
                call_graph,
            );
        }
    }
}

/// Extract function name from C++ function_definition node, handling nested declarators
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

/// Extract C++ function name from declarator (handles complex declarators)
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
                    if sibling.id() == node.id() {
                        return is_public;
                    }
                }
                return false;
            }
            _ => {}
        }
        current = parent.parent();
    }

    // Not in a class/struct, assume public
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