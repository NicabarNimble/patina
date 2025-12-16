//! Retrieval module - multi-oracle knowledge retrieval with RRF fusion
//!
//! Public interface:
//! - `QueryEngine` for parallel multi-oracle queries
//! - `RetrievalConfig` for tuning RRF parameters
//! - `FusedResult` for query results (includes metadata)
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
