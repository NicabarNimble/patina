//! Semantic search using embeddings and sqlite-vss

use crate::embeddings::EmbeddingEngine;
use anyhow::{Context, Result};
use rusqlite::Connection;

/// Result of a belief search: (belief_id, similarity_score)
pub type BeliefSearchResult = (i64, f32);

/// Result of an observation search: (observation_id, observation_type, similarity_score)
pub type ObservationSearchResult = (i64, String, f32);

/// Search for beliefs using semantic similarity
///
/// # Arguments
/// * `conn` - SQLite database connection (must have vss0 extension loaded)
/// * `query` - Query text to search for
/// * `embedder` - Embedding engine to generate query vector
/// * `top_k` - Number of results to return
///
/// # Returns
/// Vector of (belief_id, similarity_score) tuples, sorted by similarity (highest first)
///
/// # Example
/// ```no_run
/// use patina::embeddings::create_embedder;
/// use patina::query::search_beliefs;
/// use rusqlite::Connection;
///
/// let conn = Connection::open(".patina/db/facts.db")?;
/// let mut embedder = create_embedder()?;
/// let results = search_beliefs(&conn, "prefer rust for cli tools", &mut *embedder, 10)?;
///
/// for (belief_id, similarity) in results {
///     println!("Belief {} - similarity: {:.3}", belief_id, similarity);
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn search_beliefs(
    conn: &Connection,
    query: &str,
    embedder: &mut dyn EmbeddingEngine,
    top_k: usize,
) -> Result<Vec<BeliefSearchResult>> {
    // Generate query embedding
    let query_embedding = embedder
        .embed(query)
        .context("Failed to generate query embedding")?;

    // Convert Vec<f32> to bytes for SQLite blob
    let embedding_bytes = vec_f32_to_bytes(&query_embedding);

    // Search using sqlite-vss
    // Note: vss_search returns distance (lower is better)
    // We convert to similarity: similarity = 1.0 / (1.0 + distance)
    let mut stmt = conn
        .prepare(
            "SELECT belief_id, distance
             FROM belief_vectors
             WHERE vss_search(embedding, ?)
             LIMIT ?",
        )
        .context("Failed to prepare belief search query")?;

    let results = stmt
        .query_map(rusqlite::params![&embedding_bytes[..], top_k], |row| {
            let belief_id: i64 = row.get(0)?;
            let distance: f32 = row.get(1)?;
            let similarity = distance_to_similarity(distance);
            Ok((belief_id, similarity))
        })
        .context("Failed to execute belief search query")?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to collect belief search results")?;

    Ok(results)
}

/// Search for observations using semantic similarity
///
/// # Arguments
/// * `conn` - SQLite database connection (must have vss0 extension loaded)
/// * `query` - Query text to search for
/// * `observation_type` - Optional filter by observation type ('pattern', 'technology', 'decision', 'challenge')
/// * `embedder` - Embedding engine to generate query vector
/// * `top_k` - Number of results to return
///
/// # Returns
/// Vector of (observation_id, observation_type, similarity_score) tuples, sorted by similarity (highest first)
///
/// # Example
/// ```no_run
/// use patina::embeddings::create_embedder;
/// use patina::query::search_observations;
/// use rusqlite::Connection;
///
/// let conn = Connection::open(".patina/db/facts.db")?;
/// let mut embedder = create_embedder()?;
///
/// // Search all observations
/// let results = search_observations(&conn, "security patterns", None, &mut *embedder, 10)?;
///
/// // Search only patterns
/// let patterns = search_observations(&conn, "security patterns", Some("pattern"), &mut *embedder, 10)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn search_observations(
    conn: &Connection,
    query: &str,
    observation_type: Option<&str>,
    embedder: &mut dyn EmbeddingEngine,
    top_k: usize,
) -> Result<Vec<ObservationSearchResult>> {
    // Generate query embedding
    let query_embedding = embedder
        .embed(query)
        .context("Failed to generate query embedding")?;

    // Convert Vec<f32> to bytes for SQLite blob
    let embedding_bytes = vec_f32_to_bytes(&query_embedding);

    // Build query with optional type filter
    let sql = if observation_type.is_some() {
        "SELECT observation_id, observation_type, distance
         FROM observation_vectors
         WHERE vss_search(embedding, ?) AND observation_type = ?
         LIMIT ?"
    } else {
        "SELECT observation_id, observation_type, distance
         FROM observation_vectors
         WHERE vss_search(embedding, ?)
         LIMIT ?"
    };

    let mut stmt = conn
        .prepare(sql)
        .context("Failed to prepare observation search query")?;

    let results = if let Some(obs_type) = observation_type {
        stmt.query_map(
            rusqlite::params![&embedding_bytes[..], obs_type, top_k],
            |row| {
                let observation_id: i64 = row.get(0)?;
                let observation_type: String = row.get(1)?;
                let distance: f32 = row.get(2)?;
                let similarity = distance_to_similarity(distance);
                Ok((observation_id, observation_type, similarity))
            },
        )
        .context("Failed to execute observation search query")?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to collect observation search results")?
    } else {
        stmt.query_map(rusqlite::params![&embedding_bytes[..], top_k], |row| {
            let observation_id: i64 = row.get(0)?;
            let observation_type: String = row.get(1)?;
            let distance: f32 = row.get(2)?;
            let similarity = distance_to_similarity(distance);
            Ok((observation_id, observation_type, similarity))
        })
        .context("Failed to execute observation search query")?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to collect observation search results")?
    };

    Ok(results)
}

/// Convert f32 vector to bytes for SQLite blob
pub fn vec_f32_to_bytes(vec: &[f32]) -> Vec<u8> {
    vec.iter().flat_map(|&f| f.to_le_bytes()).collect()
}

/// Convert distance to similarity score
///
/// sqlite-vss returns distance (lower is better).
/// We convert to similarity where higher is better: similarity = 1.0 / (1.0 + distance)
///
/// This gives a score in range [0, 1]:
/// - distance = 0 → similarity = 1.0 (perfect match)
/// - distance = 1 → similarity = 0.5
/// - distance = ∞ → similarity = 0.0
pub fn distance_to_similarity(distance: f32) -> f32 {
    1.0 / (1.0 + distance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_distance_to_similarity() {
        assert_relative_eq!(distance_to_similarity(0.0), 1.0, epsilon = 0.001);
        assert_relative_eq!(distance_to_similarity(1.0), 0.5, epsilon = 0.001);
        assert_relative_eq!(distance_to_similarity(9.0), 0.1, epsilon = 0.001);
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
