use anyhow::Result;
use std::path::Path;
use tree_sitter::Node;

use super::{
    CallGraph, Field, FunctionInfo, ImportInfo, LanguageExtractor,
    Parameter, SemanticData, TypeInfo, TypeKind, Visibility,
};
use crate::scrape::discovery::Language;
use crate::semantic::languages::create_parser;

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
    
    fn add_call(&mut self, callee: String, line: usize) {
        if let Some(ref caller) = self.current_function {
            self.call_entries.push(CallGraph {
                caller: caller.clone(),
                callee,
                line_number: line,
                is_external: false, // We'll improve this later
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
                context.enter_function(func.name.clone());
                data.functions.push(func);
            }
        }
        "struct_item" => {
            if let Some(type_info) = extract_struct(node, source) {
                data.types.push(type_info);
            }
        }
        "enum_item" => {
            if let Some(type_info) = extract_enum(node, source) {
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
                        context.enter_function(func.name.clone());
                        data.functions.push(func);
                    }
                }
            }
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
                func.parameters = extract_parameters(&child, source);
            }
            "type" => {
                // This is the return type (after ->)
                func.return_type = Some(child.utf8_text(source.as_bytes()).ok()?.to_string());
            }
            _ => {}
        }
    }
    
    // Check for async/unsafe modifiers
    let func_text = node.utf8_text(source.as_bytes()).ok()?;
    func.is_async = func_text.starts_with("async ") || func_text.contains(" async ");
    func.is_unsafe = func_text.starts_with("unsafe ") || func_text.contains(" unsafe ");
    
    Some(func)
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
    
    Some(type_info)
}

/// Extract import/use declaration
fn extract_import(node: &Node, source: &str) -> Option<ImportInfo> {
    let mut import = ImportInfo {
        module: String::new(),
        items: Vec::new(),
        is_wildcard: false,
        line_number: node.start_position().row + 1,
    };
    
    // Get the full use statement text and parse it
    let use_text = node.utf8_text(source.as_bytes()).ok()?;
    
    // Check for wildcard imports
    import.is_wildcard = use_text.contains("::*");
    
    // Simple extraction - could be improved
    if let Some(path_start) = use_text.find("use ") {
        let path = &use_text[path_start + 4..];
        if let Some(semicolon) = path.find(';') {
            import.module = path[..semicolon].trim().to_string();
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
                doc_comment: None,
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
            context.add_call(callee.to_string(), line);
        }
    }
}

/// Extract method call expressions
fn extract_method_call(node: &Node, source: &str, context: &mut ParseContext) {
    if let Some(method_node) = node.child_by_field_name("name") {
        if let Ok(callee) = method_node.utf8_text(source.as_bytes()) {
            let line = node.start_position().row + 1;
            context.add_call(callee.to_string(), line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rust_extractor_basic() {
        let source = r#"
        /// Main function
        pub fn main() {
            println!("Hello");
        }
        
        pub struct Point {
            pub x: f64,
            pub y: f64,
        }
        
        impl Point {
            pub fn new(x: f64, y: f64) -> Self {
                Point { x, y }
            }
        }
        "#;
        
        let extractor = RustExtractor;
        let path = Path::new("test.rs");
        let result = extractor.extract(path, source).unwrap();
        
        // Should find 2 functions: main and new
        assert_eq!(result.functions.len(), 2);
        assert_eq!(result.functions[0].name, "main");
        assert_eq!(result.functions[1].name, "new");
        
        // Should find 1 struct
        assert_eq!(result.types.len(), 1);
        assert_eq!(result.types[0].name, "Point");
        assert_eq!(result.types[0].fields.len(), 2);
    }
}