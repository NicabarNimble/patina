// Dependable Rust: Black-box boundary for indexer
// Exposes domain primitives (Pattern, Location, Confidence, Layer)
// Hides implementation mechanisms (GitState internals, SqliteClient, HybridDatabase)

use anyhow::Result;
use std::path::Path;

// Re-export ONLY the domain primitives that consumers need
// These are the "format" - the language we speak about patterns
pub use crate::indexer::{
    Confidence,     // Domain primitive: How confident are we?
    GitState,       // Domain primitive: What's the git status?
    Layer,          // Domain primitive: Which layer (Core/Surface/Dust)?
    Location,       // Domain primitive: Where is the pattern?
    NavigationResponse, // Domain primitive: Navigation results
    Pattern,        // Domain primitive: The pattern itself
};

/// Pattern indexer facade - hides all implementation complexity
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
    pub fn index_directory(&self, dir: &Path) -> Result<()> {
        self.inner.index_directory(dir)
    }

    /// Navigate to find patterns matching a query
    pub fn navigate(&self, query: &str) -> NavigationResponse {
        self.inner.navigate(query)
    }
}

// Everything else is private - all implementation hidden
mod implementation {
    use super::*;
    use crate::indexer as original;

    pub(super) struct IndexerImpl {
        // We just delegate to the original indexer
        // This allows us to completely rewrite the implementation later
        // without changing the public API
        indexer: original::PatternIndexer,
    }

    impl IndexerImpl {
        pub(super) fn new() -> Result<Self> {
            Ok(Self {
                indexer: original::PatternIndexer::new()?,
            })
        }

        pub(super) fn with_database(db_path: &Path) -> Result<Self> {
            Ok(Self {
                indexer: original::PatternIndexer::with_database(db_path)?,
            })
        }

        pub(super) fn with_hybrid_database(db_path: &Path, enable_crdt: bool) -> Result<Self> {
            Ok(Self {
                indexer: original::PatternIndexer::with_hybrid_database(db_path, enable_crdt)?,
            })
        }

        pub(super) fn index_directory(&self, dir: &Path) -> Result<()> {
            self.indexer.index_directory(dir)
        }

        pub(super) fn navigate(&self, query: &str) -> NavigationResponse {
            self.indexer.navigate(query)
        }
    }
}