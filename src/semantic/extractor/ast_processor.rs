use crate::semantic::languages::Language;
use crate::semantic::fingerprint::Fingerprint;
use super::documentation::{self, Documentation};
use super::call_graph::{self, CallRelation};

/// Result of processing an AST node
#[derive(Debug, Default)]
pub struct ProcessingResult {
    pub functions: Vec<FunctionFact>,
    pub types: Vec<TypeFact>,
    pub imports: Vec<ImportFact>,
    pub behavioral_hints: Vec<BehavioralHint>,
    pub fingerprints: Vec<FingerprintFact>,
    pub documentation: Vec<DocumentationFact>,
    pub call_graph: Vec<CallRelation>,
}

/// Function fact extracted from AST
#[derive(Debug, Clone)]
pub struct FunctionFact {
    pub file: String,
    pub name: String,
    pub line_number: usize,
    pub parameters: String,
    pub return_type: String,
    pub is_async: bool,
    pub is_public: bool,
    pub is_unsafe: bool,
    pub generics_count: usize,
    pub takes_mut_self: bool,
    pub returns_result: bool,
    pub returns_option: bool,
}

/// Type definition fact
#[derive(Debug, Clone)]
pub struct TypeFact {
    pub file: String,
    pub name: String,
    pub line_number: usize,
    pub kind: String,
    pub is_public: bool,
}

/// Import fact
#[derive(Debug, Clone)]
pub struct ImportFact {
    pub file: String,
    pub import_path: String,
    pub is_external: bool,
}

/// Behavioral hint
#[derive(Debug, Clone)]
pub struct BehavioralHint {
    pub file: String,
    pub hint_type: String,
    pub location: String,
    pub context: String,
}

/// Fingerprint fact
#[derive(Debug, Clone)]
pub struct FingerprintFact {
    pub file: String,
    pub symbol: String,
    pub fingerprint: Fingerprint,
}

/// Documentation fact
#[derive(Debug, Clone)]
pub struct DocumentationFact {
    pub file: String,
    pub symbol_name: String,
    pub symbol_type: String,
    pub line_number: usize,
    pub documentation: Documentation,
}

/// Process an entire AST tree
pub fn process_tree(
    tree: &tree_sitter::Tree,
    source: &[u8],
    file_path: &str,
    language: Language,
) -> ProcessingResult {
    let mut result = ProcessingResult::default();
    let root = tree.root_node();
    
    process_node_recursive(
        root,
        source,
        file_path,
        language,
        None, // No current function at root
        &mut result,
    );
    
    result
}

/// Recursively process an AST node
fn process_node_recursive(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    language: Language,
    current_function: Option<&str>,
    result: &mut ProcessingResult,
) {
    // Normalize node kind for cross-language compatibility
    let normalized_kind = language.normalize_node_kind(node.kind());
    
    match normalized_kind {
        "function" => {
            process_function(node, source, file_path, language, result);
        },
        "struct" | "class" => {
            process_type(node, source, file_path, "struct", result);
        },
        "trait" | "interface" => {
            process_type(node, source, file_path, "trait", result);
        },
        "enum" => {
            process_type(node, source, file_path, "enum", result);
        },
        _ => {
            // Check for imports
            if is_import_node(node.kind(), language) {
                process_import(node, source, file_path, result);
            }
            
            // Check for behavioral hints
            check_behavioral_hints(node, source, file_path, result);
        }
    }
    
    // Extract call graph if we're inside a function
    if let Some(func_name) = current_function {
        let calls = call_graph::extract_calls(node, source, Some(func_name), language);
        result.call_graph.extend(calls);
    }
    
    // Determine current function context for children
    let child_function = if normalized_kind == "function" {
        extract_function_name(node, source)
    } else {
        current_function.map(|s| s.to_string())
    };
    
    // Recurse into children
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            process_node_recursive(
                cursor.node(),
                source,
                file_path,
                language,
                child_function.as_deref(),
                result,
            );
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn process_function(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    language: Language,
    result: &mut ProcessingResult,
) {
    let name = extract_function_name(node, source).unwrap_or_else(|| "anonymous".to_string());
    let line_number = node.start_position().row + 1;
    
    // Extract documentation
    if let Some(doc) = documentation::extract(node, source, language) {
        result.documentation.push(DocumentationFact {
            file: file_path.to_string(),
            symbol_name: name.clone(),
            symbol_type: "function".to_string(),
            line_number,
            documentation: doc,
        });
    }
    
    // Extract function facts
    let parameters = extract_parameters(node, source);
    let return_type = extract_return_type(node, source);
    let is_async = check_is_async(node, source);
    let is_public = check_is_public(node, source);
    let is_unsafe = check_is_unsafe(node, source);
    let takes_mut_self = parameters.contains("&mut self");
    let returns_result = return_type.contains("Result");
    let returns_option = return_type.contains("Option");
    
    result.functions.push(FunctionFact {
        file: file_path.to_string(),
        name: name.clone(),
        line_number,
        parameters,
        return_type,
        is_async,
        is_public,
        is_unsafe,
        generics_count: count_generics(node),
        takes_mut_self,
        returns_result,
        returns_option,
    });
    
    // Generate fingerprint
    let fingerprint = Fingerprint::from_ast(node, source);
    result.fingerprints.push(FingerprintFact {
        file: file_path.to_string(),
        symbol: name,
        fingerprint,
    });
}

fn process_type(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    kind: &str,
    result: &mut ProcessingResult,
) {
    let name = extract_type_name(node, source).unwrap_or_else(|| "anonymous".to_string());
    let line_number = node.start_position().row + 1;
    let is_public = check_is_public(node, source);
    
    result.types.push(TypeFact {
        file: file_path.to_string(),
        name,
        line_number,
        kind: kind.to_string(),
        is_public,
    });
}

fn process_import(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    result: &mut ProcessingResult,
) {
    if let Ok(import_text) = node.utf8_text(source) {
        let is_external = import_text.contains("crate::") == false 
            && import_text.contains("super::") == false
            && import_text.contains("self::") == false;
        
        result.imports.push(ImportFact {
            file: file_path.to_string(),
            import_path: import_text.to_string(),
            is_external,
        });
    }
}

fn check_behavioral_hints(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    result: &mut ProcessingResult,
) {
    // Check for unwrap calls
    if node.kind() == "method_call_expression" {
        if let Some(method) = node.child_by_field_name("name") {
            if let Ok(method_name) = method.utf8_text(source) {
                if method_name == "unwrap" || method_name == "expect" {
                    result.behavioral_hints.push(BehavioralHint {
                        file: file_path.to_string(),
                        hint_type: "unwrap".to_string(),
                        location: format!("line {}", node.start_position().row + 1),
                        context: method_name.to_string(),
                    });
                }
            }
        }
    }
    
    // Check for todo/fixme comments
    if node.kind().contains("comment") {
        if let Ok(comment) = node.utf8_text(source) {
            if comment.contains("TODO") || comment.contains("FIXME") || comment.contains("HACK") {
                result.behavioral_hints.push(BehavioralHint {
                    file: file_path.to_string(),
                    hint_type: "todo".to_string(),
                    location: format!("line {}", node.start_position().row + 1),
                    context: comment.to_string(),
                });
            }
        }
    }
}

// Helper functions

fn extract_function_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.to_string())
}

fn extract_type_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.to_string())
}

fn extract_parameters(node: tree_sitter::Node, source: &[u8]) -> String {
    node.child_by_field_name("parameters")
        .and_then(|n| n.utf8_text(source).ok())
        .unwrap_or("")
        .to_string()
}

fn extract_return_type(node: tree_sitter::Node, source: &[u8]) -> String {
    node.child_by_field_name("return_type")
        .and_then(|n| n.utf8_text(source).ok())
        .unwrap_or("")
        .to_string()
}

fn check_is_async(node: tree_sitter::Node, source: &[u8]) -> bool {
    // Check for async keyword
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if let Ok(text) = child.utf8_text(source) {
                if text == "async" {
                    return true;
                }
            }
        }
    }
    false
}

fn check_is_public(node: tree_sitter::Node, source: &[u8]) -> bool {
    // Check for pub keyword or visibility modifier
    if let Some(vis) = node.child_by_field_name("visibility") {
        if let Ok(text) = vis.utf8_text(source) {
            return text.contains("pub");
        }
    }
    false
}

fn check_is_unsafe(node: tree_sitter::Node, source: &[u8]) -> bool {
    // Check for unsafe keyword
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if let Ok(text) = child.utf8_text(source) {
                if text == "unsafe" {
                    return true;
                }
            }
        }
    }
    false
}

fn count_generics(node: tree_sitter::Node) -> usize {
    node.child_by_field_name("type_parameters")
        .map(|n| n.child_count())
        .unwrap_or(0)
}

fn is_import_node(kind: &str, language: Language) -> bool {
    match language {
        Language::Rust => kind == "use_declaration",
        Language::Go => kind == "import_declaration",
        Language::Python => kind == "import_statement" || kind == "import_from_statement",
        Language::JavaScript | Language::JavaScriptJSX | 
        Language::TypeScript | Language::TypeScriptTSX => {
            kind == "import_statement" || kind == "import_declaration"
        },
        Language::Solidity => kind == "import_directive",
        _ => false,
    }
}