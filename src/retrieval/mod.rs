//! Retrieval module - semantic vector search
//!
//! After the semantic-structural split, this module handles semantic queries only.
//! Factual search (FTS5, temporal, belief, persona) lives in `commands::assay`.
//!
//! Public interface:
//! - `QueryEngine` for semantic vector search with multi-repo federation
//! - `RetrievalConfig` for tuning search parameters
//! - `FusedResult` for query results (single-oracle, no fusion)
//!
//! Internal:
//! - `Oracle` trait and SemanticOracle implementation
//! - Fusion types kept for backward compatibility (MCP, eval, bench)

mod engine;
pub(crate) mod enrichment;
mod fusion;
mod oracle;
mod oracles;
#[cfg(test)]
mod snippet;

pub use engine::{QueryEngine, QueryOptions, RetrievalConfig};
pub use fusion::FusedResult;

// Re-export types for MCP JSON serialization and annotations
#[allow(unused_imports)]
pub use fusion::{OracleContribution, StructuralAnnotations};
