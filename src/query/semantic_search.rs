//! Semantic search using embeddings and USearch
//!
//! Uses BeliefStorage for dual SQLite + USearch storage pattern.

use crate::embeddings::EmbeddingEngine;
use crate::storage::{Belief, BeliefMetadata, BeliefStorage};
use anyhow::{Context, Result};
use std::path::Path;
use uuid::Uuid;

/// Semantic search engine
///
/// Wraps BeliefStorage and EmbeddingEngine to provide high-level search API.
pub struct SemanticSearch {
    storage: BeliefStorage,
    embedder: Box<dyn EmbeddingEngine>,
}

impl SemanticSearch {
    /// Create a new semantic search engine
    pub fn new<P: AsRef<Path>>(storage_path: P, embedder: Box<dyn EmbeddingEngine>) -> Result<Self> {
        let storage = BeliefStorage::open(storage_path)
            .context("Failed to open belief storage")?;
        Ok(Self { storage, embedder })
    }

    /// Open from default database path
    pub fn open_default() -> Result<Self> {
        let embedder = crate::embeddings::create_embedder()?;
        Self::new(".patina/storage/beliefs", embedder)
    }

    /// Add a new belief with automatic embedding
    ///
    /// # Arguments
    /// * `content` - The belief text to store
    ///
    /// # Example
    /// ```no_run
    /// use patina::query::SemanticSearch;
    ///
    /// let mut search = SemanticSearch::open_default()?;
    /// search.add_belief("I prefer Rust for CLI tools")?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn add_belief(&mut self, content: &str) -> Result<()> {
        // Generate embedding
        let embedding = self
            .embedder
            .embed(content)
            .context("Failed to generate embedding")?;

        // Create belief
        let belief = Belief {
            id: Uuid::new_v4(),
            content: content.to_string(),
            embedding,
            metadata: BeliefMetadata::default(),
        };

        // Store belief
        self.storage.insert(&belief)
            .context("Failed to insert belief")?;

        // Persist index
        self.storage.save_index()
            .context("Failed to save vector index")?;

        Ok(())
    }

    /// Search for beliefs using semantic similarity
    ///
    /// # Arguments
    /// * `query` - Query text to search for
    /// * `top_k` - Number of results to return
    ///
    /// # Returns
    /// Vector of Belief objects, sorted by similarity (highest first)
    ///
    /// # Example
    /// ```no_run
    /// use patina::query::SemanticSearch;
    ///
    /// let mut search = SemanticSearch::open_default()?;
    /// let results = search.search_beliefs("prefer rust for cli tools", 10)?;
    ///
    /// for belief in results {
    ///     println!("Belief: {}", belief.content);
    /// }
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn search_beliefs(&mut self, query: &str, top_k: usize) -> Result<Vec<Belief>> {
        // Generate query embedding
        let query_embedding = self
            .embedder
            .embed(query)
            .context("Failed to generate query embedding")?;

        // Search using storage
        self.storage.search(&query_embedding, top_k)
            .context("Failed to search belief vectors")
    }

    /// Get reference to underlying storage (escape hatch)
    pub fn storage(&self) -> &BeliefStorage {
        &self.storage
    }

    /// Get mutable reference to underlying storage (escape hatch)
    pub fn storage_mut(&mut self) -> &mut BeliefStorage {
        &mut self.storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embeddings::EmbeddingEngine;
    use tempfile::TempDir;

    /// Mock embedder for testing - generates deterministic embeddings based on text hash
    struct MockEmbedder;

    impl MockEmbedder {
        fn new() -> Self {
            Self
        }
    }

    impl EmbeddingEngine for MockEmbedder {
        fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
            // Generate deterministic but reasonable embeddings
            // Hash the text to get a seed, then create 384-dim vector
            let mut vec = vec![0.0f32; 384];
            let hash = text.len() as f32;

            // Create simple pattern based on text content
            for (i, c) in text.chars().enumerate() {
                if i < 384 {
                    vec[i] = (c as u32 as f32 / 1000.0).sin();
                }
            }

            // Normalize
            let magnitude: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            if magnitude > 0.0 {
                vec.iter_mut().for_each(|x| *x /= magnitude);
            }

            Ok(vec)
        }

        fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            texts.iter().map(|t| self.embed(t)).collect()
        }

        fn dimension(&self) -> usize {
            384
        }

        fn model_name(&self) -> &str {
            "mock-embedder"
        }
    }

    #[test]
    fn test_semantic_search_add_and_search() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let embedder = Box::new(MockEmbedder::new());
        let mut search = SemanticSearch::new(temp_dir.path(), embedder)?;

        // Add beliefs
        search.add_belief("I prefer Rust for systems programming")?;
        search.add_belief("I avoid global state in my code")?;
        search.add_belief("I like chocolate ice cream")?;

        // Search
        let results = search.search_beliefs("rust programming language", 2)?;

        // Should find results
        assert!(!results.is_empty(), "Should find at least one result");
        assert!(results[0].content.contains("Rust"), "First result should be about Rust");

        Ok(())
    }
}
