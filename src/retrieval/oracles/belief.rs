//! Belief oracle - hybrid vector + FTS5 search against beliefs
//!
//! Channel A: Vector search via shared USearch index (filter to BELIEF_ID_OFFSET range)
//! Channel B: FTS5 keyword search via existing belief_fts table
//! Internal merge: weighted sum, one ranked list for RRF

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

use crate::commands::oxidize::trainer::Projection;
use crate::retrieval::oracle::{Oracle, OracleMetadata, OracleResult};
use patina::embeddings::{create_embedder, EmbeddingEngine};

const BELIEF_ID_OFFSET: i64 = 4_000_000_000;
const FORGE_ID_OFFSET: i64 = 5_000_000_000;

const VECTOR_WEIGHT: f32 = 0.7;
const TEXT_WEIGHT: f32 = 0.3;

/// Cached resources for belief vector search (loaded once, reused)
struct BeliefCache {
    embedder: Mutex<Box<dyn EmbeddingEngine>>,
    projection: Option<Projection>,
    index: Index,
    index_size: usize,
}

pub struct BeliefOracle {
    db_path: PathBuf,
    index_path: PathBuf,
    projection_path: PathBuf,
    cache: OnceLock<Result<BeliefCache, String>>,
}

impl BeliefOracle {
    pub fn new() -> Self {
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

    fn init_cache(&self) -> Result<BeliefCache, String> {
        let embedder =
            create_embedder().map_err(|e| format!("Failed to create embedder: {}", e))?;

        let projection = if self.projection_path.exists() {
            Some(
                Projection::load_safetensors(&self.projection_path)
                    .map_err(|e| format!("Failed to load projection: {}", e))?,
            )
        } else {
            None
        };

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

        let index_size = index.size();

        Ok(BeliefCache {
            embedder: Mutex::new(embedder),
            projection,
            index,
            index_size,
        })
    }

    fn get_cache(&self) -> Result<&BeliefCache> {
        let cache_result = self.cache.get_or_init(|| self.init_cache());
        match cache_result {
            Ok(cache) => Ok(cache),
            Err(msg) => Err(anyhow::anyhow!("{}", msg)),
        }
    }

    /// Channel A: Vector search with over-fetch, filtered to belief ID range
    fn vector_search(&self, query: &str, limit: usize) -> Result<Vec<BeliefHit>> {
        let cache = self.get_cache()?;

        let query_embedding = {
            let mut embedder = cache
                .embedder
                .lock()
                .map_err(|e| anyhow::anyhow!("Embedder lock poisoned: {}", e))?;
            embedder.embed_query(query)?
        };

        let projected = match &cache.projection {
            Some(proj) => proj.forward(&query_embedding),
            None => query_embedding,
        };

        // Over-fetch aggressively: ~47 beliefs in index of thousands
        let over_fetch = (limit * 50).min(cache.index_size / 2).max(limit);

        let matches = cache
            .index
            .search(&projected, over_fetch)
            .with_context(|| "Vector search failed")?;

        let conn = Connection::open(&self.db_path)?;

        let mut hits = Vec::new();
        for i in 0..matches.keys.len() {
            let key = matches.keys[i] as i64;

            // Filter to belief range: [BELIEF_ID_OFFSET, FORGE_ID_OFFSET)
            if !(BELIEF_ID_OFFSET..FORGE_ID_OFFSET).contains(&key) {
                continue;
            }

            let score = 1.0 - matches.distances[i]; // cosine: 1 - distance
            if score <= 0.0 {
                continue;
            }

            let rowid = key - BELIEF_ID_OFFSET;
            if let Ok(hit) = enrich_belief(&conn, rowid, score) {
                hits.push(hit);
            }

            if hits.len() >= limit {
                break;
            }
        }

        Ok(hits)
    }

    /// Channel B: FTS5 keyword search against belief_fts
    fn fts_search(&self, query: &str, limit: usize) -> Result<Vec<BeliefHit>> {
        let conn = Connection::open(&self.db_path)?;

        // Check if belief_fts exists
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='belief_fts'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !exists {
            return Ok(Vec::new());
        }

        // Tokenize query for FTS5: join terms with OR
        let terms: Vec<&str> = query.split_whitespace().filter(|t| t.len() > 1).collect();
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let fts_query = terms.join(" OR ");

        let mut stmt = conn.prepare(
            "SELECT id, statement, bm25(belief_fts) as score
             FROM belief_fts
             WHERE belief_fts MATCH ?1
             ORDER BY score
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(rusqlite::params![fts_query, limit], |row| {
            let id: String = row.get(0)?;
            let statement: String = row.get(1)?;
            let bm25_score: f64 = row.get(2)?;
            Ok((id, statement, bm25_score))
        })?;

        let mut hits = Vec::new();
        for row in rows {
            let (id, statement, bm25_raw) = row?;

            // BM25 returns negative scores (more negative = better match)
            let score = (-bm25_raw) as f32;
            if score <= 0.0 {
                continue;
            }

            // Look up full belief metadata
            if let Ok(hit) = enrich_belief_by_id(&conn, &id, &statement, score) {
                hits.push(hit);
            }
        }

        Ok(hits)
    }
}

/// Internal belief hit before merge
struct BeliefHit {
    belief_id: String,
    content: String,
    file_path: String,
    vector_score: Option<f32>,
    fts_score: Option<f32>,
}

impl BeliefHit {
    /// Compute merged score: weighted sum of available channels
    fn merged_score(&self) -> f32 {
        match (self.vector_score, self.fts_score) {
            (Some(v), Some(f)) => VECTOR_WEIGHT * v + TEXT_WEIGHT * f,
            (Some(v), None) => v,
            (None, Some(f)) => f,
            (None, None) => 0.0,
        }
    }
}

/// Enrich belief by rowid (for vector search results)
fn enrich_belief(conn: &Connection, rowid: i64, score: f32) -> Result<BeliefHit> {
    conn.query_row(
        "SELECT id, statement, entrenchment, file_path,
                evidence_count, evidence_verified, applied_in
         FROM beliefs WHERE rowid = ?",
        [rowid],
        |row| {
            let id: String = row.get(0)?;
            let statement: String = row.get(1)?;
            let entrenchment: String = row.get(2)?;
            let file_path: String = row.get(3)?;
            let evidence_count: i32 = row.get(4)?;
            let evidence_verified: i32 = row.get(5)?;
            let applied_in: i32 = row.get(6)?;

            let content = format_belief_content(
                &statement,
                &entrenchment,
                &file_path,
                evidence_count,
                evidence_verified,
                applied_in,
            );

            Ok(BeliefHit {
                belief_id: id,
                content,
                file_path,
                vector_score: Some(score),
                fts_score: None,
            })
        },
    )
    .map_err(|e| anyhow::anyhow!("Belief lookup failed for rowid {}: {}", rowid, e))
}

/// Enrich belief by ID (for FTS5 results — statement already available)
fn enrich_belief_by_id(
    conn: &Connection,
    id: &str,
    statement: &str,
    fts_score: f32,
) -> Result<BeliefHit> {
    conn.query_row(
        "SELECT entrenchment, file_path, evidence_count, evidence_verified, applied_in
         FROM beliefs WHERE id = ?",
        [id],
        |row| {
            let entrenchment: String = row.get(0)?;
            let file_path: String = row.get(1)?;
            let evidence_count: i32 = row.get(2)?;
            let evidence_verified: i32 = row.get(3)?;
            let applied_in: i32 = row.get(4)?;

            let content = format_belief_content(
                statement,
                &entrenchment,
                &file_path,
                evidence_count,
                evidence_verified,
                applied_in,
            );

            Ok(BeliefHit {
                belief_id: id.to_string(),
                content,
                file_path,
                vector_score: None,
                fts_score: Some(fts_score),
            })
        },
    )
    .map_err(|e| anyhow::anyhow!("Belief lookup failed for id {}: {}", id, e))
}

fn format_belief_content(
    statement: &str,
    entrenchment: &str,
    file_path: &str,
    evidence_count: i32,
    evidence_verified: i32,
    applied_in: i32,
) -> String {
    let mut parts = vec![format!(
        "evidence: {}/{}",
        evidence_count, evidence_verified
    )];
    if applied_in > 0 {
        parts.push(format!("{} applied", applied_in));
    }
    format!(
        "{} ({}, {}) [{}]",
        statement,
        entrenchment,
        file_path,
        parts.join(", ")
    )
}

/// Normalize FTS5 BM25 scores to [0, 1] range for weighted merge with cosine
fn normalize_fts_scores(hits: &mut [BeliefHit]) {
    let max_score = hits
        .iter()
        .filter_map(|h| h.fts_score)
        .fold(0.0_f32, f32::max);

    if max_score > 0.0 {
        for hit in hits.iter_mut() {
            if let Some(ref mut s) = hit.fts_score {
                *s /= max_score;
            }
        }
    }
}

impl Oracle for BeliefOracle {
    fn name(&self) -> &'static str {
        "belief"
    }

    fn query(&self, query: &str, limit: usize) -> Result<Vec<OracleResult>> {
        // Run both channels
        let vector_hits = self.vector_search(query, limit).unwrap_or_default();
        let mut fts_hits = self.fts_search(query, limit).unwrap_or_default();

        // Normalize FTS scores to [0,1] before merging with cosine scores
        normalize_fts_scores(&mut fts_hits);

        // Merge by belief_id: combine scores from both channels
        let mut merged: HashMap<String, BeliefHit> = HashMap::new();

        for hit in vector_hits {
            merged.insert(hit.belief_id.clone(), hit);
        }

        for hit in fts_hits {
            if let Some(existing) = merged.get_mut(&hit.belief_id) {
                existing.fts_score = hit.fts_score;
            } else {
                merged.insert(hit.belief_id.clone(), hit);
            }
        }

        // Sort by merged score
        let mut results: Vec<BeliefHit> = merged.into_values().collect();
        results.sort_by(|a, b| {
            b.merged_score()
                .partial_cmp(&a.merged_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);

        // Convert to OracleResult
        let source = self.name();
        Ok(results
            .into_iter()
            .map(|hit| {
                let score = hit.merged_score();
                OracleResult {
                    doc_id: format!("belief:{}", hit.belief_id),
                    content: hit.content,
                    source,
                    score,
                    score_type: "hybrid_belief",
                    metadata: OracleMetadata {
                        file_path: Some(hit.file_path),
                        timestamp: None,
                        event_type: Some("belief".to_string()),
                        matches: None,
                    },
                }
            })
            .collect())
    }

    fn is_available(&self) -> bool {
        if !self.index_path.exists() || !self.db_path.exists() {
            return false;
        }

        // Check beliefs table has data
        Connection::open(&self.db_path)
            .and_then(|conn| {
                conn.query_row("SELECT EXISTS(SELECT 1 FROM beliefs LIMIT 1)", [], |row| {
                    row.get::<_, bool>(0)
                })
            })
            .unwrap_or(false)
    }
}
