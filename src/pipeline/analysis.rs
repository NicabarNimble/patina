use tree_sitter::{Node, TreeCursor};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Calculate cyclomatic complexity for a function node
pub fn calculate_complexity(node: Node) -> u16 {
    let mut complexity = 1; // Base complexity
    let mut cursor = node.walk();
    count_branches(&mut cursor, &mut complexity);
    complexity as u16
}

fn count_branches(cursor: &mut TreeCursor, complexity: &mut usize) {
    let node = cursor.node();
    
    // Language-agnostic branch detection
    match node.kind() {
        // Control flow
        "if_statement" | "if_expression" | "conditional_expression" | "ternary_expression" => {
            *complexity += 1;
        }
        // Loops
        "while_statement" | "while_expression" | "for_statement" | "for_expression" | 
        "for_in_statement" | "do_statement" | "loop_expression" => {
            *complexity += 1;
        }
        // Pattern matching / switch
        "match_expression" | "switch_statement" | "case_statement" | "match_arm" => {
            *complexity += 1;
        }
        // Exception handling
        "catch_clause" | "except_clause" | "rescue_clause" => {
            *complexity += 1;
        }
        // Logical operators that create branches
        "binary_expression" => {
            if let Some(op) = node.child_by_field_name("operator") {
                let op_text = op.utf8_text(&[]).unwrap_or("");
                if op_text == "&&" || op_text == "||" || op_text == "and" || op_text == "or" {
                    *complexity += 1;
                }
            }
        }
        _ => {}
    }
    
    // Recurse into children
    if cursor.goto_first_child() {
        loop {
            count_branches(cursor, complexity);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Generate an AST pattern hash for fingerprinting
pub fn hash_ast_pattern(node: Node, source: &[u8]) -> u32 {
    let mut hasher = DefaultHasher::new();
    hash_node_shape(&node, source, &mut hasher, 0, 3); // Max depth 3 for pattern
    (hasher.finish() & 0xFFFFFFFF) as u32
}

fn hash_node_shape(node: &Node, source: &[u8], hasher: &mut impl Hasher, depth: usize, max_depth: usize) {
    if depth >= max_depth {
        return;
    }
    
    // Hash the node kind
    node.kind().hash(hasher);
    
    // For certain node types, include the text
    match node.kind() {
        "identifier" | "string_literal" | "number_literal" => {
            if let Ok(text) = node.utf8_text(source) {
                text.hash(hasher);
            }
        }
        _ => {}
    }
    
    // Recurse into children
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            hash_node_shape(&cursor.node(), source, hasher, depth + 1, max_depth);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Detect if a function is a test
pub fn is_test_function(node: Node, source: &[u8], language: &str) -> bool {
    let name = extract_function_name(node, source).unwrap_or_default();
    
    // Common test patterns
    if name.starts_with("test_") || name.ends_with("_test") || name == "test" {
        return true;
    }
    
    // Language-specific patterns
    match language {
        "rust" => {
            // Check for #[test] attribute
            if let Some(attr) = node.prev_sibling() {
                if attr.kind() == "attribute_item" {
                    let attr_text = attr.utf8_text(source).unwrap_or("");
                    return attr_text.contains("test");
                }
            }
            false
        }
        "go" => {
            // Go test functions start with Test
            name.starts_with("Test")
        }
        "python" => {
            // Python test methods
            name.starts_with("test_") || name.starts_with("Test")
        }
        "javascript" | "typescript" => {
            // Jest/Mocha patterns - check parent context
            if let Some(parent) = node.parent() {
                let parent_text = parent.utf8_text(source).unwrap_or("");
                parent_text.contains("describe") || parent_text.contains("it(") || 
                parent_text.contains("test(")
            } else {
                false
            }
        }
        _ => false
    }
}

/// Extract function name from node
pub fn extract_function_name(node: Node, source: &[u8]) -> Option<String> {
    if let Some(name_node) = node.child_by_field_name("name") {
        name_node.utf8_text(source).ok().map(|s| s.to_string())
    } else {
        None
    }
}

/// Build a full function signature
pub fn build_function_signature(node: Node, source: &[u8]) -> String {
    // Get the full text but limit to first line or 100 chars
    let full_text = node.utf8_text(source).unwrap_or("<unknown>");
    let first_line = full_text.lines().next().unwrap_or(full_text);
    
    if first_line.len() > 100 {
        format!("{}...", &first_line[..100])
    } else {
        first_line.to_string()
    }
}

/// Count lines of code, comments, and blank lines
pub fn count_lines(source: &str) -> (usize, usize, usize, usize) {
    let mut total = 0;
    let mut code = 0;
    let mut comments = 0;
    let mut blank = 0;
    
    for line in source.lines() {
        total += 1;
        let trimmed = line.trim();
        
        if trimmed.is_empty() {
            blank += 1;
        } else if trimmed.starts_with("//") || trimmed.starts_with("#") || 
                  trimmed.starts_with("/*") || trimmed.starts_with("*") {
            comments += 1;
        } else {
            code += 1;
        }
    }
    
    (total, code, comments, blank)
}