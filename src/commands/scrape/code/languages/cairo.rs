// ============================================================================
// CAIRO LANGUAGE PROCESSOR V2 - STRUCT-BASED
// ============================================================================
//! Cairo language processor that returns typed structs instead of SQL strings.
//!
//! This is the refactored version that:
//! - Returns ExtractedData with typed structs
//! - No SQL string generation
//! - Direct data extraction to domain types
//!
//! Cairo is unique - it uses cairo-lang-parser instead of tree-sitter,
//! requiring special handling but the same output format.

use crate::commands::scrape::code::database::{
    CodeSymbol, FunctionFact, ImportFact, TypeFact,
};
use crate::commands::scrape::code::extracted_data::ExtractedData;
use crate::commands::scrape::code::types::FilePath;
use anyhow::Result;

/// Cairo processor for extracting symbols without tree-sitter
pub struct CairoProcessor;

impl CairoProcessor {
    /// Process a Cairo file and extract all symbols to typed structs
    pub fn process_file(file_path: FilePath, content: &str) -> Result<ExtractedData> {
        let mut data = ExtractedData::new();
        let symbols = patina_metal::cairo::parse_cairo(content, file_path.as_str())?;

        // Extract functions with full details
        for func in symbols.functions {
            // Build signature from parameters
            let signature = if func.parameters.is_empty() {
                format!("fn {}()", func.name)
            } else {
                format!("fn {}({})", func.name, func.parameters.join(", "))
            };

            // Create FunctionFact struct
            let function_fact = FunctionFact {
                file: file_path.as_str().to_string(),
                name: func.name.clone(),
                takes_mut_self: false,
                takes_mut_params: false,
                returns_result: func.return_type.as_deref().unwrap_or("").contains("Result"),
                returns_option: func.return_type.as_deref().unwrap_or("").contains("Option"),
                is_async: false,
                is_unsafe: false,
                is_public: func.is_public,
                parameter_count: func.parameters.len() as i32,
                generic_count: 0,
                parameters: func.parameters.clone(),
                return_type: func.return_type.clone(),
            };
            data.add_function(function_fact);

            // Create CodeSymbol for search
            let code_symbol = CodeSymbol {
                path: file_path.as_str().to_string(),
                name: func.name,
                kind: "function".to_string(),
                line: 0, // Line number not available from cairo parser
                context: signature,
            };
            data.add_symbol(code_symbol);
        }

        // Extract structs as types and their fields as members
        for s in symbols.structs {
            let definition = if s.fields.is_empty() {
                format!("struct {} {{}}", s.name)
            } else {
                format!("struct {} {{ {} }}", s.name, s.fields.join(", "))
            };

            let type_fact = TypeFact {
                file: file_path.as_str().to_string(),
                name: s.name.clone(),
                definition,
                kind: "struct".to_string(),
                visibility: if s.is_public { "pub" } else { "private" }.to_string(),
                usage_count: 0,
            };
            data.add_type(type_fact);

            // Extract struct fields as MemberFacts
            for field in s.fields.iter() {
                use crate::commands::scrape::code::extracted_data::MemberFact;

                data.members.push(MemberFact {
                    file: file_path.as_str().to_string(),
                    container: s.name.clone(),
                    name: field.clone(),
                    member_type: "field".to_string(),
                    visibility: if s.is_public { "pub".to_string() } else { "private".to_string() },
                    modifiers: vec![],
                    line: s.start_line,
                });
            }
        }

        // Extract traits as types
        for t in symbols.traits {
            let type_fact = TypeFact {
                file: file_path.as_str().to_string(),
                name: t.name.clone(),
                definition: format!("trait {}", t.name),
                kind: "trait".to_string(),
                visibility: if t.is_public { "pub" } else { "private" }.to_string(),
                usage_count: 0,
            };
            data.add_type(type_fact);
        }

        // Extract imports
        for imp in symbols.imports {
            let imported_item = imp.path.split("::").last().unwrap_or(&imp.path).to_string();

            let import_fact = ImportFact {
                file: file_path.as_str().to_string(),
                import_path: imp.path,
                imported_names: vec![imported_item],
                import_kind: "use".to_string(),
                line_number: 0, // Line number not available from cairo parser
            };
            data.add_import(import_fact);
        }

        // Extract modules as types (they define a namespace)
        for m in symbols.modules {
            let type_fact = TypeFact {
                file: file_path.as_str().to_string(),
                name: m.name.clone(),
                definition: format!("mod {}", m.name),
                kind: "module".to_string(),
                visibility: if m.is_public { "pub" } else { "private" }.to_string(),
                usage_count: 0,
            };
            data.add_type(type_fact);
        }

        // Extract trait implementations as ConstantFacts
        // Following the same pattern as Rust: impl Trait for Type
        for impl_sym in symbols.impls {
            use crate::commands::scrape::code::extracted_data::ConstantFact;

            if let Some(trait_name) = impl_sym.trait_name {
                let trait_clean = trait_name.trim();
                let type_clean = impl_sym.type_name.trim();

                // Skip empty trait names (happens with inherent impls)
                if !trait_clean.is_empty() && trait_clean != "EmptyTraitPath" {
                    // Store as ConstantFact: TypeName::implements::TraitName
                    data.constants.push(ConstantFact {
                        file: file_path.as_str().to_string(),
                        name: format!("{}::implements::{}", type_clean, trait_clean),
                        value: None,
                        const_type: "trait_impl".to_string(),
                        scope: type_clean.to_string(),
                        line: impl_sym.start_line,
                    });

                    // Also add as searchable symbol
                    data.add_symbol(CodeSymbol {
                        path: file_path.as_str().to_string(),
                        name: format!("{} impl {}", type_clean, trait_clean),
                        kind: "impl".to_string(),
                        line: impl_sym.start_line,
                        context: format!("impl {} for {}", trait_clean, type_clean),
                    });
                }
            }
        }

        Ok(data)
    }
}
