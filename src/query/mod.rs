//! Query module - Semantic search and hybrid retrieval
//!
//! Provides semantic search capabilities using embeddings + sqlite-vec,
//! and hybrid retrieval combining semantic search with Prolog reasoning.
//!
//! Refactored to follow scrape/code pattern with concrete types.

pub mod semantic_search;

pub use semantic_search::{BeliefSearchResult, ObservationSearchResult, SemanticSearch};
