//! Retrieval module - multi-oracle knowledge retrieval with RRF fusion
//!
//! Public interface:
//! - `QueryEngine` for parallel multi-oracle queries
//! - `RetrievalConfig` for tuning RRF parameters
//! - `FusedResult` for query results (includes per-oracle contributions)
//! - `OracleContribution` for per-oracle rank and score details
//!
//! Internal (not exported):
//! - `Oracle` trait and implementations (semantic, lexical, persona)
//! - RRF fusion algorithm

mod engine;
mod fusion;
mod oracle;
mod oracles;

pub use engine::{QueryEngine, QueryOptions, RetrievalConfig};
pub use fusion::FusedResult;

// Re-export types for MCP JSON serialization and annotations
#[allow(unused_imports)]
pub use fusion::{OracleContribution, StructuralAnnotations};
