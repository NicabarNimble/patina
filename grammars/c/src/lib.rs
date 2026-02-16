//! C grammar extraction plugin for Patina pipeline.
//!
//! Parses C source code using tree-sitter-c (compiled to WASM via wasi-sdk)
//! and returns ExtractedData JSON matching the host's contract.

use patina_sdk::pipeline::{parse_request, PipelinePlugin};
use patina_sdk::register_pipeline;
use tree_sitter::{Language, Node, Parser};

extern "C" {
    fn tree_sitter_c() -> Language;
}

fn language_c() -> Language {
    unsafe { tree_sitter_c() }
}

#[derive(Default)]
struct CGrammar {
    parser: Option<Parser>,
}

impl PipelinePlugin for CGrammar {
    fn name(&self) -> String {
        "grammar-c".into()
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
                .set_language(&language_c())
                .map_err(|e| format!("failed to set language: {}", e))?;
            self.parser = Some(parser);
        }
        let parser = self.parser.as_mut().unwrap();

        let tree = parser
            .parse(source.as_bytes(), None)
            .ok_or("tree-sitter parse failed")?;

        let extracted = extract_c_symbols(&tree.root_node(), source.as_bytes(), path);

        serde_json::to_string(&extracted).map_err(|e| format!("serialize error: {}", e))
    }
}

register_pipeline!(CGrammar);

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
// Extraction logic — ported from src/commands/scrape/code/languages/c.rs
// =========================================================================

fn extract_c_symbols(node: &Node, source: &[u8], file_path: &str) -> ExtractedData {
    let mut data = ExtractedData {
        symbols: Vec::new(),
        functions: Vec::new(),
        types: Vec::new(),
        imports: Vec::new(),
        call_edges: Vec::new(),
        constants: Vec::new(),
        members: Vec::new(),
    };

    walk_node(node, source, file_path, &mut data, None);
    data
}

fn walk_node(
    node: &Node,
    source: &[u8],
    file_path: &str,
    data: &mut ExtractedData,
    current_function: Option<&str>,
) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = extract_function_name(node, source) {
                process_c_function(node, source, file_path, &name, data);
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    walk_node(&child, source, file_path, data, Some(&name));
                }
                return;
            }
        }
        "struct_specifier" | "union_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    let kind = if node.kind() == "struct_specifier" {
                        "struct"
                    } else {
                        "union"
                    };
                    process_c_type(node, source, file_path, name, kind, data);
                }
            }
        }
        "enum_specifier" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    process_c_type(node, source, file_path, name, "enum", data);
                    process_c_enum_values(node, source, file_path, name, data);
                }
            }
        }
        "type_definition" => {
            if let Some(declarator) = node.child_by_field_name("declarator") {
                if let Some(name) = extract_typedef_name(&declarator, source) {
                    process_c_typedef(node, source, file_path, &name, data);
                }
            }
        }
        "preproc_include" => {
            process_c_include(node, source, file_path, data);
        }
        "preproc_def" => {
            process_c_macro(node, source, file_path, data);
        }
        "declaration" => {
            if current_function.is_none() {
                process_c_declaration(node, source, file_path, data);
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
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(&child, source, file_path, data, current_function);
    }
}

// ---- Function extraction ----

fn process_c_function(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let params = extract_parameters(node, source);
    let return_type = node
        .child_by_field_name("type")
        .and_then(|t| t.utf8_text(source).ok())
        .map(String::from);
    let is_public = file_path.ends_with(".h");

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "function".into(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });

    data.functions.push(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: false,
        takes_mut_params: params.iter().any(|p| p.contains('*')),
        returns_result: false,
        returns_option: false,
        is_async: false,
        is_unsafe: true,
        is_public,
        parameter_count: params.len() as i32,
        generic_count: 0,
        parameters: params,
        return_type,
    });
}

// ---- Type extraction ----

fn process_c_type(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: &str,
    data: &mut ExtractedData,
) {
    let is_public = file_path.ends_with(".h");

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
        definition: format!("{} {}", kind, name),
        kind: kind.to_string(),
        visibility: if is_public { "public" } else { "private" }.into(),
        usage_count: 0,
    });

    if matches!(kind, "struct" | "union") {
        process_c_struct_fields(node, source, file_path, name, data);
    }
}

fn process_c_typedef(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = file_path.ends_with(".h");

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "typedef".into(),
        line: node.start_position().row + 1,
        context: extract_context(node, source),
    });

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: format!("typedef {}", name),
        kind: "typedef".into(),
        visibility: if is_public { "public" } else { "private" }.into(),
        usage_count: 0,
    });
}

fn process_c_struct_fields(
    node: &Node,
    source: &[u8],
    file_path: &str,
    struct_name: &str,
    data: &mut ExtractedData,
) {
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "field_declaration" {
                let field_type = child
                    .child_by_field_name("type")
                    .and_then(|t| t.utf8_text(source).ok())
                    .unwrap_or("unknown");

                if let Some(declarator) = child.child_by_field_name("declarator") {
                    if let Some(field_name) = extract_declarator_name(&declarator, source) {
                        data.members.push(MemberFact {
                            file: file_path.to_string(),
                            container: struct_name.to_string(),
                            name: field_name.clone(),
                            member_type: "field".into(),
                            visibility: "public".into(),
                            modifiers: vec![],
                            line: child.start_position().row + 1,
                        });

                        data.symbols.push(CodeSymbol {
                            path: file_path.to_string(),
                            name: format!("{}::{}", struct_name, field_name),
                            kind: "field".into(),
                            line: child.start_position().row + 1,
                            context: format!("{} {}", field_type, field_name),
                        });
                    }
                }
            }
        }
    }
}

fn process_c_enum_values(
    node: &Node,
    source: &[u8],
    file_path: &str,
    enum_name: &str,
    data: &mut ExtractedData,
) {
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

                        let full_name = format!("{}::{}", enum_name, value_name);

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
                            scope: enum_name.to_string(),
                            line: child.start_position().row + 1,
                        });
                    }
                }
            }
        }
    }
}

// ---- Include extraction ----

fn process_c_include(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    if let Ok(include_text) = node.utf8_text(source) {
        let header = include_text
            .trim_start_matches("#include")
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

// ---- Macro extraction ----

fn process_c_macro(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    if let Some(name_node) = node.child_by_field_name("name") {
        if let Ok(name) = name_node.utf8_text(source) {
            let value = node
                .child_by_field_name("value")
                .and_then(|v| v.utf8_text(source).ok())
                .map(|s| s.to_string());

            let context = first_line(node, source);

            data.symbols.push(CodeSymbol {
                path: file_path.to_string(),
                name: name.to_string(),
                kind: "macro".into(),
                line: node.start_position().row + 1,
                context,
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

// ---- Declaration extraction ----

fn process_c_declaration(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    let mut is_static = false;
    let mut is_const = false;
    let mut is_extern = false;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "storage_class_specifier" => {
                if let Ok(text) = child.utf8_text(source) {
                    match text {
                        "static" => is_static = true,
                        "extern" => is_extern = true,
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
                    let kind = if is_const {
                        "const"
                    } else if is_static {
                        "static"
                    } else if is_extern {
                        "extern"
                    } else {
                        "global"
                    };

                    let context = first_line(node, source);

                    data.symbols.push(CodeSymbol {
                        path: file_path.to_string(),
                        name: name.clone(),
                        kind: kind.to_string(),
                        line: node.start_position().row + 1,
                        context,
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

fn extract_function_name(node: &Node, source: &[u8]) -> Option<String> {
    let declarator = node.child_by_field_name("declarator")?;
    extract_c_function_name_from(declarator)
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.to_string())
}

/// Iterative function name extraction to avoid stack overflow with nested declarators
fn extract_c_function_name_from(declarator: Node) -> Option<Node> {
    let mut current = declarator;

    loop {
        if current.kind() == "identifier" {
            return Some(current);
        }

        if current.kind() == "function_declarator" {
            if let Some(inner) = current.child_by_field_name("declarator") {
                current = inner;
                continue;
            }
        }

        if current.kind() == "pointer_declarator" {
            if let Some(inner) = current.child_by_field_name("declarator") {
                current = inner;
                continue;
            }
        }

        let mut cursor = current.walk();
        for child in current.children(&mut cursor) {
            if child.kind() == "identifier" {
                return Some(child);
            }
        }

        return None;
    }
}

fn extract_typedef_name(declarator: &Node, source: &[u8]) -> Option<String> {
    if declarator.kind() == "type_identifier" || declarator.kind() == "identifier" {
        return declarator.utf8_text(source).ok().map(|s| s.to_string());
    }

    if declarator.kind() == "pointer_declarator" {
        if let Some(inner) = declarator.child_by_field_name("declarator") {
            return extract_typedef_name(&inner, source);
        }
    }

    let mut cursor = declarator.walk();
    for child in declarator.children(&mut cursor) {
        if child.kind() == "type_identifier" || child.kind() == "identifier" {
            return child.utf8_text(source).ok().map(|s| s.to_string());
        }
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
                if child.kind() == "parameter_declaration" {
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

fn extract_context(node: &Node, source: &[u8]) -> String {
    let start_byte = node.start_byte();
    let end_byte = node.end_byte().min(start_byte + 200);

    if let Ok(context) = std::str::from_utf8(&source[start_byte..end_byte]) {
        context.lines().take(3).collect::<Vec<_>>().join(" ")
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
