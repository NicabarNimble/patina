use anyhow::Result;
use tree_sitter::Query;
use crate::{Metal, QueryType};

/// Load tree-sitter queries from .scm files
pub struct QueryLoader;

impl QueryLoader {
    /// Load a query for a specific metal and type
    pub fn load_query(metal: Metal, query_type: QueryType) -> Result<Query> {
        let query_text = match (metal, query_type) {
            (Metal::Rust, QueryType::Symbols) => RUST_SYMBOLS_QUERY,
            (Metal::Solidity, QueryType::Symbols) => SOLIDITY_SYMBOLS_QUERY,
            _ => DEFAULT_QUERY,
        };
        
        let language = metal.tree_sitter_language()
            .ok_or_else(|| anyhow::anyhow!("No language for {:?}", metal))?;
            
        Query::new(&language, query_text)
            .map_err(|e| anyhow::anyhow!("Failed to create query: {}", e))
    }
}

// Default queries embedded for now (later we'll load from .scm files)

const RUST_SYMBOLS_QUERY: &str = r#"
(function_item
  name: (identifier) @function.name) @function

(struct_item
  name: (type_identifier) @struct.name) @struct

(trait_item
  name: (type_identifier) @trait.name) @trait

(impl_item
  trait: (type_identifier)? @impl.trait
  type: (type_identifier) @impl.type) @impl
"#;

const SOLIDITY_SYMBOLS_QUERY: &str = r#"
(contract_declaration
  name: (identifier) @contract.name) @contract

(function_definition
  name: (identifier) @function.name) @function

(event_definition
  name: (identifier) @event.name) @event

(modifier_definition
  name: (identifier) @modifier.name) @modifier
"#;

const DEFAULT_QUERY: &str = r#"
(identifier) @name
"#;