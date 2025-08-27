use anyhow::{Context, Result};
use patina_metal::{Analyzer, Metal};
use std::path::Path;

use crate::pipeline::schema::{AstData, Call, Function, Import, TypeDef};

/// Parse a Rust file and extract semantic information
pub fn parse_rust_file(path: &Path) -> Result<AstData> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    
    let mut analyzer = Analyzer::new()?;
    let parsed = analyzer.parse(&source, Metal::Rust)?;
    
    let mut ast_data = AstData::from_path(path);
    
    // Extract symbols using patina-metal's analyzer
    let symbols = analyzer.extract_symbols(&parsed);
    
    // Convert symbols to our schema
    for symbol in symbols {
        match symbol.kind {
            patina_metal::SymbolKind::Function => {
                ast_data.functions.push(Function {
                    name: symbol.name,
                    visibility: "public".to_string(), // TODO: Extract actual visibility
                    is_async: false, // TODO: Extract from signature
                    is_unsafe: false, // TODO: Extract from signature
                    params: Vec::new(), // TODO: Parse parameters from signature
                    returns: None, // TODO: Parse return type
                    line_start: symbol.start_line + 1,
                    line_end: symbol.end_line + 1,
                    doc_comment: None,
                });
            },
            patina_metal::SymbolKind::Struct | 
            patina_metal::SymbolKind::Trait => {
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
            },
            _ => {}
        }
    }
    
    // For now, we'll use tree-sitter directly for detailed extraction
    // This is a simplified version - in production we'd extract more details
    extract_imports(&source, &parsed.tree, &mut ast_data)?;
    extract_calls(&source, &parsed.tree, &mut ast_data)?;
    
    Ok(ast_data)
}


fn extract_imports(source: &str, tree: &tree_sitter::Tree, ast_data: &mut AstData) -> Result<()> {
    // Simple import extraction using tree traversal
    let mut cursor = tree.walk();
    extract_imports_recursive(source, &mut cursor, ast_data);
    Ok(())
}

fn extract_imports_recursive(source: &str, cursor: &mut tree_sitter::TreeCursor, ast_data: &mut AstData) {
    let node = cursor.node();
    
    if node.kind() == "use_declaration" {
        if let Some(arg_node) = node.child_by_field_name("argument") {
            let path = source[arg_node.byte_range()].to_string();
            let line = node.start_position().row + 1;
            
            ast_data.imports.push(Import {
                path,
                items: Vec::new(),
                alias: None,
                line,
            });
        }
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
    // Simple call extraction using tree traversal
    let mut cursor = tree.walk();
    extract_calls_recursive(source, &mut cursor, ast_data, "<module>");
    Ok(())
}

fn extract_calls_recursive(source: &str, cursor: &mut tree_sitter::TreeCursor, ast_data: &mut AstData, current_function: &str) {
    let node = cursor.node();
    
    // Track current function context
    let function_name = if node.kind() == "function_item" {
        node.child_by_field_name("name")
            .map(|n| source[n.byte_range()].to_string())
            .unwrap_or_else(|| current_function.to_string())
    } else {
        current_function.to_string()
    };
    
    // Extract call expressions
    match node.kind() {
        "call_expression" => {
            let target = if let Some(func_node) = node.child_by_field_name("function") {
                // Handle method calls
                if func_node.kind() == "field_expression" {
                    func_node.child_by_field_name("field")
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
                    .map(|n| n.kind() == "field_expression")
                    .unwrap_or(false),
                is_async: false, // Simplified for now
            });
        },
        "macro_invocation" => {
            if let Some(macro_node) = node.child_by_field_name("macro") {
                let target = format!("{}!", source[macro_node.byte_range()].to_string());
                
                ast_data.calls.push(Call {
                    target,
                    caller: function_name.clone(),
                    line: node.start_position().row + 1,
                    is_method: false,
                    is_async: false,
                });
            }
        },
        _ => {}
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

