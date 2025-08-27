use anyhow::{Context, Result};
use rusqlite::Connection;

use super::transform::{DatabaseRecord, SqlValue};

/// Initialize database with schema
pub fn initialize_database(db_path: &str) -> Result<()> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database at {}", db_path))?;
    
    // Create tables
    conn.execute_batch(SCHEMA)
        .context("Failed to create database schema")?;
    
    println!("✓ Database initialized at {}", db_path);
    Ok(())
}

/// Run a query against the database
pub fn run_query(query: &str, db_path: &str) -> Result<()> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database at {}", db_path))?;
    
    // Prepare and execute query
    let mut stmt = conn.prepare(query)
        .context("Failed to prepare query")?;
    
    // Try to get column names
    let column_count = stmt.column_count();
    let column_names: Vec<String> = (0..column_count)
        .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
        .collect();
    
    // Print header
    println!("{}", column_names.join(" | "));
    println!("{}", "-".repeat(column_names.join(" | ").len()));
    
    // Execute and print results
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let values: Vec<String> = (0..column_count)
            .map(|i| {
                row.get::<_, rusqlite::types::Value>(i)
                    .map(|v| format!("{:?}", v))
                    .unwrap_or_else(|_| "NULL".to_string())
            })
            .collect();
        println!("{}", values.join(" | "));
    }
    
    Ok(())
}

/// Save a batch of records to the database
pub fn save_batch(db_path: &str, records: Vec<DatabaseRecord>) -> Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    
    let mut conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database at {}", db_path))?;
    
    // Start transaction for batch insert
    let tx = conn.transaction()?;
    
    for record in records {
        insert_record(&tx, &record)
            .with_context(|| format!("Failed to insert record into {}", record.table))?;
    }
    
    // Commit transaction
    tx.commit()
        .context("Failed to commit transaction")?;
    
    Ok(())
}

/// Insert a single record
fn insert_record(conn: &Connection, record: &DatabaseRecord) -> Result<()> {
    let sql = generate_insert_sql(record);
    let params = record_to_params(record);
    
    // Use rusqlite::params_from_iter for dynamic params
    conn.execute(&sql, rusqlite::params_from_iter(params.iter()))
        .with_context(|| format!("Failed to execute SQL: {}", sql))?;
    
    Ok(())
}

/// Generate INSERT SQL for a record
fn generate_insert_sql(record: &DatabaseRecord) -> String {
    let columns: Vec<String> = record.values.keys().cloned().collect();
    let placeholders: Vec<String> = (1..=columns.len())
        .map(|i| format!("?{}", i))
        .collect();
    
    // Add fingerprint column if present
    let (all_columns, all_placeholders) = if record.fingerprint.is_some() {
        let mut cols = columns.clone();
        cols.push("fingerprint".to_string());
        let mut places = placeholders.clone();
        places.push(format!("?{}", columns.len() + 1));
        (cols, places)
    } else {
        (columns, placeholders)
    };
    
    format!(
        "INSERT OR REPLACE INTO {} ({}) VALUES ({})",
        record.table,
        all_columns.join(", "),
        all_placeholders.join(", ")
    )
}

/// Convert record values to rusqlite params
fn record_to_params(record: &DatabaseRecord) -> Vec<rusqlite::types::Value> {
    let mut params = Vec::new();
    
    // Add regular values
    for value in record.values.values() {
        params.push(sql_value_to_rusqlite(value));
    }
    
    // Add fingerprint if present
    if let Some(fingerprint) = &record.fingerprint {
        params.push(rusqlite::types::Value::Blob(fingerprint.to_vec()));
    }
    
    params
}

/// Convert our SqlValue to rusqlite Value
fn sql_value_to_rusqlite(value: &SqlValue) -> rusqlite::types::Value {
    match value {
        SqlValue::Text(s) => rusqlite::types::Value::Text(s.clone()),
        SqlValue::Integer(i) => rusqlite::types::Value::Integer(*i),
        SqlValue::Real(f) => rusqlite::types::Value::Real(*f),
        SqlValue::Blob(b) => rusqlite::types::Value::Blob(b.clone()),
        SqlValue::Null => rusqlite::types::Value::Null,
    }
}

/// Database schema
const SCHEMA: &str = r#"
-- Function facts table
CREATE TABLE IF NOT EXISTS function_facts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    name TEXT NOT NULL,
    visibility TEXT,
    parameters TEXT, -- JSON string
    return_type TEXT,
    is_async INTEGER DEFAULT 0,
    is_unsafe INTEGER DEFAULT 0,
    line_start INTEGER,
    line_end INTEGER,
    doc_comment TEXT,
    fingerprint BLOB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(file_path, name, line_start)
);

-- Type facts table (structs, enums, etc)
CREATE TABLE IF NOT EXISTS type_facts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL, -- struct, enum, interface, etc
    visibility TEXT,
    fields TEXT, -- JSON string
    generics TEXT, -- Comma-separated
    line_start INTEGER,
    line_end INTEGER,
    doc_comment TEXT,
    fingerprint BLOB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(file_path, name, line_start)
);

-- Import facts table
CREATE TABLE IF NOT EXISTS import_facts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    module TEXT NOT NULL,
    items TEXT, -- Comma-separated
    is_wildcard INTEGER DEFAULT 0,
    line_number INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(file_path, module, line_number)
);

-- Call graph table
CREATE TABLE IF NOT EXISTS call_graph (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    caller TEXT NOT NULL,
    callee TEXT NOT NULL,
    line_number INTEGER,
    is_external INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Documentation table
CREATE TABLE IF NOT EXISTS documentation (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    kind TEXT NOT NULL,
    content TEXT,
    line_start INTEGER,
    line_end INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_functions_file ON function_facts(file_path);
CREATE INDEX IF NOT EXISTS idx_functions_name ON function_facts(name);
CREATE INDEX IF NOT EXISTS idx_types_file ON type_facts(file_path);
CREATE INDEX IF NOT EXISTS idx_types_name ON type_facts(name);
CREATE INDEX IF NOT EXISTS idx_imports_file ON import_facts(file_path);
CREATE INDEX IF NOT EXISTS idx_callgraph_caller ON call_graph(caller);
CREATE INDEX IF NOT EXISTS idx_callgraph_callee ON call_graph(callee);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::collections::HashMap;
    
    #[test]
    fn test_initialize_database() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let db_path = temp_file.path().to_str().unwrap();
        
        initialize_database(db_path)?;
        
        // Verify tables exist
        let conn = Connection::open(db_path)?;
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
        )?;
        
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        
        assert!(tables.contains(&"function_facts".to_string()));
        assert!(tables.contains(&"type_facts".to_string()));
        assert!(tables.contains(&"import_facts".to_string()));
        assert!(tables.contains(&"call_graph".to_string()));
        assert!(tables.contains(&"documentation".to_string()));
        
        Ok(())
    }
    
    #[test]
    fn test_save_batch() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let db_path = temp_file.path().to_str().unwrap();
        
        initialize_database(db_path)?;
        
        // Create test records
        let mut values = HashMap::new();
        values.insert("file_path".to_string(), SqlValue::Text("test.rs".to_string()));
        values.insert("name".to_string(), SqlValue::Text("test_func".to_string()));
        values.insert("visibility".to_string(), SqlValue::Text("public".to_string()));
        values.insert("parameters".to_string(), SqlValue::Text("[]".to_string()));
        values.insert("return_type".to_string(), SqlValue::Null);
        values.insert("is_async".to_string(), SqlValue::Integer(0));
        values.insert("is_unsafe".to_string(), SqlValue::Integer(0));
        values.insert("line_start".to_string(), SqlValue::Integer(1));
        values.insert("line_end".to_string(), SqlValue::Integer(5));
        values.insert("doc_comment".to_string(), SqlValue::Null);
        
        let record = DatabaseRecord {
            table: "function_facts".to_string(),
            values,
            fingerprint: Some([0u8; 16]),
        };
        
        save_batch(db_path, vec![record])?;
        
        // Verify record was inserted
        let conn = Connection::open(db_path)?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM function_facts WHERE name = 'test_func'",
            [],
            |row| row.get(0)
        )?;
        
        assert_eq!(count, 1);
        
        Ok(())
    }
    
    #[test]
    fn test_generate_insert_sql() {
        let mut values = HashMap::new();
        values.insert("name".to_string(), SqlValue::Text("test".to_string()));
        values.insert("age".to_string(), SqlValue::Integer(25));
        
        let record = DatabaseRecord {
            table: "users".to_string(),
            values,
            fingerprint: None,
        };
        
        let sql = generate_insert_sql(&record);
        assert!(sql.contains("INSERT OR REPLACE INTO users"));
        assert!(sql.contains("name") && sql.contains("age"));
        assert!(sql.contains("?1") && sql.contains("?2"));
    }
}