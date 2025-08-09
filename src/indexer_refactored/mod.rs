// Dependable Rust: Black-box boundary for indexer
// This is the ONLY public interface - all 17 exports are now hidden

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Navigation result from pattern search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationResult {
    pub query: String,
    pub locations: Vec<LocationInfo>,
}

/// Location information for a pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationInfo {
    pub path: PathBuf,
    pub line: Option<usize>,
    pub confidence: f32,
    pub preview: Option<String>,
}

/// Main pattern indexer facade - hides all complexity
pub struct PatternIndexer {
    inner: Box<implementation::IndexerImpl>,
}

impl PatternIndexer {
    /// Create a new pattern indexer without database
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: Box::new(implementation::IndexerImpl::new()?),
        })
    }

    /// Create indexer with SQLite database
    pub fn with_database(db_path: &Path) -> Result<Self> {
        Ok(Self {
            inner: Box::new(implementation::IndexerImpl::with_database(db_path)?),
        })
    }

    /// Create indexer with hybrid database (SQLite + optional CRDT)
    pub fn with_hybrid_database(db_path: &Path, enable_crdt: bool) -> Result<Self> {
        Ok(Self {
            inner: Box::new(implementation::IndexerImpl::with_hybrid_database(
                db_path,
                enable_crdt,
            )?),
        })
    }

    /// Index a directory of patterns
    pub fn index_directory(&self, path: &Path) -> Result<()> {
        self.inner.index_directory(path)
    }

    /// Navigate to find patterns
    pub fn navigate(&self, query: &str) -> NavigationResult {
        self.inner.navigate(query)
    }

    /// Clear the index
    pub fn clear(&mut self) -> Result<()> {
        self.inner.clear()
    }
}

// Everything else is private - all submodules hidden
mod implementation;

// Re-export the implementation module as the entire indexer 
pub(super) use implementation::*;