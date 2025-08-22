use anyhow::Result;
use std::collections::HashMap;
use tree_sitter::Query;
use crate::{Metal, QueryType};

/// Load tree-sitter queries from .scm files
pub struct QueryLoader;

impl QueryLoader {
    /// Load all queries for available metals
    pub fn load_all_queries() -> HashMap<(Metal, QueryType), String> {
        let mut queries = HashMap::new();
        
        // Rust queries
        queries.insert((Metal::Rust, QueryType::Symbols), include_str!("../queries/rust/symbols.scm").to_string());
        queries.insert((Metal::Rust, QueryType::Complexity), include_str!("../queries/rust/complexity.scm").to_string());
        queries.insert((Metal::Rust, QueryType::Patterns), include_str!("../queries/rust/patterns.scm").to_string());
        
        // Go queries
        queries.insert((Metal::Go, QueryType::Symbols), include_str!("../queries/go/symbols.scm").to_string());
        queries.insert((Metal::Go, QueryType::Complexity), include_str!("../queries/go/complexity.scm").to_string());
        queries.insert((Metal::Go, QueryType::Patterns), include_str!("../queries/go/patterns.scm").to_string());
        
        queries
    }
    
    /// Load a specific query for a metal and type
    pub fn load_query(metal: Metal, query_type: QueryType) -> Result<Query> {
        let queries = Self::load_all_queries();
        
        let query_text = queries.get(&(metal, query_type))
            .ok_or_else(|| anyhow::anyhow!("No query for {:?} {:?}", metal, query_type))?;
        
        let language = metal.tree_sitter_language()
            .ok_or_else(|| anyhow::anyhow!("No language for {:?}", metal))?;
            
        Query::new(&language, query_text)
            .map_err(|e| anyhow::anyhow!("Failed to create query: {}", e))
    }
    
    /// Create a query directly from text
    pub fn create_query(metal: Metal, query_text: &str) -> Result<Query> {
        let language = metal.tree_sitter_language()
            .ok_or_else(|| anyhow::anyhow!("No language for {:?}", metal))?;
            
        Query::new(&language, query_text)
            .map_err(|e| anyhow::anyhow!("Failed to create query: {}", e))
    }
}