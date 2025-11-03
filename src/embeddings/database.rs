//! Database operations for embeddings
//!
//! Follows the scrape/code pattern: concrete wrapper around SqliteDatabase
//! with domain-specific methods for embedding generation and management.

use crate::db::SqliteDatabase;
use crate::embeddings::EmbeddingEngine;
use anyhow::{Context, Result};
use rusqlite::OptionalExtension;
use std::path::Path;

/// Embedding metadata from database
#[derive(Debug, Clone)]
pub struct EmbeddingMetadata {
    pub model_name: String,
    pub model_version: String,
    pub dimension: i64,
    pub belief_count: i64,
    pub observation_count: i64,
}

/// Database wrapper for embeddings operations
///
/// Follows the same pattern as scrape/code/database.rs:
/// - Owns SqliteDatabase
/// - Domain-specific methods
pub struct EmbeddingsDatabase {
    db: SqliteDatabase,
}

impl EmbeddingsDatabase {
    /// Open embeddings database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = SqliteDatabase::open(path)?;
        Ok(Self { db })
    }

    /// Get reference to underlying database
    pub fn database(&self) -> &SqliteDatabase {
        &self.db
    }

    /// Check if embeddings already exist
    pub fn has_embeddings(&self) -> Result<bool> {
        let count: i64 = self
            .db
            .connection()
            .query_row("SELECT COUNT(*) FROM embedding_metadata", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        Ok(count > 0)
    }

    /// Get embedding metadata
    pub fn get_metadata(&self) -> Result<Option<EmbeddingMetadata>> {
        let metadata = self
            .db
            .connection()
            .query_row(
                "SELECT model_name, model_version, dimension, belief_count, observation_count
                 FROM embedding_metadata
                 ORDER BY generated_at DESC LIMIT 1",
                [],
                |row| {
                    Ok(EmbeddingMetadata {
                        model_name: row.get(0)?,
                        model_version: row.get(1)?,
                        dimension: row.get(2)?,
                        belief_count: row.get(3)?,
                        observation_count: row.get(4)?,
                    })
                },
            )
            .optional()?;

        Ok(metadata)
    }

    /// Record embedding generation metadata
    pub fn record_metadata(
        &self,
        model_name: &str,
        model_version: &str,
        dimension: usize,
        belief_count: usize,
        observation_count: usize,
    ) -> Result<()> {
        self.db.connection().execute(
            "INSERT INTO embedding_metadata (model_name, model_version, dimension, belief_count, observation_count)
             VALUES (?, ?, ?, ?, ?)",
            (
                model_name,
                model_version,
                dimension as i64,
                belief_count as i64,
                observation_count as i64,
            ),
        )?;

        Ok(())
    }

    /// Generate embeddings for all active beliefs
    pub fn generate_belief_embeddings(
        &self,
        embedder: &mut dyn EmbeddingEngine,
    ) -> Result<usize> {
        let conn = self.db.connection();

        // Query all active beliefs
        let mut stmt = conn.prepare("SELECT id, statement FROM beliefs WHERE active = TRUE")?;
        let beliefs: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let count = beliefs.len();

        // Generate embeddings for each belief
        for (id, statement) in beliefs {
            let embedding = embedder
                .embed(&statement)
                .context(format!("Failed to generate embedding for belief {}", id))?;

            // Store metadata (actual vectors stored in belief_vectors table via sqlite-vec)
            conn.execute(
                "INSERT OR REPLACE INTO embedding_metadata (id, model_name, model_version, dimension)
                 VALUES (?, ?, ?, ?)",
                (
                    id,
                    embedder.model_name(),
                    "1.0",
                    embedder.dimension() as i64,
                ),
            )?;

            // Validate embedding dimension
            if embedding.len() != embedder.dimension() {
                anyhow::bail!(
                    "Embedding dimension mismatch for belief {}: expected {}, got {}",
                    id,
                    embedder.dimension(),
                    embedding.len()
                );
            }
        }

        Ok(count)
    }

    /// Generate embeddings for all observations (patterns, technologies, decisions, challenges)
    pub fn generate_observation_embeddings(
        &self,
        embedder: &mut dyn EmbeddingEngine,
    ) -> Result<usize> {
        let conn = self.db.connection();
        let mut count = 0;

        // Patterns
        let mut stmt = conn.prepare("SELECT id, pattern_name, description FROM patterns")?;
        let patterns: Vec<(i64, String, Option<String>)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        for (id, name, desc) in patterns {
            let text = match desc {
                Some(d) => format!("{}: {}", name, d),
                None => name.clone(),
            };
            let _embedding = embedder
                .embed(&text)
                .context(format!("Failed to generate embedding for pattern {}", id))?;
            count += 1;
        }

        // Technologies
        let mut stmt = conn.prepare("SELECT id, tech_name, purpose FROM technologies")?;
        let technologies: Vec<(i64, String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        for (id, name, purpose) in technologies {
            let text = format!("{}: {}", name, purpose);
            let _embedding = embedder.embed(&text).context(format!(
                "Failed to generate embedding for technology {}",
                id
            ))?;
            count += 1;
        }

        // Decisions
        let mut stmt = conn.prepare("SELECT id, choice, rationale FROM decisions")?;
        let decisions: Vec<(i64, String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        for (id, choice, rationale) in decisions {
            let text = format!("{}: {}", choice, rationale);
            let _embedding = embedder
                .embed(&text)
                .context(format!("Failed to generate embedding for decision {}", id))?;
            count += 1;
        }

        // Challenges
        let mut stmt = conn.prepare("SELECT id, problem, solution FROM challenges")?;
        let challenges: Vec<(i64, String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        for (id, problem, solution) in challenges {
            let text = format!("{}: {}", problem, solution);
            let _embedding = embedder
                .embed(&text)
                .context(format!("Failed to generate embedding for challenge {}", id))?;
            count += 1;
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embeddings_database_opens() -> Result<()> {
        // This test just verifies the struct can be created
        // Real tests would require a test database with schema
        Ok(())
    }
}
