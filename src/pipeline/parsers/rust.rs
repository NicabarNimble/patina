use anyhow::{Context, Result};
use patina_metal::{Analyzer, Metal};
use std::path::Path;

use crate::pipeline::schema::{AstData, Call, Function, Import, TypeDef, CodeFingerprint, Symbol, FileMetrics};
use crate::pipeline::analysis::{
    calculate_complexity, hash_ast_pattern, is_test_function, 
    build_function_signature, count_lines
};

/// Parse a Rust file and extract semantic information with rich analysis
pub fn parse_rust_file(path: &Path) -> Result<AstData> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    
    let source_bytes = source.as_bytes();
    
    let mut analyzer = Analyzer::new()?;
    let parsed = analyzer.parse(&source, Metal::Rust)?;
    
    let mut ast_data = AstData::from_path(path);
    
    // Calculate file metrics
    let (total_lines, code_lines, comment_lines, blank_lines) = count_lines(&source);
    let mut complexity_sum = 0u32;
    let mut max_complexity = 0u16;
    
    // Use tree-sitter directly for richer extraction
    let tree = &parsed.tree;
    let mut cursor = tree.walk();
    
    // Extract all elements with rich analysis
    extract_rust_elements_recursive(
        &source,
        source_bytes,
        &mut cursor, 
        &mut ast_data,
        &mut complexity_sum,
        &mut max_complexity,
    )?;
    
    // Set file metrics
    ast_data.file_metrics = Some(FileMetrics {
        total_lines,
        code_lines,
        comment_lines, 
        blank_lines,
        complexity_sum,
        max_complexity,
        function_count: ast_data.functions.len(),
        type_count: ast_data.types.len(),
    });
    
    Ok(ast_data)
}

fn extract_rust_elements_recursive(
    source: &str,
    source_bytes: &[u8],
    cursor: &mut tree_sitter::TreeCursor,
    ast_data: &mut AstData,
    complexity_sum: &mut u32,
    max_complexity: &mut u16,
) -> Result<()> {
    let node = cursor.node();
    
    match node.kind() {
        "function_item" | "impl_item" => {
            extract_function(node, source, source_bytes, ast_data, complexity_sum, max_complexity)?;
            
            // Also extract calls within this function
            let mut call_cursor = node.walk();
            extract_calls_in_function(source_bytes, &mut call_cursor, ast_data, 
                &extract_name(node, source_bytes).unwrap_or_else(|| "<anonymous>".to_string()));
        }
        
        "struct_item" | "enum_item" | "trait_item" | "type_item" => {
            extract_type(node, source_bytes, ast_data)?;
        }
        
        "use_declaration" => {
            extract_import(node, source_bytes, ast_data)?;
        }
        
        _ => {}
    }
    
    // Recurse into children
    if cursor.goto_first_child() {
        loop {
            extract_rust_elements_recursive(source, source_bytes, cursor, ast_data, complexity_sum, max_complexity)?;
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
    
    Ok(())
}

fn extract_function(
    node: tree_sitter::Node,
    source: &str,
    source_bytes: &[u8],
    ast_data: &mut AstData,
    complexity_sum: &mut u32,
    max_complexity: &mut u16,
) -> Result<()> {
    let name = extract_name(node, source_bytes).unwrap_or_else(|| "<anonymous>".to_string());
    let signature = build_function_signature(node, source_bytes);
    
    // Calculate complexity
    let complexity = calculate_complexity(node);
    *complexity_sum += complexity as u32;
    if complexity > *max_complexity {
        *max_complexity = complexity;
    }
    
    // Detect features
    let is_async = node.utf8_text(source_bytes)?.contains("async");
    let is_unsafe = node.utf8_text(source_bytes)?.contains("unsafe");
    let is_test = is_test_function(node, source_bytes, "rust");
    
    // Extract visibility
    let visibility = if let Some(vis) = node.child_by_field_name("visibility") {
        vis.utf8_text(source_bytes)?.to_string()
    } else {
        "private".to_string()
    };
    
    // Create function entry with rich data
    let function = Function {
        name: name.clone(),
        visibility: visibility.clone(),
        is_async,
        is_unsafe,
        params: Vec::new(), // Could be extracted from parameters node
        returns: extract_return_type(node, source_bytes),
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
        doc_comment: extract_doc_comment(node, source_bytes),
        // Rich analysis fields
        signature: Some(signature.clone()),
        complexity: Some(complexity),
        cognitive_complexity: None, // Could add cognitive complexity
        pattern_hash: Some(hash_ast_pattern(node, source_bytes)),
        is_test,
        is_generated: signature.contains("derive") || signature.contains("generated"),
    };
    
    ast_data.functions.push(function);
    
    // Add to symbols for search
    ast_data.symbols.push(Symbol {
        name: name.clone(),
        kind: "function".to_string(),
        signature: Some(signature.clone()),
        context: Some(extract_context(node, source_bytes)),
        line: node.start_position().row + 1,
    });
    
    // Add fingerprint
    ast_data.fingerprints.push(CodeFingerprint {
        name: name.clone(),
        kind: "function".to_string(),
        pattern: hash_ast_pattern(node, source_bytes),
        imports: 0, // Could calculate import hash
        complexity,
        flags: build_flags(is_async, is_unsafe, is_test),
    });
    
    Ok(())
}

fn extract_type(
    node: tree_sitter::Node,
    source_bytes: &[u8],
    ast_data: &mut AstData,
) -> Result<()> {
    let name = extract_name(node, source_bytes).unwrap_or_else(|| "<anonymous>".to_string());
    let kind = match node.kind() {
        "struct_item" => "struct",
        "enum_item" => "enum",
        "trait_item" => "trait",
        "type_item" => "type_alias",
        _ => "unknown",
    }.to_string();
    
    let visibility = if let Some(vis) = node.child_by_field_name("visibility") {
        vis.utf8_text(source_bytes)?.to_string()
    } else {
        "private".to_string()
    };
    
    ast_data.types.push(TypeDef {
        name: name.clone(),
        kind: kind.clone(),
        visibility,
        fields: Vec::new(), // Could extract fields
        methods: Vec::new(), // Could extract methods
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
        doc_comment: extract_doc_comment(node, source_bytes),
    });
    
    // Add to symbols
    ast_data.symbols.push(Symbol {
        name: name.clone(),
        kind: kind.clone(),
        signature: Some(build_function_signature(node, source_bytes)),
        context: Some(extract_context(node, source_bytes)),
        line: node.start_position().row + 1,
    });
    
    // Add fingerprint
    ast_data.fingerprints.push(CodeFingerprint {
        name,
        kind,
        pattern: hash_ast_pattern(node, source_bytes),
        imports: 0,
        complexity: 0,
        flags: 0,
    });
    
    Ok(())
}

fn extract_import(
    node: tree_sitter::Node,
    source_bytes: &[u8],
    ast_data: &mut AstData,
) -> Result<()> {
    if let Some(arg_node) = node.child_by_field_name("argument") {
        let path = arg_node.utf8_text(source_bytes)?.to_string();
        let line = node.start_position().row + 1;
        
        ast_data.imports.push(Import {
            path,
            items: Vec::new(),
            alias: None,
            line,
        });
    }
    Ok(())
}

fn extract_calls_in_function(
    source_bytes: &[u8],
    cursor: &mut tree_sitter::TreeCursor,
    ast_data: &mut AstData,
    function_name: &str,
) {
    let node = cursor.node();
    
    match node.kind() {
        "call_expression" => {
            if let Some(func_node) = node.child_by_field_name("function") {
                let target = if func_node.kind() == "field_expression" {
                    // Method call
                    func_node.child_by_field_name("field")
                        .and_then(|n| n.utf8_text(source_bytes).ok())
                        .unwrap_or("unknown")
                        .to_string()
                } else {
                    // Function call
                    func_node.utf8_text(source_bytes)
                        .unwrap_or("unknown")
                        .to_string()
                };
                
                ast_data.calls.push(Call {
                    target,
                    caller: function_name.to_string(),
                    line: node.start_position().row + 1,
                    is_method: func_node.kind() == "field_expression",
                    is_async: false, // Could check for .await
                });
            }
        }
        "macro_invocation" => {
            if let Some(macro_node) = node.child_by_field_name("macro") {
                let target = format!("{}!", 
                    macro_node.utf8_text(source_bytes).unwrap_or("unknown"));
                
                ast_data.calls.push(Call {
                    target,
                    caller: function_name.to_string(),
                    line: node.start_position().row + 1,
                    is_method: false,
                    is_async: false,
                });
            }
        }
        _ => {}
    }
    
    // Recurse into children
    if cursor.goto_first_child() {
        loop {
            extract_calls_in_function(source_bytes, cursor, ast_data, function_name);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

// Helper functions

fn extract_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.to_string())
}

fn extract_return_type(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("return_type")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.trim_start_matches("->").trim().to_string())
}

fn extract_doc_comment(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    // Look for preceding comment
    if let Some(prev) = node.prev_sibling() {
        if prev.kind() == "line_comment" || prev.kind() == "block_comment" {
            return prev.utf8_text(source).ok().map(|s| s.to_string());
        }
    }
    None
}

fn extract_context(node: tree_sitter::Node, source: &[u8]) -> String {
    // Get surrounding context (up to 200 chars)
    let start = node.start_byte().saturating_sub(50);
    let end = (node.end_byte() + 50).min(source.len());
    
    String::from_utf8_lossy(&source[start..end])
        .lines()
        .take(3)
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_flags(is_async: bool, is_unsafe: bool, is_test: bool) -> u16 {
    let mut flags = 0u16;
    if is_async { flags |= 1; }
    if is_unsafe { flags |= 2; }
    if is_test { flags |= 4; }
    flags
}