use anyhow::{Context, Result};
use duckdb::Connection;
use std::path::Path;

/// Load SQL file into DuckDB database
pub fn load_into_duckdb(sql_path: &Path, db_path: &Path) -> Result<()> {
    // Remove existing database to start fresh
    if db_path.exists() {
        std::fs::remove_file(db_path).ok();
    }
    
    // Connect to DuckDB database
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open DuckDB database: {}", db_path.display()))?;
    
    // Read SQL file
    let sql = std::fs::read_to_string(sql_path)
        .with_context(|| format!("Failed to read SQL file: {}", sql_path.display()))?;
    
    // Execute SQL statements
    // DuckDB can handle multiple statements in execute_batch
    conn.execute_batch(&sql)
        .with_context(|| "Failed to execute SQL in DuckDB")?;
    
    Ok(())
}

/// Initialize a new DuckDB database with schema
pub fn initialize_database(db_path: &Path) -> Result<()> {
    // Remove existing database if present
    if db_path.exists() {
        std::fs::remove_file(db_path)?;
    }
    
    // Create new DuckDB database with schema
    let conn = Connection::open(db_path)?;
    
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS functions (
            file TEXT NOT NULL,
            name TEXT NOT NULL,
            visibility TEXT,
            is_async BOOLEAN,
            is_unsafe BOOLEAN,
            params_count INTEGER,
            returns TEXT,
            line_start INTEGER,
            line_end INTEGER,
            doc_comment TEXT
        );
        
        CREATE TABLE IF NOT EXISTS types (
            file TEXT NOT NULL,
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            visibility TEXT,
            fields_count INTEGER,
            methods_count INTEGER,
            line_start INTEGER,
            line_end INTEGER,
            doc_comment TEXT
        );
        
        CREATE TABLE IF NOT EXISTS imports (
            file TEXT NOT NULL,
            path TEXT NOT NULL,
            items_count INTEGER,
            alias TEXT,
            line INTEGER
        );
        
        CREATE TABLE IF NOT EXISTS calls (
            file TEXT NOT NULL,
            target TEXT NOT NULL,
            caller TEXT NOT NULL,
            line INTEGER,
            is_method BOOLEAN,
            is_async BOOLEAN
        );
        
        CREATE INDEX idx_functions_name ON functions(name);
        CREATE INDEX idx_functions_file ON functions(file);
        CREATE INDEX idx_types_name ON types(name);
        CREATE INDEX idx_types_file ON types(file);
        CREATE INDEX idx_imports_file ON imports(file);
        CREATE INDEX idx_calls_target ON calls(target);
        CREATE INDEX idx_calls_caller ON calls(caller);
        "
    )?;
    
    Ok(())
}

/// Query functions from the database
pub fn query_functions(db_path: &Path, pattern: &str) -> Result<Vec<String>> {
    let conn = Connection::open(db_path)?;
    
    let mut stmt = conn.prepare(
        "SELECT DISTINCT file || ':' || name || ' (' || line_start || '-' || line_end || ')' 
         FROM functions 
         WHERE name LIKE ?
         ORDER BY file, line_start"
    )?;
    
    let results = stmt.query_map([format!("%{}%", pattern)], |row| {
        Ok(row.get(0)?)
    })?
    .collect::<Result<Vec<_>, _>>()?;
    
    Ok(results)
}

/// Execute a custom query and return results as strings
pub fn execute_query(db_path: &Path, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>)> {
    use duckdb::arrow::util::pretty;
    
    let conn = Connection::open(db_path)?;
    
    // Execute the query and get Arrow result
    let mut stmt = conn.prepare(query)?;
    let arrow = stmt.query_arrow([])?;
    
    // Convert Arrow to string representation
    let batches: Vec<_> = arrow.collect();
    if batches.is_empty() {
        return Ok((vec![], vec![]));
    }
    
    // Use Arrow's pretty printer to format the results
    let formatted = pretty::pretty_format_batches(&batches)?.to_string();
    let lines: Vec<&str> = formatted.lines().collect();
    
    // Parse the formatted output to extract column names and rows
    if lines.len() < 3 {
        return Ok((vec![], vec![]));
    }
    
    // Extract column names from the first line
    let column_names: Vec<String> = lines[0]
        .split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    // Extract data rows (skip header and separator lines)
    let mut rows = Vec::new();
    for line in lines.iter().skip(2) {
        if !line.starts_with('+') && !line.is_empty() {
            let row: Vec<String> = line
                .split('|')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if row.len() == column_names.len() {
                rows.push(row);
            }
        }
    }
    
    Ok((column_names, rows))
}

/// Get database statistics
pub fn get_stats(db_path: &Path) -> Result<String> {
    let conn = Connection::open(db_path)?;
    
    let function_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM functions")?
        .query_row([], |row| Ok(row.get(0)?))?;
    
    let type_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM types")?
        .query_row([], |row| Ok(row.get(0)?))?;
    
    let import_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM imports")?
        .query_row([], |row| Ok(row.get(0)?))?;
    
    let call_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM calls")?
        .query_row([], |row| Ok(row.get(0)?))?;
    
    let file_count: i64 = conn
        .prepare("SELECT COUNT(DISTINCT file) FROM functions")?
        .query_row([], |row| Ok(row.get(0)?))?;
    
    Ok(format!(
        "Database Statistics:\n\
         - Files: {}\n\
         - Functions: {}\n\
         - Types: {}\n\
         - Imports: {}\n\
         - Calls: {}",
        file_count, function_count, type_count, import_count, call_count
    ))
}