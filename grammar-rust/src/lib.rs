//! Rust grammar extraction plugin for Patina pipeline.
//!
//! Parses Rust source code using tree-sitter-rust (compiled to WASM via wasi-sdk)
//! and returns ExtractedData JSON matching the host's contract.
//!
//! This is the Phase 2 proof: tree-sitter C grammar compiled to wasm32-wasip2.

use patina_sdk::pipeline::{parse_request, PipelinePlugin};
use patina_sdk::register_pipeline;
use tree_sitter::{Language, Parser};

extern "C" {
    fn tree_sitter_rust() -> Language;
}

fn language_rust() -> Language {
    unsafe { tree_sitter_rust() }
}

#[derive(Default)]
struct RustGrammar {
    parser: Option<Parser>,
}

impl PipelinePlugin for RustGrammar {
    fn name(&self) -> String {
        "grammar-rust".into()
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

        // Lazy-init parser (reuse across calls within same instantiation)
        if self.parser.is_none() {
            let mut parser = Parser::new();
            parser
                .set_language(&language_rust())
                .map_err(|e| format!("failed to set language: {}", e))?;
            self.parser = Some(parser);
        }
        let parser = self.parser.as_mut().unwrap();

        let tree = parser
            .parse(source.as_bytes(), None)
            .ok_or("tree-sitter parse failed")?;

        let extracted = extract_rust_symbols(&tree.root_node(), source.as_bytes(), path);

        serde_json::to_string(&extracted).map_err(|e| format!("serialize error: {}", e))
    }
}

register_pipeline!(RustGrammar);

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
// Extraction logic — ported from src/commands/scrape/code/languages/rust.rs
// =========================================================================

use tree_sitter::Node;

fn extract_rust_symbols(node: &Node, source: &[u8], file_path: &str) -> ExtractedData {
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
        "function_item" => {
            if let Some(name) = get_node_field_text(node, "name", source) {
                process_function(node, source, file_path, &name, data);
                extract_calls(node, source, file_path, Some(&name), data);
            }
        }
        "const_item" | "static_item" => {
            if let Some(name) = get_node_field_text(node, "name", source) {
                process_constant(node, source, file_path, &name, data);
            }
        }
        "struct_item" => {
            if let Some(name) = get_node_field_text(node, "name", source) {
                process_type_with_members(node, source, file_path, &name, "struct", data);
            }
        }
        "enum_item" => {
            if let Some(name) = get_node_field_text(node, "name", source) {
                process_type_with_members(node, source, file_path, &name, "enum", data);
            }
        }
        "trait_item" => {
            if let Some(name) = get_node_field_text(node, "name", source) {
                process_type(node, source, file_path, &name, "trait", data);
            }
        }
        "type_alias" => {
            if let Some(name) = get_node_field_text(node, "name", source) {
                process_type(node, source, file_path, &name, "type_alias", data);
            }
        }
        "impl_item" => {
            // process_impl handles its own recursion into body children
            process_impl(node, source, file_path, data);
            return;
        }
        "mod_item" => {
            if let Some(name) = get_node_field_text(node, "name", source) {
                process_type(node, source, file_path, &name, "module", data);
            }
        }
        "use_declaration" => {
            process_import(node, source, file_path, data);
        }
        "macro_definition" => {
            if let Some(name) = get_node_field_text(node, "name", source) {
                process_macro(node, source, file_path, &name, data);
            }
        }
        _ => {}
    }

    // Recurse into children (skipped for impl_item which handles its own)
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
    let params = extract_params(node, source);
    let return_type = extract_return_type(node, source);
    let is_public = has_visibility_modifier(node);
    let is_async = has_modifier(node, "async");
    let is_unsafe = has_modifier(node, "unsafe");

    let takes_mut_self = params.iter().any(|p| p.contains("&mut self"));
    let takes_mut_params = params
        .iter()
        .any(|p| p.contains("&mut ") && !p.contains("self"));
    let returns_result = return_type.as_deref().unwrap_or("").contains("Result");
    let returns_option = return_type.as_deref().unwrap_or("").contains("Option");

    let generic_count = node
        .child_by_field_name("type_parameters")
        .map(|n| n.named_child_count() as i32)
        .unwrap_or(0);

    data.functions.push(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self,
        takes_mut_params,
        returns_result,
        returns_option,
        is_async,
        is_unsafe,
        is_public,
        parameter_count: params.len() as i32,
        generic_count,
        parameters: params,
        return_type,
    });

    let context = node
        .utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .unwrap_or("")
        .to_string();

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "function".into(),
        line: node.start_position().row + 1,
        context,
    });
}

// ---- Type extraction ----

fn process_type(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: &str,
    data: &mut ExtractedData,
) {
    let is_public = has_visibility_modifier(node);
    let definition = first_line(node, source);

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: definition.clone(),
        kind: kind.to_string(),
        visibility: if is_public { "pub" } else { "private" }.into(),
        usage_count: 0,
    });

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: kind.to_string(),
        line: node.start_position().row + 1,
        context: definition,
    });
}

fn process_type_with_members(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    kind: &str,
    data: &mut ExtractedData,
) {
    process_type(node, source, file_path, name, kind, data);

    if let Some(body) = node.child_by_field_name("body") {
        if kind == "struct" {
            extract_struct_fields(&body, source, file_path, name, data);
        } else if kind == "enum" {
            extract_enum_variants(&body, source, file_path, name, data);
        }
    }
}

fn extract_struct_fields(
    body: &Node,
    source: &[u8],
    file_path: &str,
    struct_name: &str,
    data: &mut ExtractedData,
) {
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "field_declaration" {
            if let Some(field_name) = get_node_field_text(&child, "name", source) {
                let visibility = if has_visibility_modifier(&child) {
                    "pub"
                } else {
                    "private"
                };

                let field_type = child
                    .child_by_field_name("type")
                    .and_then(|t| t.utf8_text(source).ok())
                    .map(|s| s.to_string());

                let mut modifiers = Vec::new();
                if let Some(ref ft) = field_type {
                    modifiers.push(ft.clone());
                }

                data.members.push(MemberFact {
                    file: file_path.to_string(),
                    container: struct_name.to_string(),
                    name: field_name,
                    member_type: "field".into(),
                    visibility: visibility.into(),
                    modifiers,
                    line: child.start_position().row + 1,
                });
            }
        }
    }
}

fn extract_enum_variants(
    body: &Node,
    source: &[u8],
    file_path: &str,
    enum_name: &str,
    data: &mut ExtractedData,
) {
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if child.kind() == "enum_variant" {
            if let Some(variant_name) = get_node_field_text(&child, "name", source) {
                let value = child
                    .child_by_field_name("value")
                    .and_then(|v| v.utf8_text(source).ok())
                    .map(|s| s.to_string());

                data.constants.push(ConstantFact {
                    file: file_path.to_string(),
                    name: format!("{}::{}", enum_name, variant_name),
                    value,
                    const_type: "enum_variant".into(),
                    scope: enum_name.to_string(),
                    line: child.start_position().row + 1,
                });

                let mut modifiers = Vec::new();
                if let Some(variant_body) = child.child_by_field_name("body") {
                    if variant_body.kind() == "field_declaration_list" {
                        modifiers.push("struct_variant".to_string());
                    } else if variant_body.kind() == "ordered_field_declaration_list" {
                        modifiers.push("tuple_variant".to_string());
                    }
                }

                data.members.push(MemberFact {
                    file: file_path.to_string(),
                    container: enum_name.to_string(),
                    name: variant_name,
                    member_type: "variant".into(),
                    visibility: "pub".into(),
                    modifiers,
                    line: child.start_position().row + 1,
                });
            }
        }
    }
}

// ---- Constant extraction ----

fn process_constant(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let value = node
        .child_by_field_name("value")
        .and_then(|v| v.utf8_text(source).ok())
        .map(|s| s.to_string());

    let const_type = if node.kind() == "static_item" {
        "static"
    } else {
        "const"
    };
    let is_public = has_visibility_modifier(node);
    let visibility = if is_public { "pub" } else { "private" };

    data.constants.push(ConstantFact {
        file: file_path.to_string(),
        name: name.to_string(),
        value,
        const_type: const_type.into(),
        scope: "module".into(),
        line: node.start_position().row + 1,
    });

    let definition = first_line(node, source);

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: definition.clone(),
        kind: const_type.into(),
        visibility: visibility.into(),
        usage_count: 0,
    });

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: const_type.into(),
        line: node.start_position().row + 1,
        context: definition,
    });
}

// ---- Impl extraction ----

fn process_impl(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    // Trait implementations
    if let Some(trait_node) = node.child_by_field_name("trait") {
        if let Ok(trait_name) = trait_node.utf8_text(source) {
            if let Some(type_node) = node.child_by_field_name("type") {
                if let Ok(type_name) = type_node.utf8_text(source) {
                    data.constants.push(ConstantFact {
                        file: file_path.to_string(),
                        name: format!("impl {} for {}", trait_name, type_name),
                        value: None,
                        const_type: "trait_impl".into(),
                        scope: "module".into(),
                        line: node.start_position().row + 1,
                    });
                }
            }
        }
    }

    // Process methods within the impl block
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            walk_node(&child, source, file_path, data, None);
        }
    }
}

// ---- Import extraction ----

fn process_import(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    if let Some(use_tree) = node.child_by_field_name("argument") {
        if let Ok(import_text) = use_tree.utf8_text(source) {
            let import_path = import_text.trim();

            let imported_names = if import_path.contains('{') {
                if let (Some(start), Some(end)) = (import_path.find('{'), import_path.find('}')) {
                    import_path[start + 1..end]
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect()
                } else {
                    vec![import_path.to_string()]
                }
            } else if let Some(last_part) = import_path.split("::").last() {
                vec![last_part.to_string()]
            } else {
                vec![import_path.to_string()]
            };

            data.imports.push(ImportFact {
                file: file_path.to_string(),
                import_path: import_path.to_string(),
                imported_names,
                import_kind: "use".into(),
                line_number: (node.start_position().row + 1) as i32,
            });

            data.symbols.push(CodeSymbol {
                path: file_path.to_string(),
                name: import_path.to_string(),
                kind: "import".into(),
                line: node.start_position().row + 1,
                context: format!("use {};", import_path),
            });
        }
    }
}

// ---- Macro extraction ----

fn process_macro(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let definition = first_line(node, source);
    let macro_name = format!("{}!", name);

    data.constants.push(ConstantFact {
        file: file_path.to_string(),
        name: macro_name.clone(),
        value: Some(definition.clone()),
        const_type: "macro_definition".into(),
        scope: "module".into(),
        line: node.start_position().row + 1,
    });

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: macro_name.clone(),
        definition: definition.clone(),
        kind: "macro".into(),
        visibility: "pub".into(),
        usage_count: 0,
    });

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: macro_name,
        kind: "macro".into(),
        line: node.start_position().row + 1,
        context: definition,
    });
}

// ---- Call graph extraction ----

fn extract_calls(
    node: &Node,
    source: &[u8],
    file_path: &str,
    current_function: Option<&str>,
    data: &mut ExtractedData,
) {
    let Some(caller) = current_function else {
        return;
    };

    match node.kind() {
        "call_expression" => {
            if let Some(function_node) = node.child_by_field_name("function") {
                if let Ok(callee) = function_node.utf8_text(source) {
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
        "macro_invocation" => {
            if let Some(macro_node) = node.child_by_field_name("macro") {
                if let Ok(macro_name) = macro_node.utf8_text(source) {
                    data.call_edges.push(CallEdge {
                        caller: caller.to_string(),
                        callee: format!("{}!", macro_name),
                        file: file_path.to_string(),
                        call_type: "macro".into(),
                        line_number: (node.start_position().row + 1) as i32,
                    });
                }
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_calls(&child, source, file_path, current_function, data);
    }
}

// =========================================================================
// Helpers
// =========================================================================

fn get_node_field_text(node: &Node, field: &str, source: &[u8]) -> Option<String> {
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

fn has_visibility_modifier(node: &Node) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            return true;
        }
    }
    false
}

fn has_modifier(node: &Node, modifier: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == modifier {
            return true;
        }
    }
    false
}

fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut params = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if child.kind() == "parameter" || child.kind() == "self_parameter" {
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

fn extract_return_type(node: &Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("return_type")
        .and_then(|n| n.utf8_text(source).ok())
        .map(|s| s.trim_start_matches("->").trim().to_string())
}
