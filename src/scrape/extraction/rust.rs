use anyhow::Result;
use std::path::Path;
// use tree_sitter::Parser; // Will uncomment when we add tree-sitter-rust

use super::{
    Field, FunctionInfo, ImportInfo, LanguageExtractor, Parameter,
    SemanticData, TypeInfo, TypeKind, Visibility,
};
use crate::scrape::discovery::Language;

/// Rust-specific semantic extractor
pub struct RustExtractor;

impl LanguageExtractor for RustExtractor {
    fn extract(&self, path: &Path, _source: &str) -> Result<SemanticData> {
        // For now, return empty data until we add tree-sitter-rust dependency
        // This is a placeholder implementation
        Ok(SemanticData {
            file_path: path.to_string_lossy().to_string(),
            language: Language::Rust,
            functions: Vec::new(),
            types: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
            docs: Vec::new(),
        })
    }
}

// Commenting out tree-sitter specific code for now
/*
/// Recursively extract semantic information from AST nodes
fn extract_node(node: &tree_sitter::Node, source: &str, data: &mut SemanticData) {
    match node.kind() {
        "function_item" => {
            if let Some(func) = extract_function(node, source) {
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
            for child in node.children(&mut node.walk()) {
                if child.kind() == "function_item" {
                    if let Some(func) = extract_function(&child, source) {
                        data.functions.push(func);
                    }
                }
            }
        }
        _ => {}
    }
    
    // Recurse into children
    for child in node.children(&mut node.walk()) {
        extract_node(&child, source, data);
    }
}

/// Extract function information from a function_item node
fn extract_function(node: &tree_sitter::Node, source: &str) -> Option<FunctionInfo> {
    let mut func = FunctionInfo {
        name: String::new(),
        visibility: Visibility::Private,
        parameters: Vec::new(),
        return_type: None,
        is_async: false,
        is_unsafe: false,
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
        doc_comment: None,
    };
    
    for child in node.children(&mut node.walk()) {
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
fn extract_struct(node: &tree_sitter::Node, source: &str) -> Option<TypeInfo> {
    let mut type_info = TypeInfo {
        name: String::new(),
        kind: TypeKind::Struct,
        visibility: Visibility::Private,
        fields: Vec::new(),
        generics: Vec::new(),
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
        doc_comment: None,
    };
    
    for child in node.children(&mut node.walk()) {
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
fn extract_enum(node: &tree_sitter::Node, source: &str) -> Option<TypeInfo> {
    let mut type_info = TypeInfo {
        name: String::new(),
        kind: TypeKind::Enum,
        visibility: Visibility::Private,
        fields: Vec::new(), // Enum variants stored as fields
        generics: Vec::new(),
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
        doc_comment: None,
    };
    
    for child in node.children(&mut node.walk()) {
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
fn extract_import(node: &tree_sitter::Node, source: &str) -> Option<ImportInfo> {
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
fn extract_parameters(node: &tree_sitter::Node, source: &str) -> Vec<Parameter> {
    let mut params = Vec::new();
    
    for child in node.children(&mut node.walk()) {
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
fn extract_fields(node: &tree_sitter::Node, source: &str) -> Vec<Field> {
    let mut fields = Vec::new();
    
    for child in node.children(&mut node.walk()) {
        if child.kind() == "field_declaration" {
            let mut field = Field {
                name: String::new(),
                type_annotation: None,
                visibility: Visibility::Private,
                doc_comment: None,
            };
            
            for field_child in child.children(&mut child.walk()) {
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
fn extract_enum_variants(node: &tree_sitter::Node, source: &str) -> Vec<Field> {
    let mut variants = Vec::new();
    
    for child in node.children(&mut node.walk()) {
        if child.kind() == "enum_variant" {
            let mut variant = Field {
                name: String::new(),
                type_annotation: None,
                visibility: Visibility::Public, // Enum variants inherit enum visibility
                doc_comment: None,
            };
            
            // Get variant name (first identifier)
            for variant_child in child.children(&mut child.walk()) {
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
fn extract_generics(node: &tree_sitter::Node, source: &str) -> Vec<String> {
    let mut generics = Vec::new();
    
    for child in node.children(&mut node.walk()) {
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
*/

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_simple_function() {
        let source = r#"
        pub fn main() {
            println!("Hello");
        }
        "#;
        
        let extractor = RustExtractor;
        let path = Path::new("test.rs");
        let result = extractor.extract(path, source).unwrap();
        
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "main");
        assert_eq!(result.functions[0].visibility, Visibility::Public);
        assert!(result.functions[0].parameters.is_empty());
    }
    
    #[test]
    fn test_extract_struct() {
        let source = r#"
        pub struct Point {
            pub x: f64,
            pub y: f64,
        }
        "#;
        
        let extractor = RustExtractor;
        let path = Path::new("test.rs");
        let result = extractor.extract(path, source).unwrap();
        
        assert_eq!(result.types.len(), 1);
        assert_eq!(result.types[0].name, "Point");
        assert_eq!(result.types[0].kind, TypeKind::Struct);
        assert_eq!(result.types[0].fields.len(), 2);
        assert_eq!(result.types[0].fields[0].name, "x");
        assert_eq!(result.types[0].fields[1].name, "y");
    }
    
    #[test]
    fn test_extract_enum() {
        let source = r#"
        enum Color {
            Red,
            Green,
            Blue,
        }
        "#;
        
        let extractor = RustExtractor;
        let path = Path::new("test.rs");
        let result = extractor.extract(path, source).unwrap();
        
        assert_eq!(result.types.len(), 1);
        assert_eq!(result.types[0].name, "Color");
        assert_eq!(result.types[0].kind, TypeKind::Enum);
        assert_eq!(result.types[0].fields.len(), 3);
        assert_eq!(result.types[0].fields[0].name, "Red");
    }
    
    #[test]
    fn test_extract_imports() {
        let source = r#"
        use std::collections::HashMap;
        use anyhow::Result;
        "#;
        
        let extractor = RustExtractor;
        let path = Path::new("test.rs");
        let result = extractor.extract(path, source).unwrap();
        
        assert_eq!(result.imports.len(), 2);
        assert!(result.imports[0].module.contains("HashMap"));
        assert!(result.imports[1].module.contains("Result"));
    }
}