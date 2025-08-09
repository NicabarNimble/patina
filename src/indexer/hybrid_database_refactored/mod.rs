// Dependable Rust: Black-box boundary for hybrid_database  
// Only public interface exposed - 641 lines hidden

use anyhow::Result;
use std::path::Path;

/// Hybrid database combining SQLite with optional CRDT support
pub struct HybridDatabase {
    inner: Box<implementation::HybridDatabaseImpl>,
}

impl HybridDatabase {
    /// Create a new hybrid database
    pub fn new(db_path: &Path, enable_crdt: bool) -> Result<Self> {
        Ok(Self {
            inner: Box::new(implementation::HybridDatabaseImpl::new(db_path, enable_crdt)?),
        })
    }

    /// Initialize the database schema
    pub fn initialize_schema(&self) -> Result<()> {
        self.inner.initialize_schema()
    }

    /// Store a document
    pub fn store_document(&self, doc: &super::navigation_state::DocumentInfo) -> Result<()> {
        self.inner.store_document(doc)
    }

    /// Get all patterns
    pub fn get_all_patterns(&self) -> Result<Vec<Pattern>> {
        self.inner.get_all_patterns()
    }
}

/// Simple pattern struct for external use
#[derive(Debug, Clone)]
pub struct Pattern {
    pub id: String,
    pub name: String,
    pub content: String,
}

// Everything else is private
mod implementation;