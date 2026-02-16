//! Cairo grammar extraction plugin for Patina pipeline.
//!
//! Parses Cairo source code using cairo-lang-parser and returns
//! ExtractedData JSON matching the host's contract.
//!
//! This is a pure Rust plugin — no tree-sitter, no C toolchain, no wasi-sdk.
//! Proves the end-to-end pipeline plugin pattern for grammar extraction.

mod parser;

use patina_sdk::pipeline::{parse_request, PipelinePlugin};
use patina_sdk::register_pipeline;

#[derive(Default)]
struct CairoGrammar {
    parser: Option<parser::CairoParser>,
}

impl PipelinePlugin for CairoGrammar {
    fn name(&self) -> String {
        "grammar-cairo".into()
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
            self.parser = Some(parser::CairoParser::new());
        }
        let parser = self.parser.as_mut().unwrap();

        let symbols = parser
            .parse(source)
            .map_err(|e| format!("cairo parse error: {}", e))?;

        let extracted = convert_to_extracted_data(&symbols, path);

        serde_json::to_string(&extracted).map_err(|e| format!("serialize error: {}", e))
    }
}

register_pipeline!(CairoGrammar);

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
// Conversion: CairoSymbols → ExtractedData
// =========================================================================
// Ported from src/commands/scrape/code/languages/cairo.rs (CairoProcessor)

fn convert_to_extracted_data(symbols: &parser::CairoSymbols, file_path: &str) -> ExtractedData {
    let mut data = ExtractedData {
        symbols: Vec::new(),
        functions: Vec::new(),
        types: Vec::new(),
        imports: Vec::new(),
        call_edges: Vec::new(),
        constants: Vec::new(),
        members: Vec::new(),
    };

    // Functions
    for func in &symbols.functions {
        let signature = if func.parameters.is_empty() {
            format!("fn {}()", func.name)
        } else {
            format!("fn {}({})", func.name, func.parameters.join(", "))
        };

        data.functions.push(FunctionFact {
            file: file_path.to_string(),
            name: func.name.clone(),
            takes_mut_self: false,
            takes_mut_params: false,
            returns_result: func
                .return_type
                .as_deref()
                .unwrap_or("")
                .contains("Result"),
            returns_option: func
                .return_type
                .as_deref()
                .unwrap_or("")
                .contains("Option"),
            is_async: false,
            is_unsafe: false,
            is_public: func.is_public,
            parameter_count: func.parameters.len() as i32,
            generic_count: 0,
            parameters: func.parameters.clone(),
            return_type: func.return_type.clone(),
        });

        data.symbols.push(CodeSymbol {
            path: file_path.to_string(),
            name: func.name.clone(),
            kind: "function".into(),
            line: func.start_line,
            context: signature,
        });
    }

    // Structs → types + members
    for s in &symbols.structs {
        let definition = if s.fields.is_empty() {
            format!("struct {} {{}}", s.name)
        } else {
            format!("struct {} {{ {} }}", s.name, s.fields.join(", "))
        };

        data.types.push(TypeFact {
            file: file_path.to_string(),
            name: s.name.clone(),
            definition,
            kind: "struct".into(),
            visibility: if s.is_public { "pub" } else { "private" }.into(),
            usage_count: 0,
        });

        for field in &s.fields {
            data.members.push(MemberFact {
                file: file_path.to_string(),
                container: s.name.clone(),
                name: field.clone(),
                member_type: "field".into(),
                visibility: if s.is_public { "pub" } else { "private" }.into(),
                modifiers: vec![],
                line: s.start_line,
            });
        }
    }

    // Traits → types
    for t in &symbols.traits {
        data.types.push(TypeFact {
            file: file_path.to_string(),
            name: t.name.clone(),
            definition: format!("trait {}", t.name),
            kind: "trait".into(),
            visibility: if t.is_public { "pub" } else { "private" }.into(),
            usage_count: 0,
        });
    }

    // Imports
    for imp in &symbols.imports {
        let imported_item = imp
            .path
            .split("::")
            .last()
            .unwrap_or(&imp.path)
            .to_string();

        data.imports.push(ImportFact {
            file: file_path.to_string(),
            import_path: imp.path.clone(),
            imported_names: vec![imported_item],
            import_kind: "use".into(),
            line_number: imp.line as i32,
        });
    }

    // Modules → types
    for m in &symbols.modules {
        data.types.push(TypeFact {
            file: file_path.to_string(),
            name: m.name.clone(),
            definition: format!("mod {}", m.name),
            kind: "module".into(),
            visibility: if m.is_public { "pub" } else { "private" }.into(),
            usage_count: 0,
        });
    }

    // Trait impls → constants + symbols
    for impl_sym in &symbols.impls {
        if let Some(ref trait_name) = impl_sym.trait_name {
            let trait_clean = trait_name.trim();
            let type_clean = impl_sym.type_name.trim();

            if !trait_clean.is_empty() && trait_clean != "EmptyTraitPath" {
                data.constants.push(ConstantFact {
                    file: file_path.to_string(),
                    name: format!("{}::implements::{}", type_clean, trait_clean),
                    value: None,
                    const_type: "trait_impl".into(),
                    scope: type_clean.to_string(),
                    line: impl_sym.start_line,
                });

                data.symbols.push(CodeSymbol {
                    path: file_path.to_string(),
                    name: format!("{} impl {}", type_clean, trait_clean),
                    kind: "impl".into(),
                    line: impl_sym.start_line,
                    context: format!("impl {} for {}", trait_clean, type_clean),
                });
            }
        }
    }

    data
}
