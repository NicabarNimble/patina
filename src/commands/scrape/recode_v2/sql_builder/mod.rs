// ============================================================================
// TYPED SQL BUILDER FOR DUCKDB
// ============================================================================
//! Type-safe SQL construction for the recode_v2 semantic extraction pipeline.
//! 
//! This module provides compile-time safe SQL generation specifically for
//! DuckDB, eliminating SQL injection risks and string concatenation errors.
//! 
//! ## Design Principles
//! - Type-safe: Leverage Rust's type system to prevent SQL injection
//! - DuckDB-specific: Optimized for DuckDB's SQL dialect and features
//! - Zero-cost: Compiles to efficient string building
//! - Domain-focused: Tailored for code intelligence storage

use std::fmt;

pub mod value;
pub mod insert;
pub mod schema;
pub mod query;

pub use value::{SqlValue, escape_string};
pub use insert::InsertBuilder;
pub use schema::{CreateTableBuilder, ColumnType};
pub use query::{DeleteBuilder, SelectBuilder};

// ============================================================================
// CORE TYPES
// ============================================================================

/// Type-safe table name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableName(&'static str);

impl TableName {
    // Pre-defined tables for our schema
    pub const CODE_SEARCH: Self = Self("code_search");
    pub const TYPE_VOCABULARY: Self = Self("type_vocabulary");
    pub const FUNCTION_FACTS: Self = Self("function_facts");
    pub const IMPORT_FACTS: Self = Self("import_facts");
    pub const DOCUMENTATION: Self = Self("documentation");
    pub const CALL_GRAPH: Self = Self("call_graph");
    pub const INDEX_STATE: Self = Self("index_state");
    pub const SKIPPED_FILES: Self = Self("skipped_files");
    
    pub fn as_str(&self) -> &str {
        self.0
    }
}

impl fmt::Display for TableName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type-safe column name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnName(&'static str);

impl ColumnName {
    pub fn new(name: &'static str) -> Self {
        Self(name)
    }
    
    pub fn as_str(&self) -> &str {
        self.0
    }
}

impl fmt::Display for ColumnName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// TRANSACTION BUILDER
// ============================================================================

/// Builds a complete transaction with multiple statements
pub struct TransactionBuilder {
    statements: Vec<String>,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
        }
    }
    
    pub fn add_statement(&mut self, sql: String) -> &mut Self {
        self.statements.push(sql);
        self
    }
    
    pub fn build(self) -> String {
        if self.statements.is_empty() {
            return String::new();
        }
        
        let mut result = String::with_capacity(
            self.statements.iter().map(|s| s.len() + 2).sum()
        );
        
        result.push_str("BEGIN TRANSACTION;\n");
        for statement in self.statements {
            result.push_str(&statement);
            if !statement.ends_with(';') {
                result.push(';');
            }
            result.push('\n');
        }
        result.push_str("COMMIT;\n");
        
        result
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}