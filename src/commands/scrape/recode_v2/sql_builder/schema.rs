// ============================================================================
// SCHEMA BUILDERS (CREATE TABLE)
// ============================================================================
//! Type-safe schema construction for DuckDB.

use std::fmt;

/// DuckDB column types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnType {
    Varchar,
    Text,
    Integer,
    BigInt,
    Boolean,
    Timestamp,
    Array(Box<ColumnType>),
}

impl fmt::Display for ColumnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColumnType::Varchar => write!(f, "VARCHAR"),
            ColumnType::Text => write!(f, "TEXT"),
            ColumnType::Integer => write!(f, "INTEGER"),
            ColumnType::BigInt => write!(f, "BIGINT"),
            ColumnType::Boolean => write!(f, "BOOLEAN"),
            ColumnType::Timestamp => write!(f, "TIMESTAMP"),
            ColumnType::Array(inner) => write!(f, "{}[]", inner),
        }
    }
}

/// Column definition for CREATE TABLE
pub struct Column {
    name: String,
    column_type: ColumnType,
    not_null: bool,
    primary_key: bool,
    default: Option<String>,
}

impl Column {
    pub fn new(name: impl Into<String>, column_type: ColumnType) -> Self {
        Self {
            name: name.into(),
            column_type,
            not_null: false,
            primary_key: false,
            default: None,
        }
    }
    
    pub fn not_null(mut self) -> Self {
        self.not_null = true;
        self
    }
    
    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self.not_null = true; // Primary keys are implicitly NOT NULL
        self
    }
    
    pub fn default(mut self, value: impl Into<String>) -> Self {
        self.default = Some(value.into());
        self
    }
    
    fn to_sql(&self) -> String {
        let mut sql = format!("{} {}", self.name, self.column_type);
        
        if self.not_null {
            sql.push_str(" NOT NULL");
        }
        
        if let Some(ref default) = self.default {
            sql.push_str(&format!(" DEFAULT {}", default));
        }
        
        if self.primary_key {
            sql.push_str(" PRIMARY KEY");
        }
        
        sql
    }
}

/// Builder for CREATE TABLE statements
pub struct CreateTableBuilder {
    table_name: String,
    if_not_exists: bool,
    columns: Vec<Column>,
    composite_primary_key: Option<Vec<String>>,
}

impl CreateTableBuilder {
    pub fn new(table_name: impl Into<String>) -> Self {
        Self {
            table_name: table_name.into(),
            if_not_exists: true,
            columns: Vec::new(),
            composite_primary_key: None,
        }
    }
    
    pub fn if_not_exists(mut self, value: bool) -> Self {
        self.if_not_exists = value;
        self
    }
    
    pub fn add_column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }
    
    pub fn column(self, name: impl Into<String>, column_type: ColumnType) -> Self {
        self.add_column(Column::new(name, column_type))
    }
    
    pub fn composite_primary_key(mut self, columns: Vec<impl Into<String>>) -> Self {
        self.composite_primary_key = Some(columns.into_iter().map(|s| s.into()).collect());
        self
    }
    
    pub fn build(self) -> String {
        let mut sql = String::with_capacity(512);
        
        sql.push_str("CREATE TABLE ");
        if self.if_not_exists {
            sql.push_str("IF NOT EXISTS ");
        }
        sql.push_str(&self.table_name);
        sql.push_str(" (\n");
        
        // Add columns
        let mut parts = Vec::new();
        for column in self.columns {
            parts.push(format!("    {}", column.to_sql()));
        }
        
        // Add composite primary key if specified
        if let Some(pk_columns) = self.composite_primary_key {
            parts.push(format!("    PRIMARY KEY ({})", pk_columns.join(", ")));
        }
        
        sql.push_str(&parts.join(",\n"));
        sql.push_str("\n)");
        
        sql
    }
}

/// Builder for CREATE INDEX statements
pub struct CreateIndexBuilder {
    index_name: String,
    if_not_exists: bool,
    table_name: String,
    columns: Vec<String>,
}

impl CreateIndexBuilder {
    pub fn new(index_name: impl Into<String>) -> Self {
        Self {
            index_name: index_name.into(),
            if_not_exists: true,
            table_name: String::new(),
            columns: Vec::new(),
        }
    }
    
    pub fn if_not_exists(mut self, value: bool) -> Self {
        self.if_not_exists = value;
        self
    }
    
    pub fn on_table(mut self, table: impl Into<String>) -> Self {
        self.table_name = table.into();
        self
    }
    
    pub fn column(mut self, column: impl Into<String>) -> Self {
        self.columns.push(column.into());
        self
    }
    
    pub fn columns(mut self, columns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.columns.extend(columns.into_iter().map(|s| s.into()));
        self
    }
    
    pub fn build(self) -> String {
        if self.table_name.is_empty() {
            panic!("Table name is required for CREATE INDEX");
        }
        if self.columns.is_empty() {
            panic!("At least one column is required for CREATE INDEX");
        }
        
        let mut sql = String::with_capacity(128);
        
        sql.push_str("CREATE INDEX ");
        if self.if_not_exists {
            sql.push_str("IF NOT EXISTS ");
        }
        sql.push_str(&self.index_name);
        sql.push_str(" ON ");
        sql.push_str(&self.table_name);
        sql.push_str("(");
        sql.push_str(&self.columns.join(", "));
        sql.push(')');
        
        sql
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_table_simple() {
        let sql = CreateTableBuilder::new("test_table")
            .column("id", ColumnType::Integer)
            .column("name", ColumnType::Varchar)
            .build();
        
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS test_table"));
        assert!(sql.contains("id INTEGER"));
        assert!(sql.contains("name VARCHAR"));
    }
    
    #[test]
    fn test_create_table_with_constraints() {
        let sql = CreateTableBuilder::new("users")
            .add_column(
                Column::new("id", ColumnType::Integer)
                    .not_null()
                    .primary_key()
            )
            .add_column(
                Column::new("name", ColumnType::Varchar)
                    .not_null()
            )
            .add_column(
                Column::new("created_at", ColumnType::Timestamp)
                    .default("CURRENT_TIMESTAMP")
            )
            .build();
        
        assert!(sql.contains("id INTEGER NOT NULL PRIMARY KEY"));
        assert!(sql.contains("name VARCHAR NOT NULL"));
        assert!(sql.contains("created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP"));
    }
    
    #[test]
    fn test_create_table_with_composite_key() {
        let sql = CreateTableBuilder::new("call_graph")
            .column("caller", ColumnType::Varchar)
            .column("callee", ColumnType::Varchar)
            .column("file", ColumnType::Varchar)
            .composite_primary_key(vec!["caller", "callee", "file"])
            .build();
        
        assert!(sql.contains("PRIMARY KEY (caller, callee, file)"));
    }
    
    #[test]
    fn test_create_index() {
        let sql = CreateIndexBuilder::new("idx_caller")
            .on_table("call_graph")
            .column("caller")
            .build();
        
        assert_eq!(sql, "CREATE INDEX IF NOT EXISTS idx_caller ON call_graph(caller)");
    }
    
    #[test]
    fn test_array_type() {
        let array_type = ColumnType::Array(Box::new(ColumnType::Varchar));
        assert_eq!(array_type.to_string(), "VARCHAR[]");
    }
}