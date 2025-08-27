use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::Path;

/// Load SQL file into DuckDB database
pub fn load_into_duckdb(sql_path: &Path, db_path: &Path) -> Result<()> {
    // For now, we'll use SQLite as it's already in our dependencies
    // In production, we'd use actual DuckDB bindings
    load_into_sqlite(sql_path, db_path)
}

/// Load SQL into SQLite (temporary implementation until DuckDB integration)
fn load_into_sqlite(sql_path: &Path, db_path: &Path) -> Result<()> {
    // Remove existing database to start fresh
    if db_path.exists() {
        std::fs::remove_file(db_path).ok();
    }
    
    // Connect to database
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database: {}", db_path.display()))?;
    
    // Read SQL file
    let sql = std::fs::read_to_string(sql_path)
        .with_context(|| format!("Failed to read SQL file: {}", sql_path.display()))?;
    
    // Execute SQL statements
    // SQLite doesn't support executing multiple statements at once with execute_batch for some operations
    // So we'll split by semicolon and execute each statement
    for statement in sql.split(';') {
        let trimmed = statement.trim();
        if !trimmed.is_empty() {
            conn.execute(trimmed, [])
                .with_context(|| format!("Failed to execute SQL: {}", &trimmed[..trimmed.len().min(100)]))?;
        }
    }
    
    Ok(())
}

/// Query functions from the database
pub fn query_functions(db_path: &Path, pattern: &str) -> Result<Vec<String>> {
    let conn = Connection::open(db_path)?;
    
    let mut stmt = conn.prepare(
        "SELECT DISTINCT file || ':' || name || ' (' || line_start || '-' || line_end || ')' 
         FROM functions 
         WHERE name LIKE ?1 
         ORDER BY file, line_start"
    )?;
    
    let results = stmt.query_map([format!("%{}%", pattern)], |row| {
        row.get(0)
    })?
    .collect::<Result<Vec<_>, _>>()?;
    
    Ok(results)
}

/// Get database statistics
pub fn get_stats(db_path: &Path) -> Result<String> {
    let conn = Connection::open(db_path)?;
    
    let function_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM functions",
        [],
        |row| row.get(0)
    )?;
    
    let type_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM types",
        [],
        |row| row.get(0)
    )?;
    
    let import_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM imports",
        [],
        |row| row.get(0)
    )?;
    
    let call_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM calls",
        [],
        |row| row.get(0)
    )?;
    
    let file_count: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT file) FROM functions",
        [],
        |row| row.get(0)
    )?;
    
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