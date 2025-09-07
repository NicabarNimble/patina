// ============================================================================
// CAIRO LANGUAGE MODULE - Special Non-Tree-Sitter Parser
// ============================================================================
// Cairo uses cairo-lang-parser instead of tree-sitter, requiring special handling.
// This module provides direct symbol extraction from Cairo's parsed AST.

use anyhow::Result;

/// Cairo processor for extracting symbols without tree-sitter
pub struct CairoProcessor;

impl CairoProcessor {
    /// Process a Cairo file and extract all symbols to SQL statements
    pub fn process_file(
        file_path: &str,
        content: &str,
    ) -> Result<(Vec<String>, usize, usize, usize)> {
        let symbols = patina_metal::cairo::parse_cairo(content, file_path)?;
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
            sql_statements.push(format!(
                "INSERT OR REPLACE INTO function_facts (file, name, takes_mut_self, takes_mut_params, returns_result, returns_option, is_async, is_unsafe, is_public, parameter_count, generic_count, parameters, return_type) VALUES ('{}', '{}', 0, 0, {}, {}, 0, 0, {}, {}, 0, '{}', '{}');",
                escape_sql(file_path),
                escape_sql(&func.name),
                if func.return_type.as_deref().unwrap_or("").contains("Result") { 1 } else { 0 },
                if func.return_type.as_deref().unwrap_or("").contains("Option") { 1 } else { 0 },
                if func.is_public { 1 } else { 0 },
                func.parameters.len(),
                escape_sql(&func.parameters.join(", ")),
                escape_sql(&func.return_type.as_deref().unwrap_or(""))
            ));
            
            // Also insert into code_search for consistency - match the actual schema
            sql_statements.push(format!(
                "INSERT OR REPLACE INTO code_search (path, name, signature, context) VALUES ('{}', '{}', '{}', '{}');",
                escape_sql(file_path),
                escape_sql(&func.name),
                escape_sql(&signature),
                escape_sql("") // Context not available from cairo parser
            ));
            
            functions += 1;
        }
        
        // Extract structs as types
        for s in symbols.structs {
            let definition = if s.fields.is_empty() {
                format!("struct {} {{}}", s.name)
            } else {
                format!("struct {} {{ {} }}", s.name, s.fields.join(", "))
            };
            
            sql_statements.push(format!(
                "INSERT OR REPLACE INTO type_vocabulary (file, name, definition, kind, visibility) VALUES ('{}', '{}', '{}', 'struct', '{}');",
                escape_sql(file_path),
                escape_sql(&s.name),
                escape_sql(&definition),
                if s.is_public { "pub" } else { "private" }
            ));
            types += 1;
        }
        
        // Extract traits as types
        for t in symbols.traits {
            sql_statements.push(format!(
                "INSERT OR REPLACE INTO type_vocabulary (file, name, definition, kind, visibility) VALUES ('{}', '{}', 'trait {}', 'trait', '{}');",
                escape_sql(file_path),
                escape_sql(&t.name),
                escape_sql(&t.name),
                if t.is_public { "pub" } else { "private" }
            ));
            types += 1;
        }
        
        // Extract imports
        for imp in symbols.imports {
            // Determine if import is external (not relative)
            let is_external = !imp.path.starts_with("super::") && !imp.path.starts_with("self::");
            let imported_item = imp.path.split("::").last().unwrap_or(&imp.path);
            
            sql_statements.push(format!(
                "INSERT OR REPLACE INTO import_facts (importer_file, imported_item, imported_from, is_external, import_kind) VALUES ('{}', '{}', '{}', {}, 'use');",
                escape_sql(file_path),
                escape_sql(imported_item),
                escape_sql(&imp.path),
                if is_external { 1 } else { 0 }
            ));
            imports += 1;
        }
        
        // Extract modules as types (they define a namespace)
        for m in symbols.modules {
            sql_statements.push(format!(
                "INSERT OR REPLACE INTO type_vocabulary (file, name, definition, kind, visibility) VALUES ('{}', '{}', 'mod {}', 'module', '{}');",
                escape_sql(file_path),
                escape_sql(&m.name),
                escape_sql(&m.name),
                if m.is_public { "pub" } else { "private" }
            ));
            types += 1;
        }
        
        // Note: We could also extract impls for call graph analysis in the future
        // For now, we're focusing on the main symbols
        
        Ok((sql_statements, functions, types, imports))
    }
}

/// Escape SQL special characters
fn escape_sql(s: &str) -> String {
    s.replace('\'', "''")
        .replace('\\', "\\\\")
        .replace('\n', " ")
        .replace('\r', " ")
        .replace('\t', " ")
}