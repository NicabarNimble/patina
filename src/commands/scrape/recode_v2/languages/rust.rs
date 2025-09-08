// ============================================================================
// RUST LANGUAGE MODULE - Self-Contained Processor
// ============================================================================
//! Rust language processor with complete isolation.
//!
//! Handles Rust's unique features:
//! - Ownership and borrowing patterns
//! - Trait implementations
//! - Async/await support
//! - Unsafe blocks
//! - Macro usage

use crate::commands::scrape::recode_v2::sql_builder::{InsertBuilder, TableName};
use crate::commands::scrape::recode_v2::types::{CallType, FilePath, SymbolKind};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// Rust language processor - completely self-contained
pub struct RustProcessor;

impl RustProcessor {
    /// Process a Rust file and extract all symbols to SQL statements
    pub fn process_file(
        file_path: FilePath,
        content: &[u8],
    ) -> Result<(Vec<String>, usize, usize, usize)> {
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

        let root = tree.root_node();
        let mut sql_statements = Vec::new();
        let mut functions = 0;
        let mut types = 0;
        let mut imports = 0;

        // Track current function for call graph
        let mut current_function: Option<String> = None;
        let mut call_graph_entries = Vec::new();

        // Walk the tree and extract symbols (mimicking extract_symbols_from_tree)
        extract_symbols(
            root,
            content,
            &file_path,
            &mut sql_statements,
            &mut functions,
            &mut types,
            &mut imports,
            &mut current_function,
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
    call_graph: &mut Vec<(String, String, CallType, i32)>,
) {
    // First extract any calls
    extract_calls(&node, source, current_function, call_graph);

    // Determine symbol kind using the same logic as the SPEC
    let symbol_kind = match node.kind() {
        "function_item" => SymbolKind::Function,
        "struct_item" => SymbolKind::Struct,
        "enum_item" => SymbolKind::Enum,
        "trait_item" => SymbolKind::Trait,
        "impl_item" => SymbolKind::Impl,
        "type_alias" => SymbolKind::TypeAlias,
        "const_item" | "static_item" => SymbolKind::Const,
        "use_declaration" => SymbolKind::Import,
        "mod_item" => SymbolKind::Module,
        _ => SymbolKind::Unknown,
    };

    // Handle imports specially
    if symbol_kind == SymbolKind::Import {
        if let Ok(import_text) = node.utf8_text(source) {
            let import_clean = import_text.trim_start_matches("use ").trim_end_matches(';');
            
            let is_external = !import_clean.starts_with("crate::")
                && !import_clean.starts_with("super::")
                && !import_clean.starts_with("self::");
            
            let imported_item = import_clean.split("::").last().unwrap_or(import_clean);
            let imported_from = if import_clean.contains("::") {
                import_clean
                    .rsplit_once("::")
                    .map(|(from, _)| from)
                    .unwrap_or(import_clean)
            } else {
                import_clean
            };

            let import_sql = InsertBuilder::new(TableName::IMPORT_FACTS)
                .or_replace()
                .value("importer_file", file_path.as_str())
                .value("imported_item", imported_item)
                .value("imported_from", imported_from)
                .value("is_external", is_external)
                .value("import_kind", "use")
                .build();
            sql.push(format!("{};\n", import_sql));
            *imports += 1;
        }
    } else if symbol_kind == SymbolKind::Impl || symbol_kind == SymbolKind::Module {
        // Skip impl blocks and modules - they don't get stored
        // but still recurse into their children
    } else if symbol_kind != SymbolKind::Unknown {
        // Process regular symbol
        if let Some(name) = node.child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok())
            .map(|s| s.to_string())
        {
            // Track function context for call graph
            let old_func = if symbol_kind == SymbolKind::Function {
                let old = current_function.clone();
                *current_function = Some(name.clone());
                old
            } else {
                None
            };

            // Process the symbol based on its kind
            match symbol_kind {
                SymbolKind::Function => {
                    process_function(&node, source, file_path, &name, sql);
                    *functions += 1;
                }
                SymbolKind::Struct | SymbolKind::Enum | SymbolKind::Trait | 
                SymbolKind::TypeAlias | SymbolKind::Const => {
                    process_type(&node, source, file_path, &name, symbol_kind, sql);
                    *types += 1;
                }
                _ => {}
            }

            // Restore function context
            if symbol_kind == SymbolKind::Function {
                *current_function = old_func;
            }
        }
    }

    // Recurse into children
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
            call_graph,
        );
    }
}

/// Process a function symbol
fn process_function(node: &Node, source: &[u8], file_path: &FilePath, name: &str, sql: &mut Vec<String>) {
    // Extract function details using the SPEC logic
    let params = extract_params(node, source);
    let return_type = extract_return_type(node, source);
    let is_public = has_visibility_modifier(node);
    let is_async = has_async(node);
    let is_unsafe = has_unsafe(node);
    
    // Check for specific patterns in params and return type
    let takes_mut_self = params.iter().any(|p| p.contains("&mut self"));
    let takes_mut_params = params.iter().any(|p| p.contains("&mut ") && !p.contains("self"));
    let returns_result = return_type.as_deref().unwrap_or("").contains("Result");
    let returns_option = return_type.as_deref().unwrap_or("").contains("Option");
    
    // Count generics
    let generics = node.child_by_field_name("type_parameters")
        .and_then(|tp| tp.utf8_text(source).ok())
        .map(String::from);
    let generic_count = generics.as_ref().map(|g| g.matches(',').count() + 1).unwrap_or(0);
    
    // Build signature
    let mut signature = String::new();
    if is_async { signature.push_str("async "); }
    if is_unsafe { signature.push_str("unsafe "); }
    signature.push_str("fn ");
    signature.push_str(name);
    if let Some(g) = &generics {
        signature.push_str(g);
    }
    signature.push('(');
    signature.push_str(&params.join(", "));
    signature.push(')');
    if let Some(ret) = &return_type {
        signature.push_str(" -> ");
        signature.push_str(ret);
    }

    // Insert into function_facts
    let func_sql = InsertBuilder::new(TableName::FUNCTION_FACTS)
        .or_replace()
        .value("file", file_path.as_str())
        .value("name", name)
        .value("takes_mut_self", takes_mut_self)
        .value("takes_mut_params", takes_mut_params)
        .value("returns_result", returns_result)
        .value("returns_option", returns_option)
        .value("is_async", is_async)
        .value("is_unsafe", is_unsafe)
        .value("is_public", is_public)
        .value("parameter_count", params.len() as i64)
        .value("generic_count", generic_count as i64)
        .value("parameters", params.join(", "))
        .value("return_type", return_type.as_deref().unwrap_or(""))
        .build();
    sql.push(format!("{};\n", func_sql));

    // Also insert into code_search
    let context = node.utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .to_string();
    
    let search_sql = InsertBuilder::new(TableName::CODE_SEARCH)
        .or_replace()
        .value("path", file_path.as_str())
        .value("name", name)
        .value("signature", signature)
        .value("context", context)
        .build();
    sql.push(format!("{};\n", search_sql));
}

/// Process a type symbol (struct, enum, trait, etc.)
fn process_type(node: &Node, source: &[u8], file_path: &FilePath, name: &str, kind: SymbolKind, sql: &mut Vec<String>) {
    let is_public = has_visibility_modifier(node);
    
    // Get the node text for definition
    let definition = node.utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .to_string();
    
    let kind_str = match kind {
        SymbolKind::Struct => "struct",
        SymbolKind::Enum => "enum",
        SymbolKind::Trait => "trait",
        SymbolKind::TypeAlias => "type_alias",
        SymbolKind::Const => if node.kind() == "static_item" { "static" } else { "const" },
        _ => "unknown",
    };
    
    let type_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
        .or_replace()
        .value("file", file_path.as_str())
        .value("name", name)
        .value("definition", definition)
        .value("kind", kind_str)
        .value("visibility", if is_public { "pub" } else { "private" })
        .build();
    sql.push(format!("{};\n", type_sql));
}

/// Extract function calls for call graph
fn extract_calls(
    node: &Node, 
    source: &[u8], 
    current_function: &Option<String>,
    call_graph: &mut Vec<(String, String, CallType, i32)>
) {
    if let Some(ref caller) = current_function {
        let line_number = (node.start_position().row + 1) as i32;
        
        match node.kind() {
            "call_expression" => {
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
            "method_call_expression" => {
                if let Some(method_node) = node.child_by_field_name("name") {
                    if let Ok(callee) = method_node.utf8_text(source) {
                        call_graph.push((
                            caller.clone(),
                            callee.to_string(),
                            CallType::Method,
                            line_number,
                        ));
                    }
                }
            }
            "macro_invocation" => {
                if let Some(macro_node) = node.child_by_field_name("macro") {
                    if let Ok(callee) = macro_node.utf8_text(source) {
                        call_graph.push((
                            caller.clone(),
                            callee.to_string(),
                            CallType::Macro,
                            line_number,
                        ));
                    }
                }
            }
            _ => {}
        }
    }
}

/// Helper functions from the original SPEC

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
        .and_then(|rt| rt.utf8_text(source).ok())
        .map(|s| s.trim_start_matches("->").trim().to_string())
}