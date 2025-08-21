use anyhow::{Context, Result};
use std::collections::HashMap;
use tree_sitter::{Language, Parser, Query, QueryCursor};

pub mod analyzer;
pub mod patterns;
pub mod queries;
pub mod deep_analyzer;
pub mod fingerprint;

/// Semantic information extracted from code
#[derive(Debug, Clone)]
pub struct SemanticSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file: String,
    pub line: usize,
    pub patterns: Vec<PatternMatch>,
    pub dependencies: Vec<String>,
    pub complexity: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Struct,
    Trait,
    Impl,
    Module,
    Enum,
}

#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub pattern: String,
    pub confidence: f32,
    pub evidence: String,
}

/// Initialize tree-sitter parser for Rust
pub fn init_parser() -> Result<Parser> {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::LANGUAGE.into();
    parser
        .set_language(&language)
        .context("Failed to set Rust language")?;
    Ok(parser)
}

/// Extract semantic symbols from Rust code
pub fn extract_symbols(code: &str, file_path: &str) -> Result<Vec<SemanticSymbol>> {
    let mut parser = init_parser()?;
    let tree = parser.parse(code, None).context("Failed to parse code")?;

    let mut symbols = Vec::new();
    let mut cursor = tree.walk();

    // Visit all nodes in the AST
    visit_node(&mut cursor, code, file_path, &mut symbols);

    Ok(symbols)
}

fn visit_node(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
    file_path: &str,
    symbols: &mut Vec<SemanticSymbol>,
) {
    let node = cursor.node();

    match node.kind() {
        "function_item" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = name_node
                    .utf8_text(source.as_bytes())
                    .unwrap_or("")
                    .to_string();
                let line = name_node.start_position().row + 1;

                // Analyze the function for patterns
                let patterns = analyze_function_patterns(&node, source);
                let complexity = calculate_complexity(&node);

                symbols.push(SemanticSymbol {
                    name,
                    kind: SymbolKind::Function,
                    file: file_path.to_string(),
                    line,
                    patterns,
                    dependencies: vec![],
                    complexity,
                });
            }
        }
        "struct_item" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = name_node
                    .utf8_text(source.as_bytes())
                    .unwrap_or("")
                    .to_string();
                let line = name_node.start_position().row + 1;

                symbols.push(SemanticSymbol {
                    name,
                    kind: SymbolKind::Struct,
                    file: file_path.to_string(),
                    line,
                    patterns: vec![],
                    dependencies: vec![],
                    complexity: 1,
                });
            }
        }
        "trait_item" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = name_node
                    .utf8_text(source.as_bytes())
                    .unwrap_or("")
                    .to_string();
                let line = name_node.start_position().row + 1;

                symbols.push(SemanticSymbol {
                    name,
                    kind: SymbolKind::Trait,
                    file: file_path.to_string(),
                    line,
                    patterns: vec![],
                    dependencies: vec![],
                    complexity: 1,
                });
            }
        }
        "impl_item" => {
            // Extract impl block information
            let line = node.start_position().row + 1;
            let impl_text = node.utf8_text(source.as_bytes()).unwrap_or("");

            // Simple extraction of what's being implemented
            let name = if impl_text.contains(" for ") {
                impl_text
                    .split(" for ")
                    .nth(1)
                    .and_then(|s| s.split_whitespace().next())
                    .unwrap_or("impl")
                    .to_string()
            } else {
                "impl".to_string()
            };

            symbols.push(SemanticSymbol {
                name,
                kind: SymbolKind::Impl,
                file: file_path.to_string(),
                line,
                patterns: vec![],
                dependencies: vec![],
                complexity: 1,
            });
        }
        _ => {}
    }

    // Recursively visit children
    if cursor.goto_first_child() {
        loop {
            visit_node(cursor, source, file_path, symbols);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Analyze a function node for semantic patterns
fn analyze_function_patterns(node: &tree_sitter::Node, source: &str) -> Vec<PatternMatch> {
    let mut patterns = Vec::new();
    let function_text = node.utf8_text(source.as_bytes()).unwrap_or("");

    // Check for error handling patterns
    if function_text.contains(".context(") || function_text.contains(".with_context(") {
        patterns.push(PatternMatch {
            pattern: "error-context".to_string(),
            confidence: 0.9,
            evidence: "Uses .context() for error handling".to_string(),
        });
    }

    if function_text.contains("Result<") {
        patterns.push(PatternMatch {
            pattern: "result-return".to_string(),
            confidence: 1.0,
            evidence: "Returns Result type".to_string(),
        });
    }

    // Check for async patterns
    if function_text.starts_with("async ") || function_text.contains("pub async fn") {
        patterns.push(PatternMatch {
            pattern: "async-function".to_string(),
            confidence: 1.0,
            evidence: "Async function".to_string(),
        });
    }

    // Check for builder pattern
    if function_text.contains("self") && function_text.contains("-> Self") {
        patterns.push(PatternMatch {
            pattern: "builder-pattern".to_string(),
            confidence: 0.8,
            evidence: "Returns Self for chaining".to_string(),
        });
    }

    // Check for command pattern
    if function_text.contains("execute(") || function_text.contains("fn execute") {
        patterns.push(PatternMatch {
            pattern: "command-pattern".to_string(),
            confidence: 0.7,
            evidence: "Has execute method".to_string(),
        });
    }

    patterns
}

/// Calculate cyclomatic complexity of a node
fn calculate_complexity(node: &tree_sitter::Node) -> usize {
    let mut complexity = 1; // Base complexity
    let mut cursor = node.walk();

    count_complexity_nodes(&mut cursor, &mut complexity);
    complexity
}

fn count_complexity_nodes(cursor: &mut tree_sitter::TreeCursor, complexity: &mut usize) {
    let node = cursor.node();

    // Each branch adds to complexity
    match node.kind() {
        "if_expression" | "match_expression" | "while_expression" | "for_expression"
        | "loop_expression" => {
            *complexity += 1;
        }
        "match_arm" => {
            *complexity += 1;
        }
        _ => {}
    }

    // Recursively count in children
    if cursor.goto_first_child() {
        loop {
            count_complexity_nodes(cursor, complexity);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}
