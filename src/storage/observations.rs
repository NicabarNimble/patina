//! Observation storage using SQLite + USearch hybrid approach
//!
//! SQLite stores the source of truth (observation content, type, metadata).
//! USearch provides fast vector similarity search via HNSW indices.

use crate::storage::types::{Observation, ObservationMetadata};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};
use uuid::Uuid;

/// Type alias for observation row data from SQLite
/// (rowid, id, observation_type, content, metadata)
type ObservationRow = (i64, String, String, String, String);

/// Dual storage for observations: SQLite + USearch
pub struct ObservationStorage {
    vectors: Index,
    db: Connection,
    index_path: PathBuf,
}

impl ObservationStorage {
    /// Open or create observation storage at the given path
    ///
    /// Creates two files:
    /// - `{path}/observations.db` - SQLite database
    /// - `{path}/observations.usearch` - USearch vector index
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let base = path.as_ref();
        std::fs::create_dir_all(base)?;

        // Open SQLite database
        let db_path = base.join("observations.db");
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

        let index_path = base.join("observations.usearch");

        // Load existing index if present
        // Note: .view() creates immutable index - cannot add new vectors
        // TODO: Use mutable loading or rebuild strategy
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
            "CREATE TABLE IF NOT EXISTS observations (
                rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                id TEXT UNIQUE NOT NULL,
                observation_type TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    /// Insert an observation into both SQLite and USearch
    ///
    /// Observations are immutable - duplicate UUIDs will result in an error.
    pub fn insert(&mut self, observation: &Observation) -> Result<()> {
        // Insert into SQLite (source of truth) and get rowid atomically
        let metadata_json = serde_json::to_string(&observation.metadata)?;
        let created_at = observation
            .metadata
            .created_at
            .unwrap_or_else(chrono::Utc::now)
            .to_rfc3339();

        let rowid: i64 = self.db.query_row(
            "INSERT INTO observations (id, observation_type, content, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             RETURNING rowid",
            params![
                observation.id.to_string(),
                &observation.observation_type,
                &observation.content,
                metadata_json,
                created_at,
            ],
            |row| row.get(0),
        )?;

        // Insert into USearch (vector index) using rowid as key
        self.vectors
            .add(rowid as u64, &observation.embedding)
            .context("Failed to add vector to USearch index")?;

        Ok(())
    }

    /// Search for observations using vector similarity
    ///
    /// Returns top-k most similar observations based on query embedding.
    pub fn search(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<Observation>> {
        // Vector search in USearch
        let matches = self
            .vectors
            .search(query_embedding, limit)
            .context("Failed to search USearch index")?;

        // Hydrate from SQLite
        let mut observations = Vec::new();
        for rowid in matches.keys {
            if let Some(observation) = self.load_by_rowid(rowid as i64)? {
                observations.push(observation);
            }
        }

        Ok(observations)
    }

    /// Search for observations filtered by type
    ///
    /// Returns top-k most similar observations of the specified type.
    pub fn search_by_type(
        &self,
        query_embedding: &[f32],
        observation_type: &str,
        limit: usize,
    ) -> Result<Vec<Observation>> {
        // Search for more results to account for filtering
        let search_limit = limit * 3;
        let matches = self
            .vectors
            .search(query_embedding, search_limit)
            .context("Failed to search USearch index")?;

        // Hydrate from SQLite and filter by type
        let mut observations = Vec::new();
        for rowid in matches.keys {
            if let Some(observation) = self.load_by_rowid(rowid as i64)? {
                if observation.observation_type == observation_type {
                    observations.push(observation);
                    if observations.len() >= limit {
                        break;
                    }
                }
            }
        }

        Ok(observations)
    }

    /// Search and return results with similarity scores
    pub fn search_with_scores(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(Observation, f32)>> {
        // Vector search in USearch
        let matches = self
            .vectors
            .search(query_embedding, limit)
            .context("Failed to search USearch index")?;

        // Hydrate from SQLite and pair with distances
        let mut results = Vec::new();
        for (rowid, distance) in matches.keys.iter().zip(matches.distances.iter()) {
            if let Some(observation) = self.load_by_rowid(*rowid as i64)? {
                // Convert distance to similarity (1 - distance for cosine)
                let similarity = 1.0 - distance;
                results.push((observation, similarity));
            }
        }

        Ok(results)
    }

    /// Load an observation by rowid from SQLite
    fn load_by_rowid(&self, rowid: i64) -> Result<Option<Observation>> {
        let result = self.db.query_row(
            "SELECT id, observation_type, content, metadata, created_at FROM observations WHERE rowid = ?1",
            params![rowid],
            |row| {
                let metadata_str: String = row.get(3)?;
                let metadata: ObservationMetadata = serde_json::from_str(&metadata_str)
                    .unwrap_or_default();

                Ok(Observation {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    observation_type: row.get(1)?,
                    content: row.get(2)?,
                    embedding: vec![], // Don't load embeddings in results
                    metadata,
                })
            },
        );

        match result {
            Ok(observation) => Ok(Some(observation)),
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

    /// Get count of observations in storage
    pub fn count(&self) -> Result<usize> {
        let count: i64 = self
            .db
            .query_row("SELECT COUNT(*) FROM observations", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Get count of observations by type
    pub fn count_by_type(&self, observation_type: &str) -> Result<usize> {
        let count: i64 = self.db.query_row(
            "SELECT COUNT(*) FROM observations WHERE observation_type = ?1",
            params![observation_type],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Add observation to USearch index only (assumes already in SQLite)
    ///
    /// Use when rebuilding index from existing SQLite observations.
    /// Does NOT insert into SQLite - only adds vector to USearch index.
    pub fn add_to_index_only(&mut self, rowid: i64, embedding: &[f32]) -> Result<()> {
        self.vectors
            .add(rowid as u64, embedding)
            .context("Failed to add vector to USearch index")?;
        Ok(())
    }

    /// Query all observations from SQLite (for index rebuilding)
    ///
    /// Returns (rowid, id, observation_type, content, metadata) tuples.
    pub fn query_all(&self) -> Result<Vec<ObservationRow>> {
        let mut stmt = self
            .db
            .prepare("SELECT rowid, id, observation_type, content, metadata FROM observations")?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?, // rowid
                    row.get(1)?, // id
                    row.get(2)?, // observation_type
                    row.get(3)?, // content
                    row.get(4)?, // metadata
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_observation_storage_creation() -> Result<()> {
        let temp = TempDir::new()?;
        let storage = ObservationStorage::open(temp.path())?;
        assert_eq!(storage.count()?, 0);
        Ok(())
    }

    #[test]
    fn test_observation_storage_roundtrip() -> Result<()> {
        let temp = TempDir::new()?;
        let mut storage = ObservationStorage::open(temp.path())?;

        // Create a test observation
        let observation = Observation {
            id: Uuid::new_v4(),
            observation_type: "pattern".to_string(),
            content: "Always validate user input".to_string(),
            embedding: vec![0.1; 384],
            metadata: ObservationMetadata::default(),
        };

        // Insert
        storage.insert(&observation)?;
        storage.save_index()?;

        // Verify count
        assert_eq!(storage.count()?, 1);

        // Search (should find the observation we just inserted)
        let results = storage.search(&observation.embedding, 1)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, observation.content);
        assert_eq!(results[0].observation_type, "pattern");

        Ok(())
    }

    #[test]
    fn test_observation_storage_search_by_type() -> Result<()> {
        let temp = TempDir::new()?;
        let mut storage = ObservationStorage::open(temp.path())?;

        // Insert observations of different types
        let mut embedding1 = vec![0.0; 384];
        embedding1[0] = 1.0;

        let mut embedding2 = vec![0.0; 384];
        embedding2[1] = 1.0;

        let pattern = Observation {
            id: Uuid::new_v4(),
            observation_type: "pattern".to_string(),
            content: "Validate all inputs".to_string(),
            embedding: embedding1.clone(),
            metadata: ObservationMetadata::default(),
        };

        let technology = Observation {
            id: Uuid::new_v4(),
            observation_type: "technology".to_string(),
            content: "Rust for systems programming".to_string(),
            embedding: embedding2.clone(),
            metadata: ObservationMetadata::default(),
        };

        storage.insert(&pattern)?;
        storage.insert(&technology)?;
        storage.save_index()?;

        // Search for patterns only
        let query = vec![0.9; 384]; // Will match both, but we filter by type
        let results = storage.search_by_type(&query, "pattern", 10)?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].observation_type, "pattern");

        // Check counts
        assert_eq!(storage.count()?, 2);
        assert_eq!(storage.count_by_type("pattern")?, 1);
        assert_eq!(storage.count_by_type("technology")?, 1);

        Ok(())
    }
}
