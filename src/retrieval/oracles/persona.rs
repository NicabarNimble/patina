//! Persona oracle - cross-project user knowledge
//!
//! Owns embedder and index - loads once, reuses across queries.
//! Uses raw 768-dim embeddings (no projection, unlike semantic oracle).

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

use crate::retrieval::oracle::{Oracle, OracleMetadata, OracleResult};
use patina::embeddings::{create_embedder, EmbeddingEngine};
use patina::paths::persona as persona_paths;

/// Cached resources for persona search
struct PersonaCache {
    embedder: Mutex<Box<dyn EmbeddingEngine>>,
    index: Index,
}

pub struct PersonaOracle {
    db_path: PathBuf,
    index_path: PathBuf,
    cache: OnceLock<Result<PersonaCache, String>>,
}

impl PersonaOracle {
    pub fn new() -> Self {
        let cache_dir = persona_paths::cache_dir();
        Self {
            db_path: cache_dir.join("persona.db"),
            index_path: cache_dir.join("persona.usearch"),
            cache: OnceLock::new(),
        }
    }

    fn init_cache(&self) -> Result<PersonaCache, String> {
        // Create embedder
        let embedder =
            create_embedder().map_err(|e| format!("Failed to create embedder: {}", e))?;

        // Load index (persona uses raw 768-dim, no projection)
        let index_options = IndexOptions {
            dimensions: 768,
            metric: MetricKind::Cos,
            quantization: ScalarKind::F32,
            ..Default::default()
        };

        let index =
            Index::new(&index_options).map_err(|e| format!("Failed to create index: {}", e))?;

        index
            .load(self.index_path.to_str().unwrap_or(""))
            .map_err(|e| format!("Failed to load persona index: {}", e))?;

        Ok(PersonaCache {
            embedder: Mutex::new(embedder),
            index,
        })
    }

    fn get_cache(&self) -> Result<&PersonaCache> {
        let cache_result = self.cache.get_or_init(|| self.init_cache());

        match cache_result {
            Ok(cache) => Ok(cache),
            Err(msg) => Err(anyhow::anyhow!("{}", msg)),
        }
    }
}

impl Oracle for PersonaOracle {
    fn name(&self) -> &'static str {
        "persona"
    }

    fn query(&self, query: &str, limit: usize) -> Result<Vec<OracleResult>> {
        // Return empty if no index exists
        if !self.index_path.exists() {
            return Ok(Vec::new());
        }

        let cache = self.get_cache()?;

        // Embed query
        let query_embedding = {
            let mut embedder = cache
                .embedder
                .lock()
                .map_err(|e| anyhow::anyhow!("Embedder lock poisoned: {}", e))?;
            embedder.embed_query(query)?
        };

        // Search index
        let matches = cache
            .index
            .search(&query_embedding, limit)
            .with_context(|| "Persona search failed")?;

        // Enrich with metadata from database
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("Failed to open persona db: {:?}", self.db_path))?;

        let source = self.name();
        let mut results = Vec::new();

        for i in 0..matches.keys.len() {
            let key = matches.keys[i] as i64;
            let distance = matches.distances[i];
            let score = 1.0 - distance;

            // Look up knowledge from database (was "beliefs", now "knowledge")
            let result = conn.query_row(
                "SELECT id, source, content, domains, timestamp
                 FROM knowledge
                 WHERE rowid = ?",
                [key],
                |row| {
                    let _id: String = row.get(0)?;
                    let belief_source: String = row.get(1)?;
                    let content: String = row.get(2)?;
                    let domains: Option<String> = row.get(3)?;
                    let timestamp: String = row.get(4)?;

                    Ok(OracleResult {
                        doc_id: format!("{}:{}:{}", source, belief_source, timestamp),
                        content,
                        source,
                        score,
                        score_type: "cosine",
                        metadata: OracleMetadata {
                            file_path: None,
                            timestamp: Some(timestamp),
                            event_type: Some(format!(
                                "{} ({})",
                                belief_source,
                                domains.unwrap_or_default()
                            )),
                            matches: None,
                        },
                    })
                },
            );

            if let Ok(r) = result {
                results.push(r);
            }
        }

        Ok(results)
    }

    fn is_available(&self) -> bool {
        self.db_path.exists() && self.index_path.exists()
    }
}
