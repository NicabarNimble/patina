//! Go grammar extraction plugin for Patina pipeline.
//!
//! Parses Go source code using tree-sitter-go (compiled to WASM via wasi-sdk)
//! and returns ExtractedData JSON matching the host's contract.

use patina_sdk::pipeline::{parse_request, PipelinePlugin};
use patina_sdk::register_pipeline;
use tree_sitter::{Language, Node, Parser};

extern "C" {
    fn tree_sitter_go() -> Language;
}

fn language_go() -> Language {
    unsafe { tree_sitter_go() }
}

#[derive(Default)]
struct GoGrammar {
    parser: Option<Parser>,
}

impl PipelinePlugin for GoGrammar {
    fn name(&self) -> String {
        "grammar-go".into()
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
                .set_language(&language_go())
                .map_err(|e| format!("failed to set language: {}", e))?;
            self.parser = Some(parser);
        }
        let parser = self.parser.as_mut().unwrap();

        let tree = parser
            .parse(source.as_bytes(), None)
            .ok_or("tree-sitter parse failed")?;

        let extracted = extract_go_symbols(&tree.root_node(), source.as_bytes(), path);

        serde_json::to_string(&extracted).map_err(|e| format!("serialize error: {}", e))
    }
}

register_pipeline!(GoGrammar);

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
// Extraction logic — ported from src/commands/scrape/code/languages/go.rs
// =========================================================================

fn extract_go_symbols(node: &Node, source: &[u8], file_path: &str) -> ExtractedData {
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
    // Extract calls first
    extract_go_calls(node, source, file_path, current_function, data);

    match node.kind() {
        "package_clause" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "package_identifier" {
                    if let Ok(package_name) = child.utf8_text(source) {
                        data.constants.push(ConstantFact {
                            file: file_path.to_string(),
                            name: format!("package:{}", package_name),
                            value: Some(package_name.to_string()),
                            const_type: "package".into(),
                            scope: "file".into(),
                            line: node.start_position().row + 1,
                        });
                        break;
                    }
                }
            }
            return;
        }
        "const_declaration" => {
            process_go_constants(node, source, file_path, data);
            return;
        }
        "var_declaration" => {
            if current_function.is_none() {
                process_go_globals(node, source, file_path, data);
            }
            return;
        }
        "function_declaration" | "method_declaration" => {
            if let Some(name) = get_field_text(node, "name", source) {
                process_go_function(node, source, file_path, &name, data);
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    walk_node(&child, source, file_path, data, Some(&name));
                }
                return;
            }
        }
        "type_declaration" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "type_spec" {
                    if let Some((name, kind)) = extract_type_info(&child, source) {
                        process_go_type(&child, source, file_path, &name, &kind, data);
                    }
                }
            }
        }
        "import_declaration" => {
            process_go_import(node, source, file_path, data);
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(&child, source, file_path, data, current_function);
    }
}

// ---- Function extraction ----

fn process_go_function(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = is_exported(name);
    let params = extract_params(node, source);
    let return_type = get_field_text(node, "result", source);
    let generics = get_field_text(node, "type_parameters", source);

    data.functions.push(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: false,
        takes_mut_params: false,
        returns_result: false,
        returns_option: false,
        is_async: false,
        is_unsafe: false,
        is_public,
        parameter_count: params.len() as i32,
        generic_count: if generics.is_some() { 1 } else { 0 },
        parameters: params,
        return_type,
    });

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "function".into(),
        line: node.start_position().row + 1,
        context: first_line(node, source),
    });
}

// ---- Type extraction ----

fn process_go_type(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: &str,
    data: &mut ExtractedData,
) {
    let is_public = is_exported(name);

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: kind.to_string(),
        line: node.start_position().row + 1,
        context: first_line(node, source),
    });

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: type_definition(node, source),
        kind: kind.to_string(),
        visibility: if is_public { "public" } else { "private" }.into(),
        usage_count: 0,
    });

    if kind == "struct" {
        extract_struct_fields(node, source, file_path, name, data);
    } else if kind == "interface" {
        extract_interface_methods(node, source, file_path, name, data);
    }
}

fn extract_type_info(node: &Node, source: &[u8]) -> Option<(String, String)> {
    let name = get_field_text(node, "name", source)?;
    let kind = if let Some(type_node) = node.child_by_field_name("type") {
        match type_node.kind() {
            "struct_type" => "struct",
            "interface_type" => "interface",
            _ => "type_alias",
        }
    } else {
        "type_alias"
    };
    Some((name, kind.to_string()))
}

fn extract_struct_fields(
    node: &Node,
    source: &[u8],
    file_path: &str,
    struct_name: &str,
    data: &mut ExtractedData,
) {
    let struct_node = if node.kind() == "type_spec" {
        node.child_by_field_name("type")
    } else {
        Some(*node)
    };

    if let Some(struct_node) = struct_node {
        if struct_node.kind() == "struct_type" {
            let mut cursor = struct_node.walk();
            for child in struct_node.children(&mut cursor) {
                if child.kind() == "field_declaration_list" {
                    let mut field_cursor = child.walk();
                    for field in child.children(&mut field_cursor) {
                        if field.kind() == "field_declaration" {
                            extract_field_declaration(&field, source, file_path, struct_name, data);
                        }
                    }
                }
            }
        }
    }
}

fn extract_field_declaration(
    node: &Node,
    source: &[u8],
    file_path: &str,
    struct_name: &str,
    data: &mut ExtractedData,
) {
    let mut field_names = Vec::new();
    let mut field_tag = false;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "field_identifier" => {
                if let Ok(name) = child.utf8_text(source) {
                    field_names.push(name.to_string());
                }
            }
            "tag" => {
                field_tag = true;
            }
            _ => {}
        }
    }

    for name in field_names {
        let visibility = if is_exported(&name) {
            "public"
        } else {
            "private"
        };
        let mut modifiers = Vec::new();
        if field_tag {
            modifiers.push("tagged".to_string());
        }

        data.members.push(MemberFact {
            file: file_path.to_string(),
            container: struct_name.to_string(),
            name,
            member_type: "field".into(),
            visibility: visibility.into(),
            modifiers,
            line: node.start_position().row + 1,
        });
    }
}

fn extract_interface_methods(
    node: &Node,
    source: &[u8],
    file_path: &str,
    interface_name: &str,
    data: &mut ExtractedData,
) {
    let interface_node = if node.kind() == "type_spec" {
        node.child_by_field_name("type")
    } else {
        Some(*node)
    };

    if let Some(interface_node) = interface_node {
        if interface_node.kind() == "interface_type" {
            let mut cursor = interface_node.walk();
            for child in interface_node.children(&mut cursor) {
                match child.kind() {
                    "method_elem" => {
                        if let Some(name_node) = child.child_by_field_name("name") {
                            if let Ok(method_name) = name_node.utf8_text(source) {
                                let visibility = if is_exported(method_name) {
                                    "public"
                                } else {
                                    "private"
                                };
                                data.members.push(MemberFact {
                                    file: file_path.to_string(),
                                    container: interface_name.to_string(),
                                    name: method_name.to_string(),
                                    member_type: "method".into(),
                                    visibility: visibility.into(),
                                    modifiers: vec!["abstract".to_string()],
                                    line: child.start_position().row + 1,
                                });
                            }
                        }
                    }
                    "type_elem" => {
                        let mut type_cursor = child.walk();
                        for type_child in child.children(&mut type_cursor) {
                            if matches!(type_child.kind(), "type_identifier" | "qualified_type") {
                                if let Ok(embedded_name) = type_child.utf8_text(source) {
                                    data.members.push(MemberFact {
                                        file: file_path.to_string(),
                                        container: interface_name.to_string(),
                                        name: embedded_name.to_string(),
                                        member_type: "embedded".into(),
                                        visibility: "public".into(),
                                        modifiers: vec!["embedded".to_string()],
                                        line: type_child.start_position().row + 1,
                                    });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

// ---- Import extraction ----

fn process_go_import(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "import_spec" {
            extract_import_spec(&child, source, file_path, data);
        } else if child.kind() == "import_spec_list" {
            let mut list_cursor = child.walk();
            for spec in child.children(&mut list_cursor) {
                if spec.kind() == "import_spec" {
                    extract_import_spec(&spec, source, file_path, data);
                }
            }
        }
    }
}

fn extract_import_spec(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    let mut alias = None;
    let mut path = None;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "package_identifier" | "dot" | "blank_identifier" => {
                alias = child.utf8_text(source).ok().map(|s| s.to_string());
            }
            "interpreted_string_literal" => {
                path = child
                    .utf8_text(source)
                    .ok()
                    .map(|s| s.trim_matches('"').to_string());
            }
            _ => {}
        }
    }

    if let Some(import_path) = path {
        let imported_names = if let Some(alias) = alias {
            vec![alias]
        } else {
            vec![import_path
                .split('/')
                .next_back()
                .unwrap_or(&import_path)
                .to_string()]
        };

        let is_external = !import_path.starts_with('.');

        data.imports.push(ImportFact {
            file: file_path.to_string(),
            import_path,
            imported_names,
            import_kind: if is_external { "external" } else { "relative" }.into(),
            line_number: (node.start_position().row + 1) as i32,
        });
    }
}

// ---- Constant extraction ----

fn process_go_constants(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    let mut cursor = node.walk();
    let mut iota_counter = 0;

    for child in node.children(&mut cursor) {
        if child.kind() == "const_spec" {
            let mut const_name = None;
            let mut const_value = None;

            let mut spec_cursor = child.walk();
            for spec_child in child.children(&mut spec_cursor) {
                match spec_child.kind() {
                    "identifier" => {
                        if const_name.is_none() {
                            const_name = spec_child.utf8_text(source).ok().map(String::from);
                        }
                    }
                    "expression_list" => {
                        if let Ok(text) = spec_child.utf8_text(source) {
                            const_value = Some(if text.contains("iota") {
                                format!("{} (={})", text, iota_counter)
                            } else {
                                text.to_string()
                            });
                        }
                    }
                    _ => {
                        if (spec_child.kind().ends_with("_literal")
                            || spec_child.kind() == "identifier")
                            && const_value.is_none()
                        {
                            const_value = spec_child.utf8_text(source).ok().map(String::from);
                        }
                    }
                }
            }

            if let Some(name) = const_name {
                if const_value.is_none() && node.child_count() > 1 {
                    const_value = Some(iota_counter.to_string());
                }

                data.constants.push(ConstantFact {
                    file: file_path.to_string(),
                    name: name.clone(),
                    value: const_value,
                    const_type: "const".into(),
                    scope: "global".into(),
                    line: child.start_position().row + 1,
                });

                data.symbols.push(CodeSymbol {
                    path: file_path.to_string(),
                    name,
                    kind: "constant".into(),
                    line: child.start_position().row + 1,
                    context: first_line(&child, source),
                });

                iota_counter += 1;
            }
        }
    }
}

fn process_go_globals(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "var_spec" {
            let mut var_name = None;
            let mut var_value = None;
            let mut var_type = None;

            let mut spec_cursor = child.walk();
            for spec_child in child.children(&mut spec_cursor) {
                match spec_child.kind() {
                    "identifier" => {
                        if var_name.is_none() {
                            var_name = spec_child.utf8_text(source).ok().map(String::from);
                        }
                    }
                    "type_identifier" | "qualified_type" | "pointer_type" | "slice_type"
                    | "map_type" => {
                        var_type = spec_child.utf8_text(source).ok().map(String::from);
                    }
                    "expression_list" => {
                        var_value = spec_child.utf8_text(source).ok().map(String::from);
                    }
                    _ => {}
                }
            }

            if let Some(name) = var_name {
                data.constants.push(ConstantFact {
                    file: file_path.to_string(),
                    name: name.clone(),
                    value: var_value.or(var_type),
                    const_type: "global".into(),
                    scope: "global".into(),
                    line: child.start_position().row + 1,
                });

                data.symbols.push(CodeSymbol {
                    path: file_path.to_string(),
                    name,
                    kind: "variable".into(),
                    line: child.start_position().row + 1,
                    context: first_line(&child, source),
                });
            }
        }
    }
}

// ---- Call graph extraction ----

fn extract_go_calls(
    node: &Node,
    source: &[u8],
    file_path: &str,
    current_function: Option<&str>,
    data: &mut ExtractedData,
) {
    let line_number = (node.start_position().row + 1) as i32;

    match node.kind() {
        "call_expression" => {
            if let Some(caller) = current_function {
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        data.call_edges.push(CallEdge {
                            caller: caller.to_string(),
                            callee: callee.to_string(),
                            file: file_path.to_string(),
                            call_type: "direct".into(),
                            line_number,
                        });
                    }
                }
            }
        }
        "go_statement" => {
            if let Some(caller) = current_function {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "call_expression" {
                        if let Some(func_node) = child.child_by_field_name("function") {
                            if let Ok(callee) = func_node.utf8_text(source) {
                                data.call_edges.push(CallEdge {
                                    caller: caller.to_string(),
                                    callee: callee.to_string(),
                                    file: file_path.to_string(),
                                    call_type: "goroutine".into(),
                                    line_number,
                                });
                            }
                        }
                    }
                }
            }
        }
        "defer_statement" => {
            if let Some(caller) = current_function {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "call_expression" {
                        if let Some(func_node) = child.child_by_field_name("function") {
                            if let Ok(callee) = func_node.utf8_text(source) {
                                data.call_edges.push(CallEdge {
                                    caller: caller.to_string(),
                                    callee: callee.to_string(),
                                    file: file_path.to_string(),
                                    call_type: "defer".into(),
                                    line_number,
                                });
                            }
                        }
                    }
                }
            }
        }
        "selector_expression" => {
            if let Some(parent) = node.parent() {
                if parent.kind() == "call_expression" {
                    if let Some(caller) = current_function {
                        if let Some(field_node) = node.child_by_field_name("field") {
                            if let Ok(callee) = field_node.utf8_text(source) {
                                data.call_edges.push(CallEdge {
                                    caller: caller.to_string(),
                                    callee: callee.to_string(),
                                    file: file_path.to_string(),
                                    call_type: "method".into(),
                                    line_number,
                                });
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

// =========================================================================
// Helpers
// =========================================================================

fn get_field_text(node: &Node, field: &str, source: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.to_string())
}

fn first_line(node: &Node, source: &[u8]) -> String {
    node.utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .to_string()
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

fn is_exported(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase())
}

fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut params = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if child.kind() == "parameter_declaration" {
                if let Ok(param_text) = child.utf8_text(source) {
                    params.push(param_text.to_string());
                }
            }
        }
        params
    } else {
        Vec::new()
    }
}
