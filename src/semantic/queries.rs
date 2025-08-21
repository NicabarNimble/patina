use tree_sitter::{Query, Node};
use anyhow::Result;

/// Tree-sitter queries for semantic pattern detection
/// These are like SQL for AST - they find structural patterns

pub struct SemanticQueries {
    pub error_propagation: Query,
    pub error_context: Query,
    pub api_boundary: Query,
    pub state_machine: Query,
    pub dependency_injection: Query,
}

impl SemanticQueries {
    pub fn new(language: tree_sitter::Language) -> Result<Self> {
        // Create each query and handle errors properly
        let error_propagation = Query::new(
            &language,
            r#"
            (function_item
              body: (block
                (expression_statement
                  (try_expression
                    (field_expression
                      field: (field_identifier) @method
                      (#any-of? @method "context" "with_context")
                    )
                  )
                )
              )
              return_type: (generic_type
                type: (type_identifier) @return_type
                (#eq? @return_type "Result")
              )
            ) @function
            "#
        )?;
        
        let error_context = Query::new(
            &language,
            r#"
            (method_call_expression
              receiver: (_)
              method: (field_identifier) @method
              (#any-of? @method "context" "with_context" "map_err")
              arguments: (arguments
                (string_literal) @context_msg
              )
            ) @error_handling
            "#
        )?;
        
        let api_boundary = Query::new(
            &language,
            r#"
            (function_item
              visibility: (visibility_modifier) @vis
              (#eq? @vis "pub")
              name: (identifier) @name
              return_type: (generic_type
                type: (type_identifier) @return_type
                (#eq? @return_type "Result")
              )
            ) @api_function
            "#
        )?;
        
        let state_machine = Query::new(
            &language,
            r#"
            (match_expression
              value: (field_expression
                field: (field_identifier) @field
                (#any-of? @field "state" "status" "phase")
              )
              body: (match_block
                (match_arm)+ @arms
              )
            ) @state_machine
            "#
        )?;
        
        let dependency_injection = Query::new(
            &language,
            r#"
            (function_item
              parameters: (parameters
                (parameter
                  type: (reference_type
                    type: (generic_type
                      type: (type_identifier) @trait_type
                      (#any-of? @trait_type "dyn" "impl")
                    )
                  )
                )
              )
            ) @injected_function
            "#
        )?;
        
        Ok(Self {
            error_propagation,
            error_context,
            api_boundary,
            state_machine,
            dependency_injection,
        })
    }
}

/// Semantic patterns we can detect from AST structure
#[derive(Debug, Clone)]
pub enum SemanticPattern {
    /// Function handles errors properly
    ErrorPropagation {
        propagates: bool,
        adds_context: bool,
        error_types: Vec<String>,
    },
    
    /// Function is an API boundary
    ApiBoundary {
        is_public: bool,
        returns_result: bool,
        has_documentation: bool,
    },
    
    /// Code implements state machine
    StateMachine {
        states: Vec<String>,
        transitions: usize,
    },
    
    /// Function uses dependency injection
    DependencyInjection {
        trait_params: Vec<String>,
        mock_testable: bool,
    },
    
    /// Resource management pattern
    ResourceManagement {
        has_drop_impl: bool,
        uses_raii: bool,
        cleanup_in_drop: bool,
    },
}

/// Extract semantic meaning from AST node
pub fn extract_semantic_meaning(node: Node, source: &str) -> Vec<SemanticPattern> {
    let mut patterns = Vec::new();
    
    // Walk the node and extract patterns
    let mut cursor = node.walk();
    extract_patterns_recursive(&mut cursor, source, &mut patterns);
    
    patterns
}

fn extract_patterns_recursive(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
    patterns: &mut Vec<SemanticPattern>,
) {
    let node = cursor.node();
    
    match node.kind() {
        "function_item" => {
            // Check if it's an API boundary
            let is_public = node.child_by_field_name("visibility").is_some();
            let returns_result = node
                .child_by_field_name("return_type")
                .and_then(|rt| rt.utf8_text(source.as_bytes()).ok())
                .map(|s| s.contains("Result"))
                .unwrap_or(false);
            
            if is_public && returns_result {
                patterns.push(SemanticPattern::ApiBoundary {
                    is_public: true,
                    returns_result: true,
                    has_documentation: check_has_docs(&node, source),
                });
            }
            
            // Check error handling
            let body = node.child_by_field_name("body");
            if let Some(body) = body {
                let (propagates, adds_context) = check_error_handling(&body, source);
                if propagates {
                    patterns.push(SemanticPattern::ErrorPropagation {
                        propagates,
                        adds_context,
                        error_types: extract_error_types(&body, source),
                    });
                }
            }
        }
        
        "impl_item" => {
            // Check for Drop implementation
            let trait_name = node
                .child_by_field_name("trait")
                .and_then(|t| t.utf8_text(source.as_bytes()).ok());
            
            if trait_name == Some("Drop") {
                patterns.push(SemanticPattern::ResourceManagement {
                    has_drop_impl: true,
                    uses_raii: true,
                    cleanup_in_drop: check_cleanup_in_drop(&node, source),
                });
            }
        }
        
        "match_expression" => {
            // Check for state machine pattern
            if let Some(value) = node.child_by_field_name("value") {
                let value_text = value.utf8_text(source.as_bytes()).unwrap_or("");
                if value_text.contains("state") || value_text.contains("status") {
                    let states = extract_match_arms(&node, source);
                    patterns.push(SemanticPattern::StateMachine {
                        transitions: states.len(),
                        states,
                    });
                }
            }
        }
        
        _ => {}
    }
    
    // Recurse to children
    if cursor.goto_first_child() {
        loop {
            extract_patterns_recursive(cursor, source, patterns);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

// Helper functions
fn check_has_docs(node: &Node, _source: &str) -> bool {
    // Check if previous sibling is a doc comment
    if let Some(prev) = node.prev_sibling() {
        prev.kind() == "doc_comment" || prev.kind() == "line_comment"
    } else {
        false
    }
}

fn check_error_handling(node: &Node, source: &str) -> (bool, bool) {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");
    let propagates = text.contains("?") || text.contains("return Err");
    let adds_context = text.contains(".context(") || text.contains(".with_context(");
    (propagates, adds_context)
}

fn extract_error_types(_node: &Node, _source: &str) -> Vec<String> {
    // This would need more sophisticated parsing
    vec![]
}

fn check_cleanup_in_drop(node: &Node, source: &str) -> bool {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");
    text.contains("close") || text.contains("flush") || text.contains("cleanup")
}

fn extract_match_arms(node: &Node, source: &str) -> Vec<String> {
    let mut arms = Vec::new();
    let mut cursor = node.walk();
    
    if cursor.goto_first_child() {
        loop {
            if cursor.node().kind() == "match_arm" {
                if let Some(pattern) = cursor.node().child_by_field_name("pattern") {
                    let arm_text = pattern.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                    arms.push(arm_text);
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    
    arms
}