//! JavaScript grammar extraction plugin for Patina pipeline.
//!
//! Parses JavaScript source code using tree-sitter-javascript (compiled to WASM via wasi-sdk)
//! and returns ExtractedData JSON matching the host's contract.

use patina_sdk::pipeline::{parse_request, PipelinePlugin};
use patina_sdk::register_pipeline;
use tree_sitter::{Language, Node, Parser};

extern "C" {
    fn tree_sitter_javascript() -> Language;
}

fn language_javascript() -> Language {
    unsafe { tree_sitter_javascript() }
}

#[derive(Default)]
struct JavaScriptGrammar {
    parser: Option<Parser>,
}

impl PipelinePlugin for JavaScriptGrammar {
    fn name(&self) -> String {
        "grammar-javascript".into()
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
                .set_language(&language_javascript())
                .map_err(|e| format!("failed to set language: {}", e))?;
            self.parser = Some(parser);
        }
        let parser = self.parser.as_mut().unwrap();

        let tree = parser
            .parse(source.as_bytes(), None)
            .ok_or("tree-sitter parse failed")?;

        let extracted = extract_js_symbols(&tree.root_node(), source.as_bytes(), path);

        serde_json::to_string(&extracted).map_err(|e| format!("serialize error: {}", e))
    }
}

register_pipeline!(JavaScriptGrammar);

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
// Extraction logic — ported from src/commands/scrape/code/languages/javascript.rs
// =========================================================================

fn extract_js_symbols(node: &Node, source: &[u8], file_path: &str) -> ExtractedData {
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
    extract_calls(node, source, file_path, current_function, data);

    match node.kind() {
        "function_declaration" | "function" | "arrow_function"
        | "generator_function_declaration" => {
            if let Some(name) = extract_function_name(node, source) {
                process_function(node, source, file_path, &name, data);
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    walk_node(&child, source, file_path, data, Some(&name));
                }
                return;
            }
        }
        "method_definition" => {
            if let Some(name) = get_field_text(node, "name", source) {
                process_method(node, source, file_path, &name, data);
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    walk_node(&child, source, file_path, data, Some(&name));
                }
                return;
            }
        }
        "class_declaration" => {
            if let Some(name) = get_field_text(node, "name", source) {
                process_class(node, source, file_path, &name, data);

                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        if child.kind() == "method_definition" {
                            if let Some(method_name) = get_field_text(&child, "name", source) {
                                let full_name = format!("{}.{}", name, method_name);
                                process_method(&child, source, file_path, &full_name, data);
                                let mut mc = child.walk();
                                for mchild in child.children(&mut mc) {
                                    walk_node(
                                        &mchild,
                                        source,
                                        file_path,
                                        data,
                                        Some(&full_name),
                                    );
                                }
                            }
                        } else {
                            walk_node(&child, source, file_path, data, current_function);
                        }
                    }
                }
                return;
            }
        }
        "lexical_declaration" | "variable_declaration" => {
            process_variable_declaration(node, source, file_path, data, current_function);
            // Walk children with correct current_function for variable-assigned functions
            let mut decl_cursor = node.walk();
            for decl_child in node.children(&mut decl_cursor) {
                if decl_child.kind() == "variable_declarator" {
                    if let Some(name_node) = decl_child.child_by_field_name("name") {
                        if let Some(value_node) = decl_child.child_by_field_name("value") {
                            if matches!(value_node.kind(), "arrow_function" | "function") {
                                if let Ok(fn_name) = name_node.utf8_text(source) {
                                    // Recurse into function body with the variable name as caller
                                    let mut body_cursor = value_node.walk();
                                    for body_child in value_node.children(&mut body_cursor) {
                                        walk_node(
                                            &body_child,
                                            source,
                                            file_path,
                                            data,
                                            Some(fn_name),
                                        );
                                    }
                                    continue;
                                }
                            }
                        }
                    }
                }
                walk_node(&decl_child, source, file_path, data, current_function);
            }
            return;
        }
        "import_statement" => {
            process_es6_import(node, source, file_path, data);
        }
        "call_expression" => {
            if is_require_call(node, source) {
                process_commonjs_require(node, source, file_path, data);
            }
        }
        "export_statement" => {
            if let Some(decl) = node.child_by_field_name("declaration") {
                walk_node(&decl, source, file_path, data, current_function);
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

fn process_function(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_async = has_async_keyword(node, source);
    let is_generator = node.kind() == "generator_function_declaration";
    let params = extract_parameters(node, source);

    data.functions.push(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: false,
        takes_mut_params: false,
        returns_result: false,
        returns_option: false,
        is_async,
        is_unsafe: false,
        is_public: true,
        parameter_count: params.len() as i32,
        generic_count: 0,
        parameters: params,
        return_type: None,
    });

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: if is_generator { "generator" } else { "function" }.into(),
        line: node.start_position().row + 1,
        context: first_line(node, source),
    });
}

// ---- Method extraction ----

fn process_method(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_async = has_async_keyword(node, source);
    let is_static = has_static_keyword(node, source);
    let is_getter = node
        .child_by_field_name("kind")
        .and_then(|n| n.utf8_text(source).ok())
        == Some("get");
    let is_setter = node
        .child_by_field_name("kind")
        .and_then(|n| n.utf8_text(source).ok())
        == Some("set");

    let params = extract_parameters(node, source);

    data.functions.push(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: !is_static,
        takes_mut_params: false,
        returns_result: false,
        returns_option: false,
        is_async,
        is_unsafe: false,
        is_public: true,
        parameter_count: params.len() as i32,
        generic_count: 0,
        parameters: params,
        return_type: None,
    });

    let kind = if is_getter {
        "getter"
    } else if is_setter {
        "setter"
    } else if is_static {
        "static_method"
    } else {
        "method"
    };

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: kind.into(),
        line: node.start_position().row + 1,
        context: first_line(node, source),
    });
}

// ---- Class extraction ----

fn process_class(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let definition = first_line(node, source);

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: definition.clone(),
        kind: "class".into(),
        visibility: "public".into(),
        usage_count: 0,
    });

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "class".into(),
        line: node.start_position().row + 1,
        context: definition,
    });

    // Extract inheritance
    if let Some(heritage) = node.child_by_field_name("heritage") {
        if let Ok(parent_name) = heritage.utf8_text(source) {
            let parent = parent_name
                .trim()
                .strip_prefix("extends ")
                .unwrap_or(parent_name.trim());
            if !parent.is_empty() {
                data.constants.push(ConstantFact {
                    file: file_path.to_string(),
                    name: format!("{}::extends::{}", name, parent),
                    value: Some(parent.to_string()),
                    const_type: "inheritance".into(),
                    scope: name.to_string(),
                    line: heritage.start_position().row + 1,
                });
            }
        }
    }

    // Extract class members
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "field_definition" | "public_field_definition" => {
                    if let Some(prop_name) = child.child_by_field_name("property") {
                        if let Ok(field_name) = prop_name.utf8_text(source) {
                            let is_static = has_static_keyword(&child, source);
                            let mut modifiers = Vec::new();
                            if is_static {
                                modifiers.push("static".to_string());
                            }

                            data.members.push(MemberFact {
                                file: file_path.to_string(),
                                container: name.to_string(),
                                name: field_name.to_string(),
                                member_type: "field".into(),
                                visibility: "public".into(),
                                modifiers,
                                line: child.start_position().row + 1,
                            });
                        }
                    }
                }
                "method_definition" => {
                    if let Some(method_name) = get_field_text(&child, "name", source) {
                        let is_static = has_static_keyword(&child, source);
                        let is_async = has_async_keyword(&child, source);
                        let is_getter = child
                            .child_by_field_name("kind")
                            .and_then(|n| n.utf8_text(source).ok())
                            == Some("get");
                        let is_setter = child
                            .child_by_field_name("kind")
                            .and_then(|n| n.utf8_text(source).ok())
                            == Some("set");

                        let mut modifiers = Vec::new();
                        if is_static {
                            modifiers.push("static".to_string());
                        }
                        if is_async {
                            modifiers.push("async".to_string());
                        }

                        let member_type = if method_name == "constructor" {
                            "constructor"
                        } else if is_getter {
                            "getter"
                        } else if is_setter {
                            "setter"
                        } else {
                            "method"
                        };

                        let visibility = if method_name.starts_with('#') {
                            "private"
                        } else {
                            "public"
                        };

                        data.members.push(MemberFact {
                            file: file_path.to_string(),
                            container: name.to_string(),
                            name: method_name,
                            member_type: member_type.into(),
                            visibility: visibility.into(),
                            modifiers,
                            line: child.start_position().row + 1,
                        });
                    }
                }
                _ => {}
            }
        }
    }
}

// ---- Variable declaration extraction ----

fn process_variable_declaration(
    node: &Node,
    source: &[u8],
    file_path: &str,
    data: &mut ExtractedData,
    current_function: Option<&str>,
) {
    let is_const = node.kind() == "lexical_declaration"
        && node
            .utf8_text(source)
            .is_ok_and(|text| text.starts_with("const "));

    let is_module_level = current_function.is_none();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            if let Some(name_node) = child.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source) {
                    if let Some(value_node) = child.child_by_field_name("value") {
                        match value_node.kind() {
                            "arrow_function" | "function" => {
                                process_function(&value_node, source, file_path, name, data);
                            }
                            "class" => {
                                process_class(&value_node, source, file_path, name, data);
                            }
                            _ => {
                                if is_const && is_module_level {
                                    let value =
                                        value_node.utf8_text(source).ok().map(|s| s.to_string());
                                    let is_upper_case = name
                                        .chars()
                                        .all(|c| c.is_uppercase() || c == '_' || c.is_numeric());

                                    data.constants.push(ConstantFact {
                                        file: file_path.to_string(),
                                        name: name.to_string(),
                                        value,
                                        const_type: if is_upper_case { "const" } else { "variable" }
                                            .into(),
                                        scope: "module".into(),
                                        line: child.start_position().row + 1,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---- Import extraction ----

fn process_es6_import(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    let module_path = node
        .children(&mut node.walk())
        .find(|n| n.kind() == "string")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.trim_matches(|c: char| c == '"' || c == '\'' || c == '`'))
        .unwrap_or("");

    let mut imported_names = Vec::new();

    if let Some(import_clause) = node.child_by_field_name("import") {
        let mut cursor = import_clause.walk();
        for child in import_clause.children(&mut cursor) {
            match child.kind() {
                "named_imports" => {
                    let mut import_cursor = child.walk();
                    for import_child in child.children(&mut import_cursor) {
                        if import_child.kind() == "import_specifier" {
                            if let Some(name_node) = import_child.child_by_field_name("name") {
                                if let Ok(name) = name_node.utf8_text(source) {
                                    imported_names.push(name.to_string());
                                }
                            }
                        }
                    }
                }
                "identifier" => {
                    if let Ok(name) = child.utf8_text(source) {
                        imported_names.push(name.to_string());
                    }
                }
                "namespace_import" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if let Ok(name) = name_node.utf8_text(source) {
                            imported_names.push(format!("* as {}", name));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if imported_names.is_empty() && !module_path.is_empty() {
        imported_names.push("*".to_string());
    }

    let is_external = !module_path.starts_with('.') && !module_path.starts_with('/');

    data.imports.push(ImportFact {
        file: file_path.to_string(),
        import_path: module_path.to_string(),
        imported_names,
        import_kind: if is_external { "external" } else { "internal" }.into(),
        line_number: (node.start_position().row + 1) as i32,
    });

    // Add import as searchable symbol
    let import_text = node.utf8_text(source).unwrap_or("").to_string();
    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: import_text.clone(),
        kind: "import".into(),
        line: node.start_position().row + 1,
        context: import_text,
    });
}

fn process_commonjs_require(
    node: &Node,
    source: &[u8],
    file_path: &str,
    data: &mut ExtractedData,
) {
    if let Some(args_node) = node.child_by_field_name("arguments") {
        let mut cursor = args_node.walk();
        for child in args_node.children(&mut cursor) {
            if child.kind() == "string" {
                if let Ok(raw) = child.utf8_text(source) {
                    let module_path = raw.trim_matches(|c: char| c == '"' || c == '\'' || c == '`');
                    let is_external =
                        !module_path.starts_with('.') && !module_path.starts_with('/');

                    data.imports.push(ImportFact {
                        file: file_path.to_string(),
                        import_path: module_path.to_string(),
                        imported_names: vec!["*".to_string()],
                        import_kind: if is_external { "external" } else { "internal" }.into(),
                        line_number: (node.start_position().row + 1) as i32,
                    });

                    // Add require as searchable symbol
                    let context = node.utf8_text(source).unwrap_or("").to_string();
                    data.symbols.push(CodeSymbol {
                        path: file_path.to_string(),
                        name: format!("require('{}')", module_path),
                        kind: "require".into(),
                        line: node.start_position().row + 1,
                        context,
                    });
                    break;
                }
            }
        }
    }
}

// ---- Call extraction ----

fn extract_calls(
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
                        if callee != "require" {
                            let call_type = if node
                                .parent()
                                .is_some_and(|p| p.kind() == "await_expression")
                            {
                                "async"
                            } else {
                                "direct"
                            };

                            data.call_edges.push(CallEdge {
                                caller: caller.to_string(),
                                callee: callee.to_string(),
                                file: file_path.to_string(),
                                call_type: call_type.into(),
                                line_number,
                            });
                        }
                    }
                }
            }
        }
        "await_expression" => {
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
                                    call_type: "async".into(),
                                    line_number,
                                });
                            }
                        }
                    }
                }
            }
        }
        "new_expression" => {
            if let Some(caller) = current_function {
                if let Some(constructor_node) = node.child_by_field_name("constructor") {
                    if let Ok(callee) = constructor_node.utf8_text(source) {
                        data.call_edges.push(CallEdge {
                            caller: caller.to_string(),
                            callee: format!("new {}", callee),
                            file: file_path.to_string(),
                            call_type: "constructor".into(),
                            line_number,
                        });
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

fn extract_function_name(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source).ok())
        .map(String::from)
}

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

fn extract_parameters(node: &Node, source: &[u8]) -> Vec<String> {
    let params_node = node
        .child_by_field_name("parameters")
        .or_else(|| node.child_by_field_name("parameter"));

    if let Some(params) = params_node {
        let mut result = Vec::new();
        let mut cursor = params.walk();
        for child in params.children(&mut cursor) {
            if matches!(
                child.kind(),
                "identifier" | "rest_pattern" | "object_pattern" | "array_pattern"
            ) {
                if let Ok(param_text) = child.utf8_text(source) {
                    result.push(param_text.to_string());
                }
            }
        }
        result
    } else {
        Vec::new()
    }
}

fn has_async_keyword(node: &Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "async" {
            return true;
        }
    }
    node.utf8_text(source)
        .is_ok_and(|text| text.starts_with("async "))
}

fn has_static_keyword(node: &Node, _source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "static" {
            return true;
        }
    }
    false
}

fn is_require_call(node: &Node, source: &[u8]) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    node.child_by_field_name("function")
        .and_then(|f| f.utf8_text(source).ok())
        == Some("require")
}
