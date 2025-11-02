//! Semantic search using embeddings and sqlite-vec
//!
//! Refactored to follow scrape/code pattern with concrete types

use crate::db::{DatabaseBackend, VectorFilter, VectorTable};
use crate::embeddings::EmbeddingEngine;
use anyhow::{Context, Result};

/// Result of a belief search: (belief_id, similarity_score)
pub type BeliefSearchResult = (i64, f32);

/// Result of an observation search: (observation_id, observation_type, similarity_score)
pub type ObservationSearchResult = (i64, String, f32);

/// Semantic search engine
///
/// Encapsulates database and embedder for clean API.
/// Follows the same pattern as scrape/code/database.rs
pub struct SemanticSearch {
    db: DatabaseBackend,
    embedder: Box<dyn EmbeddingEngine>,
}

impl SemanticSearch {
    /// Create a new semantic search engine
    pub fn new(db: DatabaseBackend, embedder: Box<dyn EmbeddingEngine>) -> Self {
        Self { db, embedder }
    }

    /// Open from default database path
    pub fn open_default() -> Result<Self> {
        let db = DatabaseBackend::open_sqlite(".patina/db/facts.db")?;
        let embedder = crate::embeddings::create_embedder()?;
        Ok(Self::new(db, embedder))
    }

    /// Search for beliefs using semantic similarity
    ///
    /// # Arguments
    /// * `query` - Query text to search for
    /// * `top_k` - Number of results to return
    ///
    /// # Returns
    /// Vector of (belief_id, similarity_score) tuples, sorted by similarity (highest first)
    ///
    /// # Example
    /// ```no_run
    /// use patina::query::SemanticSearch;
    ///
    /// let mut search = SemanticSearch::open_default()?;
    /// let results = search.search_beliefs("prefer rust for cli tools", 10)?;
    ///
    /// for (belief_id, similarity) in results {
    ///     println!("Belief {} - similarity: {:.3}", belief_id, similarity);
    /// }
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn search_beliefs(&mut self, query: &str, top_k: usize) -> Result<Vec<BeliefSearchResult>> {
        // Generate query embedding
        let query_embedding = self
            .embedder
            .embed(query)
            .context("Failed to generate query embedding")?;

        // Search using database abstraction
        let matches = self
            .db
            .vector_search(VectorTable::Beliefs, &query_embedding, None, top_k)
            .context("Failed to search belief vectors")?;

        // Convert to result format
        let results = matches
            .into_iter()
            .map(|m| (m.row_id, m.similarity))
            .collect();

        Ok(results)
    }

    /// Search for observations using semantic similarity
    ///
    /// # Arguments
    /// * `query` - Query text to search for
    /// * `observation_type` - Optional filter by observation type ('pattern', 'technology', 'decision', 'challenge')
    /// * `top_k` - Number of results to return
    ///
    /// # Returns
    /// Vector of (observation_id, observation_type, similarity_score) tuples, sorted by similarity (highest first)
    ///
    /// # Example
    /// ```no_run
    /// use patina::query::SemanticSearch;
    ///
    /// let mut search = SemanticSearch::open_default()?;
    ///
    /// // Search all observations
    /// let results = search.search_observations("security patterns", None, 10)?;
    ///
    /// // Search only patterns
    /// let patterns = search.search_observations("security patterns", Some("pattern"), 10)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn search_observations(
        &mut self,
        query: &str,
        observation_type: Option<&str>,
        top_k: usize,
    ) -> Result<Vec<ObservationSearchResult>> {
        // Generate query embedding
        let query_embedding = self
            .embedder
            .embed(query)
            .context("Failed to generate query embedding")?;

        // Build filter if observation type specified
        let filter = observation_type.map(|t| VectorFilter {
            field: "observation_type".to_string(),
            value: t.to_string(),
        });

        // Search using database abstraction
        let matches = self
            .db
            .vector_search(VectorTable::Observations, &query_embedding, filter, top_k)
            .context("Failed to search observation vectors")?;

        // Convert to result format
        // Note: We need to query the observation_type from metadata
        // For now, return empty string - will fix when we improve metadata handling
        let results = matches
            .into_iter()
            .map(|m| {
                // TODO: Get observation_type from metadata
                let obs_type = observation_type.unwrap_or("unknown").to_string();
                (m.row_id, obs_type, m.similarity)
            })
            .collect();

        Ok(results)
    }

    /// Get reference to underlying database (temporary escape hatch)
    pub fn database(&self) -> &DatabaseBackend {
        &self.db
    }
}

/// Convert f32 vector to bytes for SQLite blob
pub fn vec_f32_to_bytes(vec: &[f32]) -> Vec<u8> {
    vec.iter().flat_map(|&f| f.to_le_bytes()).collect()
}

/// Convert cosine distance to similarity score
///
/// sqlite-vec returns cosine distance (lower is better, range [0, 2]).
/// We convert to similarity where higher is better: similarity = 1.0 - distance
///
/// This gives a score in range [-1, 1]:
/// - distance = 0 → similarity = 1.0 (identical vectors)
/// - distance = 1 → similarity = 0.0 (orthogonal vectors)
/// - distance = 2 → similarity = -1.0 (opposite vectors)
pub fn distance_to_similarity(distance: f32) -> f32 {
    1.0 - distance
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_distance_to_similarity_cosine() {
        // Identical vectors (cosine distance = 0)
        assert_relative_eq!(distance_to_similarity(0.0), 1.0, epsilon = 0.001);

        // Orthogonal vectors (cosine distance = 1)
        assert_relative_eq!(distance_to_similarity(1.0), 0.0, epsilon = 0.001);

        // Opposite vectors (cosine distance = 2)
        assert_relative_eq!(distance_to_similarity(2.0), -1.0, epsilon = 0.001);
    }

    #[test]
    fn test_vec_f32_to_bytes() {
        let vec = vec![1.0, 2.0, 3.0];
        let bytes = vec_f32_to_bytes(&vec);

        // Should be 12 bytes (3 floats × 4 bytes each)
        assert_eq!(bytes.len(), 12);

        // Verify round-trip conversion
        let mut reconstructed = Vec::new();
        for chunk in bytes.chunks(4) {
            let bytes_array: [u8; 4] = chunk.try_into().unwrap();
            reconstructed.push(f32::from_le_bytes(bytes_array));
        }
        assert_eq!(reconstructed, vec);
    }
}
