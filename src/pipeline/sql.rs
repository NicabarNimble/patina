use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

use super::schema::AstData;

/// Generate SQL from AST cache JSON files
pub fn generate_sql(ast_cache: &Path, output: &Path) -> Result<()> {
    let mut sql_file = fs::File::create(output)
        .with_context(|| format!("Failed to create SQL file: {}", output.display()))?;
    
    // Write header
    writeln!(sql_file, "-- Generated SQL for bulk loading into DuckDB")?;
    writeln!(sql_file, "-- Generated at: {}", chrono::Utc::now().to_rfc3339())?;
    writeln!(sql_file)?;
    
    // Create tables
    write_table_definitions(&mut sql_file)?;
    
    // Clear existing data
    writeln!(sql_file, "-- Clear existing data")?;
    writeln!(sql_file, "DELETE FROM calls;")?;
    writeln!(sql_file, "DELETE FROM imports;")?;
    writeln!(sql_file, "DELETE FROM functions;")?;
    writeln!(sql_file, "DELETE FROM types;")?;
    writeln!(sql_file)?;
    
    // Process all JSON files
    let json_files = find_json_files(ast_cache)?;
    
    writeln!(sql_file, "-- Insert data from {} files", json_files.len())?;
    writeln!(sql_file)?;
    
    for json_file in json_files {
        process_json_file(&json_file, &mut sql_file)?;
    }
    
    writeln!(sql_file, "-- Create indexes for better query performance")?;
    write_indexes(&mut sql_file)?;
    
    Ok(())
}

fn write_table_definitions(file: &mut fs::File) -> Result<()> {
    writeln!(file, "-- Create tables if they don't exist")?;
    
    writeln!(file, "CREATE TABLE IF NOT EXISTS functions (")?;
    writeln!(file, "    file TEXT NOT NULL,")?;
    writeln!(file, "    name TEXT NOT NULL,")?;
    writeln!(file, "    visibility TEXT,")?;
    writeln!(file, "    is_async BOOLEAN,")?;
    writeln!(file, "    is_unsafe BOOLEAN,")?;
    writeln!(file, "    params_count INTEGER,")?;
    writeln!(file, "    returns TEXT,")?;
    writeln!(file, "    line_start INTEGER,")?;
    writeln!(file, "    line_end INTEGER,")?;
    writeln!(file, "    doc_comment TEXT")?;
    writeln!(file, ");")?;
    writeln!(file)?;
    
    writeln!(file, "CREATE TABLE IF NOT EXISTS types (")?;
    writeln!(file, "    file TEXT NOT NULL,")?;
    writeln!(file, "    name TEXT NOT NULL,")?;
    writeln!(file, "    kind TEXT NOT NULL,")?;
    writeln!(file, "    visibility TEXT,")?;
    writeln!(file, "    fields_count INTEGER,")?;
    writeln!(file, "    methods_count INTEGER,")?;
    writeln!(file, "    line_start INTEGER,")?;
    writeln!(file, "    line_end INTEGER,")?;
    writeln!(file, "    doc_comment TEXT")?;
    writeln!(file, ");")?;
    writeln!(file)?;
    
    writeln!(file, "CREATE TABLE IF NOT EXISTS imports (")?;
    writeln!(file, "    file TEXT NOT NULL,")?;
    writeln!(file, "    path TEXT NOT NULL,")?;
    writeln!(file, "    items_count INTEGER,")?;
    writeln!(file, "    alias TEXT,")?;
    writeln!(file, "    line INTEGER")?;
    writeln!(file, ");")?;
    writeln!(file)?;
    
    writeln!(file, "CREATE TABLE IF NOT EXISTS calls (")?;
    writeln!(file, "    file TEXT NOT NULL,")?;
    writeln!(file, "    target TEXT NOT NULL,")?;
    writeln!(file, "    caller TEXT NOT NULL,")?;
    writeln!(file, "    line INTEGER,")?;
    writeln!(file, "    is_method BOOLEAN,")?;
    writeln!(file, "    is_async BOOLEAN")?;
    writeln!(file, ");")?;
    writeln!(file)?;
    
    Ok(())
}

fn write_indexes(file: &mut fs::File) -> Result<()> {
    writeln!(file, "CREATE INDEX IF NOT EXISTS idx_functions_name ON functions(name);")?;
    writeln!(file, "CREATE INDEX IF NOT EXISTS idx_functions_file ON functions(file);")?;
    writeln!(file, "CREATE INDEX IF NOT EXISTS idx_types_name ON types(name);")?;
    writeln!(file, "CREATE INDEX IF NOT EXISTS idx_types_file ON types(file);")?;
    writeln!(file, "CREATE INDEX IF NOT EXISTS idx_calls_target ON calls(target);")?;
    writeln!(file, "CREATE INDEX IF NOT EXISTS idx_calls_caller ON calls(caller);")?;
    writeln!(file, "CREATE INDEX IF NOT EXISTS idx_imports_path ON imports(path);")?;
    writeln!(file)?;
    
    Ok(())
}

fn find_json_files(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut json_files = Vec::new();
    
    fn visit_dir(dir: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    visit_dir(&path, files)?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    // Skip git_metrics.json and other non-AST files
                    let filename = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    if !filename.starts_with("git_metrics") {
                        files.push(path);
                    }
                }
            }
        }
        Ok(())
    }
    
    visit_dir(dir, &mut json_files)?;
    json_files.sort();
    
    Ok(json_files)
}

fn process_json_file(json_path: &Path, sql_file: &mut fs::File) -> Result<()> {
    let content = fs::read_to_string(json_path)
        .with_context(|| format!("Failed to read JSON file: {}", json_path.display()))?;
    
    let ast_data: AstData = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON: {}", json_path.display()))?;
    
    // Insert functions
    for func in &ast_data.functions {
        writeln!(
            sql_file,
            "INSERT INTO functions VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {});",
            escape_string(&ast_data.file),
            escape_string(&func.name),
            escape_string(&func.visibility),
            func.is_async,
            func.is_unsafe,
            func.params.len(),
            func.returns.as_deref().map(escape_string).unwrap_or_else(|| "NULL".to_string()),
            func.line_start,
            func.line_end,
            func.doc_comment.as_deref().map(escape_string).unwrap_or_else(|| "NULL".to_string())
        )?;
    }
    
    // Insert types
    for type_def in &ast_data.types {
        writeln!(
            sql_file,
            "INSERT INTO types VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {});",
            escape_string(&ast_data.file),
            escape_string(&type_def.name),
            escape_string(&type_def.kind),
            escape_string(&type_def.visibility),
            type_def.fields.len(),
            type_def.methods.len(),
            type_def.line_start,
            type_def.line_end,
            type_def.doc_comment.as_deref().map(escape_string).unwrap_or_else(|| "NULL".to_string())
        )?;
    }
    
    // Insert imports
    for import in &ast_data.imports {
        writeln!(
            sql_file,
            "INSERT INTO imports VALUES ({}, {}, {}, {}, {});",
            escape_string(&ast_data.file),
            escape_string(&import.path),
            import.items.len(),
            import.alias.as_deref().map(escape_string).unwrap_or_else(|| "NULL".to_string()),
            import.line
        )?;
    }
    
    // Insert calls
    for call in &ast_data.calls {
        writeln!(
            sql_file,
            "INSERT INTO calls VALUES ({}, {}, {}, {}, {}, {});",
            escape_string(&ast_data.file),
            escape_string(&call.target),
            escape_string(&call.caller),
            call.line,
            call.is_method,
            call.is_async
        )?;
    }
    
    Ok(())
}

fn escape_string(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Load the generated SQL file into an SQLite database
pub fn load_into_sqlite(sql_path: &Path, db_path: &Path) -> Result<()> {
    use rusqlite::Connection;
    
    // Read the SQL file
    let sql_content = fs::read_to_string(sql_path)
        .with_context(|| format!("Failed to read SQL file: {}", sql_path.display()))?;
    
    // Open or create the database
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database: {}", db_path.display()))?;
    
    // Execute the SQL in a transaction for better performance
    conn.execute_batch(&sql_content)
        .with_context(|| "Failed to execute SQL statements")?;
    
    Ok(())
}