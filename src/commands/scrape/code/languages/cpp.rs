// ============================================================================
// C++ LANGUAGE PROCESSOR V2 - STRUCT-BASED
// ============================================================================
//! C++ language processor that returns typed structs instead of SQL strings.
//!
//! This is the refactored version that:
//! - Returns ExtractedData with typed structs
//! - No SQL string generation
//! - Direct data extraction to domain types
//! - Uses iterative approach for nested declarators to avoid stack overflow
//!
//! Handles C++'s features:
//! - Classes with access modifiers (public/private/protected)
//! - Templates and template specialization
//! - Namespaces
//! - Function overloading
//! - RAII and constructors/destructors
//! - Modern C++ features (auto, lambdas, etc.)

use crate::commands::scrape::code::database::{
    CodeSymbol, FunctionFact, ImportFact, TypeFact,
};
use crate::commands::scrape::code::extracted_data::{ExtractedData, ConstantFact, MemberFact};
use crate::commands::scrape::code::types::{CallGraphEntry, CallType, FilePath, SymbolKind};
use anyhow::{Context, Result};
use tree_sitter::{Node, Parser};

/// C++ language processor - returns typed structs
pub struct CppProcessor;

impl CppProcessor {
    /// Process a C++ file and extract all symbols to typed structs
    pub fn process_file(file_path: FilePath, content: &[u8]) -> Result<ExtractedData> {
        let mut data = ExtractedData::new();

        // Set up tree-sitter parser for C++
        let mut parser = Parser::new();
        let metal = patina_metal::Metal::Cpp;
        let language = metal
            .tree_sitter_language_for_ext("cpp")
            .ok_or_else(|| anyhow::anyhow!("No C++ parser available"))?;
        parser
            .set_language(&language)
            .context("Failed to set C++ language")?;

        // Parse the file
        let tree = parser
            .parse(content, None)
            .context("Failed to parse C++ file")?;

        // Walk the AST and extract symbols
        let mut namespace_stack = Vec::new();
        extract_cpp_symbols(
            &tree.root_node(),
            content,
            &file_path.to_string(),
            &mut data,
            None,
            &mut namespace_stack,
        );

        Ok(data)
    }
}

/// Recursively extract symbols from the C++ AST
fn extract_cpp_symbols(
    node: &Node,
    source: &[u8],
    file_path: &str,
    data: &mut ExtractedData,
    current_function: Option<String>,
    namespace_stack: &mut Vec<String>,
) {
    match node.kind() {
        "namespace_definition" => {
            // Enter namespace
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    namespace_stack.push(name.to_string());
                }
            }

            // Process namespace body
            if let Some(body) = node.child_by_field_name("body") {
                let mut cursor = body.walk();
                for child in body.children(&mut cursor) {
                    extract_cpp_symbols(
                        &child,
                        source,
                        file_path,
                        data,
                        current_function.clone(),
                        namespace_stack,
                    );
                }
            }

            // Exit namespace
            namespace_stack.pop();
            return; // Don't recurse again
        }
        "function_definition" => {
            if let Some(name) = extract_function_name(node, source) {
                // Check if this is a method (inside a class) or free function
                let is_method = namespace_stack.iter().any(|ns| {
                    // Check if any namespace entry is actually a class name
                    // This is a simplified check - could be improved
                    true // For now, assume functions in namespace stack might be methods
                });
                
                // Include namespace/class in function name
                let full_name = if namespace_stack.is_empty() {
                    name.clone()
                } else {
                    format!("{}::{}", namespace_stack.join("::"), name)
                };
                
                // Check for special method types
                let is_constructor = namespace_stack.last()
                    .map(|class| name == class.as_str())
                    .unwrap_or(false);
                let is_destructor = name.starts_with('~');

                process_cpp_function_enhanced(
                    node, 
                    source, 
                    file_path, 
                    &full_name, 
                    data,
                    is_method,
                    is_constructor,
                    is_destructor,
                );

                // Process function body with updated context
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    extract_cpp_symbols(
                        &child,
                        source,
                        file_path,
                        data,
                        Some(full_name.clone()),
                        namespace_stack,
                    );
                }
                return; // Don't recurse again
            }
        }
        "class_specifier" | "struct_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    let kind = if node.kind() == "class_specifier" {
                        SymbolKind::Class
                    } else {
                        SymbolKind::Struct
                    };

                    // Include namespace in type name
                    let full_name = if namespace_stack.is_empty() {
                        name.to_string()
                    } else {
                        format!("{}::{}", namespace_stack.join("::"), name)
                    };

                    process_cpp_class(node, source, file_path, &full_name, kind, data);

                    // Process class body with updated namespace and extract members
                    namespace_stack.push(name.to_string());
                    if let Some(body) = node.child_by_field_name("body") {
                        // Extract class members with visibility tracking
                        extract_class_members(&body, source, file_path, &full_name, data);
                        
                        // Also process nested types and methods
                        let mut cursor = body.walk();
                        for child in body.children(&mut cursor) {
                            extract_cpp_symbols(
                                &child,
                                source,
                                file_path,
                                data,
                                current_function.clone(),
                                namespace_stack,
                            );
                        }
                    }
                    namespace_stack.pop();
                    return; // Don't recurse again
                }
            }
        }
        "enum_specifier" | "enum_class_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    let full_name = if namespace_stack.is_empty() {
                        name.to_string()
                    } else {
                        format!("{}::{}", namespace_stack.join("::"), name)
                    };
                    process_cpp_enum(node, source, file_path, &full_name, data);
                }
            }
        }
        "type_definition" | "alias_declaration" => {
            if let Some(name) = extract_typedef_name(node, source) {
                let full_name = if namespace_stack.is_empty() {
                    name.clone()
                } else {
                    format!("{}::{}", namespace_stack.join("::"), name)
                };
                process_cpp_typedef(node, source, file_path, &full_name, data);
            }
        }
        "preproc_include" | "preproc_import" => {
            process_cpp_include(node, source, file_path, data);
        }
        "preproc_def" => {
            // Extract #define macros (same as C)
            process_cpp_macro(node, source, file_path, data);
        }
        "declaration" => {
            // Extract global variables and constants (at file/namespace scope)
            if current_function.is_none() {
                process_cpp_declaration(node, source, file_path, data);
            }
        }
        "call_expression" => {
            // Track function calls for call graph
            if let Some(ref caller) = current_function {
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        data.add_call_edge(CallGraphEntry::new(
                            caller.clone(),
                            callee.to_string(),
                            file_path.to_string(),
                            CallType::Direct,
                            (node.start_position().row + 1) as i32,
                        ));
                    }
                }
            }
        }
        "new_expression" => {
            // Constructor calls
            if let Some(ref caller) = current_function {
                if let Some(type_node) = node.child_by_field_name("type") {
                    if let Ok(class_name) = type_node.utf8_text(source) {
                        data.add_call_edge(CallGraphEntry::new(
                            caller.clone(),
                            format!("{}::constructor", class_name),
                            file_path.to_string(),
                            CallType::Constructor,
                            (node.start_position().row + 1) as i32,
                        ));
                    }
                }
            }
        }
        "delete_expression" => {
            // Destructor calls
            if let Some(ref caller) = current_function {
                if let Some(arg_node) = node.child_by_field_name("argument") {
                    if let Ok(var_name) = arg_node.utf8_text(source) {
                        data.add_call_edge(CallGraphEntry::new(
                            caller.clone(),
                            format!("~{}", var_name),
                            file_path.to_string(),
                            CallType::Destructor,
                            (node.start_position().row + 1) as i32,
                        ));
                    }
                }
            }
        }
        _ => {}
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_cpp_symbols(
            &child,
            source,
            file_path,
            data,
            current_function.clone(),
            namespace_stack,
        );
    }
}

/// Process a C++ function and add to ExtractedData
fn process_cpp_function(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let params = extract_parameters(node, source);
    let return_type = extract_return_type(node, source);
    let _is_template = has_template_parent(node);
    let is_public = is_public_member(node, source);

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "function".to_string(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });

    // Add function fact
    data.add_function(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: false, // Would need more analysis
        takes_mut_params: params.iter().any(|p| !p.contains("const")),
        returns_result: false, // C++ uses exceptions
        returns_option: return_type.as_ref().is_some_and(|r| r.contains("optional")),
        is_async: false, // C++ doesn't have built-in async
        is_unsafe: true, // All C++ is unsafe
        is_public,
        parameter_count: params.len() as i32,
        generic_count: if _is_template { 1 } else { 0 },
        parameters: params,
        return_type,
    });
}

/// Process a C++ class/struct and add to ExtractedData
fn process_cpp_class(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: SymbolKind,
    data: &mut ExtractedData,
) {
    let _is_template = has_template_parent(node);

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: kind.to_string(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });

    // Add type fact
    data.add_type(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: get_type_definition(node, source),
        kind: kind.to_string(),
        visibility: "public".to_string(), // Top-level types are public
        usage_count: 0,
    });
}

/// Process a C++ enum and add to ExtractedData
fn process_cpp_enum(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_enum_class = node.kind() == "enum_class_specifier";

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: if is_enum_class { "enum_class" } else { "enum" }.to_string(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });

    // Add type fact
    data.add_type(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: get_type_definition(node, source),
        kind: if is_enum_class { "enum_class" } else { "enum" }.to_string(),
        visibility: "public".to_string(),
        usage_count: 0,
    });
    
    // Extract enum values
    if let Some(list_node) = node.child_by_field_name("body") {
        let mut cursor = list_node.walk();
        for child in list_node.children(&mut cursor) {
            if child.kind() == "enumerator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(value_name) = name_node.utf8_text(source) {
                        let value = child
                            .child_by_field_name("value")
                            .and_then(|v| v.utf8_text(source).ok())
                            .map(|s| s.to_string());
                        
                        let full_name = format!("{}::{}", name, value_name);
                        
                        // Add as symbol for backwards compatibility
                        data.add_symbol(CodeSymbol {
                            path: file_path.to_string(),
                            name: full_name.clone(),
                            kind: "enum_value".to_string(),
                            line: child.start_position().row + 1,
                            context: if let Some(val) = &value {
                                format!("{} = {}", value_name, val)
                            } else {
                                value_name.to_string()
                            },
                        });
                        
                        // Add as ConstantFact for better organization
                        data.add_constant(ConstantFact {
                            file: file_path.to_string(),
                            name: full_name,
                            value: value.clone(),
                            const_type: "enum_value".to_string(),
                            scope: name.to_string(),
                            line: child.start_position().row + 1,
                        });
                    }
                }
            }
        }
    }
}

/// Process a C++ typedef/using and add to ExtractedData
fn process_cpp_typedef(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_using = node.kind() == "alias_declaration";

    // Add code symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: if is_using { "using" } else { "typedef" }.to_string(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });

    // Add type fact
    data.add_type(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: get_type_definition(node, source),
        kind: if is_using { "using" } else { "typedef" }.to_string(),
        visibility: "public".to_string(),
        usage_count: 0,
    });
}

/// Process a C++ include directive and add to ExtractedData
fn process_cpp_include(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    if let Ok(include_text) = node.utf8_text(source) {
        let header = include_text
            .trim_start_matches("#include")
            .trim_start_matches("#import")
            .trim()
            .trim_start_matches('<')
            .trim_start_matches('"')
            .trim_end_matches('>')
            .trim_end_matches('"');
        let is_external = include_text.contains('<');

        data.add_import(ImportFact {
            file: file_path.to_string(),
            import_path: header.to_string(),
            imported_names: vec![header.to_string()],
            import_kind: if is_external { "system" } else { "local" }.to_string(),
            line_number: (node.start_position().row + 1) as i32,
        });
    }
}

/// Extract class members with visibility tracking
fn extract_class_members(
    body: &Node,
    source: &[u8],
    file_path: &str,
    class_name: &str,
    data: &mut ExtractedData,
) {
    let mut current_visibility = "private"; // Default for class
    
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        match child.kind() {
            "access_specifier" => {
                // Track visibility changes (public:, private:, protected:)
                if let Ok(text) = child.utf8_text(source) {
                    current_visibility = text.trim_end_matches(':');
                }
            }
            "field_declaration" => {
                // Extract member fields
                if let Some(declarator) = child.child_by_field_name("declarator") {
                    if let Some(name) = extract_declarator_name(&declarator, source) {
                        // Check for static/const modifiers
                        let mut modifiers = Vec::new();
                        let text = child.utf8_text(source).unwrap_or("");
                        if text.contains("static") { modifiers.push("static".to_string()); }
                        if text.contains("const") { modifiers.push("const".to_string()); }
                        if text.contains("mutable") { modifiers.push("mutable".to_string()); }
                        
                        // Add as symbol for backwards compatibility
                        data.add_symbol(CodeSymbol {
                            path: file_path.to_string(),
                            name: format!("{}::{}", class_name, name),
                            kind: "field".to_string(),
                            line: child.start_position().row + 1,
                            context: format!("{} {}", current_visibility, 
                                text.lines().next().unwrap_or("")),
                        });
                        
                        // Add as MemberFact for better organization
                        data.add_member(MemberFact {
                            file: file_path.to_string(),
                            container: class_name.to_string(),
                            name: name.clone(),
                            member_type: "field".to_string(),
                            visibility: current_visibility.to_string(),
                            modifiers,
                            line: child.start_position().row + 1,
                        });
                    }
                }
            }
            "function_definition" | "declaration" => {
                // Methods are handled separately but we track visibility
                if let Some(name) = extract_function_name(&child, source) {
                    let text = child.utf8_text(source).unwrap_or("");
                    let is_static = text.contains("static");
                    let is_virtual = text.contains("virtual");
                    let is_const = text.contains(") const");
                    let is_override = text.contains("override");
                    let is_constructor = name == class_name.split("::").last().unwrap_or(class_name);
                    let is_destructor = name.starts_with('~');
                    
                    // Build modifiers list
                    let mut modifiers = Vec::new();
                    if is_static { modifiers.push("static".to_string()); }
                    if is_virtual { modifiers.push("virtual".to_string()); }
                    if is_const { modifiers.push("const".to_string()); }
                    if is_override { modifiers.push("override".to_string()); }
                    
                    let member_type = if is_constructor {
                        "constructor"
                    } else if is_destructor {
                        "destructor"
                    } else {
                        "method"
                    };
                    
                    let kind = if is_constructor {
                        "constructor"
                    } else if is_destructor {
                        "destructor"
                    } else if is_static {
                        "static_method"
                    } else if is_virtual {
                        "virtual_method"
                    } else {
                        "method"
                    };
                    
                    // Add as symbol for backwards compatibility
                    data.add_symbol(CodeSymbol {
                        path: file_path.to_string(),
                        name: format!("{}::{}", class_name, name),
                        kind: kind.to_string(),
                        line: child.start_position().row + 1,
                        context: format!("{} {}", current_visibility,
                            text.lines().next().unwrap_or("")),
                    });
                    
                    // Add as MemberFact for better organization
                    data.add_member(MemberFact {
                        file: file_path.to_string(),
                        container: class_name.to_string(),
                        name: name.clone(),
                        member_type: member_type.to_string(),
                        visibility: current_visibility.to_string(),
                        modifiers,
                        line: child.start_position().row + 1,
                    });
                }
            }
            _ => {}
        }
    }
}

/// Enhanced function processing with method detection
fn process_cpp_function_enhanced(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
    is_method: bool,
    is_constructor: bool,
    is_destructor: bool,
) {
    let params = extract_parameters(node, source);
    let return_type = if is_constructor || is_destructor {
        None
    } else {
        extract_return_type(node, source)
    };
    
    // Check for const method
    let is_const_method = node.utf8_text(source)
        .unwrap_or("")
        .contains(") const");
    
    // Determine function kind
    let kind = if is_constructor {
        "constructor"
    } else if is_destructor {
        "destructor"
    } else if is_method {
        if is_const_method { "const_method" } else { "method" }
    } else {
        "function"
    };
    
    // Add enhanced symbol
    data.add_symbol(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: kind.to_string(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });
    
    // Add function fact with method info
    data.add_function(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: is_method && !is_const_method,
        takes_mut_params: params.iter().any(|p| !p.contains("const")),
        returns_result: false,
        returns_option: false,
        is_async: false,
        is_unsafe: false,
        is_public: true, // Would need more context for accurate visibility
        parameter_count: params.len() as i32,
        generic_count: count_template_params(node),
        parameters: params,
        return_type,
    });
}

/// Process C++ macros (same as C)
fn process_cpp_macro(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    if let Some(name_node) = node.child_by_field_name("name") {
        if let Ok(name) = name_node.utf8_text(source) {
            let value = node
                .child_by_field_name("value")
                .and_then(|v| v.utf8_text(source).ok())
                .map(|s| s.to_string());
            
            let context = node
                .utf8_text(source)
                .ok()
                .and_then(|s| s.lines().next())
                .unwrap_or("")
                .to_string();
            
            // Add as symbol for backwards compatibility
            data.add_symbol(CodeSymbol {
                path: file_path.to_string(),
                name: name.to_string(),
                kind: "macro".to_string(),
                line: node.start_position().row + 1,
                context,
            });
            
            // Add as ConstantFact for better organization
            data.add_constant(ConstantFact {
                file: file_path.to_string(),
                name: name.to_string(),
                value: value.clone(),
                const_type: "macro".to_string(),
                scope: "global".to_string(),
                line: node.start_position().row + 1,
            });
        }
    }
}

/// Process C++ global declarations (constants, statics, etc.)
fn process_cpp_declaration(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    let mut is_static = false;
    let mut is_const = false;
    let mut is_constexpr = false;
    let mut is_extern = false;
    
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "storage_class_specifier" => {
                if let Ok(text) = child.utf8_text(source) {
                    match text {
                        "static" => is_static = true,
                        "extern" => is_extern = true,
                        "constexpr" => is_constexpr = true,
                        _ => {}
                    }
                }
            }
            "type_qualifier" => {
                if let Ok(text) = child.utf8_text(source) {
                    if text == "const" {
                        is_const = true;
                    }
                }
            }
            "init_declarator" | "declarator" => {
                if let Some(name) = extract_declarator_name(&child, source) {
                    let kind = if is_constexpr {
                        "constexpr"
                    } else if is_const {
                        "const"
                    } else if is_static {
                        "static"
                    } else if is_extern {
                        "extern"
                    } else {
                        "global"
                    };
                    
                    let context = node
                        .utf8_text(source)
                        .ok()
                        .and_then(|s| s.lines().next())
                        .unwrap_or("")
                        .to_string();
                    
                    // Add as symbol for backwards compatibility
                    data.add_symbol(CodeSymbol {
                        path: file_path.to_string(),
                        name: name.clone(),
                        kind: kind.to_string(),
                        line: node.start_position().row + 1,
                        context: context.clone(),
                    });
                    
                    // Add as ConstantFact for better organization
                    let const_type = if is_constexpr {
                        "constexpr"
                    } else if is_const {
                        "const"
                    } else if is_static {
                        "static"
                    } else if is_extern {
                        "extern"
                    } else {
                        "global"
                    }.to_string();
                    
                    data.add_constant(ConstantFact {
                        file: file_path.to_string(),
                        name: name.clone(),
                        value: None, // Could extract initializer value here
                        const_type,
                        scope: "global".to_string(),
                        line: node.start_position().row + 1,
                    });
                }
            }
            _ => {}
        }
    }
}

/// Count template parameters for a function/class
fn count_template_params(node: &Node) -> i32 {
    // Look for template_declaration parent
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "template_declaration" {
            if let Some(params) = parent.child_by_field_name("parameters") {
                let mut count = 0;
                let mut cursor = params.walk();
                for child in params.children(&mut cursor) {
                    if matches!(child.kind(), "type_parameter_declaration" | "parameter_declaration") {
                        count += 1;
                    }
                }
                return count;
            }
        }
        current = parent.parent();
    }
    0
}

/// Extract declarator name (reused from C)
fn extract_declarator_name(node: &Node, source: &[u8]) -> Option<String> {
    if node.kind() == "identifier" {
        return node.utf8_text(source).ok().map(|s| s.to_string());
    }
    
    let mut current = Some(*node);
    while let Some(n) = current {
        if n.kind() == "identifier" {
            return n.utf8_text(source).ok().map(|s| s.to_string());
        }
        let mut cursor = n.walk();
        current = n.children(&mut cursor).find(|c| 
            c.kind() == "identifier" || 
            c.kind() == "declarator" || 
            c.kind() == "pointer_declarator" ||
            c.kind() == "array_declarator" ||
            c.kind() == "reference_declarator"
        );
    }
    None
}

/// Extract function name from C++ function_definition node
fn extract_function_name(node: &Node, source: &[u8]) -> Option<String> {
    // First check for simple declarator with name
    if let Some(declarator) = node.child_by_field_name("declarator") {
        return extract_cpp_function_name(declarator)
            .and_then(|n| n.utf8_text(source).ok())
            .map(|s| s.to_string());
    }

    // Fallback to standard name field
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.to_string())
}

/// Extract C++ function name from declarator (iterative to avoid stack overflow)
fn extract_cpp_function_name(declarator: Node) -> Option<Node> {
    let mut current = declarator;

    loop {
        match current.kind() {
            "identifier" | "field_identifier" | "destructor_name" | "operator_name" => {
                return Some(current);
            }
            "qualified_identifier" => {
                // For qualified names like Class::method
                if let Some(name) = current.child_by_field_name("name") {
                    return Some(name);
                }
            }
            "function_declarator" => {
                if let Some(inner) = current.child_by_field_name("declarator") {
                    current = inner;
                    continue;
                }
            }
            "pointer_declarator" | "reference_declarator" => {
                if let Some(inner) = current.child_by_field_name("declarator") {
                    current = inner;
                    continue;
                }
            }
            _ => {}
        }

        // Check children
        let mut cursor = current.walk();
        for child in current.children(&mut cursor) {
            if matches!(
                child.kind(),
                "identifier" | "field_identifier" | "destructor_name" | "operator_name"
            ) {
                return Some(child);
            }
        }

        return None;
    }
}

/// Extract typedef/using name
fn extract_typedef_name(node: &Node, source: &[u8]) -> Option<String> {
    // For using declarations
    if node.kind() == "alias_declaration" {
        return node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok())
            .map(|s| s.to_string());
    }

    // For typedef
    if let Some(declarator) = node.child_by_field_name("declarator") {
        return extract_declarator_name(&declarator, source);
    }

    None
}


/// Extract function parameters
fn extract_parameters(node: &Node, source: &[u8]) -> Vec<String> {
    if let Some(declarator) = node.child_by_field_name("declarator") {
        if let Some(params_node) = declarator.child_by_field_name("parameters") {
            let mut params = Vec::new();
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if matches!(
                    child.kind(),
                    "parameter_declaration" | "optional_parameter_declaration"
                ) {
                    if let Ok(param_text) = child.utf8_text(source) {
                        params.push(param_text.to_string());
                    }
                }
            }
            return params;
        }
    }
    Vec::new()
}

/// Extract return type
fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
    // Check for trailing return type (C++11)
    if let Some(trailing) = node.child_by_field_name("trailing_return_type") {
        if let Ok(text) = trailing.utf8_text(source) {
            return Some(text.trim_start_matches("->").trim().to_string());
        }
    }

    // Standard return type
    node.child_by_field_name("type")
        .and_then(|t| t.utf8_text(source).ok())
        .map(String::from)
}

/// Check if node has a template parent
fn has_template_parent(node: &Node) -> bool {
    let mut current = Some(*node);
    while let Some(n) = current {
        if n.kind() == "template_declaration" {
            return true;
        }
        current = n.parent();
    }
    false
}

/// Check if a member is public
fn is_public_member(node: &Node, source: &[u8]) -> bool {
    // Check parent for class/struct context
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "struct_specifier" => return true, // Struct members are public by default
            "class_specifier" => {
                // Class members are private by default
                // Look for access specifier before this node
                let mut is_public = false;
                let mut cursor = parent.walk();
                for sibling in parent.children(&mut cursor) {
                    if sibling.kind() == "access_specifier" {
                        if let Ok(text) = sibling.utf8_text(source) {
                            is_public = text.contains("public");
                        }
                    }
                    if sibling.start_byte() >= node.start_byte() {
                        return is_public;
                    }
                }
                return false;
            }
            _ => current = parent.parent(),
        }
    }

    // Not in a class/struct, so it's public
    true
}

/// Extract context around a symbol
fn extract_context(node: &Node, source: &[u8]) -> String {
    let start_byte = node.start_byte();
    let end_byte = node.end_byte().min(start_byte + 200);

    if let Ok(context) = std::str::from_utf8(&source[start_byte..end_byte]) {
        context.lines().take(3).collect::<Vec<_>>().join(" ")
    } else {
        String::new()
    }
}

/// Get type definition text
fn get_type_definition(node: &Node, source: &[u8]) -> String {
    if let Ok(text) = node.utf8_text(source) {
        let lines: Vec<&str> = text.lines().take(3).collect();
        let preview = lines.join("\n");
        if preview.len() > 200 {
            format!("{}...", &preview[..200])
        } else {
            preview
        }
    } else {
        String::new()
    }
}
