use anyhow::Result;
use std::path::Path;
use tree_sitter::Node;

use super::{
    BehavioralHints, CallGraph, Documentation, DocKind, Field, FunctionFingerprint,
    FunctionInfo, ImportInfo, LanguageExtractor, Parameter, SemanticData, TypeInfo, TypeKind,
    Visibility, extract_keywords, extract_summary,
};
use crate::scrape::discovery::Language;
use crate::semantic::languages::create_parser;
use crate::semantic::fingerprint::Fingerprint;

/// Go-specific semantic extractor
pub struct GoExtractor;

impl LanguageExtractor for GoExtractor {
    fn extract(&self, path: &Path, source: &str) -> Result<SemanticData> {
        let mut data = SemanticData {
            file_path: path.to_string_lossy().to_string(),
            language: Language::Go,
            functions: Vec::new(),
            types: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
            docs: Vec::new(),
            fingerprints: Vec::new(),
            behavioral_hints: Vec::new(),
        };

        // Create parser for Go
        let mut parser = create_parser(crate::semantic::languages::Language::Go)?;

        // Parse the source code
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Go file"))?;

        // Extract semantic information from the AST
        let mut context = ParseContext::new();
        extract_node(&tree.root_node(), source, &mut data, &mut context);

        // Add call graph entries
        data.calls.extend(context.get_calls());

        Ok(data)
    }
}

/// Parse context for tracking state during extraction
struct ParseContext {
    current_function: Option<String>,
    call_entries: Vec<CallGraph>,
}

impl ParseContext {
    fn new() -> Self {
        Self {
            current_function: None,
            call_entries: Vec::new(),
        }
    }

    fn enter_function(&mut self, name: String) {
        self.current_function = Some(name);
    }

    fn exit_function(&mut self) {
        self.current_function = None;
    }

    fn add_call(&mut self, callee: String, call_type: &str, line: usize) {
        if let Some(ref caller) = self.current_function {
            self.call_entries.push(CallGraph {
                caller: caller.clone(),
                callee: callee.clone(),
                line_number: line,
                call_type: call_type.to_string(),
                is_external: is_external_call(&callee),
            });
        }
    }

    fn get_calls(self) -> Vec<CallGraph> {
        self.call_entries
    }
}

/// Recursively extract semantic information from AST nodes
fn extract_node(node: &Node, source: &str, data: &mut SemanticData, context: &mut ParseContext) {
    match node.kind() {
        "function_declaration" => {
            if let Some(func) = extract_function(node, source) {
                // Calculate fingerprint
                let fingerprint = calculate_fingerprint(node, source);
                data.fingerprints.push((func.name.clone(), fingerprint));
                
                // Extract behavioral hints
                let hints = extract_behavioral_hints(&func.name, node, source);
                if has_interesting_behavior(&hints) {
                    data.behavioral_hints.push(hints);
                }
                
                context.enter_function(func.name.clone());
                data.functions.push(func);
            }
        }
        "method_declaration" => {
            if let Some(func) = extract_method(node, source) {
                // Calculate fingerprint
                let fingerprint = calculate_fingerprint(node, source);
                data.fingerprints.push((func.name.clone(), fingerprint));
                
                // Extract behavioral hints
                let hints = extract_behavioral_hints(&func.name, node, source);
                if has_interesting_behavior(&hints) {
                    data.behavioral_hints.push(hints);
                }
                
                context.enter_function(func.name.clone());
                data.functions.push(func);
            }
        }
        "type_spec" => {
            if let Some(type_info) = extract_type_spec(node, source) {
                // Calculate fingerprint for structs/interfaces
                if matches!(type_info.kind, TypeKind::Struct | TypeKind::Interface) {
                    let fingerprint = calculate_fingerprint(node, source);
                    data.fingerprints.push((type_info.name.clone(), fingerprint));
                }
                
                data.types.push(type_info);
            }
        }
        "import_declaration" => {
            if let Some(import) = extract_import(node, source) {
                data.imports.push(import);
            }
        }
        "const_declaration" => {
            // We could extract constants as type vocabulary if needed
        }
        // Extract call expressions
        "call_expression" => {
            extract_call_expression(node, source, context);
        }
        "selector_expression" => {
            // Go method calls are selector expressions followed by call_expression
            if let Some(parent) = node.parent() {
                if parent.kind() == "call_expression" {
                    extract_method_call(node, source, context);
                }
            }
        }
        _ => {}
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_node(&child, source, data, context);
    }

    // Exit function context if we're leaving a function
    if node.kind() == "function_declaration" || node.kind() == "method_declaration" {
        context.exit_function();
    }
}

/// Extract function information
fn extract_function(node: &Node, source: &str) -> Option<FunctionInfo> {
    let mut func = FunctionInfo {
        name: String::new(),
        visibility: Visibility::Public, // Go defaults to lowercase = private, uppercase = public
        parameters: Vec::new(),
        return_type: None,
        is_async: false, // Go uses goroutines, not async/await
        is_unsafe: false, // Go doesn't have unsafe keyword like Rust
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
        doc_comment: extract_doc_comment(node, source),
        signature: String::new(),
        takes_mut_self: false, // Go doesn't have &mut self concept
        takes_mut_params: false, // Go passes by value or pointer
        returns_result: false, // Check for error return
        returns_option: false, // Go doesn't have Option, but might return pointer
        parameter_count: 0,
        has_self: false, // Methods have receivers
        context_snippet: extract_context(node, source),
    };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                let name = child.utf8_text(source.as_bytes()).ok()?.to_string();
                func.name = name.clone();
                // In Go, lowercase first letter means private
                func.visibility = if name.chars().next()?.is_lowercase() {
                    Visibility::Private
                } else {
                    Visibility::Public
                };
            }
            "parameter_list" => {
                func.parameters = extract_parameters(&child, source);
                func.parameter_count = func.parameters.len();
                
                // Check for pointer parameters (similar to &mut)
                for param in &func.parameters {
                    if let Some(ref type_ann) = param.type_annotation {
                        if type_ann.starts_with('*') {
                            func.takes_mut_params = true;
                        }
                    }
                }
            }
            "result" => {
                let return_info = extract_return_type(&child, source);
                if let Some(ref ret_type) = return_info {
                    // Check for error return (Go's equivalent of Result)
                    func.returns_result = ret_type.contains("error");
                    // Check for pointer return (similar to Option)
                    func.returns_option = ret_type.starts_with('*') && !ret_type.contains("error");
                }
                func.return_type = return_info;
            }
            _ => {}
        }
    }
    
    // Generate signature
    func.signature = generate_function_signature(&func);

    Some(func)
}

/// Extract method information
fn extract_method(node: &Node, source: &str) -> Option<FunctionInfo> {
    let mut func = FunctionInfo {
        name: String::new(),
        visibility: Visibility::Public,
        parameters: Vec::new(),
        return_type: None,
        is_async: false,
        is_unsafe: false,
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
        doc_comment: extract_doc_comment(node, source),
        signature: String::new(),
        takes_mut_self: false,
        takes_mut_params: false,
        returns_result: false,
        returns_option: false,
        parameter_count: 0,
        has_self: true, // Methods always have receivers
        context_snippet: extract_context(node, source),
    };

    let mut cursor = node.walk();
    let mut receiver_type = String::new();
    
    for child in node.children(&mut cursor) {
        match child.kind() {
            "parameter_list" => {
                // First parameter list is the receiver
                if receiver_type.is_empty() {
                    if let Some(receiver) = extract_receiver(&child, source) {
                        receiver_type = receiver.clone();
                        // Check if receiver is a pointer (similar to &mut self)
                        func.takes_mut_self = receiver.starts_with('*');
                    }
                } else {
                    // Second parameter list is the actual parameters
                    func.parameters = extract_parameters(&child, source);
                    func.parameter_count = func.parameters.len();
                    
                    // Check for pointer parameters
                    for param in &func.parameters {
                        if let Some(ref type_ann) = param.type_annotation {
                            if type_ann.starts_with('*') {
                                func.takes_mut_params = true;
                            }
                        }
                    }
                }
            }
            "field_identifier" => {
                let name = child.utf8_text(source.as_bytes()).ok()?.to_string();
                func.name = if receiver_type.is_empty() {
                    name.clone()
                } else {
                    format!("{}.{}", receiver_type.trim_start_matches('*'), name.clone())
                };
                // Check visibility based on first letter
                func.visibility = if name.chars().next()?.is_lowercase() {
                    Visibility::Private
                } else {
                    Visibility::Public
                };
            }
            "result" => {
                let return_info = extract_return_type(&child, source);
                if let Some(ref ret_type) = return_info {
                    func.returns_result = ret_type.contains("error");
                    func.returns_option = ret_type.starts_with('*') && !ret_type.contains("error");
                }
                func.return_type = return_info;
            }
            _ => {}
        }
    }
    
    // Generate signature
    func.signature = generate_method_signature(&func, &receiver_type);

    Some(func)
}

/// Generate function signature
fn generate_function_signature(func: &FunctionInfo) -> String {
    let mut sig = String::new();
    
    sig.push_str("func ");
    sig.push_str(&func.name);
    sig.push('(');
    
    let param_strs: Vec<String> = func.parameters.iter().map(|p| {
        if let Some(ref ty) = p.type_annotation {
            format!("{} {}", p.name, ty)
        } else {
            p.name.clone()
        }
    }).collect();
    sig.push_str(&param_strs.join(", "));
    sig.push(')');
    
    if let Some(ref ret) = func.return_type {
        sig.push(' ');
        sig.push_str(ret);
    }
    
    sig
}

/// Generate method signature
fn generate_method_signature(func: &FunctionInfo, receiver: &str) -> String {
    let mut sig = String::new();
    
    sig.push_str("func (");
    sig.push_str(receiver);
    sig.push_str(") ");
    
    // Extract just the method name (not the full qualified name)
    let method_name = func.name.split('.').last().unwrap_or(&func.name);
    sig.push_str(method_name);
    sig.push('(');
    
    let param_strs: Vec<String> = func.parameters.iter().map(|p| {
        if let Some(ref ty) = p.type_annotation {
            format!("{} {}", p.name, ty)
        } else {
            p.name.clone()
        }
    }).collect();
    sig.push_str(&param_strs.join(", "));
    sig.push(')');
    
    if let Some(ref ret) = func.return_type {
        sig.push(' ');
        sig.push_str(ret);
    }
    
    sig
}

/// Extract type spec (struct, interface, type alias)
fn extract_type_spec(node: &Node, source: &str) -> Option<TypeInfo> {
    let mut type_info = TypeInfo {
        name: String::new(),
        kind: TypeKind::Struct,
        visibility: Visibility::Public,
        fields: Vec::new(),
        generics: Vec::new(), // Go 1.18+ has type parameters
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
        doc_comment: extract_doc_comment(node, source),
        full_definition: node.utf8_text(source.as_bytes()).ok()?.to_string(),
        signature: String::new(),
        context_snippet: extract_context(node, source),
    };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "type_identifier" => {
                let name = child.utf8_text(source.as_bytes()).ok()?.to_string();
                type_info.name = name.clone();
                // Check visibility based on first letter
                type_info.visibility = if name.chars().next()?.is_lowercase() {
                    Visibility::Private
                } else {
                    Visibility::Public
                };
            }
            "struct_type" => {
                type_info.kind = TypeKind::Struct;
                type_info.fields = extract_struct_fields(&child, source);
            }
            "interface_type" => {
                type_info.kind = TypeKind::Interface;
                type_info.fields = extract_interface_methods(&child, source);
            }
            "type_alias" => {
                type_info.kind = TypeKind::TypeAlias;
            }
            "type_parameter_list" => {
                // Go 1.18+ generics
                type_info.generics = extract_type_parameters(&child, source);
            }
            _ => {}
        }
    }

    if type_info.name.is_empty() {
        return None;
    }
    
    // Generate signature
    type_info.signature = generate_type_signature(&type_info);

    Some(type_info)
}

/// Generate type signature
fn generate_type_signature(type_info: &TypeInfo) -> String {
    let mut sig = String::new();
    
    sig.push_str("type ");
    sig.push_str(&type_info.name);
    
    if !type_info.generics.is_empty() {
        sig.push('[');
        sig.push_str(&type_info.generics.join(", "));
        sig.push(']');
    }
    
    match type_info.kind {
        TypeKind::Struct => sig.push_str(" struct"),
        TypeKind::Interface => sig.push_str(" interface"),
        _ => {}
    }
    
    sig
}

/// Extract import declaration
fn extract_import(node: &Node, source: &str) -> Option<ImportInfo> {
    let mut import = ImportInfo {
        module: String::new(),
        items: Vec::new(),
        is_wildcard: false,
        is_external: true, // Most Go imports are external
        line_number: node.start_position().row + 1,
    };

    // Get the import spec
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "import_spec_list" {
            let mut spec_cursor = child.walk();
            for spec in child.children(&mut spec_cursor) {
                if spec.kind() == "import_spec" {
                    if let Ok(import_text) = spec.utf8_text(source.as_bytes()) {
                        let cleaned = import_text.trim().trim_matches('"');
                        import.module = cleaned.to_string();
                        
                        // Check if it's an internal import (starts with current module)
                        import.is_external = !cleaned.starts_with("./") && !cleaned.starts_with("../");
                        
                        // Extract the package name as an item
                        if let Some(last_part) = cleaned.split('/').last() {
                            import.items.push(last_part.to_string());
                        }
                    }
                }
            }
        } else if child.kind() == "import_spec" {
            if let Ok(import_text) = child.utf8_text(source.as_bytes()) {
                let cleaned = import_text.trim().trim_matches('"');
                import.module = cleaned.to_string();
                import.is_external = !cleaned.starts_with("./") && !cleaned.starts_with("../");
                
                if let Some(last_part) = cleaned.split('/').last() {
                    import.items.push(last_part.to_string());
                }
            }
        }
    }

    if import.module.is_empty() {
        return None;
    }

    Some(import)
}

/// Extract function parameters
fn extract_parameters(node: &Node, source: &str) -> Vec<Parameter> {
    let mut params = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "parameter_declaration" {
            let mut param_cursor = child.walk();
            let mut param_names = Vec::new();
            let mut param_type = None;

            for param_child in child.children(&mut param_cursor) {
                match param_child.kind() {
                    "identifier" => {
                        if let Ok(name) = param_child.utf8_text(source.as_bytes()) {
                            param_names.push(name.to_string());
                        }
                    }
                    "type_identifier" | "pointer_type" | "slice_type" | "array_type" 
                    | "map_type" | "channel_type" | "interface_type" => {
                        if let Ok(type_text) = param_child.utf8_text(source.as_bytes()) {
                            param_type = Some(type_text.to_string());
                        }
                    }
                    _ => {}
                }
            }

            // Create parameter for each name with the same type
            for name in param_names {
                params.push(Parameter {
                    name,
                    type_annotation: param_type.clone(),
                    default_value: None,
                });
            }
        } else if child.kind() == "variadic_parameter_declaration" {
            // Handle variadic parameters (...)
            if let Ok(param_text) = child.utf8_text(source.as_bytes()) {
                let name = extract_variadic_param_name(param_text);
                params.push(Parameter {
                    name,
                    type_annotation: Some(param_text.to_string()),
                    default_value: None,
                });
            }
        }
    }

    params
}

/// Extract receiver for methods
fn extract_receiver(node: &Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "parameter_declaration" {
            // Get the full receiver text
            if let Ok(receiver_text) = child.utf8_text(source.as_bytes()) {
                return Some(receiver_text.to_string());
            }
        }
    }
    None
}

/// Extract return type
fn extract_return_type(node: &Node, source: &str) -> Option<String> {
    // Go can have multiple return values
    let mut return_types = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "type_identifier" | "pointer_type" | "slice_type" | "interface_type" 
            | "array_type" | "map_type" | "channel_type" => {
                if let Ok(type_text) = child.utf8_text(source.as_bytes()) {
                    return_types.push(type_text.to_string());
                }
            }
            "parameter_list" => {
                // Named return values
                let params = extract_parameters(&child, source);
                for param in params {
                    if let Some(type_ann) = param.type_annotation {
                        if !param.name.is_empty() {
                            return_types.push(format!("{} {}", param.name, type_ann));
                        } else {
                            return_types.push(type_ann);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if return_types.is_empty() {
        None
    } else if return_types.len() == 1 {
        Some(return_types[0].clone())
    } else {
        Some(format!("({})", return_types.join(", ")))
    }
}

/// Extract struct fields
fn extract_struct_fields(node: &Node, source: &str) -> Vec<Field> {
    let mut fields = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "field_declaration_list" {
            let mut field_cursor = child.walk();
            for field_node in child.children(&mut field_cursor) {
                if field_node.kind() == "field_declaration" {
                    if let Some(field) = extract_field(&field_node, source) {
                        fields.push(field);
                    }
                }
            }
        }
    }

    fields
}

/// Extract a single field
fn extract_field(node: &Node, source: &str) -> Option<Field> {
    let mut field = Field {
        name: String::new(),
        type_annotation: None,
        visibility: Visibility::Public,
        doc_comment: extract_doc_comment(node, source),
    };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "field_identifier" => {
                let name = child.utf8_text(source.as_bytes()).ok()?.to_string();
                field.name = name.clone();
                // Check visibility based on first letter
                field.visibility = if name.chars().next()?.is_lowercase() {
                    Visibility::Private
                } else {
                    Visibility::Public
                };
            }
            "type_identifier" | "pointer_type" | "slice_type" | "array_type" 
            | "map_type" | "channel_type" | "interface_type" => {
                field.type_annotation = Some(child.utf8_text(source.as_bytes()).ok()?.to_string());
            }
            _ => {}
        }
    }

    if field.name.is_empty() {
        // Anonymous field (embedded type)
        if let Some(ref type_ann) = field.type_annotation {
            field.name = type_ann.clone();
        }
    }

    if field.name.is_empty() {
        return None;
    }

    Some(field)
}

/// Extract interface methods
fn extract_interface_methods(node: &Node, source: &str) -> Vec<Field> {
    let mut methods = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "method_spec_list" || child.kind() == "interface_body" {
            let mut method_cursor = child.walk();
            for method_node in child.children(&mut method_cursor) {
                if method_node.kind() == "method_spec" {
                    let mut method = Field {
                        name: String::new(),
                        type_annotation: None,
                        visibility: Visibility::Public,
                        doc_comment: extract_doc_comment(&method_node, source),
                    };

                    // Get the full method spec as type annotation
                    if let Ok(full_text) = method_node.utf8_text(source.as_bytes()) {
                        method.type_annotation = Some(full_text.to_string());
                        
                        // Extract method name
                        let mut spec_cursor = method_node.walk();
                        for spec_child in method_node.children(&mut spec_cursor) {
                            if spec_child.kind() == "field_identifier" {
                                if let Ok(name) = spec_child.utf8_text(source.as_bytes()) {
                                    method.name = name.to_string();
                                    break;
                                }
                            }
                        }
                    }

                    if !method.name.is_empty() {
                        methods.push(method);
                    }
                }
            }
        }
    }

    methods
}

/// Extract type parameters (Go 1.18+ generics)
fn extract_type_parameters(node: &Node, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_parameter" {
            let mut param_cursor = child.walk();
            for param_child in child.children(&mut param_cursor) {
                if param_child.kind() == "type_identifier" {
                    if let Ok(param_text) = param_child.utf8_text(source.as_bytes()) {
                        params.push(param_text.to_string());
                    }
                }
            }
        }
    }
    
    params
}

/// Extract documentation comment
fn extract_doc_comment(node: &Node, source: &str) -> Option<String> {
    // Look for comment in previous sibling
    if let Some(prev) = node.prev_sibling() {
        if prev.kind() == "comment" {
            let text = prev.utf8_text(source.as_bytes()).ok()?;
            if text.trim_start().starts_with("//") {
                // Clean up doc comment
                let cleaned = text
                    .lines()
                    .map(|line| line.trim_start().strip_prefix("//").unwrap_or(line).trim())
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                return Some(cleaned);
            }
        }
    }
    None
}

/// Extract call expression
fn extract_call_expression(node: &Node, source: &str, context: &mut ParseContext) {
    if let Some(func_node) = node.child_by_field_name("function") {
        if let Ok(callee) = func_node.utf8_text(source.as_bytes()) {
            let line = node.start_position().row + 1;
            
            // Check for goroutine call
            let call_type = if is_goroutine_call(node, source) {
                "async" // Goroutine call
            } else {
                "direct"
            };
            
            context.add_call(callee.to_string(), call_type, line);
        }
    }
}

/// Check if a call is a goroutine
fn is_goroutine_call(node: &Node, source: &str) -> bool {
    // Check if preceded by 'go' keyword
    if let Some(prev) = node.prev_sibling() {
        if prev.kind() == "go" {
            return true;
        }
    }
    
    // Check parent for go statement
    if let Some(parent) = node.parent() {
        if parent.kind() == "go_statement" {
            return true;
        }
    }
    
    false
}

/// Extract method call
fn extract_method_call(node: &Node, source: &str, context: &mut ParseContext) {
    if let Some(field_node) = node.child_by_field_name("field") {
        if let Ok(callee) = field_node.utf8_text(source.as_bytes()) {
            let line = node.start_position().row + 1;
            context.add_call(callee.to_string(), "method", line);
        }
    }
}

/// Calculate fingerprint for a node
fn calculate_fingerprint(node: &Node, source: &str) -> FunctionFingerprint {
    let fingerprint = Fingerprint::from_ast(*node, source.as_bytes());
    
    FunctionFingerprint {
        pattern: fingerprint.pattern,
        imports: fingerprint.imports,
        complexity: fingerprint.complexity,
        flags: fingerprint.flags,
    }
}

/// Extract behavioral hints from a function
fn extract_behavioral_hints(name: &str, node: &Node, source: &str) -> BehavioralHints {
    let mut hints = BehavioralHints {
        function_name: name.to_string(),
        calls_unwrap: 0, // Go doesn't have unwrap, but we'll track panic patterns
        calls_expect: 0, // Go doesn't have expect
        has_panic_macro: false,
        has_todo_macro: false, // Go doesn't have todo!
        has_unsafe_block: false, // Go has unsafe package usage
        has_mutex: false,
        has_arc: false, // Go doesn't have Arc, but has channels for sharing
    };
    
    // Walk the function body AST
    let mut cursor = node.walk();
    count_behavioral_patterns(&mut cursor, source, &mut hints);
    
    hints
}

/// Count behavioral patterns in AST
fn count_behavioral_patterns(cursor: &mut tree_sitter::TreeCursor, source: &str, hints: &mut BehavioralHints) {
    let node = cursor.node();
    
    match node.kind() {
        "call_expression" => {
            if let Some(func_node) = node.child_by_field_name("function") {
                if let Ok(func_name) = func_node.utf8_text(source.as_bytes()) {
                    // Check for panic calls
                    if func_name == "panic" {
                        hints.has_panic_macro = true;
                    }
                    // Check for log.Fatal (similar to panic)
                    if func_name.ends_with("Fatal") || func_name.ends_with("Fatalf") {
                        hints.has_panic_macro = true;
                    }
                }
            }
        }
        "selector_expression" => {
            if let Ok(selector_text) = node.utf8_text(source.as_bytes()) {
                // Check for sync.Mutex usage
                if selector_text.contains("Mutex") || selector_text.contains("RWMutex") {
                    hints.has_mutex = true;
                }
                // Check for unsafe package usage
                if selector_text.starts_with("unsafe.") {
                    hints.has_unsafe_block = true;
                }
            }
        }
        "type_identifier" => {
            if let Ok(type_name) = node.utf8_text(source.as_bytes()) {
                if type_name == "Mutex" || type_name == "RWMutex" {
                    hints.has_mutex = true;
                }
            }
        }
        "import_spec" => {
            if let Ok(import_text) = node.utf8_text(source.as_bytes()) {
                if import_text.contains("unsafe") {
                    hints.has_unsafe_block = true;
                }
            }
        }
        _ => {}
    }
    
    // Recurse into children
    if cursor.goto_first_child() {
        loop {
            count_behavioral_patterns(cursor, source, hints);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Check if hints indicate interesting behavior
fn has_interesting_behavior(hints: &BehavioralHints) -> bool {
    hints.calls_unwrap > 0 ||
    hints.calls_expect > 0 ||
    hints.has_panic_macro ||
    hints.has_todo_macro ||
    hints.has_unsafe_block ||
    hints.has_mutex ||
    hints.has_arc
}

/// Extract surrounding context for search
fn extract_context(node: &Node, source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let start_line = node.start_position().row.saturating_sub(2);
    let end_line = (node.end_position().row + 3).min(lines.len());
    
    if start_line < lines.len() && end_line <= lines.len() {
        lines[start_line..end_line].join("\n")
    } else {
        String::new()
    }
}

/// Helper to extract variadic parameter name
fn extract_variadic_param_name(param_text: &str) -> String {
    param_text
        .split("...")
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Check if a call is to an external package
fn is_external_call(callee: &str) -> bool {
    // In Go, external calls typically have a package prefix
    callee.contains('.') && !callee.starts_with("self.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_extractor_comprehensive() {
        let source = r#"
        package main
        
        import (
            "fmt"
            "sync"
        )
        
        // Point represents a 2D point
        type Point struct {
            X float64
            Y float64
            mu sync.Mutex
        }
        
        // NewPoint creates a new point
        func NewPoint(x, y float64) *Point {
            return &Point{X: x, Y: y}
        }
        
        // Distance calculates distance from origin
        func (p *Point) Distance() (float64, error) {
            p.mu.Lock()
            defer p.mu.Unlock()
            
            if p.X < 0 || p.Y < 0 {
                panic("negative coordinates")
            }
            
            return math.Sqrt(p.X*p.X + p.Y*p.Y), nil
        }
        
        func main() {
            p := NewPoint(3, 4)
            dist, err := p.Distance()
            if err != nil {
                fmt.Println("Error:", err)
            }
            fmt.Println(dist)
        }
        "#;

        let extractor = GoExtractor;
        let path = Path::new("test.go");
        let result = extractor.extract(path, source).unwrap();

        // Check functions
        assert!(result.functions.len() >= 3); // NewPoint, Distance, main
        
        // Check behavioral analysis
        let distance = result.functions.iter()
            .find(|f| f.name.contains("Distance"))
            .unwrap();
        assert!(distance.returns_result); // Returns error
        assert!(distance.takes_mut_self); // *Point receiver
        
        // Check behavioral hints
        assert!(result.behavioral_hints.iter()
            .any(|h| h.function_name.contains("Distance") && h.has_panic_macro));
        assert!(result.behavioral_hints.iter()
            .any(|h| h.has_mutex));
        
        // Check types
        assert!(result.types.iter().any(|t| t.name == "Point"));
        
        // Check imports
        assert!(result.imports.len() >= 2);
        assert!(result.imports.iter().any(|i| i.module.contains("fmt")));
        assert!(result.imports.iter().any(|i| i.module.contains("sync")));
        
        // Check fingerprints
        assert!(!result.fingerprints.is_empty());
    }
}