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

use super::database::Database;
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
    let mut all_constants = Vec::new();
    let mut all_members = Vec::new();

    let mut files_with_errors = 0;
    let mut _files_processed = 0;
    let total_files = all_files.len();

    // Process each file and collect data
    for (i, (file_path, language)) in all_files.into_iter().enumerate() {
        let relative_path = if let Ok(stripped) = file_path.strip_prefix(work_dir) {
            format!("./{}", stripped.to_string_lossy())
        } else {
            file_path.to_string_lossy().to_string()
        };

        // Show progress every 100 files or on large repos every 500 files
        let progress_interval = if total_files > 1000 { 500 } else { 100 };
        if i % progress_interval == 0 || i == total_files - 1 {
            print!("\r  📝 Processing files: {}/{} ({:.1}%)", i + 1, total_files, (i + 1) as f64 / total_files as f64 * 100.0);
            use std::io::Write;
            std::io::stdout().flush().ok();
        }

        // Read file content
        let content = match std::fs::read(&file_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("\n  ⚠️  Failed to read {}: {}", relative_path, e);
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
                all_constants.extend(extracted.constants);
                all_members.extend(extracted.members);
                _files_processed += 1;
            }
            Err(e) => {
                eprintln!("\n  ⚠️  Processing error in {}: {}", relative_path, e);
                db.mark_skipped(&relative_path, &e.to_string())?;
                files_with_errors += 1;
            }
        }
    }

    // Clear progress line and show completion
    println!("\r  ✅ Processed {}/{} files", total_files, total_files);

    // Bulk insert all collected data
    println!("  💾 Writing to database using bulk operations...");

    let symbols_count = db.insert_symbols(&all_symbols)?;
    let functions_count = db.insert_functions(&all_functions)?;
    let types_count = db.insert_types(&all_types)?;
    let imports_count = db.insert_imports(&all_imports)?;
    let edges_count = db.insert_call_edges(&all_call_edges)?;
    let constants_count = db.insert_constants(&all_constants)?;
    let members_count = db.insert_members(&all_members)?;

    println!(
        "  ✅ Inserted: {} symbols, {} functions, {} types, {} imports, {} call edges, {} constants, {} members",
        symbols_count, functions_count, types_count, imports_count, edges_count, constants_count, members_count
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
        constants: Vec::new(),
        members: Vec::new(),
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
        Ok(extracted) => {
            // Merge the extracted data
            data.symbols.extend(extracted.symbols);
            data.functions.extend(extracted.functions);
            data.types.extend(extracted.types);
            data.imports.extend(extracted.imports);
            data.call_edges.extend(extracted.call_edges);
            data.constants.extend(extracted.constants);
            data.members.extend(extracted.members);
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
            data.constants.extend(extracted.constants);
            data.members.extend(extracted.members);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn process_javascript_file(
    file_path: &str,
    content: &[u8],
    data: &mut ExtractedData,
) -> Result<()> {
    use super::languages::javascript::JavaScriptProcessor;

    match JavaScriptProcessor::process_file(FilePath::from(file_path), content) {
        Ok(extracted) => {
            // Merge the extracted data
            data.symbols.extend(extracted.symbols);
            data.functions.extend(extracted.functions);
            data.types.extend(extracted.types);
            data.imports.extend(extracted.imports);
            data.call_edges.extend(extracted.call_edges);
            data.constants.extend(extracted.constants);
            data.members.extend(extracted.members);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn process_typescript_file(
    file_path: &str,
    content: &[u8],
    data: &mut ExtractedData,
) -> Result<()> {
    use super::languages::typescript::TypeScriptProcessor;

    match TypeScriptProcessor::process_file(FilePath::from(file_path), content) {
        Ok(extracted) => {
            // Merge the extracted data
            data.symbols.extend(extracted.symbols);
            data.functions.extend(extracted.functions);
            data.types.extend(extracted.types);
            data.imports.extend(extracted.imports);
            data.call_edges.extend(extracted.call_edges);
            data.constants.extend(extracted.constants);
            data.members.extend(extracted.members);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn process_c_file(file_path: &str, content: &[u8], data: &mut ExtractedData) -> Result<()> {
    use super::languages::c::CProcessor;

    match CProcessor::process_file(FilePath::from(file_path), content) {
        Ok(extracted) => {
            // Merge the extracted data
            data.symbols.extend(extracted.symbols);
            data.functions.extend(extracted.functions);
            data.types.extend(extracted.types);
            data.imports.extend(extracted.imports);
            data.call_edges.extend(extracted.call_edges);
            data.constants.extend(extracted.constants);
            data.members.extend(extracted.members);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn process_cpp_file(file_path: &str, content: &[u8], data: &mut ExtractedData) -> Result<()> {
    use super::languages::cpp::CppProcessor;

    match CppProcessor::process_file(FilePath::from(file_path), content) {
        Ok(extracted) => {
            // Merge the extracted data
            data.symbols.extend(extracted.symbols);
            data.functions.extend(extracted.functions);
            data.types.extend(extracted.types);
            data.imports.extend(extracted.imports);
            data.call_edges.extend(extracted.call_edges);
            data.constants.extend(extracted.constants);
            data.members.extend(extracted.members);
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
        Ok(extracted) => {
            // Merge the extracted data
            data.symbols.extend(extracted.symbols);
            data.functions.extend(extracted.functions);
            data.types.extend(extracted.types);
            data.imports.extend(extracted.imports);
            data.call_edges.extend(extracted.call_edges);
            data.constants.extend(extracted.constants);
            data.members.extend(extracted.members);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn process_solidity_file(file_path: &str, content: &[u8], data: &mut ExtractedData) -> Result<()> {
    use super::languages::solidity::SolidityProcessor;

    match SolidityProcessor::process_file(FilePath::from(file_path), content) {
        Ok(extracted) => {
            // Merge the extracted data
            data.symbols.extend(extracted.symbols);
            data.functions.extend(extracted.functions);
            data.types.extend(extracted.types);
            data.imports.extend(extracted.imports);
            data.call_edges.extend(extracted.call_edges);
            data.constants.extend(extracted.constants);
            data.members.extend(extracted.members);
            Ok(())
        }
        Err(e) => Err(e),
    }
}
