//! Belief storage using SQLite + USearch hybrid approach
//!
//! SQLite stores the source of truth (belief content, metadata).
//! USearch provides fast vector similarity search via HNSW indices.

use crate::storage::types::{Belief, BeliefMetadata};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};
use uuid::Uuid;

/// Dual storage for beliefs: SQLite + USearch
pub struct BeliefStorage {
    vectors: Index,
    db: Connection,
    index_path: PathBuf,
}

impl BeliefStorage {
    /// Open or create belief storage at the given path
    ///
    /// Creates two files:
    /// - `{path}/beliefs.db` - SQLite database
    /// - `{path}/beliefs.usearch` - USearch vector index
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let base = path.as_ref();
        std::fs::create_dir_all(base)?;

        // Open SQLite database
        let db_path = base.join("beliefs.db");
        let db = Connection::open(&db_path).context("Failed to open SQLite database")?;

        Self::init_schema(&db)?;

        // Configure USearch index
        let options = IndexOptions {
            dimensions: 384,         // all-MiniLM-L6-v2 embedding dimension
            metric: MetricKind::Cos, // Cosine similarity
            quantization: ScalarKind::F32,
            ..Default::default()
        };

        let index = Index::new(&options).context("Failed to create USearch index")?;

        // Reserve initial capacity
        index.reserve(1000)?;

        let index_path = base.join("beliefs.usearch");

        // Load existing index if present
        // Note: .view() creates immutable index - cannot add new vectors
        if index_path.exists() {
            index
                .load(index_path.to_str().unwrap())
                .context("Failed to load existing USearch index")?;
        }

        Ok(Self {
            vectors: index,
            db,
            index_path,
        })
    }

    /// Initialize SQLite schema
    fn init_schema(db: &Connection) -> Result<()> {
        db.execute(
            "CREATE TABLE IF NOT EXISTS beliefs (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT UNIQUE NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    /// Insert a belief into both SQLite and USearch
    ///
    /// Beliefs are immutable - duplicate UUIDs will result in an error.
    pub fn insert(&mut self, belief: &Belief) -> Result<()> {
        // Insert into SQLite (source of truth) and get rowid atomically
        let metadata_json = serde_json::to_string(&belief.metadata)?;
        let created_at = belief
            .metadata
            .created_at
            .unwrap_or_else(chrono::Utc::now)
            .to_rfc3339();

        let rowid: i64 = self.db.query_row(
            "INSERT INTO beliefs (id, content, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4)
             RETURNING rowid",
            params![
                belief.id.to_string(),
                &belief.content,
                metadata_json,
                created_at,
            ],
            |row| row.get(0),
        )?;

        // Insert into USearch (vector index) using rowid as key
        self.vectors
            .add(rowid as u64, &belief.embedding)
            .context("Failed to add vector to USearch index")?;

        Ok(())
    }

    /// Search for beliefs using vector similarity
    ///
    /// Returns top-k most similar beliefs based on query embedding.
    pub fn search(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<Belief>> {
        // Vector search in USearch
        let matches = self
            .vectors
            .search(query_embedding, limit)
            .context("Failed to search USearch index")?;

        // Hydrate from SQLite
        let mut beliefs = Vec::new();
        for rowid in matches.keys {
            if let Some(belief) = self.load_by_rowid(rowid as i64)? {
                beliefs.push(belief);
            }
        }

        Ok(beliefs)
    }

    /// Search and return results with similarity scores
    pub fn search_with_scores(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(Belief, f32)>> {
        // Vector search in USearch
        let matches = self
            .vectors
            .search(query_embedding, limit)
            .context("Failed to search USearch index")?;

        // Hydrate from SQLite and pair with distances
        let mut results = Vec::new();
        for (rowid, distance) in matches.keys.iter().zip(matches.distances.iter()) {
            if let Some(belief) = self.load_by_rowid(*rowid as i64)? {
                // Convert distance to similarity (1 - distance for cosine)
                let similarity = 1.0 - distance;
                results.push((belief, similarity));
            }
        }

        Ok(results)
    }

    /// Load a belief by rowid from SQLite
    fn load_by_rowid(&self, rowid: i64) -> Result<Option<Belief>> {
        let result = self.db.query_row(
            "SELECT id, content, metadata, created_at FROM beliefs WHERE rowid = ?1",
            params![rowid],
            |row| {
                let metadata_str: String = row.get(2)?;
                let metadata: BeliefMetadata =
                    serde_json::from_str(&metadata_str).unwrap_or_default();

                Ok(Belief {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    content: row.get(1)?,
                    embedding: vec![], // Don't load embeddings in results
                    metadata,
                })
            },
        );

        match result {
            Ok(belief) => Ok(Some(belief)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Save USearch index to disk
    pub fn save_index(&self) -> Result<()> {
        self.vectors
            .save(self.index_path.to_str().unwrap())
            .context("Failed to save USearch index")?;
        Ok(())
    }

    /// Get count of beliefs in storage
    pub fn count(&self) -> Result<usize> {
        let count: i64 = self
            .db
            .query_row("SELECT COUNT(*) FROM beliefs", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_belief_storage_creation() -> Result<()> {
        let temp = TempDir::new()?;
        let storage = BeliefStorage::open(temp.path())?;
        assert_eq!(storage.count()?, 0);
        Ok(())
    }

    #[test]
    fn test_usearch_basic() -> Result<()> {
        // Test USearch directly without SQLite
        let mut options = IndexOptions::default();
        options.dimensions = 384;
        options.metric = MetricKind::Cos;
        options.quantization = ScalarKind::F32;

        let mut index = Index::new(&options)?;
        index.reserve(10)?;

        // Add a vector
        let vec = vec![1.0; 384];
        index.add(1, &vec)?;

        // Search for it
        let results = index.search(&vec, 1)?;
        eprintln!(
            "Search results: keys={:?}, distances={:?}",
            results.keys, results.distances
        );

        assert_eq!(results.keys.len(), 1);
        assert_eq!(results.keys[0], 1);

        Ok(())
    }

    #[test]
    fn test_belief_storage_roundtrip() -> Result<()> {
        let temp = TempDir::new()?;
        let mut storage = BeliefStorage::open(temp.path())?;

        // Create a test belief
        let belief = Belief {
            id: Uuid::new_v4(),
            content: "Rust ownership prevents memory bugs".to_string(),
            embedding: vec![0.1; 384],
            metadata: BeliefMetadata::default(),
        };

        // Insert
        storage.insert(&belief)?;
        storage.save_index()?;

        // Verify count
        assert_eq!(storage.count()?, 1);

        // Search (should find the belief we just inserted)
        let results = storage.search(&belief.embedding, 1)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, belief.content);

        Ok(())
    }

    #[test]
    fn test_belief_storage_search_ranking() -> Result<()> {
        let temp = TempDir::new()?;
        let mut storage = BeliefStorage::open(temp.path())?;

        // Insert beliefs with different embeddings (different directions)
        let mut embedding1 = vec![0.0; 384];
        embedding1[0] = 1.0; // Vector pointing in x direction

        let mut embedding2 = vec![0.0; 384];
        embedding2[1] = 1.0; // Vector pointing in y direction

        let belief1 = Belief {
            id: Uuid::new_v4(),
            content: "First belief".to_string(),
            embedding: embedding1.clone(),
            metadata: BeliefMetadata::default(),
        };

        let belief2 = Belief {
            id: Uuid::new_v4(),
            content: "Second belief".to_string(),
            embedding: embedding2.clone(),
            metadata: BeliefMetadata::default(),
        };

        storage.insert(&belief1)?;
        storage.insert(&belief2)?;
        storage.save_index()?;

        // Search with embedding closer to belief1 (more x than y)
        let mut query = vec![0.0; 384];
        query[0] = 0.9;
        query[1] = 0.1;

        let results = storage.search_with_scores(&query, 2)?;

        assert_eq!(results.len(), 2);
        // First result should be belief1 (more similar to query)
        assert_eq!(results[0].0.content, "First belief");
        // First result should have higher similarity than second
        assert!(results[0].1 > results[1].1);

        Ok(())
    }
}
