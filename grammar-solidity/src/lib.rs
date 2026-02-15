//! Solidity grammar extraction plugin for Patina pipeline.
//!
//! Parses Solidity source code using tree-sitter-solidity (compiled to WASM via wasi-sdk)
//! and returns ExtractedData JSON matching the host's contract.

use patina_sdk::pipeline::{parse_request, PipelinePlugin};
use patina_sdk::register_pipeline;
use tree_sitter::{Language, Node, Parser};

extern "C" {
    fn tree_sitter_solidity() -> Language;
}

fn language_solidity() -> Language {
    unsafe { tree_sitter_solidity() }
}

#[derive(Default)]
struct SolidityGrammar {
    parser: Option<Parser>,
}

impl PipelinePlugin for SolidityGrammar {
    fn name(&self) -> String {
        "grammar-solidity".into()
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
                .set_language(&language_solidity())
                .map_err(|e| format!("failed to set language: {}", e))?;
            self.parser = Some(parser);
        }
        let parser = self.parser.as_mut().unwrap();

        let tree = parser
            .parse(source.as_bytes(), None)
            .ok_or("tree-sitter parse failed")?;

        let extracted = extract_solidity_symbols(&tree.root_node(), source.as_bytes(), path);

        serde_json::to_string(&extracted).map_err(|e| format!("serialize error: {}", e))
    }
}

register_pipeline!(SolidityGrammar);

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
// Extraction logic — ported from src/commands/scrape/code/languages/solidity.rs
// =========================================================================

fn extract_solidity_symbols(node: &Node, source: &[u8], file_path: &str) -> ExtractedData {
    let mut data = ExtractedData {
        symbols: Vec::new(),
        functions: Vec::new(),
        types: Vec::new(),
        imports: Vec::new(),
        call_edges: Vec::new(),
        constants: Vec::new(),
        members: Vec::new(),
    };

    walk_node(node, source, file_path, &mut data, None, None);
    data
}

fn walk_node(
    node: &Node,
    source: &[u8],
    file_path: &str,
    data: &mut ExtractedData,
    current_function: Option<&str>,
    current_contract: Option<&str>,
) {
    // Extract calls first
    extract_calls(node, source, file_path, current_function, data);

    match node.kind() {
        "function_definition" | "modifier_definition" => {
            if let Some(name) = get_field_text(node, "name", source) {
                let full_name = if let Some(contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                process_function(node, source, file_path, &full_name, data);

                let owned = full_name.clone();
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    walk_node(
                        &child,
                        source,
                        file_path,
                        data,
                        Some(&owned),
                        current_contract,
                    );
                }
                return;
            }
        }
        "event_definition" => {
            if let Some(name) = get_field_text(node, "name", source) {
                let full_name = if let Some(contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                process_event(node, source, file_path, &full_name, data);
            }
        }
        "contract_declaration" | "library_declaration" | "interface_declaration" => {
            if let Some(name) = get_field_text(node, "name", source) {
                process_contract(node, source, file_path, &name, data);

                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    walk_node(
                        &child,
                        source,
                        file_path,
                        data,
                        current_function,
                        Some(&name),
                    );
                }
                return;
            }
        }
        "struct_declaration" => {
            if let Some(name) = get_field_text(node, "name", source) {
                let full_name = if let Some(contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                process_struct(node, source, file_path, &full_name, data);
            }
        }
        "enum_declaration" => {
            if let Some(name) = get_field_text(node, "name", source) {
                let full_name = if let Some(contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                process_enum(node, source, file_path, &full_name, data);
            }
        }
        "state_variable_declaration" => {
            if let Some(name) = get_field_text(node, "name", source) {
                let full_name = if let Some(contract) = current_contract {
                    format!("{}.{}", contract, name)
                } else {
                    name.clone()
                };
                process_state_variable(node, source, file_path, &full_name, data);
            }
        }
        "import_directive" => {
            process_import(node, source, file_path, data);
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(
            &child,
            source,
            file_path,
            data,
            current_function,
            current_contract,
        );
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
    let visibility = extract_visibility(node, source);
    let is_public = visibility != "private" && visibility != "internal";
    let mutability = extract_mutability(node, source);
    let params = extract_params(node, source);
    let return_type = extract_return_type(node, source);
    let is_unsafe = has_unchecked_block(node);

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: if node.kind() == "modifier_definition" {
            "modifier"
        } else {
            "function"
        }
        .into(),
        line: node.start_position().row + 1,
        context: first_line(node, source),
    });

    data.functions.push(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self: false,
        takes_mut_params: mutability == "payable",
        returns_result: false,
        returns_option: false,
        is_async: false,
        is_unsafe,
        is_public,
        parameter_count: params.len() as i32,
        generic_count: 0,
        parameters: params,
        return_type,
    });
}

// ---- Event extraction ----

fn process_event(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let params = extract_event_params(node, source);

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: format!("event {}", name),
        kind: "event".into(),
        line: node.start_position().row + 1,
        context: first_line(node, source),
    });

    data.functions.push(FunctionFact {
        file: file_path.to_string(),
        name: format!("event {}", name),
        takes_mut_self: false,
        takes_mut_params: false,
        returns_result: false,
        returns_option: false,
        is_async: false,
        is_unsafe: false,
        is_public: true,
        parameter_count: params.len() as i32,
        generic_count: 0,
        parameters: params,
        return_type: None,
    });
}

// ---- Contract extraction ----

fn process_contract(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let type_kind = match node.kind() {
        "contract_declaration" => "contract",
        "library_declaration" => "library",
        "interface_declaration" => "interface",
        _ => "unknown",
    };

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: type_kind.into(),
        line: node.start_position().row + 1,
        context: first_line(node, source),
    });

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: type_definition(node, source),
        kind: type_kind.into(),
        visibility: "public".into(),
        usage_count: 0,
    });

    // Extract inheritance
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "inheritance_specifier" {
            let mut inherit_cursor = child.walk();
            for inherit_child in child.children(&mut inherit_cursor) {
                if matches!(
                    inherit_child.kind(),
                    "type_name" | "user_defined_type" | "identifier"
                ) {
                    if let Ok(base_name) = inherit_child.utf8_text(source) {
                        let base_clean = base_name.trim();
                        data.constants.push(ConstantFact {
                            file: file_path.to_string(),
                            name: format!("{}::inherits::{}", name, base_clean),
                            value: None,
                            const_type: "inheritance".into(),
                            scope: name.to_string(),
                            line: child.start_position().row + 1,
                        });
                        data.symbols.push(CodeSymbol {
                            path: file_path.to_string(),
                            name: format!("{} : {}", name, base_clean),
                            kind: "inheritance".into(),
                            line: child.start_position().row + 1,
                            context: format!(
                                "contract {} inherits from {}",
                                name, base_clean
                            ),
                        });
                    }
                }
            }
        }
    }
}

// ---- Struct extraction ----

fn process_struct(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "struct".into(),
        line: node.start_position().row + 1,
        context: first_line(node, source),
    });

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: type_definition(node, source),
        kind: "struct".into(),
        visibility: "public".into(),
        usage_count: 0,
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
    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "enum".into(),
        line: node.start_position().row + 1,
        context: first_line(node, source),
    });

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: type_definition(node, source),
        kind: "enum".into(),
        visibility: "public".into(),
        usage_count: 0,
    });
}

// ---- State variable extraction ----

fn process_state_variable(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let visibility = extract_visibility(node, source);
    let is_public = visibility == "public" || visibility == "external";
    let var_type = get_field_text(node, "type", source);

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "state_variable".into(),
        line: node.start_position().row + 1,
        context: first_line(node, source),
    });

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: var_type.unwrap_or_else(|| "unknown".to_string()),
        kind: "state_variable".into(),
        visibility: if is_public { "public" } else { "private" }.into(),
        usage_count: 0,
    });
}

// ---- Import extraction ----

fn process_import(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    if let Ok(import_text) = node.utf8_text(source) {
        // Add import as searchable symbol
        data.symbols.push(CodeSymbol {
            path: file_path.to_string(),
            name: import_text.to_string(),
            kind: "import".into(),
            line: node.start_position().row + 1,
            context: import_text.to_string(),
        });

        let import_clean = import_text
            .trim_start_matches("import ")
            .trim_end_matches(';')
            .trim();

        if import_clean.contains(" from ") {
            let parts: Vec<&str> = import_clean.split(" from ").collect();
            if parts.len() == 2 {
                let imported = parts[0].trim().trim_matches('{').trim_matches('}');
                let path = parts[1].trim().trim_matches('"').trim_matches('\'');
                let is_external = !path.starts_with('.');

                data.imports.push(ImportFact {
                    file: file_path.to_string(),
                    import_path: path.to_string(),
                    imported_names: vec![imported.to_string()],
                    import_kind: if is_external { "external" } else { "relative" }.into(),
                    line_number: (node.start_position().row + 1) as i32,
                });
            }
        } else if import_clean.contains(" as ") {
            // Aliased import: import "path" as Alias
            let parts: Vec<&str> = import_clean.split(" as ").collect();
            if parts.len() == 2 {
                let path = parts[0].trim().trim_matches('"').trim_matches('\'');
                let alias = parts[1].trim();
                let is_external = !path.starts_with('.');

                data.imports.push(ImportFact {
                    file: file_path.to_string(),
                    import_path: path.to_string(),
                    imported_names: vec![alias.to_string()],
                    import_kind: if is_external { "external" } else { "relative" }.into(),
                    line_number: (node.start_position().row + 1) as i32,
                });
            }
        } else {
            // Simple import: import "path"
            let path = import_clean.trim_matches('"').trim_matches('\'');
            let is_external = !path.starts_with('.');
            let imported = path.split('/').next_back().unwrap_or(path);

            data.imports.push(ImportFact {
                file: file_path.to_string(),
                import_path: path.to_string(),
                imported_names: vec![imported.to_string()],
                import_kind: if is_external { "external" } else { "relative" }.into(),
                line_number: (node.start_position().row + 1) as i32,
            });
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
        "call_expression" | "function_call" => {
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
        "member_expression" => {
            if let Some(parent) = node.parent() {
                if parent.kind() == "call_expression" || parent.kind() == "function_call" {
                    if let Some(caller) = current_function {
                        if let Some(property) = node.child_by_field_name("property") {
                            if let Ok(callee) = property.utf8_text(source) {
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
        "new_expression" => {
            if let Some(caller) = current_function {
                if let Ok(text) = node.utf8_text(source) {
                    if let Some(contract_name) =
                        text.strip_prefix("new ").and_then(|s| s.split('(').next())
                    {
                        data.call_edges.push(CallEdge {
                            caller: caller.to_string(),
                            callee: format!("new {}", contract_name.trim()),
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

fn extract_visibility(node: &Node, source: &[u8]) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility" {
            if let Ok(vis) = child.utf8_text(source) {
                return vis.to_string();
            }
        }
    }
    "internal".to_string()
}

fn extract_mutability(node: &Node, source: &[u8]) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "state_mutability" {
            if let Ok(text) = child.utf8_text(source) {
                return text.to_string();
            }
        }
    }
    "nonpayable".to_string()
}

fn extract_params(node: &Node, source: &[u8]) -> Vec<String> {
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut params = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if child.kind() == "parameter" {
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

fn extract_event_params(node: &Node, source: &[u8]) -> Vec<String> {
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut params = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if child.kind() == "event_parameter" {
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
    get_field_text(node, "return_type", source).or_else(|| {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "return_parameters" {
                return child.utf8_text(source).ok().map(|s| s.to_string());
            }
        }
        None
    })
}

fn has_unchecked_block(node: &Node) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "unchecked_block" {
            return true;
        }
        if has_unchecked_block(&child) {
            return true;
        }
    }
    false
}
