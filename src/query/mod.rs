//! Query module - Semantic search and hybrid retrieval
//!
//! Provides semantic search capabilities using embeddings + sqlite-vss,
//! and hybrid retrieval combining semantic search with Prolog reasoning.

mod semantic_search;

pub use semantic_search::{search_beliefs, search_observations};
