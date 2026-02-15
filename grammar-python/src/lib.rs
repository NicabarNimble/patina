//! Python grammar extraction plugin for Patina pipeline.
//!
//! Parses Python source code using tree-sitter-python (compiled to WASM via wasi-sdk)
//! and returns ExtractedData JSON matching the host's contract.

use patina_sdk::pipeline::{parse_request, PipelinePlugin};
use patina_sdk::register_pipeline;
use tree_sitter::{Language, Node, Parser};

extern "C" {
    fn tree_sitter_python() -> Language;
}

fn language_python() -> Language {
    unsafe { tree_sitter_python() }
}

#[derive(Default)]
struct PythonGrammar {
    parser: Option<Parser>,
}

impl PipelinePlugin for PythonGrammar {
    fn name(&self) -> String {
        "grammar-python".into()
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
                .set_language(&language_python())
                .map_err(|e| format!("failed to set language: {}", e))?;
            self.parser = Some(parser);
        }
        let parser = self.parser.as_mut().unwrap();

        let tree = parser
            .parse(source.as_bytes(), None)
            .ok_or("tree-sitter parse failed")?;

        let extracted = extract_python_symbols(&tree.root_node(), source.as_bytes(), path);

        serde_json::to_string(&extracted).map_err(|e| format!("serialize error: {}", e))
    }
}

register_pipeline!(PythonGrammar);

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
// Extraction logic — ported from src/commands/scrape/code/languages/python.rs
// =========================================================================

fn extract_python_symbols(node: &Node, source: &[u8], file_path: &str) -> ExtractedData {
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

    // Handle decorated definitions specially
    if node.kind() == "decorated_definition" {
        process_decorated_definition(node, source, file_path, data, current_function);
        return;
    }

    match node.kind() {
        "function_definition" | "async_function_definition" => {
            if let Some(name) = get_field_text(node, "name", source) {
                process_function(node, source, file_path, &name, data);
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    walk_node(&child, source, file_path, data, Some(&name));
                }
                return;
            }
        }
        "class_definition" => {
            if let Some(name) = get_field_text(node, "name", source) {
                process_class(node, source, file_path, &name, data);

                // Process class body with methods getting class-qualified names
                if let Some(body_node) = node.child_by_field_name("body") {
                    let mut cursor = body_node.walk();
                    for child in body_node.children(&mut cursor) {
                        if matches!(
                            child.kind(),
                            "function_definition" | "async_function_definition"
                        ) {
                            if let Some(method_name) = get_field_text(&child, "name", source) {
                                let full_name = format!("{}.{}", name, method_name);
                                walk_node(&child, source, file_path, data, Some(&full_name));
                            }
                        } else {
                            walk_node(&child, source, file_path, data, current_function);
                        }
                    }
                }
                return;
            }
        }
        "import_statement" | "import_from_statement" => {
            process_import(node, source, file_path, data);
        }
        "assignment" => {
            if is_module_level(node) {
                process_module_assignment(node, source, file_path, data);
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(&child, source, file_path, data, current_function);
    }
}

// ---- Decorated definitions ----

fn process_decorated_definition(
    node: &Node,
    source: &[u8],
    file_path: &str,
    data: &mut ExtractedData,
    current_function: Option<&str>,
) {
    // Extract decorator call edges
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "decorator" {
            if let Ok(decorator_text) = child.utf8_text(source) {
                let decorator_name = decorator_text.trim_start_matches('@');
                if let Some(caller) = current_function {
                    data.call_edges.push(CallEdge {
                        caller: caller.to_string(),
                        callee: format!("@{}", decorator_name),
                        file: file_path.to_string(),
                        call_type: "decorator".into(),
                        line_number: (child.start_position().row + 1) as i32,
                    });
                }
            }
        }
    }

    // Process the actual definition
    if let Some(definition) = node.child_by_field_name("definition") {
        match definition.kind() {
            "function_definition" | "async_function_definition" => {
                if let Some(name) = get_field_text(&definition, "name", source) {
                    process_function(&definition, source, file_path, &name, data);
                }
            }
            "class_definition" => {
                if let Some(name) = get_field_text(&definition, "name", source) {
                    process_class(&definition, source, file_path, &name, data);
                }
            }
            _ => {}
        }
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
    let is_public = !name.starts_with('_');
    let is_async = node.kind() == "async_function_definition";
    let params = extract_params(node, source);
    let return_type = extract_return_type(node, source);

    let takes_mut_self = params.iter().any(|p| p == "self");
    let returns_result = return_type
        .as_ref()
        .is_some_and(|rt| rt.contains("Result") || rt.contains("Union") || rt.contains("Optional"));
    let returns_option = return_type
        .as_ref()
        .is_some_and(|rt| rt.contains("Optional") || rt.contains("None"));

    data.functions.push(FunctionFact {
        file: file_path.to_string(),
        name: name.to_string(),
        takes_mut_self,
        takes_mut_params: false,
        returns_result,
        returns_option,
        is_async,
        is_unsafe: false,
        is_public,
        parameter_count: params.len() as i32,
        generic_count: 0,
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

// ---- Class extraction ----

fn process_class(
    node: &Node,
    source: &[u8],
    file_path: &str,
    name: &str,
    data: &mut ExtractedData,
) {
    let is_public = !name.starts_with('_');
    let definition = first_line(node, source);

    // Extract inheritance
    if let Some(superclasses_node) = node.child_by_field_name("superclasses") {
        let mut cursor = superclasses_node.walk();
        for child in superclasses_node.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "attribute" {
                if let Ok(parent_name) = child.utf8_text(source) {
                    let cleaned = parent_name.trim();
                    if !cleaned.is_empty() {
                        data.constants.push(ConstantFact {
                            file: file_path.to_string(),
                            name: format!("{}::inherits_from::{}", name, cleaned),
                            value: Some(cleaned.to_string()),
                            const_type: "inheritance".into(),
                            scope: name.to_string(),
                            line: superclasses_node.start_position().row + 1,
                        });
                    }
                }
            }
        }
    }

    // Extract class members
    if let Some(body_node) = node.child_by_field_name("body") {
        let mut cursor = body_node.walk();
        for child in body_node.children(&mut cursor) {
            match child.kind() {
                "assignment" => {
                    if let Some(left_node) = child.child_by_field_name("left") {
                        if let Ok(var_name) = left_node.utf8_text(source) {
                            let value = child
                                .child_by_field_name("right")
                                .and_then(|v| v.utf8_text(source).ok())
                                .map(|s| s.trim().to_string());

                            data.constants.push(ConstantFact {
                                file: file_path.to_string(),
                                name: var_name.trim().to_string(),
                                value,
                                const_type: "class_variable".into(),
                                scope: name.to_string(),
                                line: child.start_position().row + 1,
                            });
                        }
                    }
                }
                "function_definition" | "async_function_definition" => {
                    if let Some(method_name) = get_field_text(&child, "name", source) {
                        let visibility =
                            if method_name.starts_with("__") && method_name.ends_with("__") {
                                "special"
                            } else if method_name.starts_with('_') {
                                "private"
                            } else {
                                "public"
                            };

                        let mut modifiers = Vec::new();
                        if child.kind() == "async_function_definition" {
                            modifiers.push("async".to_string());
                        }

                        let member_type = if method_name == "__init__" {
                            "constructor"
                        } else if method_name == "__del__" {
                            "destructor"
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

    data.types.push(TypeFact {
        file: file_path.to_string(),
        name: name.to_string(),
        definition: definition.clone(),
        kind: "class".into(),
        visibility: if is_public { "public" } else { "private" }.into(),
        usage_count: 0,
    });

    data.symbols.push(CodeSymbol {
        path: file_path.to_string(),
        name: name.to_string(),
        kind: "class".into(),
        line: node.start_position().row + 1,
        context: definition,
    });
}

// ---- Import extraction ----

fn process_import(node: &Node, source: &[u8], file_path: &str, data: &mut ExtractedData) {
    if let Ok(import_text) = node.utf8_text(source) {
        match node.kind() {
            "import_statement" => {
                let clean = import_text.trim_start_matches("import ").trim();
                let module_name = clean.split(" as ").next().unwrap_or(clean);
                let is_external = !module_name.starts_with('.');

                data.imports.push(ImportFact {
                    file: file_path.to_string(),
                    import_path: module_name.to_string(),
                    imported_names: vec![module_name.to_string()],
                    import_kind: if is_external { "external" } else { "internal" }.into(),
                    line_number: (node.start_position().row + 1) as i32,
                });
            }
            "import_from_statement" => {
                if let Some(module_node) = node.child_by_field_name("module_name") {
                    if let Ok(module_name) = module_node.utf8_text(source) {
                        let is_external = !import_text.contains("from .");
                        let items = if import_text.contains("import *") {
                            "*".to_string()
                        } else if let Some(idx) = import_text.find("import ") {
                            import_text[idx + 7..].trim().to_string()
                        } else {
                            module_name.to_string()
                        };

                        data.imports.push(ImportFact {
                            file: file_path.to_string(),
                            import_path: module_name.to_string(),
                            imported_names: vec![items],
                            import_kind: if is_external { "external" } else { "internal" }.into(),
                            line_number: (node.start_position().row + 1) as i32,
                        });
                    }
                }
            }
            _ => {}
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
        "call" => {
            if let Some(caller) = current_function {
                if let Some(func_node) = node.child_by_field_name("function") {
                    if let Ok(callee) = func_node.utf8_text(source) {
                        let (call_type, callee_name) = if callee.starts_with("await ") {
                            ("async", callee.strip_prefix("await ").unwrap_or(callee))
                        } else {
                            ("direct", callee)
                        };

                        data.call_edges.push(CallEdge {
                            caller: caller.to_string(),
                            callee: callee_name.to_string(),
                            file: file_path.to_string(),
                            call_type: call_type.into(),
                            line_number,
                        });
                    }
                }
            }
        }
        "await" => {
            if let Some(caller) = current_function {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "call" {
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
        _ => {}
    }
}

// ---- Module-level assignment ----

fn process_module_assignment(
    node: &Node,
    source: &[u8],
    file_path: &str,
    data: &mut ExtractedData,
) {
    if let Some(left_node) = node.child_by_field_name("left") {
        if let Ok(var_name) = left_node.utf8_text(source) {
            let name = var_name.trim();
            let value = node
                .child_by_field_name("right")
                .and_then(|v| v.utf8_text(source).ok())
                .map(|s| s.trim().to_string());

            let const_type = if name
                .chars()
                .all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())
            {
                "module_constant"
            } else {
                "module_variable"
            };

            data.constants.push(ConstantFact {
                file: file_path.to_string(),
                name: name.to_string(),
                value,
                const_type: const_type.into(),
                scope: "module".into(),
                line: node.start_position().row + 1,
            });
        }
    }
}

// =========================================================================
// Helpers
// =========================================================================

fn is_module_level(node: &Node) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "function_definition" | "async_function_definition" | "class_definition" => {
                return false;
            }
            "module" => return true,
            _ => current = parent.parent(),
        }
    }
    true
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
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut params = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if matches!(child.kind(), "," | "(" | ")") {
                continue;
            }
            if let Ok(param_text) = child.utf8_text(source) {
                let cleaned = param_text.trim();
                if !cleaned.is_empty() && cleaned != "self" && cleaned != "cls" {
                    params.push(cleaned.to_string());
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
        .and_then(|rt| rt.utf8_text(source).ok())
        .map(|s| s.trim_start_matches("->").trim().to_string())
}
