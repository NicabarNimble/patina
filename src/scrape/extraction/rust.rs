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

/// Rust-specific semantic extractor
pub struct RustExtractor;

impl LanguageExtractor for RustExtractor {
    fn extract(&self, path: &Path, source: &str) -> Result<SemanticData> {
        let mut data = SemanticData {
            file_path: path.to_string_lossy().to_string(),
            language: Language::Rust,
            functions: Vec::new(),
            types: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
            docs: Vec::new(),
            fingerprints: Vec::new(),
            behavioral_hints: Vec::new(),
        };
        
        // Create parser for Rust
        let mut parser = create_parser(crate::semantic::languages::Language::Rust)?;
        
        // Parse the source code
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Rust file"))?;
        
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
        "function_item" => {
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
        "struct_item" => {
            if let Some(type_info) = extract_struct(node, source) {
                // Calculate fingerprint for struct
                let fingerprint = calculate_fingerprint(node, source);
                data.fingerprints.push((type_info.name.clone(), fingerprint));
                
                data.types.push(type_info);
            }
        }
        "enum_item" => {
            if let Some(type_info) = extract_enum(node, source) {
                // Calculate fingerprint for enum
                let fingerprint = calculate_fingerprint(node, source);
                data.fingerprints.push((type_info.name.clone(), fingerprint));
                
                data.types.push(type_info);
            }
        }
        "trait_item" => {
            if let Some(type_info) = extract_trait(node, source) {
                // Calculate fingerprint for trait
                let fingerprint = calculate_fingerprint(node, source);
                data.fingerprints.push((type_info.name.clone(), fingerprint));
                
                data.types.push(type_info);
            }
        }
        "type_alias" => {
            if let Some(type_info) = extract_type_alias(node, source) {
                data.types.push(type_info);
            }
        }
        "use_declaration" => {
            if let Some(import) = extract_import(node, source) {
                data.imports.push(import);
            }
        }
        "impl_item" => {
            // Extract methods from impl blocks
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "function_item" {
                    if let Some(func) = extract_function(&child, source) {
                        // Calculate fingerprint
                        let fingerprint = calculate_fingerprint(&child, source);
                        data.fingerprints.push((func.name.clone(), fingerprint));
                        
                        // Extract behavioral hints
                        let hints = extract_behavioral_hints(&func.name, &child, source);
                        if has_interesting_behavior(&hints) {
                            data.behavioral_hints.push(hints);
                        }
                        
                        context.enter_function(func.name.clone());
                        data.functions.push(func);
                    }
                }
            }
            
            // Calculate fingerprint for impl block
            let fingerprint = calculate_fingerprint(node, source);
            data.fingerprints.push(("impl".to_string(), fingerprint));
        }
        // Extract call expressions
        "call_expression" => {
            extract_call_expression(node, source, context);
        }
        "method_call_expression" => {
            extract_method_call(node, source, context);
        }
        _ => {}
    }
    
    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_node(&child, source, data, context);
    }
    
    // Exit function context if we're leaving a function
    if node.kind() == "function_item" {
        context.exit_function();
    }
}

/// Extract function information from a function_item node
fn extract_function(node: &Node, source: &str) -> Option<FunctionInfo> {
    let mut func = FunctionInfo {
        name: String::new(),
        visibility: Visibility::Private,
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
        has_self: false,
        context_snippet: extract_context(node, source),
    };
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "visibility_modifier" => {
                if child.utf8_text(source.as_bytes()).ok()? == "pub" {
                    func.visibility = Visibility::Public;
                }
            }
            "identifier" => {
                func.name = child.utf8_text(source.as_bytes()).ok()?.to_string();
            }
            "parameters" => {
                let params = extract_parameters(&child, source);
                func.parameter_count = params.len();
                
                // Check for mut self and mut params
                for param in &params {
                    if param.name == "self" || param.name == "&self" {
                        func.has_self = true;
                    }
                    if param.name == "&mut self" || param.name.contains("mut self") {
                        func.takes_mut_self = true;
                        func.has_self = true;
                    }
                    if let Some(ref type_ann) = param.type_annotation {
                        if type_ann.contains("&mut ") {
                            func.takes_mut_params = true;
                        }
                    }
                }
                
                func.parameters = params;
            }
            "type" => {
                // This is the return type (after ->)
                let return_str = child.utf8_text(source.as_bytes()).ok()?.to_string();
                func.returns_result = return_str.contains("Result<");
                func.returns_option = return_str.contains("Option<");
                func.return_type = Some(return_str);
            }
            _ => {}
        }
    }
    
    // Check for async/unsafe modifiers
    let func_text = node.utf8_text(source.as_bytes()).ok()?;
    func.is_async = func_text.starts_with("async ") || func_text.contains(" async ");
    func.is_unsafe = func_text.starts_with("unsafe ") || func_text.contains(" unsafe ");
    
    // Generate signature
    func.signature = generate_function_signature(&func);
    
    Some(func)
}

/// Generate function signature
fn generate_function_signature(func: &FunctionInfo) -> String {
    let mut sig = String::new();
    
    if func.visibility == Visibility::Public {
        sig.push_str("pub ");
    }
    if func.is_async {
        sig.push_str("async ");
    }
    if func.is_unsafe {
        sig.push_str("unsafe ");
    }
    
    sig.push_str("fn ");
    sig.push_str(&func.name);
    sig.push('(');
    
    let param_strs: Vec<String> = func.parameters.iter().map(|p| {
        if let Some(ref ty) = p.type_annotation {
            format!("{}: {}", p.name, ty)
        } else {
            p.name.clone()
        }
    }).collect();
    sig.push_str(&param_strs.join(", "));
    sig.push(')');
    
    if let Some(ref ret) = func.return_type {
        sig.push_str(" -> ");
        sig.push_str(ret);
    }
    
    sig
}

/// Extract struct information
fn extract_struct(node: &Node, source: &str) -> Option<TypeInfo> {
    let mut type_info = TypeInfo {
        name: String::new(),
        kind: TypeKind::Struct,
        visibility: Visibility::Private,
        fields: Vec::new(),
        generics: Vec::new(),
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
            "visibility_modifier" => {
                if child.utf8_text(source.as_bytes()).ok()? == "pub" {
                    type_info.visibility = Visibility::Public;
                }
            }
            "type_identifier" => {
                type_info.name = child.utf8_text(source.as_bytes()).ok()?.to_string();
            }
            "field_declaration_list" => {
                type_info.fields = extract_fields(&child, source);
            }
            "type_parameters" => {
                type_info.generics = extract_generics(&child, source);
            }
            _ => {}
        }
    }
    
    // Generate signature
    type_info.signature = generate_type_signature(&type_info);
    
    Some(type_info)
}

/// Extract enum information
fn extract_enum(node: &Node, source: &str) -> Option<TypeInfo> {
    let mut type_info = TypeInfo {
        name: String::new(),
        kind: TypeKind::Enum,
        visibility: Visibility::Private,
        fields: Vec::new(), // Enum variants stored as fields
        generics: Vec::new(),
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
            "visibility_modifier" => {
                if child.utf8_text(source.as_bytes()).ok()? == "pub" {
                    type_info.visibility = Visibility::Public;
                }
            }
            "type_identifier" => {
                type_info.name = child.utf8_text(source.as_bytes()).ok()?.to_string();
            }
            "enum_variant_list" => {
                type_info.fields = extract_enum_variants(&child, source);
            }
            "type_parameters" => {
                type_info.generics = extract_generics(&child, source);
            }
            _ => {}
        }
    }
    
    // Generate signature
    type_info.signature = generate_type_signature(&type_info);
    
    Some(type_info)
}

/// Extract trait information
fn extract_trait(node: &Node, source: &str) -> Option<TypeInfo> {
    let mut type_info = TypeInfo {
        name: String::new(),
        kind: TypeKind::Trait,
        visibility: Visibility::Private,
        fields: Vec::new(), // Trait methods stored as fields
        generics: Vec::new(),
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
            "visibility_modifier" => {
                if child.utf8_text(source.as_bytes()).ok()? == "pub" {
                    type_info.visibility = Visibility::Public;
                }
            }
            "type_identifier" => {
                type_info.name = child.utf8_text(source.as_bytes()).ok()?.to_string();
            }
            "type_parameters" => {
                type_info.generics = extract_generics(&child, source);
            }
            _ => {}
        }
    }
    
    // Generate signature
    type_info.signature = generate_type_signature(&type_info);
    
    Some(type_info)
}

/// Extract type alias
fn extract_type_alias(node: &Node, source: &str) -> Option<TypeInfo> {
    let mut type_info = TypeInfo {
        name: String::new(),
        kind: TypeKind::TypeAlias,
        visibility: Visibility::Private,
        fields: Vec::new(),
        generics: Vec::new(),
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
            "visibility_modifier" => {
                if child.utf8_text(source.as_bytes()).ok()? == "pub" {
                    type_info.visibility = Visibility::Public;
                }
            }
            "type_identifier" => {
                type_info.name = child.utf8_text(source.as_bytes()).ok()?.to_string();
            }
            _ => {}
        }
    }
    
    // Generate signature
    type_info.signature = format!("type {}", type_info.name);
    
    Some(type_info)
}

/// Generate type signature
fn generate_type_signature(type_info: &TypeInfo) -> String {
    let mut sig = String::new();
    
    if type_info.visibility == Visibility::Public {
        sig.push_str("pub ");
    }
    
    match type_info.kind {
        TypeKind::Struct => sig.push_str("struct "),
        TypeKind::Enum => sig.push_str("enum "),
        TypeKind::Trait => sig.push_str("trait "),
        TypeKind::TypeAlias => sig.push_str("type "),
        _ => {}
    }
    
    sig.push_str(&type_info.name);
    
    if !type_info.generics.is_empty() {
        sig.push('<');
        sig.push_str(&type_info.generics.join(", "));
        sig.push('>');
    }
    
    sig
}

/// Extract import/use declaration
fn extract_import(node: &Node, source: &str) -> Option<ImportInfo> {
    let mut import = ImportInfo {
        module: String::new(),
        items: Vec::new(),
        is_wildcard: false,
        is_external: false,
        line_number: node.start_position().row + 1,
    };
    
    // Get the full use statement text and parse it
    let use_text = node.utf8_text(source.as_bytes()).ok()?;
    
    // Check for wildcard imports
    import.is_wildcard = use_text.contains("::*");
    
    // Check if external (doesn't start with crate:: or super:: or self::)
    import.is_external = !use_text.contains("crate::") 
        && !use_text.contains("super::") 
        && !use_text.contains("self::");
    
    // Simple extraction - could be improved
    if let Some(path_start) = use_text.find("use ") {
        let path = &use_text[path_start + 4..];
        if let Some(semicolon) = path.find(';') {
            let module = path[..semicolon].trim().to_string();
            import.module = module.clone();
            
            // Extract individual items if it's a bracketed import
            if module.contains('{') && module.contains('}') {
                if let Some(start) = module.find('{') {
                    if let Some(end) = module.find('}') {
                        let items_str = &module[start+1..end];
                        import.items = items_str.split(',')
                            .map(|s| s.trim().to_string())
                            .collect();
                    }
                }
            } else {
                // Single item import
                if let Some(last_segment) = module.split("::").last() {
                    import.items.push(last_segment.to_string());
                }
            }
        }
    }
    
    Some(import)
}

/// Extract function parameters
fn extract_parameters(node: &Node, source: &str) -> Vec<Parameter> {
    let mut params = Vec::new();
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "parameter" || child.kind() == "self_parameter" {
            if let Ok(param_text) = child.utf8_text(source.as_bytes()) {
                let param = Parameter {
                    name: extract_param_name(param_text),
                    type_annotation: extract_param_type(param_text),
                    default_value: None, // Rust doesn't have default parameters
                };
                params.push(param);
            }
        }
    }
    
    params
}

/// Extract struct fields
fn extract_fields(node: &Node, source: &str) -> Vec<Field> {
    let mut fields = Vec::new();
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "field_declaration" {
            let mut field = Field {
                name: String::new(),
                type_annotation: None,
                visibility: Visibility::Private,
                doc_comment: extract_doc_comment(&child, source),
            };
            
            let mut field_cursor = child.walk();
            for field_child in child.children(&mut field_cursor) {
                match field_child.kind() {
                    "visibility_modifier" => {
                        if field_child.utf8_text(source.as_bytes()).ok() == Some("pub") {
                            field.visibility = Visibility::Public;
                        }
                    }
                    "field_identifier" => {
                        field.name = field_child
                            .utf8_text(source.as_bytes())
                            .unwrap_or("")
                            .to_string();
                    }
                    "type" => {
                        field.type_annotation = Some(
                            field_child
                                .utf8_text(source.as_bytes())
                                .unwrap_or("")
                                .to_string(),
                        );
                    }
                    _ => {}
                }
            }
            
            if !field.name.is_empty() {
                fields.push(field);
            }
        }
    }
    
    fields
}

/// Extract enum variants
fn extract_enum_variants(node: &Node, source: &str) -> Vec<Field> {
    let mut variants = Vec::new();
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "enum_variant" {
            let mut variant = Field {
                name: String::new(),
                type_annotation: None,
                visibility: Visibility::Public, // Enum variants inherit enum visibility
                doc_comment: extract_doc_comment(&child, source),
            };
            
            // Get variant name (first identifier)
            let mut variant_cursor = child.walk();
            for variant_child in child.children(&mut variant_cursor) {
                if variant_child.kind() == "identifier" {
                    variant.name = variant_child
                        .utf8_text(source.as_bytes())
                        .unwrap_or("")
                        .to_string();
                    break;
                }
            }
            
            if !variant.name.is_empty() {
                variants.push(variant);
            }
        }
    }
    
    variants
}

/// Extract generic parameters
fn extract_generics(node: &Node, source: &str) -> Vec<String> {
    let mut generics = Vec::new();
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type_identifier" {
            if let Ok(generic) = child.utf8_text(source.as_bytes()) {
                generics.push(generic.to_string());
            }
        }
    }
    
    generics
}

/// Helper to extract parameter name from parameter text
fn extract_param_name(param_text: &str) -> String {
    if param_text.contains(':') {
        param_text.split(':').next().unwrap_or("").trim().to_string()
    } else {
        param_text.trim().to_string()
    }
}

/// Helper to extract parameter type from parameter text  
fn extract_param_type(param_text: &str) -> Option<String> {
    if param_text.contains(':') {
        Some(
            param_text
                .split(':')
                .nth(1)
                .unwrap_or("")
                .trim()
                .to_string(),
        )
    } else {
        None
    }
}

/// Extract documentation comment for a node
fn extract_doc_comment(node: &Node, source: &str) -> Option<String> {
    // Look for doc comment in previous sibling
    if let Some(prev) = node.prev_sibling() {
        if prev.kind() == "line_comment" {
            let text = prev.utf8_text(source.as_bytes()).ok()?;
            if text.trim_start().starts_with("///") {
                // Clean up doc comment
                let cleaned = text
                    .lines()
                    .map(|line| {
                        line.trim_start()
                            .strip_prefix("///")
                            .unwrap_or(line)
                            .trim()
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                return Some(cleaned);
            }
        }
    }
    None
}

/// Extract call expressions
fn extract_call_expression(node: &Node, source: &str, context: &mut ParseContext) {
    if let Some(func_node) = node.child_by_field_name("function") {
        if let Ok(callee) = func_node.utf8_text(source.as_bytes()) {
            let line = node.start_position().row + 1;
            context.add_call(callee.to_string(), "direct", line);
        }
    }
}

/// Extract method call expressions
fn extract_method_call(node: &Node, source: &str, context: &mut ParseContext) {
    if let Some(method_node) = node.child_by_field_name("name") {
        if let Ok(callee) = method_node.utf8_text(source.as_bytes()) {
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
        calls_unwrap: 0,
        calls_expect: 0,
        has_panic_macro: false,
        has_todo_macro: false,
        has_unsafe_block: false,
        has_mutex: false,
        has_arc: false,
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
        "method_call_expression" => {
            if let Some(method) = node.child_by_field_name("name") {
                if let Ok(method_name) = method.utf8_text(source.as_bytes()) {
                    match method_name {
                        "unwrap" => hints.calls_unwrap += 1,
                        "expect" => hints.calls_expect += 1,
                        _ => {}
                    }
                }
            }
        }
        "macro_invocation" => {
            if let Ok(macro_text) = node.utf8_text(source.as_bytes()) {
                if macro_text.starts_with("panic!") {
                    hints.has_panic_macro = true;
                } else if macro_text.starts_with("todo!") {
                    hints.has_todo_macro = true;
                }
            }
        }
        "unsafe_block" => {
            hints.has_unsafe_block = true;
        }
        "type_identifier" | "generic_type" => {
            if let Ok(type_name) = node.utf8_text(source.as_bytes()) {
                if type_name == "Mutex" || type_name.contains("Mutex<") {
                    hints.has_mutex = true;
                }
                if type_name == "Arc" || type_name.contains("Arc<") {
                    hints.has_arc = true;
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
    
    lines[start_line..end_line].join("\n")
}

/// Check if a call is to an external crate
fn is_external_call(callee: &str) -> bool {
    // Simple heuristic: if it contains :: and doesn't start with self/super/crate
    callee.contains("::") && 
        !callee.starts_with("self::") && 
        !callee.starts_with("super::") && 
        !callee.starts_with("crate::")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rust_extractor_comprehensive() {
        let source = r#"
        use std::collections::HashMap;
        
        /// Main function
        pub fn main() {
            println!("Hello");
            process_data().unwrap();
        }
        
        /// Process data with error handling
        pub fn process_data() -> Result<String, std::io::Error> {
            let mut data = Vec::new();
            data.push(1);
            Ok("done".to_string())
        }
        
        pub struct Point {
            pub x: f64,
            pub y: f64,
        }
        
        impl Point {
            pub fn new(x: f64, y: f64) -> Self {
                Point { x, y }
            }
            
            pub fn distance(&self) -> f64 {
                (self.x * self.x + self.y * self.y).sqrt()
            }
        }
        "#;
        
        let extractor = RustExtractor;
        let path = Path::new("test.rs");
        let result = extractor.extract(path, source).unwrap();
        
        // Check functions
        assert!(result.functions.len() >= 3); // main, process_data, new, distance
        
        // Check behavioral analysis
        let process_data = result.functions.iter()
            .find(|f| f.name == "process_data")
            .unwrap();
        assert!(process_data.returns_result);
        assert!(process_data.takes_mut_params);
        
        // Check fingerprints
        assert!(!result.fingerprints.is_empty());
        
        // Check behavioral hints
        assert!(result.behavioral_hints.iter()
            .any(|h| h.function_name == "main" && h.calls_unwrap > 0));
        
        // Check types
        assert_eq!(result.types.len(), 1);
        assert_eq!(result.types[0].name, "Point");
        assert_eq!(result.types[0].fields.len(), 2);
        
        // Check imports
        assert_eq!(result.imports.len(), 1);
        assert!(result.imports[0].module.contains("HashMap"));
        assert!(result.imports[0].is_external);
    }
}