use anyhow::Result;
use std::path::Path;
use tree_sitter::Node;

use super::{
    CallGraph, Field, FunctionInfo, ImportInfo, LanguageExtractor, Parameter, SemanticData,
    TypeInfo, TypeKind, Visibility,
};
use crate::scrape::discovery::Language;
use crate::semantic::languages::create_parser;

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
                callee,
                line_number: line,
                is_external: call_type == "external",
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
                context.enter_function(func.name.clone());
                data.functions.push(func);
            }
        }
        "method_declaration" => {
            if let Some(func) = extract_method(node, source) {
                context.enter_function(func.name.clone());
                data.functions.push(func);
            }
        }
        "type_spec" => {
            if let Some(type_info) = extract_type_spec(node, source) {
                data.types.push(type_info);
            }
        }
        "import_declaration" => {
            if let Some(import) = extract_import(node, source) {
                data.imports.push(import);
            }
        }
        "const_declaration" => {
            // We could extract constants as well if needed
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
            }
            "result" => {
                func.return_type = extract_return_type(&child, source);
            }
            _ => {}
        }
    }

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
    };

    let mut cursor = node.walk();
    let mut receiver_type = String::new();
    
    for child in node.children(&mut cursor) {
        match child.kind() {
            "parameter_list" => {
                // First parameter list is the receiver
                if receiver_type.is_empty() {
                    if let Some(receiver) = extract_receiver(&child, source) {
                        receiver_type = receiver;
                    }
                } else {
                    // Second parameter list is the actual parameters
                    func.parameters = extract_parameters(&child, source);
                }
            }
            "field_identifier" => {
                let name = child.utf8_text(source.as_bytes()).ok()?.to_string();
                func.name = if receiver_type.is_empty() {
                    name.clone()
                } else {
                    format!("{}.{}", receiver_type, name.clone())
                };
                // Check visibility based on first letter
                func.visibility = if name.chars().next()?.is_lowercase() {
                    Visibility::Private
                } else {
                    Visibility::Public
                };
            }
            "result" => {
                func.return_type = extract_return_type(&child, source);
            }
            _ => {}
        }
    }

    Some(func)
}

/// Extract type spec (struct, interface, type alias)
fn extract_type_spec(node: &Node, source: &str) -> Option<TypeInfo> {
    let mut type_info = TypeInfo {
        name: String::new(),
        kind: TypeKind::Struct,
        visibility: Visibility::Public,
        fields: Vec::new(),
        generics: Vec::new(), // Go uses type parameters in newer versions
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
        doc_comment: extract_doc_comment(node, source),
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
            _ => {}
        }
    }

    if type_info.name.is_empty() {
        return None;
    }

    Some(type_info)
}

/// Extract import declaration
fn extract_import(node: &Node, source: &str) -> Option<ImportInfo> {
    let mut import = ImportInfo {
        module: String::new(),
        items: Vec::new(),
        is_wildcard: false,
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
                        import.items.push(cleaned.to_string());
                    }
                }
            }
        } else if child.kind() == "import_spec" {
            if let Ok(import_text) = child.utf8_text(source.as_bytes()) {
                let cleaned = import_text.trim().trim_matches('"');
                import.module = cleaned.to_string();
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
                    "type_identifier" | "pointer_type" | "slice_type" | "array_type" => {
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
                params.push(Parameter {
                    name: extract_variadic_param_name(param_text),
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
            let mut param_cursor = child.walk();
            for param_child in child.children(&mut param_cursor) {
                if param_child.kind() == "type_identifier" {
                    return param_child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
                } else if param_child.kind() == "pointer_type" {
                    // Handle pointer receivers (*Type)
                    let mut ptr_cursor = param_child.walk();
                    for ptr_child in param_child.children(&mut ptr_cursor) {
                        if ptr_child.kind() == "type_identifier" {
                            return ptr_child
                                .utf8_text(source.as_bytes())
                                .ok()
                                .map(|s| format!("*{}", s));
                        }
                    }
                }
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
            "type_identifier" | "pointer_type" | "slice_type" | "interface_type" => {
                if let Ok(type_text) = child.utf8_text(source.as_bytes()) {
                    return_types.push(type_text.to_string());
                }
            }
            "parameter_list" => {
                // Named return values
                let params = extract_parameters(&child, source);
                for param in params {
                    if let Some(type_ann) = param.type_annotation {
                        return_types.push(format!("{} {}", param.name, type_ann));
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
            "type_identifier" | "pointer_type" | "slice_type" | "array_type" | "map_type" => {
                field.type_annotation = Some(child.utf8_text(source.as_bytes()).ok()?.to_string());
            }
            _ => {}
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
        if child.kind() == "method_spec_list" {
            let mut method_cursor = child.walk();
            for method_node in child.children(&mut method_cursor) {
                if method_node.kind() == "method_spec" {
                    let mut method = Field {
                        name: String::new(),
                        type_annotation: None,
                        visibility: Visibility::Public,
                        doc_comment: None,
                    };

                    let mut spec_cursor = method_node.walk();
                    for spec_child in method_node.children(&mut spec_cursor) {
                        if spec_child.kind() == "field_identifier" {
                            if let Ok(name) = spec_child.utf8_text(source.as_bytes()) {
                                method.name = name.to_string();
                                // Get the full method signature
                                if let Ok(full_text) = method_node.utf8_text(source.as_bytes()) {
                                    method.type_annotation = Some(full_text.to_string());
                                }
                                break;
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
            let call_type = if callee.starts_with("go ") {
                "async" // Goroutine call
            } else {
                "direct"
            };
            let clean_callee = callee.strip_prefix("go ").unwrap_or(callee);
            context.add_call(clean_callee.to_string(), call_type, line);
        }
    }
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

/// Helper to extract variadic parameter name
fn extract_variadic_param_name(param_text: &str) -> String {
    param_text
        .split("...")
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_extractor_basic() {
        let source = r#"
        package main
        
        import "fmt"
        
        // Point represents a 2D point
        type Point struct {
            X float64
            Y float64
        }
        
        // NewPoint creates a new point
        func NewPoint(x, y float64) *Point {
            return &Point{X: x, Y: y}
        }
        
        // Distance calculates distance from origin
        func (p *Point) Distance() float64 {
            return math.Sqrt(p.X*p.X + p.Y*p.Y)
        }
        
        func main() {
            p := NewPoint(3, 4)
            fmt.Println(p.Distance())
        }
        "#;

        let extractor = GoExtractor;
        let path = Path::new("test.go");
        let result = extractor.extract(path, source).unwrap();

        // Should find functions
        assert!(result.functions.len() >= 2); // At least main and NewPoint
        
        // Should find struct
        assert!(result.types.len() >= 1);
        assert!(result.types.iter().any(|t| t.name == "Point"));
        
        // Should find import
        assert!(result.imports.len() >= 1);
    }
}