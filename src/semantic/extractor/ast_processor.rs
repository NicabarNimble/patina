use super::call_graph::{self, CallRelation};
use super::documentation::{self, Documentation};
use crate::semantic::fingerprint::Fingerprint;
use crate::semantic::languages::Language;

/// Code search entry
#[derive(Debug, Clone)]
pub struct CodeSearchFact {
    pub file: String,
    pub name: String,
    pub signature: String,
    pub context: String,
}

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
    pub code_search: Vec<CodeSearchFact>,
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
    pub takes_mut_params: bool,
    pub parameter_count: usize,
    pub returns_result: bool,
    pub returns_option: bool,
    pub signature: String,
}

/// Type definition fact
#[derive(Debug, Clone)]
pub struct TypeFact {
    pub file: String,
    pub name: String,
    pub line_number: usize,
    pub kind: String,
    pub is_public: bool,
    pub definition: String,
    pub visibility: String,
}

/// Import fact
#[derive(Debug, Clone)]
pub struct ImportFact {
    pub file: String,
    pub imported_item: String,
    pub imported_from: String,
    pub is_external: bool,
    pub import_kind: String,
}

/// Behavioral hint
#[derive(Debug, Clone, Default)]
pub struct BehavioralHint {
    pub file: String,
    pub function: String,
    pub calls_unwrap: usize,
    pub calls_expect: usize,
    pub has_panic_macro: bool,
    pub has_todo_macro: bool,
    pub has_unsafe_block: bool,
    pub has_mutex: bool,
    pub has_arc: bool,
}

/// Fingerprint fact
#[derive(Debug, Clone)]
pub struct FingerprintFact {
    pub file: String,
    pub symbol: String,
    pub kind: String, // function, struct, trait, impl
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

    process_node_recursive(root, source, file_path, language, None, &mut result);

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
    let normalized_kind = language.normalize_node_kind(node.kind());

    match normalized_kind {
        "function" => {
            process_function(node, source, file_path, language, result);
        }
        "struct" | "class" => {
            process_type(node, source, file_path, "struct", language, result);
        }
        "trait" | "interface" => {
            process_type(node, source, file_path, "trait", language, result);
        }
        "enum" => {
            process_type(node, source, file_path, "enum", language, result);
        }
        "impl" => {
            process_impl(node, source, file_path, result);
        }
        _ => {
            if is_import_node(node.kind(), language) {
                process_import(node, source, file_path, language, result);
            }
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
    let takes_mut_params = check_takes_mut_params(&parameters);
    let parameter_count = count_parameters(&parameters);
    let returns_result = return_type.contains("Result");
    let returns_option = return_type.contains("Option");

    // Build signature
    let signature = format!(
        "{}({}){}",
        name,
        parameters,
        if !return_type.is_empty() {
            format!(" -> {}", return_type)
        } else {
            String::new()
        }
    );

    result.functions.push(FunctionFact {
        file: file_path.to_string(),
        name: name.clone(),
        line_number,
        parameters: parameters.clone(),
        return_type: return_type.clone(),
        is_async,
        is_public,
        is_unsafe,
        generics_count: count_generics(node, language),
        takes_mut_self,
        takes_mut_params,
        parameter_count,
        returns_result,
        returns_option,
        signature: signature.clone(),
    });

    // Add to code_search
    result.code_search.push(CodeSearchFact {
        file: file_path.to_string(),
        name: name.clone(),
        signature,
        context: format!("line {}", line_number),
    });

    // Generate fingerprint
    let fingerprint = Fingerprint::from_ast(node, source);
    result.fingerprints.push(FingerprintFact {
        file: file_path.to_string(),
        symbol: name.clone(),
        kind: "function".to_string(),
        fingerprint,
    });

    // Extract behavioral hints
    let hints = extract_behavioral_hints_for_function(node, source, &name, file_path);
    if hints.calls_unwrap > 0
        || hints.calls_expect > 0
        || hints.has_panic_macro
        || hints.has_todo_macro
        || hints.has_unsafe_block
        || hints.has_mutex
        || hints.has_arc
    {
        result.behavioral_hints.push(hints);
    }
}

fn process_type(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    kind: &str,
    _language: Language,
    result: &mut ProcessingResult,
) {
    let name = extract_type_name(node, source).unwrap_or_else(|| "anonymous".to_string());
    let line_number = node.start_position().row + 1;
    let is_public = check_is_public(node, source);
    let visibility = extract_visibility(node, source);
    let definition = node.utf8_text(source).unwrap_or("").to_string();

    result.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.clone(),
        line_number,
        kind: kind.to_string(),
        is_public,
        definition,
        visibility,
    });

    // Generate fingerprint for structs and traits (matching original behavior)
    if kind == "struct" || kind == "trait" {
        let fingerprint = Fingerprint::from_ast(node, source);
        result.fingerprints.push(FingerprintFact {
            file: file_path.to_string(),
            symbol: name,
            kind: kind.to_string(),
            fingerprint,
        });
    }
}

fn process_impl(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    result: &mut ProcessingResult,
) {
    // Extract impl name (e.g., "impl Foo" or "impl Trait for Foo")
    let name = node
        .utf8_text(source)
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("impl")
        .to_string();

    // Generate fingerprint for impl blocks (matching original behavior)
    let fingerprint = Fingerprint::from_ast(node, source);
    result.fingerprints.push(FingerprintFact {
        file: file_path.to_string(),
        symbol: name,
        kind: "impl".to_string(),
        fingerprint,
    });
}

fn process_import(
    node: tree_sitter::Node,
    source: &[u8],
    file_path: &str,
    language: Language,
    result: &mut ProcessingResult,
) {
    if let Ok(import_text) = node.utf8_text(source) {
        let (imported_item, imported_from, import_kind) = parse_import(import_text, language);

        let is_external = !imported_from.starts_with("crate::")
            && !imported_from.starts_with("super::")
            && !imported_from.starts_with("self::")
            && !imported_from.starts_with("./")
            && !imported_from.starts_with("../");

        result.imports.push(ImportFact {
            file: file_path.to_string(),
            imported_item,
            imported_from,
            is_external,
            import_kind,
        });
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
    if let Some(vis) = node.child_by_field_name("visibility") {
        if let Ok(text) = vis.utf8_text(source) {
            return text.contains("pub");
        }
    }
    false
}

fn extract_visibility(node: tree_sitter::Node, source: &[u8]) -> String {
    if let Some(vis) = node.child_by_field_name("visibility") {
        if let Ok(text) = vis.utf8_text(source) {
            if text.contains("pub(crate)") {
                return "crate".to_string();
            } else if text.contains("pub") {
                return "pub".to_string();
            }
        }
    }
    "priv".to_string()
}

fn check_is_unsafe(node: tree_sitter::Node, source: &[u8]) -> bool {
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

fn count_generics(node: tree_sitter::Node, language: Language) -> usize {
    match language {
        Language::Rust => node
            .child_by_field_name("type_parameters")
            .map(|tp| {
                let mut count = 0;
                let mut cursor = tp.walk();
                if cursor.goto_first_child() {
                    loop {
                        let child = cursor.node();
                        if child.kind() == "type_identifier" || child.kind() == "lifetime" {
                            count += 1;
                        }
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
                count
            })
            .unwrap_or(0),
        _ => node
            .child_by_field_name("type_parameters")
            .map(|n| n.child_count())
            .unwrap_or(0),
    }
}

fn check_takes_mut_params(params: &str) -> bool {
    params.contains("&mut") && !params.starts_with("&mut self")
}

fn count_parameters(params: &str) -> usize {
    if params.is_empty() {
        return 0;
    }

    // Simple parameter counting - split by comma
    params.split(',').filter(|p| !p.trim().is_empty()).count()
}

fn parse_import(import_text: &str, language: Language) -> (String, String, String) {
    match language {
        Language::Rust => {
            // use std::collections::HashMap;
            if let Some(path) = import_text
                .strip_prefix("use ")
                .and_then(|s| s.strip_suffix(';'))
            {
                let parts: Vec<&str> = path.split("::").collect();
                let item = parts.last().unwrap_or(&"*").to_string();
                let from = parts[0..parts.len().saturating_sub(1)].join("::");
                ("use".to_string(), item, from)
            } else {
                (import_text.to_string(), String::new(), "use".to_string())
            }
        }
        Language::Go => {
            // import "fmt" or import ( "fmt" "strings" )
            let cleaned = import_text.replace(['(', ')', '"'], "");
            let parts: Vec<&str> = cleaned.split_whitespace().collect();
            if parts.len() >= 2 {
                (
                    parts[1].to_string(),
                    parts[1].to_string(),
                    "import".to_string(),
                )
            } else {
                (import_text.to_string(), String::new(), "import".to_string())
            }
        }
        Language::Python => {
            // import foo or from foo import bar
            if import_text.starts_with("from ") {
                let parts: Vec<&str> = import_text.split(" import ").collect();
                if parts.len() == 2 {
                    let from = parts[0].strip_prefix("from ").unwrap_or(parts[0]);
                    let item = parts[1];
                    (item.to_string(), from.to_string(), "import".to_string())
                } else {
                    (import_text.to_string(), String::new(), "import".to_string())
                }
            } else if let Some(module) = import_text.strip_prefix("import ") {
                (module.to_string(), module.to_string(), "import".to_string())
            } else {
                (import_text.to_string(), String::new(), "import".to_string())
            }
        }
        _ => (import_text.to_string(), String::new(), "import".to_string()),
    }
}

fn extract_behavioral_hints_for_function(
    node: tree_sitter::Node,
    source: &[u8],
    function_name: &str,
    file_path: &str,
) -> BehavioralHint {
    let mut hints = BehavioralHint {
        file: file_path.to_string(),
        function: function_name.to_string(),
        ..Default::default()
    };

    // Recursively walk the function body
    count_behavioral_hints(&mut hints, node, source);

    hints
}

fn count_behavioral_hints(hints: &mut BehavioralHint, node: tree_sitter::Node, source: &[u8]) {
    // Check current node
    match node.kind() {
        "method_call_expression" | "call_expression" => {
            if let Ok(text) = node.utf8_text(source) {
                if text.contains(".unwrap()") {
                    hints.calls_unwrap += 1;
                }
                if text.contains(".expect(") {
                    hints.calls_expect += 1;
                }
            }
        }
        "macro_invocation" => {
            if let Some(name) = node.child_by_field_name("macro") {
                if let Ok(macro_name) = name.utf8_text(source) {
                    match macro_name {
                        "panic" => hints.has_panic_macro = true,
                        "todo" => hints.has_todo_macro = true,
                        _ => {}
                    }
                }
            }
        }
        "unsafe_block" => {
            hints.has_unsafe_block = true;
        }
        _ => {
            // Check for type usage
            if let Ok(text) = node.utf8_text(source) {
                if text.contains("Mutex<") {
                    hints.has_mutex = true;
                }
                if text.contains("Arc<") {
                    hints.has_arc = true;
                }
            }
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            count_behavioral_hints(hints, cursor.node(), source);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn is_import_node(kind: &str, language: Language) -> bool {
    match language {
        Language::Rust => kind == "use_declaration",
        Language::Go => kind == "import_declaration",
        Language::Python => kind == "import_statement" || kind == "import_from_statement",
        Language::JavaScript
        | Language::JavaScriptJSX
        | Language::TypeScript
        | Language::TypeScriptTSX => kind == "import_statement" || kind == "import_declaration",
        Language::Solidity => kind == "import_directive",
        _ => false,
    }
}
