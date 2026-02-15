//! TypeScript grammar extraction plugin for Patina pipeline.
//!
//! Parses TypeScript/TSX source code using tree-sitter-typescript (compiled to WASM via wasi-sdk)
//! and returns ExtractedData JSON matching the host's contract.
//!
//! DUAL PARSER: Uses tree_sitter_typescript for .ts files, tree_sitter_tsx for .tsx files.
//! The host sends the file extension in the path, and we pick the correct parser.

use patina_sdk::pipeline::{parse_request, PipelinePlugin};
use patina_sdk::register_pipeline;
use tree_sitter::{Language, Node, Parser};

extern "C" {
    fn tree_sitter_typescript() -> Language;
    fn tree_sitter_tsx() -> Language;
}

fn language_typescript() -> Language {
    unsafe { tree_sitter_typescript() }
}

fn language_tsx() -> Language {
    unsafe { tree_sitter_tsx() }
}

#[derive(Default)]
struct TypeScriptGrammar {
    ts_parser: Option<Parser>,
    tsx_parser: Option<Parser>,
}

impl PipelinePlugin for TypeScriptGrammar {
    fn name(&self) -> String {
        "grammar-typescript".into()
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

        // Pick parser based on file extension
        let is_tsx = path.ends_with(".tsx");

        let parser = if is_tsx {
            if self.tsx_parser.is_none() {
                let mut parser = Parser::new();
                parser
                    .set_language(&language_tsx())
                    .map_err(|e| format!("failed to set tsx language: {}", e))?;
                self.tsx_parser = Some(parser);
            }
            self.tsx_parser.as_mut().unwrap()
        } else {
            if self.ts_parser.is_none() {
                let mut parser = Parser::new();
                parser
                    .set_language(&language_typescript())
                    .map_err(|e| format!("failed to set typescript language: {}", e))?;
                self.ts_parser = Some(parser);
            }
            self.ts_parser.as_mut().unwrap()
        };

        let tree = parser
            .parse(source.as_bytes(), None)
            .ok_or("tree-sitter parse failed")?;

        let extracted = extract_ts_symbols(&tree.root_node(), source.as_bytes(), path);

        serde_json::to_string(&extracted).map_err(|e| format!("serialize error: {}", e))
    }
}

register_pipeline!(TypeScriptGrammar);

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
// Extraction logic — ported from src/commands/scrape/code/languages/typescript.rs
// =========================================================================

fn extract_ts_symbols(node: &Node, source: &[u8], file_path: &str) -> ExtractedData {
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
        "function_declaration" | "function_expression" | "arrow_function"
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
        "method_definition" | "method_signature" => {
            if let Some(name) = get_field_text(node, "name", source) {
                process_method(node, source, file_path, &name, data);
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        walk_node(&child, source, file_path, data, Some(&name));
                    }
                }
                return;
            }
        }
        "class_declaration" | "class_expression" => {
            if let Some(name) = get_field_text(node, "name", source) {
                process_class(node, source, file_path, &name, data);

                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        if matches!(child.kind(), "method_definition" | "method_signature") {
                            if let Some(method_name) = get_field_text(&child, "name", source) {
                                let full_name = format!("{}.{}", name, method_name);
                                process_method(&child, source, file_path, &full_name, data);
                                if let Some(method_body) = child.child_by_field_name("body") {
                                    let mut mc = method_body.walk();
                                    for mchild in method_body.children(&mut mc) {
                                        walk_node(
                                            &mchild,
                                            source,
                                            file_path,
                                            data,
                                            Some(&full_name),
                                        );
                                    }
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
        "interface_declaration" => {
            if let Some(name) = get_field_text(node, "name", source) {
                process_interface(node, source, file_path, &name, data);
            }
        }
        "type_alias_declaration" => {
            if let Some(name) = get_field_text(node, "name", source) {
                process_type_alias(node, source, file_path, &name, data);
            }
        }
        "enum_declaration" => {
            if let Some(name) = get_field_text(node, "name", source) {
                process_enum(node, source, file_path, &name, data);
            }
        }
        "lexical_declaration" | "variable_declaration" => {
            process_variable_declaration(node, source, file_path, data, current_function);
        }
        "import_statement" => {
            process_import(node, source, file_path, data);
        }
        "export_statement" => {
            if let Some(decl) = node.child_by_field_name("declaration") {
                walk_node(&decl, source, file_path, data, current_function);
            } else if let Some(val) = node.child_by_field_name("value") {
                walk_node(&val, source, file_path, data, current_function);
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
    let is_async = is_async_function(node, source);
    let is_generator = node.kind() == "generator_function_declaration";
    let params = extract_params(node, source);
    let return_type = extract_return_type(node, source);
    let generics = extract_generics(node, source);

    data.functions.push(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: false,
        takes_mut_params: false,
        returns_result: return_type
            .as_ref()
            .is_some_and(|rt| rt.contains("Promise")),
        returns_option: return_type
            .as_ref()
            .is_some_and(|rt| rt.contains("undefined") || rt.contains("null") || rt.contains("?")),
        is_async,
        is_unsafe: false,
        is_public: extract_visibility(node, source),
        parameter_count: params.len() as i32,
        generic_count: count_generics(generics.as_deref()),
        parameters: params,
        return_type,
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
    let is_async = is_async_function(node, source);
    let is_static = has_keyword(node, source, "static");
    let is_abstract = has_keyword(node, source, "abstract");
    let is_getter = is_getter_method(node, source);
    let is_setter = is_setter_method(node, source);

    let params = extract_params(node, source);
    let return_type = extract_return_type(node, source);
    let generics = extract_generics(node, source);

    data.functions.push(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: !is_static,
        takes_mut_params: false,
        returns_result: return_type
            .as_ref()
            .is_some_and(|rt| rt.contains("Promise")),
        returns_option: return_type
            .as_ref()
            .is_some_and(|rt| rt.contains("undefined") || rt.contains("null") || rt.contains("?")),
        is_async,
        is_unsafe: false,
        is_public: extract_visibility(node, source),
        parameter_count: params.len() as i32,
        generic_count: count_generics(generics.as_deref()),
        parameters: params,
        return_type,
    });

    let kind = if is_getter {
        "getter"
    } else if is_setter {
        "setter"
    } else if is_abstract {
        "abstract_method"
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
    let is_public = extract_visibility(node, source);
    let is_abstract = has_keyword(node, source, "abstract");
    let definition = first_line(node, source);

    // Extract inheritance
    if let Some(heritage) = node.child_by_field_name("heritage") {
        let mut cursor = heritage.walk();
        for child in heritage.children(&mut cursor) {
            if child.kind() == "extends_clause" {
                let mut ec = child.walk();
                for ec_child in child.children(&mut ec) {
                    if ec_child.kind() == "expression" || ec_child.kind() == "identifier" {
                        if let Ok(parent_name) = ec_child.utf8_text(source) {
                            if parent_name != "extends" {
                                data.constants.push(ConstantFact {
                                    file: file_path.to_string(),
                                    name: format!("{}::extends::{}", name, parent_name),
                                    value: Some(parent_name.to_string()),
                                    const_type: "inheritance".into(),
                                    scope: name.to_string(),
                                    line: child.start_position().row + 1,
                                });
                            }
                        }
                    }
                }
            } else if child.kind() == "implements_clause" {
                let mut ic = child.walk();
                for ic_child in child.children(&mut ic) {
                    if ic_child.kind() == "type" || ic_child.kind() == "identifier" {
                        if let Ok(iface_name) = ic_child.utf8_text(source) {
                            if iface_name != "implements" && !iface_name.contains(',') {
                                data.constants.push(ConstantFact {
                                    file: file_path.to_string(),
                                    name: format!("{}::implements::{}", name, iface_name),
                                    value: Some(iface_name.to_string()),
                                    const_type: "implements".into(),
                                    scope: name.to_string(),
                                    line: child.start_position().row + 1,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Extract class members
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "property_signature" | "public_field_definition" => {
                    if let Some(prop_name) = child.child_by_field_name("name") {
                        if let Ok(field_name) = prop_name.utf8_text(source) {
                            let visibility = extract_member_visibility(&child, source);
                            let is_static = has_keyword(&child, source, "static");
                            let is_readonly = has_keyword(&child, source, "readonly");

                            let mut modifiers = Vec::new();
                            if is_static {
                                modifiers.push("static".to_string());
                            }
                            if is_readonly {
                                modifiers.push("readonly".to_string());
                            }

                            data.members.push(MemberFact {
                                file: file_path.to_string(),
                                container: name.to_string(),
                                name: field_name.to_string(),
                                member_type: "field".into(),
                                visibility: visibility.into(),
                                modifiers,
                                line: child.start_position().row + 1,
                            });
                        }
                    }
                }
                "method_definition" | "method_signature" => {
                    if let Some(method_name) = get_field_text(&child, "name", source) {
                        let visibility = extract_member_visibility(&child, source);
                        let is_static = has_keyword(&child, source, "static");
                        let is_abstract = has_keyword(&child, source, "abstract");
                        let is_async = is_async_function(&child, source);

                        let mut modifiers = Vec::new();
                        if is_static {
                            modifiers.push("static".to_string());
                        }
                        if is_abstract {
                            modifiers.push("abstract".to_string());
                        }
                        if is_async {
                            modifiers.push("async".to_string());
                        }

                        let member_type = if method_name == "constructor" {
                            "constructor"
                        } else if is_getter_method(&child, source) {
                            "getter"
                        } else if is_setter_method(&child, source) {
                            "setter"
                        } else {
                            "method"
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

    let kind = if is_abstract { "abstract_class" } else { "class" };

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: definition.clone(),
        kind: kind.into(),
        visibility: if is_public { "public" } else { "private" }.into(),
        usage_count: 0,
    });

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: kind.into(),
        line: node.start_position().row + 1,
        context: definition,
    });
}

// ---- Interface extraction ----

fn process_interface(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = extract_visibility(node, source);
    let definition = first_line(node, source);

    // Extract interface members
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "property_signature" | "method_signature" => {
                    if let Some(member_name) = child.child_by_field_name("name") {
                        if let Ok(field_name) = member_name.utf8_text(source) {
                            let is_optional = child.child_by_field_name("optional").is_some();
                            let is_readonly = has_keyword(&child, source, "readonly");

                            let mut modifiers = Vec::new();
                            if is_optional {
                                modifiers.push("optional".to_string());
                            }
                            if is_readonly {
                                modifiers.push("readonly".to_string());
                            }

                            let member_type = if child.kind() == "method_signature" {
                                "method"
                            } else {
                                "property"
                            };

                            data.members.push(MemberFact {
                                file: file_path.to_string(),
                                container: name.to_string(),
                                name: field_name.to_string(),
                                member_type: member_type.into(),
                                visibility: "public".into(),
                                modifiers,
                                line: child.start_position().row + 1,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Extract interface extensions
    if let Some(extends) = node.child_by_field_name("extends") {
        let mut cursor = extends.walk();
        for child in extends.children(&mut cursor) {
            if child.kind() == "type" || child.kind() == "identifier" {
                if let Ok(parent_name) = child.utf8_text(source) {
                    if parent_name != "extends" {
                        data.constants.push(ConstantFact {
                            file: file_path.to_string(),
                            name: format!("{}::extends::{}", name, parent_name),
                            value: Some(parent_name.to_string()),
                            const_type: "extends".into(),
                            scope: name.to_string(),
                            line: child.start_position().row + 1,
                        });
                    }
                }
            }
        }
    }

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: definition.clone(),
        kind: "interface".into(),
        visibility: if is_public { "public" } else { "private" }.into(),
        usage_count: 0,
    });

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "interface".into(),
        line: node.start_position().row + 1,
        context: definition,
    });
}

// ---- Type alias extraction ----

fn process_type_alias(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = extract_visibility(node, source);
    let definition = first_line(node, source);

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: definition.clone(),
        kind: "type_alias".into(),
        visibility: if is_public { "public" } else { "private" }.into(),
        usage_count: 0,
    });

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "type_alias".into(),
        line: node.start_position().row + 1,
        context: definition,
    });
}

// ---- Enum extraction ----

fn process_enum(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = extract_visibility(node, source);
    let is_const = has_keyword(node, source, "const");
    let definition = first_line(node, source);

    // Extract enum members
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "enum_member" || child.kind() == "property_identifier" {
                if let Some(member_name_node) = child.child_by_field_name("name") {
                    if let Ok(member_name) = member_name_node.utf8_text(source) {
                        let value = child
                            .child_by_field_name("value")
                            .and_then(|v| v.utf8_text(source).ok())
                            .map(String::from);

                        data.constants.push(ConstantFact {
                            file: file_path.to_string(),
                            name: format!("{}.{}", name, member_name),
                            value,
                            const_type: "enum_member".into(),
                            scope: name.to_string(),
                            line: child.start_position().row + 1,
                        });
                    }
                } else if child.kind() == "property_identifier" {
                    if let Ok(member_name) = child.utf8_text(source) {
                        data.constants.push(ConstantFact {
                            file: file_path.to_string(),
                            name: format!("{}.{}", name, member_name),
                            value: None,
                            const_type: "enum_member".into(),
                            scope: name.to_string(),
                            line: child.start_position().row + 1,
                        });
                    }
                }
            }
        }
    }

    let kind = if is_const { "const_enum" } else { "enum" };

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: definition.clone(),
        kind: kind.into(),
        visibility: if is_public { "public" } else { "private" }.into(),
        usage_count: 0,
    });

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: kind.into(),
        line: node.start_position().row + 1,
        context: definition,
    });
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
                            "arrow_function" | "function_expression" => {
                                process_function(&value_node, source, file_path, name, data);
                            }
                            "class_expression" => {
                                process_class(&value_node, source, file_path, name, data);
                            }
                            _ => {
                                if is_const && is_module_level {
                                    let value =
                                        value_node.utf8_text(source).ok().map(String::from);
                                    let const_type = if name
                                        .chars()
                                        .all(|c| c.is_uppercase() || c == '_' || c.is_numeric())
                                    {
                                        "constant"
                                    } else {
                                        "const_variable"
                                    };

                                    data.constants.push(ConstantFact {
                                        file: file_path.to_string(),
                                        name: name.to_string(),
                                        value,
                                        const_type: const_type.into(),
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

fn process_import(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    let import_text = node.utf8_text(source).unwrap_or("");

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
                    let mut ic = child.walk();
                    for import_child in child.children(&mut ic) {
                        if import_child.kind() == "import_specifier" {
                            if let Some(name_node) = import_child.child_by_field_name("name") {
                                if let Ok(name) = name_node.utf8_text(source) {
                                    if let Some(alias_node) =
                                        import_child.child_by_field_name("alias")
                                    {
                                        if let Ok(alias) = alias_node.utf8_text(source) {
                                            imported_names
                                                .push(format!("{} as {}", name, alias));
                                        } else {
                                            imported_names.push(name.to_string());
                                        }
                                    } else {
                                        imported_names.push(name.to_string());
                                    }
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

    let is_type_import = import_text.starts_with("import type");

    if imported_names.is_empty() && !module_path.is_empty() {
        imported_names.push("*".to_string());
    }

    let is_external = !module_path.starts_with('.') && !module_path.starts_with('/');

    let import_kind = if is_type_import {
        "type_import"
    } else if is_external {
        "external"
    } else {
        "internal"
    };

    data.imports.push(ImportFact {
        file: file_path.to_string(),
        import_path: module_path.to_string(),
        imported_names,
        import_kind: import_kind.into(),
        line_number: (node.start_position().row + 1) as i32,
    });
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
                        let call_type =
                            if node.parent().is_some_and(|p| p.kind() == "await_expression") {
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
        "decorator" => {
            if let Ok(decorator_text) = node.utf8_text(source) {
                let decorator_name = decorator_text.trim_start_matches('@');
                data.constants.push(ConstantFact {
                    file: file_path.to_string(),
                    name: format!("@{}", decorator_name),
                    value: Some(decorator_text.to_string()),
                    const_type: "decorator".into(),
                    scope: current_function.unwrap_or("module").to_string(),
                    line: line_number as usize,
                });

                if let Some(caller) = current_function {
                    data.call_edges.push(CallEdge {
                        caller: caller.to_string(),
                        callee: decorator_text.to_string(),
                        file: file_path.to_string(),
                        call_type: "decorator".into(),
                        line_number,
                    });
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
    if let Some(name_node) = node.child_by_field_name("name") {
        return name_node.utf8_text(source).ok().map(String::from);
    }
    // For anonymous functions assigned to variables
    if let Some(parent) = node.parent() {
        if parent.kind() == "variable_declarator" {
            if let Some(name_node) = parent.child_by_field_name("name") {
                return name_node.utf8_text(source).ok().map(String::from);
            }
        }
    }
    None
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

fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
    let params_node = node
        .child_by_field_name("parameters")
        .or_else(|| node.child_by_field_name("parameter"));

    if let Some(params) = params_node {
        let mut result = Vec::new();
        let mut cursor = params.walk();
        for child in params.children(&mut cursor) {
            match child.kind() {
                "required_parameter" | "optional_parameter" | "identifier" | "rest_pattern"
                | "object_pattern" | "array_pattern" => {
                    if let Ok(param_text) = child.utf8_text(source) {
                        result.push(param_text.to_string());
                    }
                }
                _ => {}
            }
        }
        result
    } else {
        Vec::new()
    }
}

fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("return_type")
        .and_then(|rt| rt.child_by_field_name("type"))
        .and_then(|t| t.utf8_text(source).ok())
        .map(String::from)
}

fn extract_generics(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("type_parameters")
        .and_then(|tp| tp.utf8_text(source).ok())
        .map(String::from)
}

fn count_generics(generics: Option<&str>) -> i32 {
    generics.map_or(0, |g| {
        if g.contains('<') && g.contains('>') {
            g.matches(',').count() as i32 + 1
        } else {
            0
        }
    })
}

fn extract_visibility(node: &Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Ok(text) = child.utf8_text(source) {
            match text {
                "public" | "export" => return true,
                "private" | "protected" => return false,
                _ => {}
            }
        }
    }
    true
}

fn extract_member_visibility<'a>(node: &Node, source: &[u8]) -> &'a str {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Ok(text) = child.utf8_text(source) {
            match text {
                "public" => return "public",
                "private" => return "private",
                "protected" => return "protected",
                _ => {}
            }
        }
    }
    "public"
}

fn is_async_function(node: &Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "async" {
            return true;
        }
    }
    node.utf8_text(source)
        .is_ok_and(|text| text.starts_with("async "))
}

fn has_keyword(node: &Node, source: &[u8], keyword: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Ok(text) = child.utf8_text(source) {
            if text == keyword {
                return true;
            }
        }
    }
    false
}

fn is_getter_method(node: &Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Ok(text) = child.utf8_text(source) {
            if text == "get" {
                return true;
            }
        }
    }
    false
}

fn is_setter_method(node: &Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Ok(text) = child.utf8_text(source) {
            if text == "set" {
                return true;
            }
        }
    }
    false
}
