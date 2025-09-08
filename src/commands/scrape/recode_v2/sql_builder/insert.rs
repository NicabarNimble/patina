// ============================================================================
// INSERT STATEMENT BUILDER
// ============================================================================
//! Type-safe INSERT statement construction for DuckDB.

use super::{SqlValue, TableName};
use std::collections::HashMap;

/// Builder for INSERT statements with compile-time safety
pub struct InsertBuilder {
    table: TableName,
    columns: Vec<String>,
    values: Vec<SqlValue>,
    or_replace: bool,
}

impl InsertBuilder {
    /// Create a new INSERT builder for the specified table
    pub fn new(table: TableName) -> Self {
        Self {
            table,
            columns: Vec::new(),
            values: Vec::new(),
            or_replace: false,
        }
    }

    /// Add OR REPLACE clause (DuckDB-specific)
    pub fn or_replace(mut self) -> Self {
        self.or_replace = true;
        self
    }

    /// Add a column-value pair
    pub fn value<V: Into<SqlValue>>(mut self, column: &str, value: V) -> Self {
        self.columns.push(column.to_string());
        self.values.push(value.into());
        self
    }

    /// Add multiple values from a HashMap
    pub fn values_from_map(mut self, map: HashMap<String, SqlValue>) -> Self {
        for (col, val) in map {
            self.columns.push(col);
            self.values.push(val);
        }
        self
    }

    /// Build the final SQL statement
    pub fn build(self) -> String {
        if self.columns.is_empty() {
            panic!("INSERT statement requires at least one column");
        }

        let mut sql = String::with_capacity(256);

        // INSERT OR REPLACE INTO table_name
        sql.push_str("INSERT ");
        if self.or_replace {
            sql.push_str("OR REPLACE ");
        }
        sql.push_str("INTO ");
        sql.push_str(self.table.as_str());

        // (column1, column2, ...)
        sql.push_str(" (");
        sql.push_str(&self.columns.join(", "));
        sql.push_str(") VALUES (");

        // VALUES (value1, value2, ...)
        let value_strings: Vec<String> = self.values.iter().map(|v| v.to_sql()).collect();
        sql.push_str(&value_strings.join(", "));
        sql.push_str(")");

        sql
    }
}

/// Builder for batch INSERT statements
pub struct BatchInsertBuilder {
    table: TableName,
    columns: Vec<String>,
    rows: Vec<Vec<SqlValue>>,
    or_replace: bool,
}

impl BatchInsertBuilder {
    /// Create a new batch INSERT builder
    pub fn new(table: TableName) -> Self {
        Self {
            table,
            columns: Vec::new(),
            rows: Vec::new(),
            or_replace: false,
        }
    }

    /// Set the columns for all rows
    pub fn columns<S: Into<String>>(mut self, cols: impl IntoIterator<Item = S>) -> Self {
        self.columns = cols.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Add OR REPLACE clause
    pub fn or_replace(mut self) -> Self {
        self.or_replace = true;
        self
    }

    /// Add a row of values
    pub fn add_row<V: Into<SqlValue>>(mut self, values: impl IntoIterator<Item = V>) -> Self {
        let row: Vec<SqlValue> = values.into_iter().map(|v| v.into()).collect();
        if !self.columns.is_empty() && row.len() != self.columns.len() {
            panic!(
                "Row has {} values but {} columns were specified",
                row.len(),
                self.columns.len()
            );
        }
        self.rows.push(row);
        self
    }

    /// Build the final SQL statements
    /// Returns one INSERT per row (DuckDB doesn't support multi-row VALUES)
    pub fn build(self) -> Vec<String> {
        if self.columns.is_empty() {
            panic!("Batch INSERT requires columns to be specified");
        }

        let mut statements = Vec::with_capacity(self.rows.len());

        for row in self.rows {
            let mut sql = String::with_capacity(256);

            sql.push_str("INSERT ");
            if self.or_replace {
                sql.push_str("OR REPLACE ");
            }
            sql.push_str("INTO ");
            sql.push_str(self.table.as_str());
            sql.push_str(" (");
            sql.push_str(&self.columns.join(", "));
            sql.push_str(") VALUES (");

            let value_strings: Vec<String> = row.iter().map(|v| v.to_sql()).collect();
            sql.push_str(&value_strings.join(", "));
            sql.push_str(")");

            statements.push(sql);
        }

        statements
    }

    /// Build as a single string with semicolons
    pub fn build_combined(self) -> String {
        let statements = self.build();
        let mut result = String::with_capacity(statements.iter().map(|s| s.len() + 2).sum());
        for stmt in statements {
            result.push_str(&stmt);
            result.push_str(";\n");
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_builder() {
        let sql = InsertBuilder::new(TableName::CODE_SEARCH)
            .value("path", "src/main.rs")
            .value("name", "main")
            .value("context", "fn main() { }")
            .build();

        assert_eq!(
            sql,
            "INSERT INTO code_search (path, name, context) VALUES ('src/main.rs', 'main', 'fn main() { }')"
        );
    }

    #[test]
    fn test_insert_or_replace() {
        let sql = InsertBuilder::new(TableName::INDEX_STATE)
            .or_replace()
            .value("path", "src/lib.rs")
            .value("mtime", 1234567890i64)
            .build();

        assert_eq!(
            sql,
            "INSERT OR REPLACE INTO index_state (path, mtime) VALUES ('src/lib.rs', 1234567890)"
        );
    }

    #[test]
    fn test_insert_with_null() {
        let sql = InsertBuilder::new(TableName::FUNCTION_FACTS)
            .value("file", "src/lib.rs")
            .value("name", "process")
            .value("return_type", None::<String>)
            .build();

        assert!(sql.contains("NULL"));
    }

    #[test]
    fn test_batch_insert() {
        let builder = BatchInsertBuilder::new(TableName::CALL_GRAPH)
            .columns(vec!["caller", "callee", "file"])
            .add_row(vec!["main", "init", "src/main.rs"])
            .add_row(vec!["init", "setup", "src/init.rs"]);

        let statements = builder.build();
        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("('main', 'init', 'src/main.rs')"));
        assert!(statements[1].contains("('init', 'setup', 'src/init.rs')"));
    }
}
