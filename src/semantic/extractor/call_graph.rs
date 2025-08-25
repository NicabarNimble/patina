use crate::semantic::languages::Language;

/// A function call relationship
#[derive(Debug, Clone)]
pub struct CallRelation {
    pub caller: String,
    pub callee: String,
    pub call_type: CallType,
    pub line_number: usize,
}

/// Type of function call
#[derive(Debug, Clone)]
pub enum CallType {
    Direct,
    Method,
    Async,
    Constructor,
    Callback,
}

impl CallType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CallType::Direct => "direct",
            CallType::Method => "method",
            CallType::Async => "async",
            CallType::Constructor => "constructor",
            CallType::Callback => "callback",
        }
    }
}

/// Extract call graph relationships from an AST node
pub fn extract_calls(
    node: tree_sitter::Node,
    source: &[u8],
    current_function: Option<&str>,
    language: Language,
) -> Vec<CallRelation> {
    let mut calls = Vec::new();
    
    if let Some(caller) = current_function {
        // Look for different types of calls based on language
        match language {
            Language::Rust => extract_rust_calls(node, source, caller, &mut calls),
            Language::Go => extract_go_calls(node, source, caller, &mut calls),
            Language::Python => extract_python_calls(node, source, caller, &mut calls),
            Language::JavaScript | Language::JavaScriptJSX | 
            Language::TypeScript | Language::TypeScriptTSX => {
                extract_js_ts_calls(node, source, caller, &mut calls)
            },
            Language::Solidity => extract_solidity_calls(node, source, caller, &mut calls),
            _ => {},
        }
    }
    
    calls
}

fn extract_rust_calls(
    node: tree_sitter::Node,
    source: &[u8],
    caller: &str,
    calls: &mut Vec<CallRelation>,
) {
    match node.kind() {
        "call_expression" => {
            if let Some(function) = node.child_by_field_name("function") {
                if let Ok(callee) = function.utf8_text(source) {
                    // Clean up the callee name (remove generics, etc)
                    let callee = callee.split("::").last().unwrap_or(callee);
                    let callee = callee.split('<').next().unwrap_or(callee);
                    
                    calls.push(CallRelation {
                        caller: caller.to_string(),
                        callee: callee.to_string(),
                        call_type: CallType::Direct,
                        line_number: node.start_position().row + 1,
                    });
                }
            }
        },
        "method_call_expression" => {
            if let Some(method) = node.child_by_field_name("name") {
                if let Ok(callee) = method.utf8_text(source) {
                    // Check if it's an await
                    let call_type = if callee == "await" {
                        CallType::Async
                    } else {
                        CallType::Method
                    };
                    
                    calls.push(CallRelation {
                        caller: caller.to_string(),
                        callee: callee.to_string(),
                        call_type,
                        line_number: node.start_position().row + 1,
                    });
                }
            }
        },
        _ => {},
    }
    
    // Recurse into children
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            extract_rust_calls(cursor.node(), source, caller, calls);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_go_calls(
    node: tree_sitter::Node,
    source: &[u8],
    caller: &str,
    calls: &mut Vec<CallRelation>,
) {
    match node.kind() {
        "call_expression" => {
            if let Some(function) = node.child_by_field_name("function") {
                if let Ok(callee) = function.utf8_text(source) {
                    let call_type = if callee.starts_with("New") || callee.starts_with("new") {
                        CallType::Constructor
                    } else {
                        CallType::Direct
                    };
                    
                    calls.push(CallRelation {
                        caller: caller.to_string(),
                        callee: callee.to_string(),
                        call_type,
                        line_number: node.start_position().row + 1,
                    });
                }
            }
        },
        "selector_expression" => {
            // Method calls in Go
            if let Some(field) = node.child_by_field_name("field") {
                if let Ok(callee) = field.utf8_text(source) {
                    calls.push(CallRelation {
                        caller: caller.to_string(),
                        callee: callee.to_string(),
                        call_type: CallType::Method,
                        line_number: node.start_position().row + 1,
                    });
                }
            }
        },
        _ => {},
    }
    
    // Recurse
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            extract_go_calls(cursor.node(), source, caller, calls);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_python_calls(
    node: tree_sitter::Node,
    source: &[u8],
    caller: &str,
    calls: &mut Vec<CallRelation>,
) {
    match node.kind() {
        "call" => {
            if let Some(function) = node.child_by_field_name("function") {
                if let Ok(callee) = function.utf8_text(source) {
                    // Check for constructor pattern
                    let call_type = if callee.chars().next().map_or(false, |c| c.is_uppercase()) {
                        CallType::Constructor
                    } else {
                        CallType::Direct
                    };
                    
                    calls.push(CallRelation {
                        caller: caller.to_string(),
                        callee: callee.split('.').last().unwrap_or(callee).to_string(),
                        call_type,
                        line_number: node.start_position().row + 1,
                    });
                }
            }
        },
        "attribute" => {
            // Method calls in Python
            if let Some(attr) = node.child_by_field_name("attribute") {
                if let Ok(callee) = attr.utf8_text(source) {
                    calls.push(CallRelation {
                        caller: caller.to_string(),
                        callee: callee.to_string(),
                        call_type: CallType::Method,
                        line_number: node.start_position().row + 1,
                    });
                }
            }
        },
        _ => {},
    }
    
    // Recurse
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            extract_python_calls(cursor.node(), source, caller, calls);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_js_ts_calls(
    node: tree_sitter::Node,
    source: &[u8],
    caller: &str,
    calls: &mut Vec<CallRelation>,
) {
    match node.kind() {
        "call_expression" => {
            if let Some(function) = node.child_by_field_name("function") {
                if let Ok(callee) = function.utf8_text(source) {
                    // Check for constructor
                    let call_type = if callee == "new" || callee.starts_with("new ") {
                        CallType::Constructor
                    } else {
                        CallType::Direct
                    };
                    
                    calls.push(CallRelation {
                        caller: caller.to_string(),
                        callee: callee.split('.').last().unwrap_or(callee).to_string(),
                        call_type,
                        line_number: node.start_position().row + 1,
                    });
                }
            }
        },
        "member_expression" => {
            // Method calls
            if let Some(property) = node.child_by_field_name("property") {
                if let Ok(callee) = property.utf8_text(source) {
                    calls.push(CallRelation {
                        caller: caller.to_string(),
                        callee: callee.to_string(),
                        call_type: CallType::Method,
                        line_number: node.start_position().row + 1,
                    });
                }
            }
        },
        "await_expression" => {
            // Async calls
            if let Some(expr) = node.child(1) {
                if expr.kind() == "call_expression" {
                    if let Some(function) = expr.child_by_field_name("function") {
                        if let Ok(callee) = function.utf8_text(source) {
                            calls.push(CallRelation {
                                caller: caller.to_string(),
                                callee: callee.split('.').last().unwrap_or(callee).to_string(),
                                call_type: CallType::Async,
                                line_number: node.start_position().row + 1,
                            });
                        }
                    }
                }
            }
        },
        _ => {},
    }
    
    // Recurse
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            extract_js_ts_calls(cursor.node(), source, caller, calls);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn extract_solidity_calls(
    node: tree_sitter::Node,
    source: &[u8],
    caller: &str,
    calls: &mut Vec<CallRelation>,
) {
    match node.kind() {
        "call_expression" | "function_call_expression" => {
            if let Some(function) = node.child(0) {
                if let Ok(callee) = function.utf8_text(source) {
                    calls.push(CallRelation {
                        caller: caller.to_string(),
                        callee: callee.to_string(),
                        call_type: CallType::Direct,
                        line_number: node.start_position().row + 1,
                    });
                }
            }
        },
        "member_expression" => {
            if let Some(member) = node.child_by_field_name("property") {
                if let Ok(callee) = member.utf8_text(source) {
                    calls.push(CallRelation {
                        caller: caller.to_string(),
                        callee: callee.to_string(),
                        call_type: CallType::Method,
                        line_number: node.start_position().row + 1,
                    });
                }
            }
        },
        _ => {},
    }
    
    // Recurse
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            extract_solidity_calls(cursor.node(), source, caller, calls);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}