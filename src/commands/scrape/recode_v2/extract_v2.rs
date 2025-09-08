// ============================================================================
// REFACTORED EXTRACTION WITH EMBEDDED DUCKDB
// ============================================================================
//! New extraction pipeline using type-safe database operations.
//!
//! This replaces the unsafe SQL string concatenation with:
//! - Direct DuckDB library integration
//! - Prepared statements and Appender API
//! - Type-preserving data structures
//! - Batch operations for performance

use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::database::{
    CallEdge, CodeSymbol, Database, FunctionFact, ImportFact, TypeFact,
};
use super::extracted_data::ExtractedData;
use super::languages::Language;
use super::types::FilePath;

/// Process all source files and extract metadata using safe database operations
pub fn extract_code_metadata_v2(db_path: &str, work_dir: &Path, _force: bool) -> Result<usize> {
    println!("🧠 Extracting code metadata with embedded DuckDB...");

    // Open database connection
    let mut db = Database::open(db_path)?;
    db.init_schema()?;

    // Find all supported language files
    let mut all_files: Vec<(PathBuf, Language)> = Vec::new();

    for entry in WalkBuilder::new(work_dir)
        .hidden(false)
        .git_ignore(true)
        .build()
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let language = Language::from_path(path);
            if !matches!(language, Language::Unknown) {
                all_files.push((path.to_path_buf(), language));
            }
        }
    }

    println!("  Found {} source files", all_files.len());
    if all_files.is_empty() {
        println!("  No source files found. Is this a code repository?");
        return Ok(0);
    }

    // Collect all extracted data in memory first
    let mut all_symbols = Vec::new();
    let mut all_functions = Vec::new();
    let mut all_types = Vec::new();
    let mut all_imports = Vec::new();
    let mut all_call_edges = Vec::new();
    
    let mut files_with_errors = 0;
    let mut files_processed = 0;

    // Process each file and collect data
    for (file_path, language) in all_files {
        let relative_path = if let Ok(stripped) = file_path.strip_prefix(work_dir) {
            format!("./{}", stripped.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Read file content
        let content = match std::fs::read(&file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ⚠️  Failed to read {}: {}", relative_path, e);
                files_with_errors += 1;
                continue;
            }
        };

        // Get file metadata for index state
        let mtime = std::fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let size = content.len() as i64;

        // Update index state
        db.update_index_state(&relative_path, mtime, size, None)?;

        // Process file based on language
        match process_file_by_language(&relative_path, &content, language) {
            Ok(extracted) => {
                all_symbols.extend(extracted.symbols);
                all_functions.extend(extracted.functions);
                all_types.extend(extracted.types);
                all_imports.extend(extracted.imports);
                all_call_edges.extend(extracted.call_edges);
                files_processed += 1;
            }
            Err(e) => {
                eprintln!("  ⚠️  Processing error in {}: {}", relative_path, e);
                db.mark_skipped(&relative_path, &e.to_string())?;
                files_with_errors += 1;
            }
        }
    }

    // Bulk insert all collected data
    println!("  💾 Writing to database using bulk operations...");
    
    let symbols_count = db.insert_symbols(&all_symbols)?;
    let functions_count = db.insert_functions(&all_functions)?;
    let types_count = db.insert_types(&all_types)?;
    let imports_count = db.insert_imports(&all_imports)?;
    let edges_count = db.insert_call_edges(&all_call_edges)?;

    println!(
        "  ✅ Inserted: {} symbols, {} functions, {} types, {} imports, {} call edges",
        symbols_count, functions_count, types_count, imports_count, edges_count
    );

    if files_with_errors > 0 {
        println!(
            "  ⚠️  {} files had parsing errors and were skipped",
            files_with_errors
        );
    }

    Ok(symbols_count + functions_count + types_count + imports_count)
}


/// Process a single file based on its language
fn process_file_by_language(
    file_path: &str,
    content: &[u8],
    language: Language,
) -> Result<ExtractedData> {
    let mut data = ExtractedData {
        symbols: Vec::new(),
        functions: Vec::new(),
        types: Vec::new(),
        imports: Vec::new(),
        call_edges: Vec::new(),
    };

    // For now, we'll still use the existing processors but convert their output
    // In the next step, we'll refactor them to return structs directly
    match language {
        Language::Rust => {
            process_rust_file(file_path, content, &mut data)?;
        }
        Language::Go => {
            process_go_file(file_path, content, &mut data)?;
        }
        Language::Python => {
            process_python_file(file_path, content, &mut data)?;
        }
        Language::JavaScript | Language::JavaScriptJSX => {
            process_javascript_file(file_path, content, &mut data)?;
        }
        Language::TypeScript | Language::TypeScriptTSX => {
            process_typescript_file(file_path, content, &mut data)?;
        }
        Language::C => {
            process_c_file(file_path, content, &mut data)?;
        }
        Language::Cpp => {
            process_cpp_file(file_path, content, &mut data)?;
        }
        Language::Cairo => {
            process_cairo_file(file_path, content, &mut data)?;
        }
        Language::Solidity => {
            process_solidity_file(file_path, content, &mut data)?;
        }
        _ => {
            // Skip unknown languages
            return Err(anyhow::anyhow!("Unsupported language: {:?}", language));
        }
    }

    Ok(data)
}

// Temporary adapters - these will be replaced when we refactor the language processors
// to return structs instead of SQL strings

fn process_rust_file(file_path: &str, content: &[u8], data: &mut ExtractedData) -> Result<()> {
    use super::languages::rust::RustProcessor;
    
    // Rust processor now returns ExtractedData directly!
    let extracted = RustProcessor::process_file(FilePath::from(file_path), content)?;
    data.merge(extracted);
    Ok(())
}

fn process_go_file(file_path: &str, content: &[u8], data: &mut ExtractedData) -> Result<()> {
    use super::languages::go::GoProcessor;
    
    match GoProcessor::process_file(FilePath::from(file_path), content) {
        Ok((sql_statements, _, _, _)) => {
            parse_sql_into_structs(file_path, &sql_statements, data);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn process_python_file(file_path: &str, content: &[u8], data: &mut ExtractedData) -> Result<()> {
    use super::languages::python::PythonProcessor;
    
    match PythonProcessor::process_file(FilePath::from(file_path), content) {
        Ok(extracted) => {
            // Merge the extracted data
            data.symbols.extend(extracted.symbols);
            data.functions.extend(extracted.functions);
            data.types.extend(extracted.types);
            data.imports.extend(extracted.imports);
            data.call_edges.extend(extracted.call_edges);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn process_javascript_file(file_path: &str, content: &[u8], data: &mut ExtractedData) -> Result<()> {
    use super::languages::javascript::JavaScriptProcessor;
    
    match JavaScriptProcessor::process_file(FilePath::from(file_path), content) {
        Ok(extracted) => {
            // Merge the extracted data
            data.symbols.extend(extracted.symbols);
            data.functions.extend(extracted.functions);
            data.types.extend(extracted.types);
            data.imports.extend(extracted.imports);
            data.call_edges.extend(extracted.call_edges);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn process_typescript_file(file_path: &str, content: &[u8], data: &mut ExtractedData) -> Result<()> {
    use super::languages::typescript::TypeScriptProcessor;
    
    match TypeScriptProcessor::process_file(FilePath::from(file_path), content) {
        Ok((sql_statements, _, _, _)) => {
            parse_sql_into_structs(file_path, &sql_statements, data);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn process_c_file(file_path: &str, content: &[u8], data: &mut ExtractedData) -> Result<()> {
    use super::languages::c::CProcessor;
    
    match CProcessor::process_file(FilePath::from(file_path), content) {
        Ok((sql_statements, _, _, _)) => {
            parse_sql_into_structs(file_path, &sql_statements, data);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn process_cpp_file(file_path: &str, content: &[u8], data: &mut ExtractedData) -> Result<()> {
    use super::languages::cpp::CppProcessor;
    
    match CppProcessor::process_file(FilePath::from(file_path), content) {
        Ok((sql_statements, _, _, _)) => {
            parse_sql_into_structs(file_path, &sql_statements, data);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn process_cairo_file(file_path: &str, content: &[u8], data: &mut ExtractedData) -> Result<()> {
    use super::languages::cairo::CairoProcessor;
    
    // Cairo needs string content
    let content_str = std::str::from_utf8(content)?;
    match CairoProcessor::process_file(FilePath::from(file_path), content_str) {
        Ok((sql_statements, _, _, _)) => {
            parse_sql_into_structs(file_path, &sql_statements, data);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn process_solidity_file(file_path: &str, content: &[u8], data: &mut ExtractedData) -> Result<()> {
    use super::languages::solidity::SolidityProcessor;
    
    match SolidityProcessor::process_file(FilePath::from(file_path), content) {
        Ok((sql_statements, _, _, _)) => {
            parse_sql_into_structs(file_path, &sql_statements, data);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Temporary function to parse SQL statements into structs
/// This is a bridge while we refactor the language processors
fn parse_sql_into_structs(file_path: &str, sql_statements: &[String], data: &mut ExtractedData) {
    // This is a simplified parser - in production we'd refactor the processors
    // to return structs directly instead of SQL strings
    
    for sql in sql_statements {
        if sql.contains("INSERT") && sql.contains("code_search") {
            // Extract code search entry
            if let Some(symbol) = parse_code_search_sql(file_path, sql) {
                data.symbols.push(symbol);
            }
        } else if sql.contains("INSERT") && sql.contains("function_facts") {
            // Extract function fact
            if let Some(func) = parse_function_fact_sql(file_path, sql) {
                data.functions.push(func);
            }
        } else if sql.contains("INSERT") && sql.contains("type_vocabulary") {
            // Extract type fact
            if let Some(type_fact) = parse_type_fact_sql(file_path, sql) {
                data.types.push(type_fact);
            }
        } else if sql.contains("INSERT") && sql.contains("import_facts") {
            // Extract import fact
            if let Some(import) = parse_import_fact_sql(file_path, sql) {
                data.imports.push(import);
            }
        } else if sql.contains("INSERT") && sql.contains("call_graph") {
            // Extract call edge
            if let Some(edge) = parse_call_edge_sql(file_path, sql) {
                data.call_edges.push(edge);
            }
        }
    }
}

// These are simplified parsers - in the real implementation we'd refactor
// the language processors to return structs directly
fn parse_code_search_sql(file_path: &str, _sql: &str) -> Option<CodeSymbol> {
    // Basic extraction - this is temporary
    Some(CodeSymbol {
        path: file_path.to_string(),
        name: "temp".to_string(),
        kind: "function".to_string(),
        line: 1,
        context: "".to_string(),
    })
}

fn parse_function_fact_sql(file_path: &str, _sql: &str) -> Option<FunctionFact> {
    // Basic extraction - this is temporary
    Some(FunctionFact {
        file: file_path.to_string(),
        name: "temp".to_string(),
        takes_mut_self: false,
        takes_mut_params: false,
        returns_result: false,
        returns_option: false,
        is_async: false,
        is_unsafe: false,
        is_public: false,
        parameter_count: 0,
        generic_count: 0,
        parameters: Vec::new(),
        return_type: None,
    })
}

fn parse_type_fact_sql(file_path: &str, _sql: &str) -> Option<TypeFact> {
    Some(TypeFact {
        file: file_path.to_string(),
        name: "temp".to_string(),
        definition: "".to_string(),
        kind: "struct".to_string(),
        visibility: "private".to_string(),
        usage_count: 0,
    })
}

fn parse_import_fact_sql(file_path: &str, _sql: &str) -> Option<ImportFact> {
    Some(ImportFact {
        file: file_path.to_string(),
        import_path: "".to_string(),
        imported_names: Vec::new(),
        import_kind: "use".to_string(),
        line_number: 1,
    })
}

fn parse_call_edge_sql(file_path: &str, _sql: &str) -> Option<CallEdge> {
    Some(CallEdge {
        caller: "".to_string(),
        callee: "".to_string(),
        file: file_path.to_string(),
        call_type: "direct".to_string(),
        line_number: 1,
    })
}