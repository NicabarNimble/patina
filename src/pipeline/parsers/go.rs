use anyhow::{Context, Result};
use patina_metal::{Analyzer, Metal};
use std::path::Path;

use crate::pipeline::schema::{AstData, Call, Function, Import, TypeDef};

/// Parse a Go file and extract semantic information
pub fn parse_go_file(path: &Path) -> Result<AstData> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    
    let mut analyzer = Analyzer::new()?;
    let parsed = analyzer.parse(&source, Metal::Go)?;
    
    let mut ast_data = AstData::from_path(path);
    
    // Extract symbols using patina-metal's analyzer
    let symbols = analyzer.extract_symbols(&parsed);
    
    // Convert symbols to our schema
    for symbol in symbols {
        match symbol.kind {
            patina_metal::SymbolKind::Function => {
                ast_data.functions.push(Function {
                    name: symbol.name,
                    visibility: "public".to_string(),
                    is_async: false, // Go uses goroutines, not async/await
                    is_unsafe: false, // Go doesn't have unsafe keyword like Rust
                    params: Vec::new(),
                    returns: None,
                    line_start: symbol.start_line + 1,
                    line_end: symbol.end_line + 1,
                    doc_comment: None,
                });
            },
            patina_metal::SymbolKind::Struct => {
                ast_data.types.push(TypeDef {
                    name: symbol.name,
                    kind: "struct".to_string(),
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
    
    if node.kind() == "import_declaration" || node.kind() == "import_spec" {
        if let Some(path_node) = node.child_by_field_name("path") {
            let path = source[path_node.byte_range()]
                .trim_matches('"')
                .to_string();
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
    let mut cursor = tree.walk();
    extract_calls_recursive(source, &mut cursor, ast_data, "<module>");
    Ok(())
}

fn extract_calls_recursive(source: &str, cursor: &mut tree_sitter::TreeCursor, ast_data: &mut AstData, current_function: &str) {
    let node = cursor.node();
    
    // Track current function context
    let function_name = if node.kind() == "function_declaration" || node.kind() == "method_declaration" {
        node.child_by_field_name("name")
            .map(|n| source[n.byte_range()].to_string())
            .unwrap_or_else(|| current_function.to_string())
    } else {
        current_function.to_string()
    };
    
    // Extract call expressions
    if node.kind() == "call_expression" {
        let target = if let Some(func_node) = node.child_by_field_name("function") {
            // Handle method calls (selector expressions)
            if func_node.kind() == "selector_expression" {
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
                .map(|n| n.kind() == "selector_expression")
                .unwrap_or(false),
            is_async: false, // Go uses goroutines, not async/await
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