use anyhow::{Context, Result};
use patina_metal::{Analyzer, Metal};
use std::path::Path;

use crate::pipeline::schema::{AstData, Call, Function, Import, TypeDef};

/// Parse a Python file and extract semantic information
pub fn parse_python_file(path: &Path) -> Result<AstData> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    
    let mut analyzer = Analyzer::new()?;
    let parsed = analyzer.parse(&source, Metal::Python)?;
    
    let mut ast_data = AstData::from_path(path);
    
    // Extract symbols using patina-metal's analyzer
    let symbols = analyzer.extract_symbols(&parsed);
    
    // Convert symbols to our schema
    for symbol in symbols {
        match symbol.kind {
            patina_metal::SymbolKind::Function => {
                ast_data.functions.push(Function {
                    name: symbol.name.clone(),
                    visibility: if symbol.name.starts_with('_') { "private" } else { "public" }.to_string(),
                    is_async: symbol.signature.contains("async "),
                    is_unsafe: false, // Python doesn't have unsafe
                    params: Vec::new(),
                    returns: None,
                    line_start: symbol.start_line + 1,
                    line_end: symbol.end_line + 1,
                    doc_comment: None,
                    // Rich analysis fields - TODO: implement
                    signature: None,
                    complexity: None,
                    cognitive_complexity: None,
                    pattern_hash: None,
                    is_test: false,
                    is_generated: false,
                });
            },
            _ => {
                // Python classes are typically "function" in tree-sitter
                // We'll add them as types anyway
                ast_data.types.push(TypeDef {
                    name: symbol.name,
                    kind: format!("{:?}", symbol.kind).to_lowercase(),
                    visibility: "public".to_string(),
                    fields: Vec::new(),
                    methods: Vec::new(),
                    line_start: symbol.start_line + 1,
                    line_end: symbol.end_line + 1,
                    doc_comment: None,
                });
            }
        }
    }
    
    // Extract imports and calls
    extract_imports(&source, &parsed.tree, &mut ast_data)?;
    extract_calls(&source, &parsed.tree, &mut ast_data)?;
    
    Ok(ast_data)
}

fn extract_imports(source: &str, tree: &tree_sitter::Tree, ast_data: &mut AstData) -> Result<()> {
    let mut cursor = tree.walk();
    extract_imports_recursive(source, &mut cursor, ast_data);
    Ok(())
}

fn extract_imports_recursive(source: &str, cursor: &mut tree_sitter::TreeCursor, ast_data: &mut AstData) {
    let node = cursor.node();
    
    match node.kind() {
        "import_statement" | "import_from_statement" => {
            let line = node.start_position().row + 1;
            let import_text = source[node.byte_range()].to_string();
            
            // Simple extraction - just store the whole import line
            ast_data.imports.push(Import {
                path: import_text,
                items: Vec::new(),
                alias: None,
                line,
            });
        }
        _ => {}
    }
    
    // Recurse into children
    if cursor.goto_first_child() {
        loop {
            extract_imports_recursive(source, cursor, ast_data);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

fn extract_calls(source: &str, tree: &tree_sitter::Tree, ast_data: &mut AstData) -> Result<()> {
    let mut cursor = tree.walk();
    extract_calls_recursive(source, &mut cursor, ast_data, "<module>");
    Ok(())
}

fn extract_calls_recursive(source: &str, cursor: &mut tree_sitter::TreeCursor, ast_data: &mut AstData, current_function: &str) {
    let node = cursor.node();
    
    // Track current function context
    let function_name = if node.kind() == "function_definition" {
        node.child_by_field_name("name")
            .map(|n| source[n.byte_range()].to_string())
            .unwrap_or_else(|| current_function.to_string())
    } else if node.kind() == "class_definition" {
        node.child_by_field_name("name")
            .map(|n| format!("class {}", source[n.byte_range()].to_string()))
            .unwrap_or_else(|| current_function.to_string())
    } else {
        current_function.to_string()
    };
    
    // Extract call expressions
    if node.kind() == "call" {
        let target = if let Some(func_node) = node.child_by_field_name("function") {
            // Handle attribute access (method calls)
            if func_node.kind() == "attribute" {
                func_node.child_by_field_name("attribute")
                    .map(|n| source[n.byte_range()].to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                source[func_node.byte_range()].to_string()
            }
        } else {
            "unknown".to_string()
        };
        
        ast_data.calls.push(Call {
            target,
            caller: function_name.clone(),
            line: node.start_position().row + 1,
            is_method: node.child_by_field_name("function")
                .map(|n| n.kind() == "attribute")
                .unwrap_or(false),
            is_async: false, // We'd need to check if we're in an async context
        });
    }
    
    // Recurse into children
    if cursor.goto_first_child() {
        loop {
            extract_calls_recursive(source, cursor, ast_data, &function_name);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}