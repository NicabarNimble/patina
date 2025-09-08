// ============================================================================
// CAIRO LANGUAGE MODULE - Special Non-Tree-Sitter Parser
// ============================================================================
// Cairo uses cairo-lang-parser instead of tree-sitter, requiring special handling.
// This module provides direct symbol extraction from Cairo's parsed AST.

use anyhow::Result;
use crate::commands::scrape::recode_v2::types::FilePath;
use crate::commands::scrape::recode_v2::sql_builder::{InsertBuilder, TableName};

/// Cairo processor for extracting symbols without tree-sitter
pub struct CairoProcessor;

impl CairoProcessor {
    /// Process a Cairo file and extract all symbols to SQL statements
    pub fn process_file(
        file_path: FilePath,
        content: &str,
    ) -> Result<(Vec<String>, usize, usize, usize)> {
        let symbols = patina_metal::cairo::parse_cairo(content, file_path.as_str())?;
        let mut sql_statements = Vec::new();
        let mut functions = 0;
        let mut types = 0;
        let mut imports = 0;

        // Extract functions with full details
        for func in symbols.functions {
            // Build signature from parameters
            let signature = if func.parameters.is_empty() {
                format!("fn {}()", func.name)
            } else {
                format!("fn {}({})", func.name, func.parameters.join(", "))
            };

            // Insert into function_facts - match the actual schema
            let insert_sql = InsertBuilder::new(TableName::FUNCTION_FACTS)
                .or_replace()
                .value("file", file_path.as_str())
                .value("name", func.name.as_str())
                .value("takes_mut_self", false)
                .value("takes_mut_params", false)
                .value("returns_result", func.return_type.as_deref().unwrap_or("").contains("Result"))
                .value("returns_option", func.return_type.as_deref().unwrap_or("").contains("Option"))
                .value("is_async", false)
                .value("is_unsafe", false)
                .value("is_public", func.is_public)
                .value("parameter_count", func.parameters.len() as i64)
                .value("generic_count", 0i64)
                .value("parameters", func.parameters.join(", "))
                .value("return_type", func.return_type.as_deref().unwrap_or(""))
                .build();
            sql_statements.push(format!("{};\n", insert_sql));

            // Also insert into code_search for consistency - match the actual schema
            let search_sql = InsertBuilder::new(TableName::CODE_SEARCH)
                .or_replace()
                .value("path", file_path.as_str())
                .value("name", func.name.as_str())
                .value("signature", signature)
                .value("context", "") // Context not available from cairo parser
                .build();
            sql_statements.push(format!("{};\n", search_sql));

            functions += 1;
        }

        // Extract structs as types
        for s in symbols.structs {
            let definition = if s.fields.is_empty() {
                format!("struct {} {{}}", s.name)
            } else {
                format!("struct {} {{ {} }}", s.name, s.fields.join(", "))
            };

            let type_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                .or_replace()
                .value("file", file_path.as_str())
                .value("name", s.name.as_str())
                .value("definition", definition)
                .value("kind", "struct")
                .value("visibility", if s.is_public { "pub" } else { "private" })
                .build();
            sql_statements.push(format!("{};\n", type_sql));
            types += 1;
        }

        // Extract traits as types
        for t in symbols.traits {
            let type_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                .or_replace()
                .value("file", file_path.as_str())
                .value("name", t.name.as_str())
                .value("definition", format!("trait {}", t.name))
                .value("kind", "trait")
                .value("visibility", if t.is_public { "pub" } else { "private" })
                .build();
            sql_statements.push(format!("{};\n", type_sql));
            types += 1;
        }

        // Extract imports
        for imp in symbols.imports {
            // Determine if import is external (not relative)
            let is_external = !imp.path.starts_with("super::") && !imp.path.starts_with("self::");
            let imported_item = imp.path.split("::").last().unwrap_or(&imp.path);

            let import_sql = InsertBuilder::new(TableName::IMPORT_FACTS)
                .or_replace()
                .value("importer_file", file_path.as_str())
                .value("imported_item", imported_item)
                .value("imported_from", imp.path.as_str())
                .value("is_external", is_external)
                .value("import_kind", "use")
                .build();
            sql_statements.push(format!("{};\n", import_sql));
            imports += 1;
        }

        // Extract modules as types (they define a namespace)
        for m in symbols.modules {
            let type_sql = InsertBuilder::new(TableName::TYPE_VOCABULARY)
                .or_replace()
                .value("file", file_path.as_str())
                .value("name", m.name.as_str())
                .value("definition", format!("mod {}", m.name))
                .value("kind", "module")
                .value("visibility", if m.is_public { "pub" } else { "private" })
                .build();
            sql_statements.push(format!("{};\n", type_sql));
            types += 1;
        }

        // Note: We could also extract impls for call graph analysis in the future
        // For now, we're focusing on the main symbols

        Ok((sql_statements, functions, types, imports))
    }
}

