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
    /// Domain name for enrichment routing (e.g., "knowledge", "sessions")
    domain: String,
    /// Lazy-initialized cache - loads on first query
    cache: OnceLock<Result<SemanticCache, String>>,
}

impl SemanticOracle {
    pub fn new() -> Self {
        // Default: knowledge domain (Phase 2+)
        Self::for_domain("knowledge")
    }

    /// Create an oracle for a specific semantic domain (Phase 5b multi-domain)
    ///
    /// Each domain has its own index and projection in the embeddings directory.
    /// Falls back to legacy "semantic" name for the knowledge domain.
    pub fn for_domain(domain: &str) -> Self {
        let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let model = patina::project::load(Path::new("."))
            .ok()
            .map(|c| c.embeddings.model)
            .unwrap_or_else(|| "e5-base-v2".to_string());

        let embeddings_dir = patina::paths::project::model_projections_dir(&project_root, &model);

        // For knowledge domain, fall back to legacy "semantic" name
        let (index_name, proj_name) = if domain == "knowledge" {
            if embeddings_dir.join("knowledge.usearch").exists() {
                ("knowledge", "knowledge")
            } else {
                ("semantic", "semantic")
            }
        } else {
            (domain, domain)
        };

        Self {
            db_path: patina::eventlog::resolve_patina_db_path(&project_root),
            index_path: embeddings_dir.join(format!("{}.usearch", index_name)),
            projection_path: embeddings_dir.join(format!("{}.safetensors", proj_name)),
            domain: domain.to_string(),
            cache: OnceLock::new(),
        }
    }

    /// Discover all available semantic domains in the embeddings directory
    ///
    /// Returns domain names that have .usearch index files, excluding
    /// non-semantic projections (temporal, dependency). Projection (.safetensors)
    /// is optional — Phase 5d: knowledge/sessions use raw E5 embeddings.
    pub fn available_domains() -> Vec<String> {
        let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let model = patina::project::load(Path::new("."))
            .ok()
            .map(|c| c.embeddings.model)
            .unwrap_or_else(|| "e5-base-v2".to_string());

        let embeddings_dir = patina::paths::project::model_projections_dir(&project_root, &model);
        let dir = embeddings_dir.as_path();

        if !dir.exists() {
            return vec![];
        }

        // Non-semantic projections that shouldn't be queried as semantic domains
        let excluded = ["temporal", "dependency"];

        let mut domains = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("usearch") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        // Skip non-semantic projections
                        if excluded.contains(&stem) {
                            continue;
                        }
                        // Normalize "semantic" → "knowledge"
                        let domain = if stem == "semantic" {
                            "knowledge".to_string()
                        } else {
                            stem.to_string()
                        };
                        if !domains.contains(&domain) {
                            domains.push(domain);
                        }
                    }
                }
            }
        }

        domains.sort();
        domains
    }

    /// Initialize cache (embedder, projection, index) - called once
    fn init_cache(&self) -> Result<SemanticCache, String> {
        // Create embedder
        let embedder =
            create_embedder().map_err(|e| format!("Failed to create embedder: {}", e))?;

        // Load projection (optional — Phase 5d: knowledge/sessions use raw E5)
        let projection = if self.projection_path.exists() {
            Some(
                Projection::load_safetensors(&self.projection_path)
                    .map_err(|e| format!("Failed to load projection: {}", e))?,
            )
        } else {
            None
        };

        // Dynamic dimensions: projection output_dim when projected, raw E5 dim when not
        let dimensions = match &projection {
            Some(proj) => proj.output_dim(),
            None => embedder.dimension(), // raw E5 dim (768 for e5-base-v2)
        };

        // Load index
        let index_options = IndexOptions {
            dimensions,
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

        // Search index — use exact search for small corpora to close ANN gap.
        // USearch HNSW at 768-dim with ~615 vectors leaves 9.2pp P@10 gap vs
        // brute-force (43.3% vs 52.5%). Exact search eliminates this gap for
        // corpora below 10K vectors with negligible latency cost.
        const EXACT_SEARCH_THRESHOLD: usize = 10_000;
        let matches = if cache.index.size() < EXACT_SEARCH_THRESHOLD {
            cache
                .index
                .exact_search(&projected, limit)
                .with_context(|| "Exact vector search failed")?
        } else {
            cache
                .index
                .search(&projected, limit)
                .with_context(|| "Vector search failed")?
        };

        // Convert to SearchResults for enrichment
        let results = SearchResults {
            keys: matches.keys,
            distances: matches.distances,
        };

        // Enrich with metadata from SQLite
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("Failed to open database: {:?}", self.db_path))?;

        let enriched = enrich_results(&conn, &results, &self.domain, 0.0)?;

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
