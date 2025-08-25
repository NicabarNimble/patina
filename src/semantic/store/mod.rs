pub mod duckdb;

use anyhow::Result;
use crate::semantic::extractor::{
    ProcessingResult,
    FunctionFact,
    DocumentationFact,
    CallRelation,
};

/// Symbol information returned from queries
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub file: String,
    pub line_number: usize,
    pub symbol_type: String,
    pub doc_summary: Option<String>,
    pub parameters: Option<String>,
    pub return_type: Option<String>,
}

/// Trait for knowledge store implementations
pub trait KnowledgeStore {
    /// Initialize the database schema
    fn initialize(&self) -> Result<()>;
    
    /// Store extracted processing results
    fn store_results(&self, results: &ProcessingResult, file_path: &str) -> Result<()>;
    
    /// Query symbols by keywords
    fn query_by_keywords(&self, keywords: &[&str]) -> Result<Vec<Symbol>>;
    
    /// Get call graph for a symbol
    fn get_call_graph(&self, symbol: &str) -> Result<Vec<CallRelation>>;
    
    /// Get recursive call chain
    fn get_call_chain(&self, entry_point: &str, max_depth: usize) -> Result<Vec<String>>;
    
    /// Get documentation for a symbol
    fn get_documentation(&self, symbol: &str) -> Result<Option<DocumentationFact>>;
    
    /// Get function facts for a symbol
    fn get_function_facts(&self, symbol: &str) -> Result<Option<FunctionFact>>;
    
    /// Execute arbitrary SQL query (for advanced use)
    fn execute_query(&self, query: &str) -> Result<String>;
}