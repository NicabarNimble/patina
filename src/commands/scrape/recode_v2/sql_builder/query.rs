// ============================================================================
// QUERY BUILDERS (SELECT, DELETE)
// ============================================================================
//! Type-safe query construction for DuckDB.

use super::{TableName, SqlValue};

/// Builder for DELETE statements
pub struct DeleteBuilder {
    table: TableName,
    where_clauses: Vec<String>,
}

impl DeleteBuilder {
    /// Create a new DELETE builder
    pub fn new(table: TableName) -> Self {
        Self {
            table,
            where_clauses: Vec::new(),
        }
    }
    
    /// Add a WHERE clause with equality check
    pub fn where_eq<V: Into<SqlValue>>(mut self, column: &str, value: V) -> Self {
        let sql_value = value.into();
        self.where_clauses.push(format!("{} = {}", column, sql_value.to_sql()));
        self
    }
    
    /// Add a WHERE IN clause for multiple values
    pub fn where_in<V: Into<SqlValue>>(mut self, column: &str, values: Vec<V>) -> Self {
        if values.is_empty() {
            // Empty IN clause would be invalid SQL
            self.where_clauses.push("FALSE".to_string());
        } else {
            let sql_values: Vec<String> = values
                .into_iter()
                .map(|v| v.into().to_sql())
                .collect();
            self.where_clauses.push(format!("{} IN ({})", column, sql_values.join(", ")));
        }
        self
    }
    
    /// Build the final SQL statement
    pub fn build(self) -> String {
        let mut sql = format!("DELETE FROM {}", self.table.as_str());
        
        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.where_clauses.join(" AND "));
        }
        
        sql
    }
}

/// Builder for SELECT statements
pub struct SelectBuilder {
    table: TableName,
    columns: Vec<String>,
    where_clauses: Vec<String>,
    limit: Option<usize>,
}

impl SelectBuilder {
    /// Create a new SELECT builder
    pub fn new(table: TableName) -> Self {
        Self {
            table,
            columns: vec!["*".to_string()],
            where_clauses: Vec::new(),
            limit: None,
        }
    }
    
    /// Specify columns to select
    pub fn columns<S: Into<String>>(mut self, cols: impl IntoIterator<Item = S>) -> Self {
        self.columns = cols.into_iter().map(|s| s.into()).collect();
        self
    }
    
    /// Add a WHERE clause with equality check
    pub fn where_eq<V: Into<SqlValue>>(mut self, column: &str, value: V) -> Self {
        let sql_value = value.into();
        self.where_clauses.push(format!("{} = {}", column, sql_value.to_sql()));
        self
    }
    
    /// Add a WHERE IN clause
    pub fn where_in<V: Into<SqlValue>>(mut self, column: &str, values: Vec<V>) -> Self {
        if values.is_empty() {
            self.where_clauses.push("FALSE".to_string());
        } else {
            let sql_values: Vec<String> = values
                .into_iter()
                .map(|v| v.into().to_sql())
                .collect();
            self.where_clauses.push(format!("{} IN ({})", column, sql_values.join(", ")));
        }
        self
    }
    
    /// Add a LIMIT clause
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
    
    /// Build the final SQL statement
    pub fn build(self) -> String {
        let mut sql = String::with_capacity(256);
        
        sql.push_str("SELECT ");
        sql.push_str(&self.columns.join(", "));
        sql.push_str(" FROM ");
        sql.push_str(self.table.as_str());
        
        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.where_clauses.join(" AND "));
        }
        
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        sql
    }
}

/// Special builder for the index_state query pattern used in incremental updates
pub struct IndexStateQueryBuilder;

impl IndexStateQueryBuilder {
    /// Build the specific query used for incremental updates
    pub fn build_mtime_query() -> String {
        "SELECT path || '|' || CAST(mtime AS VARCHAR) FROM index_state".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_delete_simple() {
        let sql = DeleteBuilder::new(TableName::CODE_SEARCH)
            .where_eq("path", "src/old.rs")
            .build();
        
        assert_eq!(sql, "DELETE FROM code_search WHERE path = 'src/old.rs'");
    }
    
    #[test]
    fn test_delete_with_in() {
        let sql = DeleteBuilder::new(TableName::INDEX_STATE)
            .where_in("path", vec!["src/a.rs", "src/b.rs"])
            .build();
        
        assert_eq!(sql, "DELETE FROM index_state WHERE path IN ('src/a.rs', 'src/b.rs')");
    }
    
    #[test]
    fn test_select_all() {
        let sql = SelectBuilder::new(TableName::FUNCTION_FACTS).build();
        assert_eq!(sql, "SELECT * FROM function_facts");
    }
    
    #[test]
    fn test_select_with_columns() {
        let sql = SelectBuilder::new(TableName::TYPE_VOCABULARY)
            .columns(vec!["name", "kind"])
            .where_eq("file", "src/types.rs")
            .limit(10)
            .build();
        
        assert_eq!(
            sql,
            "SELECT name, kind FROM type_vocabulary WHERE file = 'src/types.rs' LIMIT 10"
        );
    }
    
    #[test]
    fn test_index_state_query() {
        let sql = IndexStateQueryBuilder::build_mtime_query();
        assert_eq!(sql, "SELECT path || '|' || CAST(mtime AS VARCHAR) FROM index_state");
    }
}