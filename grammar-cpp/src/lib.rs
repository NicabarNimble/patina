//! C++ grammar extraction plugin for Patina pipeline.
//!
//! Parses C++ source code using tree-sitter-cpp (compiled to WASM via wasi-sdk)
//! and returns ExtractedData JSON matching the host's contract.

use patina_sdk::pipeline::{parse_request, PipelinePlugin};
use patina_sdk::register_pipeline;
use tree_sitter::{Language, Node, Parser};

extern "C" {
    fn tree_sitter_cpp() -> Language;
}

fn language_cpp() -> Language {
    unsafe { tree_sitter_cpp() }
}

#[derive(Default)]
struct CppGrammar {
    parser: Option<Parser>,
}

impl PipelinePlugin for CppGrammar {
    fn name(&self) -> String {
        "grammar-cpp".into()
    }

    fn handle(&mut self, request: &str) -> Result<String, String> {
        let req = parse_request(request)?;

        if req.op != "parse" {
            return Err(format!("unsupported op: {}", req.op));
        }

        let source = req.payload["source"]
            .as_str()
            .ok_or("missing payload.source")?;

        let path = req.payload["path"].as_str().unwrap_or("");

        if self.parser.is_none() {
            let mut parser = Parser::new();
            parser
                .set_language(&language_cpp())
                .map_err(|e| format!("failed to set language: {}", e))?;
            self.parser = Some(parser);
        }
        let parser = self.parser.as_mut().unwrap();

        let tree = parser
            .parse(source.as_bytes(), None)
            .ok_or("tree-sitter parse failed")?;

        let extracted = extract_cpp_symbols(&tree.root_node(), source.as_bytes(), path);

        serde_json::to_string(&extracted).map_err(|e| format!("serialize error: {}", e))
    }
}

register_pipeline!(CppGrammar);

// =========================================================================
// ExtractedData — local types matching the host's JSON contract
// =========================================================================

#[derive(serde::Serialize)]
struct ExtractedData {
    symbols: Vec<CodeSymbol>,
    functions: Vec<FunctionFact>,
    types: Vec<TypeFact>,
    imports: Vec<ImportFact>,
    call_edges: Vec<CallEdge>,
    constants: Vec<ConstantFact>,
    members: Vec<MemberFact>,
}

#[derive(serde::Serialize)]
struct CodeSymbol {
    path: String,
    name: String,
    kind: String,
    line: usize,
    context: String,
}

#[derive(serde::Serialize)]
struct FunctionFact {
    file: String,
    name: String,
    takes_mut_self: bool,
    takes_mut_params: bool,
    returns_result: bool,
    returns_option: bool,
    is_async: bool,
    is_unsafe: bool,
    is_public: bool,
    parameter_count: i32,
    generic_count: i32,
    parameters: Vec<String>,
    return_type: Option<String>,
}

#[derive(serde::Serialize)]
struct TypeFact {
    file: String,
    name: String,
    definition: String,
    kind: String,
    visibility: String,
    usage_count: i32,
}

#[derive(serde::Serialize)]
struct ImportFact {
    file: String,
    import_path: String,
    imported_names: Vec<String>,
    import_kind: String,
    line_number: i32,
}

#[derive(serde::Serialize)]
struct CallEdge {
    caller: String,
    callee: String,
    file: String,
    call_type: String,
    line_number: i32,
}

#[derive(serde::Serialize)]
struct ConstantFact {
    file: String,
    name: String,
    value: Option<String>,
    const_type: String,
    scope: String,
    line: usize,
}

#[derive(serde::Serialize)]
struct MemberFact {
    file: String,
    container: String,
    name: String,
    member_type: String,
    visibility: String,
    modifiers: Vec<String>,
    line: usize,
}

// =========================================================================
// Extraction logic — ported from src/commands/scrape/code/languages/cpp.rs
// =========================================================================

fn extract_cpp_symbols(node: &Node, source: &[u8], file_path: &str) -> ExtractedData {
    let mut data = ExtractedData {
        symbols: Vec::new(),
        functions: Vec::new(),
        types: Vec::new(),
        imports: Vec::new(),
        call_edges: Vec::new(),
        constants: Vec::new(),
        members: Vec::new(),
    };

    let mut namespace_stack = Vec::new();
    walk_node(node, source, file_path, &mut data, None, &mut namespace_stack);
    data
}

fn walk_node(
    node: &Node,
    source: &[u8],
    file_path: &str,
    data: &mut ExtractedData,
    current_function: Option<&str>,
    namespace_stack: &mut Vec<String>,
) {
    match node.kind() {
        "namespace_definition" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    namespace_stack.push(name.to_string());
                }
            }
            if let Some(body) = node.child_by_field_name("body") {
                let mut cursor = body.walk();
                for child in body.children(&mut cursor) {
                    walk_node(&child, source, file_path, data, current_function, namespace_stack);
                }
            }
            namespace_stack.pop();
            return;
        }
        "function_definition" => {
            if let Some(name) = extract_function_name(node, source) {
                let full_name = qualify_name(&name, namespace_stack);

                let is_constructor = namespace_stack
                    .last()
                    .map(|class| name == *class)
                    .unwrap_or(false);
                let is_destructor = name.starts_with('~');
                let is_const_method = node
                    .utf8_text(source)
                    .unwrap_or("")
                    .contains(") const");

                process_cpp_function(
                    node, source, file_path, &full_name, data,
                    is_constructor, is_destructor, is_const_method,
                );

                let owned_name = full_name.clone();
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    walk_node(&child, source, file_path, data, Some(&owned_name), namespace_stack);
                }
                return;
            }
        }
        "class_specifier" | "struct_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    let kind = if node.kind() == "class_specifier" {
                        "class"
                    } else {
                        "struct"
                    };
                    let full_name = qualify_name(name, namespace_stack);

                    process_cpp_class(node, source, file_path, &full_name, kind, data);

                    namespace_stack.push(name.to_string());
                    if let Some(body) = node.child_by_field_name("body") {
                        extract_class_members(&body, source, file_path, &full_name, data);
                        let mut cursor = body.walk();
                        for child in body.children(&mut cursor) {
                            walk_node(&child, source, file_path, data, current_function, namespace_stack);
                        }
                    }
                    namespace_stack.pop();
                    return;
                }
            }
        }
        "enum_specifier" | "enum_class_specifier" => {
            let is_enum_class = node.kind() == "enum_class_specifier";
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    let full_name = qualify_name(name, namespace_stack);
                    process_cpp_enum(node, source, file_path, &full_name, is_enum_class, data);
                }
            }
        }
        "type_definition" | "alias_declaration" => {
            if let Some(name) = extract_typedef_name(node, source) {
                let full_name = qualify_name(&name, namespace_stack);
                let is_using = node.kind() == "alias_declaration";

                data.symbols.push(CodeSymbol {
                    path: file_path.to_string(),
                    name: full_name.clone(),
                    kind: if is_using { "using" } else { "typedef" }.into(),
                    line: node.start_position().row + 1,
                    context: extract_context(node, source),
                });

                data.types.push(TypeFact {
                    file: file_path.to_string(),
                    name: full_name,
                    definition: type_definition(node, source),
                    kind: if is_using { "using" } else { "typedef" }.into(),
                    visibility: "public".into(),
                    usage_count: 0,
                });
            }
        }
        "preproc_include" | "preproc_import" => {
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

                data.imports.push(ImportFact {
                    file: file_path.to_string(),
                    import_path: header.to_string(),
                    imported_names: vec![header.to_string()],
                    import_kind: if is_external { "system" } else { "local" }.into(),
                    line_number: (node.start_position().row + 1) as i32,
                });
            }
        }
        "preproc_def" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    let value = node
                        .child_by_field_name("value")
                        .and_then(|v| v.utf8_text(source).ok())
                        .map(|s| s.to_string());

                    data.symbols.push(CodeSymbol {
                        path: file_path.to_string(),
                        name: name.to_string(),
                        kind: "macro".into(),
                        line: node.start_position().row + 1,
                        context: first_line(node, source),
                    });

                    data.constants.push(ConstantFact {
                        file: file_path.to_string(),
                        name: name.to_string(),
                        value,
                        const_type: "macro".into(),
                        scope: "global".into(),
                        line: node.start_position().row + 1,
                    });
                }
            }
        }
        "declaration" => {
            if current_function.is_none() {
                process_cpp_declaration(node, source, file_path, data);
            }
        }
        "call_expression" => {
            if let Some(caller) = current_function {
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        data.call_edges.push(CallEdge {
                            caller: caller.to_string(),
                            callee: callee.to_string(),
                            file: file_path.to_string(),
                            call_type: "direct".into(),
                            line_number: (node.start_position().row + 1) as i32,
                        });
                    }
                }
            }
        }
        "new_expression" => {
            if let Some(caller) = current_function {
                if let Some(type_node) = node.child_by_field_name("type") {
                    if let Ok(class_name) = type_node.utf8_text(source) {
                        data.call_edges.push(CallEdge {
                            caller: caller.to_string(),
                            callee: format!("{}::constructor", class_name),
                            file: file_path.to_string(),
                            call_type: "constructor".into(),
                            line_number: (node.start_position().row + 1) as i32,
                        });
                    }
                }
            }
        }
        "delete_expression" => {
            if let Some(caller) = current_function {
                if let Some(arg_node) = node.child_by_field_name("argument") {
                    if let Ok(var_name) = arg_node.utf8_text(source) {
                        data.call_edges.push(CallEdge {
                            caller: caller.to_string(),
                            callee: format!("~{}", var_name),
                            file: file_path.to_string(),
                            call_type: "destructor".into(),
                            line_number: (node.start_position().row + 1) as i32,
                        });
                    }
                }
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(&child, source, file_path, data, current_function, namespace_stack);
    }
}

// ---- Function extraction ----

fn process_cpp_function(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
    is_constructor: bool,
    is_destructor: bool,
    is_const_method: bool,
) {
    let params = extract_parameters(node, source);
    let return_type = if is_constructor || is_destructor {
        None
    } else {
        // Check for trailing return type first (C++11: auto f() -> int)
        node.child_by_field_name("trailing_return_type")
            .and_then(|t| t.utf8_text(source).ok())
            .map(|s| s.trim_start_matches("->").trim().to_string())
            .or_else(|| {
                node.child_by_field_name("type")
                    .and_then(|t| t.utf8_text(source).ok())
                    .map(String::from)
            })
    };

    let kind = if is_constructor {
        "constructor"
    } else if is_destructor {
        "destructor"
    } else if is_const_method {
        "const_method"
    } else {
        "function"
    };

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: kind.into(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });

    data.functions.push(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: !is_const_method && !is_constructor,
        takes_mut_params: params.iter().any(|p| !p.contains("const")),
        returns_result: false,
        returns_option: false,
        is_async: false,
        is_unsafe: false,
        is_public: is_public_member(node, source),
        parameter_count: params.len() as i32,
        generic_count: count_template_params(node),
        parameters: params,
        return_type,
    });
}

// ---- Class extraction ----

fn process_cpp_class(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: &str,
    data: &mut ExtractedData,
) {
    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: kind.to_string(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: type_definition(node, source),
        kind: kind.to_string(),
        visibility: "public".into(),
        usage_count: 0,
    });

    // Extract inheritance
    let mut cursor = node.walk();
    let mut found_name = false;
    for child in node.children(&mut cursor) {
        if child.kind() == "type_identifier" && !found_name {
            found_name = true;
            continue;
        }
        if child.kind() == "base_class_clause" {
            let mut access = if kind == "struct" { "public" } else { "private" };
            let mut spec_cursor = child.walk();
            for spec_child in child.children(&mut spec_cursor) {
                if spec_child.kind() == "access_specifier" {
                    if let Ok(text) = spec_child.utf8_text(source) {
                        access = if text.contains("public") {
                            "public"
                        } else if text.contains("protected") {
                            "protected"
                        } else {
                            "private"
                        };
                    }
                }
            }

            if let Some(type_node) = child.children(&mut child.walk()).find(|c| {
                matches!(
                    c.kind(),
                    "type_identifier" | "qualified_identifier" | "template_type"
                )
            }) {
                if let Ok(base_name) = type_node.utf8_text(source) {
                    data.constants.push(ConstantFact {
                        file: file_path.to_string(),
                        name: format!("{}::inherits::{}", name, base_name),
                        value: Some(access.to_string()),
                        const_type: "inheritance".into(),
                        scope: name.to_string(),
                        line: child.start_position().row + 1,
                    });
                    data.symbols.push(CodeSymbol {
                        path: file_path.to_string(),
                        name: format!("{} : {} {}", name, access, base_name),
                        kind: "inheritance".into(),
                        line: child.start_position().row + 1,
                        context: format!("{} inherits {} from {}", name, access, base_name),
                    });
                }
            }
        }
    }
}

fn extract_class_members(
    body: &Node,
    source: &[u8],
    file_path: &str,
    class_name: &str,
    data: &mut ExtractedData,
) {
    let mut current_visibility = "private";

    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        match child.kind() {
            "access_specifier" => {
                if let Ok(text) = child.utf8_text(source) {
                    let trimmed = text.trim_end_matches(':').trim();
                    if trimmed == "public" || trimmed == "private" || trimmed == "protected" {
                        current_visibility = if trimmed == "public" {
                            "public"
                        } else if trimmed == "protected" {
                            "protected"
                        } else {
                            "private"
                        };
                    }
                }
            }
            "field_declaration" => {
                if let Some(declarator) = child.child_by_field_name("declarator") {
                    if let Some(name) = extract_declarator_name(&declarator, source) {
                        let text = child.utf8_text(source).unwrap_or("");
                        let mut modifiers = Vec::new();
                        if text.contains("static") {
                            modifiers.push("static".to_string());
                        }
                        if text.contains("const") {
                            modifiers.push("const".to_string());
                        }
                        if text.contains("mutable") {
                            modifiers.push("mutable".to_string());
                        }

                        let full_name = format!("{}::{}", class_name, name);
                        data.symbols.push(CodeSymbol {
                            path: file_path.to_string(),
                            name: full_name,
                            kind: "field".into(),
                            line: child.start_position().row + 1,
                            context: format!("{}: {}", current_visibility, child.utf8_text(source).unwrap_or("").lines().next().unwrap_or("")),
                        });
                        data.members.push(MemberFact {
                            file: file_path.to_string(),
                            container: class_name.to_string(),
                            name,
                            member_type: "field".into(),
                            visibility: current_visibility.to_string(),
                            modifiers,
                            line: child.start_position().row + 1,
                        });
                    }
                }
            }
            "function_definition" | "declaration" => {
                if let Some(name) = extract_function_name(&child, source) {
                    let text = child.utf8_text(source).unwrap_or("");
                    let is_static = text.contains("static");
                    let is_virtual = text.contains("virtual");
                    let is_const = text.contains(") const");
                    let is_override = text.contains("override");
                    let is_constructor =
                        name == class_name.split("::").last().unwrap_or(class_name);
                    let is_destructor = name.starts_with('~');

                    let mut modifiers = Vec::new();
                    if is_static { modifiers.push("static".to_string()); }
                    if is_virtual { modifiers.push("virtual".to_string()); }
                    if is_const { modifiers.push("const".to_string()); }
                    if is_override { modifiers.push("override".to_string()); }

                    let member_type = if is_constructor {
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

                    let full_name = format!("{}::{}", class_name, name);
                    data.symbols.push(CodeSymbol {
                        path: file_path.to_string(),
                        name: full_name,
                        kind: member_type.into(),
                        line: child.start_position().row + 1,
                        context: format!("{}: {}", current_visibility, child.utf8_text(source).unwrap_or("").lines().next().unwrap_or("")),
                    });

                    data.members.push(MemberFact {
                        file: file_path.to_string(),
                        container: class_name.to_string(),
                        name,
                        member_type: member_type.into(),
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

// ---- Enum extraction ----

fn process_cpp_enum(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    is_enum_class: bool,
    data: &mut ExtractedData,
) {
    let kind = if is_enum_class { "enum_class" } else { "enum" };

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: kind.into(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: type_definition(node, source),
        kind: kind.into(),
        visibility: "public".into(),
        usage_count: 0,
    });

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

                        data.symbols.push(CodeSymbol {
                            path: file_path.to_string(),
                            name: full_name.clone(),
                            kind: "enum_value".into(),
                            line: child.start_position().row + 1,
                            context: if let Some(ref val) = value {
                                format!("{} = {}", value_name, val)
                            } else {
                                value_name.to_string()
                            },
                        });

                        data.constants.push(ConstantFact {
                            file: file_path.to_string(),
                            name: full_name,
                            value,
                            const_type: "enum_value".into(),
                            scope: name.to_string(),
                            line: child.start_position().row + 1,
                        });
                    }
                }
            }
        }
    }
}

// ---- Declaration extraction ----

fn process_cpp_declaration(
    node: &Node,
    source: &[u8],
    file_path: &str,
    data: &mut ExtractedData,
) {
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

                    data.symbols.push(CodeSymbol {
                        path: file_path.to_string(),
                        name: name.clone(),
                        kind: kind.to_string(),
                        line: node.start_position().row + 1,
                        context: first_line(node, source),
                    });

                    data.constants.push(ConstantFact {
                        file: file_path.to_string(),
                        name,
                        value: None,
                        const_type: kind.to_string(),
                        scope: "global".into(),
                        line: node.start_position().row + 1,
                    });
                }
            }
            _ => {}
        }
    }
}

// =========================================================================
// Helpers
// =========================================================================

fn qualify_name(name: &str, namespace_stack: &[String]) -> String {
    if namespace_stack.is_empty() {
        name.to_string()
    } else {
        format!("{}::{}", namespace_stack.join("::"), name)
    }
}

fn extract_function_name(node: &Node, source: &[u8]) -> Option<String> {
    if let Some(declarator) = node.child_by_field_name("declarator") {
        return extract_cpp_function_name_from(declarator)
            .and_then(|n| n.utf8_text(source).ok())
            .map(|s| s.to_string());
    }
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.to_string())
}

fn extract_cpp_function_name_from(declarator: Node) -> Option<Node> {
    let mut current = declarator;
    loop {
        match current.kind() {
            "identifier" | "field_identifier" | "destructor_name" | "operator_name" => {
                return Some(current);
            }
            "qualified_identifier" => {
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

fn extract_typedef_name(node: &Node, source: &[u8]) -> Option<String> {
    if node.kind() == "alias_declaration" {
        return node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok())
            .map(|s| s.to_string());
    }
    if let Some(declarator) = node.child_by_field_name("declarator") {
        return extract_declarator_name(&declarator, source);
    }
    None
}

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
        current = n.children(&mut cursor).find(|c| {
            c.kind() == "identifier"
                || c.kind() == "declarator"
                || c.kind() == "pointer_declarator"
                || c.kind() == "array_declarator"
                || c.kind() == "reference_declarator"
        });
    }
    None
}

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

fn count_template_params(node: &Node) -> i32 {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "template_declaration" {
            if let Some(params) = parent.child_by_field_name("parameters") {
                let mut count = 0;
                let mut cursor = params.walk();
                for child in params.children(&mut cursor) {
                    if matches!(
                        child.kind(),
                        "type_parameter_declaration" | "parameter_declaration"
                    ) {
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

fn is_public_member(node: &Node, source: &[u8]) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "struct_specifier" => return true,
            "class_specifier" => {
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
    true
}

fn extract_context(node: &Node, source: &[u8]) -> String {
    let start_byte = node.start_byte();
    let end_byte = node.end_byte().min(start_byte + 200);
    if let Ok(context) = std::str::from_utf8(&source[start_byte..end_byte]) {
        context.lines().take(3).collect::<Vec<_>>().join(" ")
    } else {
        String::new()
    }
}

fn type_definition(node: &Node, source: &[u8]) -> String {
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

fn first_line(node: &Node, source: &[u8]) -> String {
    node.utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .to_string()
}
