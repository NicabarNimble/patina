//! Semantic oracle - E5 embeddings + USearch vector search
//!
//! Owns embedder, projection, and index - loads once, reuses across queries.
//! This is the Phase 1 fix for the model-loading-per-query bottleneck.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

use crate::commands::oxidize::trainer::Projection;
use crate::commands::scry::internal::enrichment::{enrich_results, SearchResults};
use crate::retrieval::oracle::{Oracle, OracleMetadata, OracleResult};
use patina::embeddings::{create_embedder, EmbeddingEngine};

/// Cached resources for semantic search (loaded once, reused)
struct SemanticCache {
    embedder: Mutex<Box<dyn EmbeddingEngine>>,
    projection: Option<Projection>,
    index: Index,
}

pub struct SemanticOracle {
    db_path: PathBuf,
    index_path: PathBuf,
    projection_path: PathBuf,
    /// Lazy-initialized cache - loads on first query
    cache: OnceLock<Result<SemanticCache, String>>,
}

impl SemanticOracle {
    pub fn new() -> Self {
        // Read model from project config
        let model = patina::project::load(Path::new("."))
            .ok()
            .map(|c| c.embeddings.model)
            .unwrap_or_else(|| "e5-base-v2".to_string());

        let embeddings_dir = format!(".patina/local/data/embeddings/{}/projections", model);

        Self {
            db_path: PathBuf::from(".patina/local/data/patina.db"),
            index_path: PathBuf::from(format!("{}/semantic.usearch", embeddings_dir)),
            projection_path: PathBuf::from(format!("{}/semantic.safetensors", embeddings_dir)),
            cache: OnceLock::new(),
        }
    }

    /// Initialize cache (embedder, projection, index) - called once
    fn init_cache(&self) -> Result<SemanticCache, String> {
        // Create embedder
        let embedder =
            create_embedder().map_err(|e| format!("Failed to create embedder: {}", e))?;

        // Load projection (optional)
        let projection = if self.projection_path.exists() {
            Some(
                Projection::load_safetensors(&self.projection_path)
                    .map_err(|e| format!("Failed to load projection: {}", e))?,
            )
        } else {
            None
        };

        // Load index
        let index_options = IndexOptions {
            dimensions: 256,
            metric: MetricKind::Cos,
            quantization: ScalarKind::F32,
            ..Default::default()
        };

        let index =
            Index::new(&index_options).map_err(|e| format!("Failed to create index: {}", e))?;

        index
            .load(self.index_path.to_str().unwrap_or(""))
            .map_err(|e| format!("Failed to load index: {}", e))?;

        Ok(SemanticCache {
            embedder: Mutex::new(embedder),
            projection,
            index,
        })
    }

    /// Get or initialize cache
    fn get_cache(&self) -> Result<&SemanticCache> {
        let cache_result = self.cache.get_or_init(|| self.init_cache());

        match cache_result {
            Ok(cache) => Ok(cache),
            Err(msg) => Err(anyhow::anyhow!("{}", msg)),
        }
    }
}

impl Oracle for SemanticOracle {
    fn name(&self) -> &'static str {
        "semantic"
    }

    fn query(&self, query: &str, limit: usize) -> Result<Vec<OracleResult>> {
        let cache = self.get_cache()?;

        // Embed query (needs mutable access to embedder)
        let query_embedding = {
            let mut embedder = cache
                .embedder
                .lock()
                .map_err(|e| anyhow::anyhow!("Embedder lock poisoned: {}", e))?;
            embedder.embed_query(query)?
        };

        // Project embedding if projection exists
        let projected = match &cache.projection {
            Some(proj) => proj.forward(&query_embedding),
            None => query_embedding,
        };

        // Search index
        let matches = cache
            .index
            .search(&projected, limit)
            .with_context(|| "Vector search failed")?;

        // Convert to SearchResults for enrichment
        let results = SearchResults {
            keys: matches.keys,
            distances: matches.distances,
        };

        // Enrich with metadata from SQLite
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("Failed to open database: {:?}", self.db_path))?;

        let enriched = enrich_results(&conn, &results, "semantic", 0.0)?;

        // Convert to OracleResult
        let source = self.name();
        Ok(enriched
            .into_iter()
            .map(|r| OracleResult {
                doc_id: r.source_id.clone(),
                content: r.content,
                source,
                score: r.score,
                score_type: "cosine",
                metadata: OracleMetadata {
                    file_path: Some(r.source_id),
                    timestamp: if r.timestamp.is_empty() {
                        None
                    } else {
                        Some(r.timestamp)
                    },
                    event_type: Some(r.event_type),
                    matches: None,
                },
            })
            .collect())
    }

    fn is_available(&self) -> bool {
        self.index_path.exists() && self.db_path.exists()
    }
}
