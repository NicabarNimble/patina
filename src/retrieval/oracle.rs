//! Oracle trait and types for multi-dimensional retrieval
//!
//! Oracles are internal retrieval strategies (not external adapters).
//! Each oracle queries one knowledge dimension and returns ranked results.

use anyhow::Result;

/// Result from a single oracle query
#[derive(Debug, Clone)]
pub struct OracleResult {
    /// Unique document identifier (for deduplication in fusion)
    pub doc_id: String,
    /// Content snippet or summary
    pub content: String,
    /// Source oracle name
    pub source: &'static str,
    /// Additional metadata
    pub metadata: OracleMetadata,
}

/// Metadata attached to oracle results
#[derive(Debug, Clone, Default)]
pub struct OracleMetadata {
    pub file_path: Option<String>,
    pub timestamp: Option<String>,
    pub event_type: Option<String>,
}

/// Oracle interface - each retrieval dimension implements this
///
/// This is a strategy pattern (not adapter pattern) because oracles
/// are internal retrieval mechanisms, not external system integrations.
pub trait Oracle: Send + Sync {
    /// Oracle name for provenance tracking
    fn name(&self) -> &'static str;

    /// Query the oracle, returning ranked results
    fn query(&self, query: &str, limit: usize) -> Result<Vec<OracleResult>>;

    /// Whether this oracle is available (index exists, etc.)
    fn is_available(&self) -> bool;
}
