//! Query module - Semantic search and hybrid retrieval
//!
//! Provides semantic search capabilities using embeddings + USearch,
//! with dual SQLite + USearch storage for local-first vector search.

pub mod semantic_search;

pub use semantic_search::SemanticSearch;
