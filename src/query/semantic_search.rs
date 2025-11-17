//! Semantic search using embeddings and USearch
//!
//! Uses dual storage pattern: BeliefStorage and ObservationStorage.

use crate::embeddings::EmbeddingEngine;
use crate::storage::{
    Belief, BeliefMetadata, BeliefStorage, Observation, ObservationMetadata, ObservationStorage,
};
use anyhow::{Context, Result};
use std::path::Path;
use uuid::Uuid;

/// Semantic search engine
///
/// Wraps BeliefStorage, ObservationStorage, and EmbeddingEngine to provide high-level search API.
pub struct SemanticSearch {
    belief_storage: BeliefStorage,
    observation_storage: ObservationStorage,
    embedder: Box<dyn EmbeddingEngine>,
}

impl SemanticSearch {
    /// Create a new semantic search engine
    pub fn new<P: AsRef<Path>>(
        storage_path: P,
        embedder: Box<dyn EmbeddingEngine>,
    ) -> Result<Self> {
        let base_path = storage_path.as_ref();
        let dimension = embedder.dimension();

        let belief_storage = BeliefStorage::open(base_path.join("beliefs"))
            .context("Failed to open belief storage")?;

        let observation_storage =
            ObservationStorage::open_with_dimension(base_path.join("observations"), dimension)
                .context("Failed to open observation storage")?;

        Ok(Self {
            belief_storage,
            observation_storage,
            embedder,
        })
    }

    /// Open from default database path
    pub fn open_default() -> Result<Self> {
        let embedder = crate::embeddings::create_embedder()?;
        Self::new(".patina/storage", embedder)
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
        // Generate embedding (passage - belief being stored)
        let embedding = self
            .embedder
            .embed_passage(content)
            .context("Failed to generate embedding")?;

        // Create belief
        let belief = Belief {
            id: Uuid::new_v4(),
            content: content.to_string(),
            embedding,
            metadata: BeliefMetadata::default(),
        };

        // Store belief
        self.belief_storage
            .insert(&belief)
            .context("Failed to insert belief")?;

        // Persist index
        self.belief_storage
            .save_index()
            .context("Failed to save vector index")?;

        Ok(())
    }

    /// Add a new observation with automatic embedding
    ///
    /// # Arguments
    /// * `content` - The observation text to store
    /// * `observation_type` - Type: "pattern", "technology", "decision", or "challenge"
    pub fn add_observation(&mut self, content: &str, observation_type: &str) -> Result<()> {
        self.add_observation_with_metadata(
            content,
            observation_type,
            ObservationMetadata::default(),
        )
    }

    /// Add a new observation with custom metadata and automatic embedding
    ///
    /// # Arguments
    /// * `content` - The observation text to store
    /// * `observation_type` - Type: "pattern", "technology", "decision", or "challenge"
    /// * `metadata` - Custom metadata including source type and reliability
    pub fn add_observation_with_metadata(
        &mut self,
        content: &str,
        observation_type: &str,
        metadata: ObservationMetadata,
    ) -> Result<()> {
        self.add_observation_with_id(Uuid::new_v4(), content, observation_type, metadata)
    }

    /// Add observation with specific ID (for loading from database)
    ///
    /// Used when rebuilding index from existing observations in database.
    /// Preserves original IDs instead of generating new UUIDs.
    pub fn add_observation_with_id(
        &mut self,
        id: Uuid,
        content: &str,
        observation_type: &str,
        metadata: ObservationMetadata,
    ) -> Result<()> {
        // Generate embedding (passage - observation being stored)
        let embedding = self
            .embedder
            .embed_passage(content)
            .context("Failed to generate embedding")?;

        // Create observation with provided ID
        let observation = Observation {
            id,
            observation_type: observation_type.to_string(),
            content: content.to_string(),
            embedding,
            metadata,
        };

        // Store observation
        self.observation_storage
            .insert(&observation)
            .context("Failed to insert observation")?;

        // Persist index
        self.observation_storage
            .save_index()
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
        // Generate query embedding (query - user search)
        let query_embedding = self
            .embedder
            .embed_query(query)
            .context("Failed to generate query embedding")?;

        // Search using storage
        self.belief_storage
            .search(&query_embedding, top_k)
            .context("Failed to search belief vectors")
    }

    /// Search for observations using semantic similarity
    ///
    /// # Arguments
    /// * `query` - Query text to search for
    /// * `observation_type` - Optional filter by type ("pattern", "technology", "decision", "challenge")
    /// * `top_k` - Number of results to return
    ///
    /// # Returns
    /// Vector of Observation objects, sorted by similarity (highest first)
    pub fn search_observations(
        &mut self,
        query: &str,
        observation_type: Option<&str>,
        top_k: usize,
    ) -> Result<Vec<Observation>> {
        // Generate query embedding (query - user search)
        let query_embedding = self
            .embedder
            .embed_query(query)
            .context("Failed to generate query embedding")?;

        // Search using storage (with optional type filter)
        match observation_type {
            Some(obs_type) => {
                self.observation_storage
                    .search_by_type(&query_embedding, obs_type, top_k)
            }
            None => self.observation_storage.search(&query_embedding, top_k),
        }
        .context("Failed to search observation vectors")
    }

    /// Search observations with quality filtering
    ///
    /// Applies metadata-based filtering to improve result quality:
    /// - Filters by source type (session, session_distillation, documentation only)
    /// - Filters by reliability (>0.85)
    /// - Deduplicates by content
    ///
    /// # Arguments
    /// * `query` - Query text to search for
    /// * `observation_type` - Optional filter by type
    /// * `top_k` - Number of results to return after filtering
    ///
    /// # Returns
    /// Vector of high-quality Observation objects, deduplicated and filtered
    pub fn search_observations_filtered(
        &mut self,
        query: &str,
        observation_type: Option<&str>,
        top_k: usize,
    ) -> Result<Vec<Observation>> {
        // Get broader candidate set (4x to account for filtering)
        let candidates = self.search_observations(query, observation_type, top_k * 4)?;

        // Filter by source type and reliability
        let mut filtered: Vec<Observation> = candidates
            .into_iter()
            .filter(|obs| {
                // Only keep high-quality sources
                let source_type = obs.metadata.source_type.as_deref().unwrap_or("unknown");
                matches!(
                    source_type,
                    "session" | "session_distillation" | "documentation"
                )
            })
            .filter(|obs| {
                // Only keep high reliability observations
                obs.metadata.reliability.unwrap_or(0.0) > 0.85
            })
            .collect();

        // Deduplicate by content (keep first occurrence = highest similarity)
        let mut seen_content = std::collections::HashSet::new();
        filtered.retain(|obs| seen_content.insert(obs.content.clone()));

        // Return top K after filtering
        filtered.truncate(top_k);
        Ok(filtered)
    }

    /// Search observations with quality filtering and return similarity scores
    ///
    /// Same as search_observations_filtered but returns (Observation, similarity) tuples
    pub fn search_observations_filtered_with_scores(
        &mut self,
        query: &str,
        observation_type: Option<&str>,
        top_k: usize,
    ) -> Result<Vec<(Observation, f32)>> {
        // Generate query embedding (query - user search)
        let query_embedding = self
            .embedder
            .embed_query(query)
            .context("Failed to generate query embedding")?;

        // Get broader candidate set with scores (4x to account for filtering)
        let candidates = self
            .observation_storage
            .search_with_scores(&query_embedding, top_k * 4)
            .context("Failed to search with scores")?;

        // Apply type filter if specified
        let candidates: Vec<(Observation, f32)> = if let Some(obs_type) = observation_type {
            candidates
                .into_iter()
                .filter(|(obs, _)| obs.observation_type == obs_type)
                .collect()
        } else {
            candidates
        };

        // Filter by source type and reliability
        let mut filtered: Vec<(Observation, f32)> = candidates
            .into_iter()
            .filter(|(obs, _)| {
                // Only keep high-quality sources
                let source_type = obs.metadata.source_type.as_deref().unwrap_or("unknown");
                matches!(
                    source_type,
                    "session" | "session_distillation" | "documentation"
                )
            })
            .filter(|(obs, _)| {
                // Only keep high reliability observations
                obs.metadata.reliability.unwrap_or(0.0) > 0.85
            })
            .collect();

        // Deduplicate by content (keep first occurrence = highest similarity)
        let mut seen_content = std::collections::HashSet::new();
        filtered.retain(|(obs, _)| seen_content.insert(obs.content.clone()));

        // Return top K after filtering
        filtered.truncate(top_k);
        Ok(filtered)
    }

    /// Get reference to underlying belief storage (escape hatch)
    pub fn belief_storage(&self) -> &BeliefStorage {
        &self.belief_storage
    }

    /// Get mutable reference to underlying belief storage (escape hatch)
    pub fn belief_storage_mut(&mut self) -> &mut BeliefStorage {
        &mut self.belief_storage
    }

    /// Get reference to underlying observation storage (escape hatch)
    pub fn observation_storage(&self) -> &ObservationStorage {
        &self.observation_storage
    }

    /// Get mutable reference to underlying observation storage (escape hatch)
    pub fn observation_storage_mut(&mut self) -> &mut ObservationStorage {
        &mut self.observation_storage
    }

    /// Generate embedding for text using the underlying embedding engine
    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        self.embedder
            .embed(text)
            .context("Failed to generate embedding")
    }

    /// Generate embedding for a query (applies model-specific query formatting)
    pub fn embed_query(&mut self, text: &str) -> Result<Vec<f32>> {
        self.embedder
            .embed_query(text)
            .context("Failed to generate query embedding")
    }

    /// Generate embedding for a passage (applies model-specific passage formatting)
    pub fn embed_passage(&mut self, text: &str) -> Result<Vec<f32>> {
        self.embedder
            .embed_passage(text)
            .context("Failed to generate passage embedding")
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

        // Search - MockEmbedder generates deterministic but not semantically meaningful embeddings
        // So we just verify the search mechanism works, not the ranking quality
        let results = search.search_beliefs("rust programming language", 3)?;

        // Should find all beliefs
        assert_eq!(results.len(), 3, "Should find all 3 beliefs");

        // All results should have content
        for belief in &results {
            assert!(
                !belief.content.is_empty(),
                "Belief should have non-empty content"
            );
        }

        Ok(())
    }
}
