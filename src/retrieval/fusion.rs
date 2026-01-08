//! Reciprocal Rank Fusion (RRF) for combining ranked lists
//!
//! RRF is a simple, effective method for fusing results from multiple retrievers.
//! k=60 is the standard value from the original paper (Cormack et al., 2009).

use std::collections::HashMap;

use super::intent::IntentWeights;
use super::oracle::{OracleMetadata, OracleResult};

/// Per-oracle contribution to a fused result
#[derive(Debug, Clone)]
pub struct OracleContribution {
    /// Rank within this oracle's results (1-indexed for display)
    pub rank: usize,
    /// Raw score from the oracle (scale varies by oracle type)
    pub raw_score: f32,
    /// Type of score (cosine, bm25, co_change_count)
    pub score_type: &'static str,
    /// Lexical matches if applicable
    pub matches: Option<Vec<String>>,
}

/// Structural annotations from module_signals table
///
/// Note: centrality_score intentionally omitted - raw value from call_graph
/// is not normalized (project-specific scale). Will add when assay derive
/// computes percentile rank like file_size_rank does.
#[derive(Debug, Clone, Default)]
pub struct StructuralAnnotations {
    /// Number of files that import this module
    pub importer_count: Option<i64>,
    /// Activity level: high, medium, low, dormant
    pub activity_level: Option<String>,
    /// Is this an entry point (main, lib, etc)
    pub is_entry_point: Option<bool>,
    /// Is this a test file
    pub is_test_file: Option<bool>,
}

/// Fused result with combined score and provenance
#[derive(Debug)]
pub struct FusedResult {
    pub doc_id: String,
    pub content: String,
    pub fused_score: f32,
    /// Legacy: list of oracle names that contributed (for backward compatibility)
    pub sources: Vec<&'static str>,
    /// Per-oracle contributions with rank and raw score
    pub contributions: HashMap<&'static str, OracleContribution>,
    pub metadata: OracleMetadata,
    /// Structural annotations from module_signals (if available)
    pub annotations: StructuralAnnotations,
}

/// Reciprocal Rank Fusion
///
/// Combines multiple ranked lists into a single ranking.
/// Score for document d = Σ 1/(k + rank_i) for each list i containing d
///
/// k=60 is standard (higher k reduces impact of top ranks)
pub fn rrf_fuse(ranked_lists: Vec<Vec<OracleResult>>, k: usize, limit: usize) -> Vec<FusedResult> {
    // Default: uniform weights
    rrf_fuse_weighted(ranked_lists, k, limit, None)
}

/// Weighted Reciprocal Rank Fusion
///
/// Like RRF, but applies per-oracle weights to boost/reduce oracle contributions.
/// Score for document d = Σ weight_i * 1/(k + rank_i)
///
/// Used for intent-aware retrieval where temporal queries boost commits/sessions.
pub fn rrf_fuse_weighted(
    ranked_lists: Vec<Vec<OracleResult>>,
    k: usize,
    limit: usize,
    weights: Option<&IntentWeights>,
) -> Vec<FusedResult> {
    let mut scores: HashMap<String, f32> = HashMap::new();
    let mut docs: HashMap<String, OracleResult> = HashMap::new();
    let mut sources: HashMap<String, Vec<&'static str>> = HashMap::new();
    let mut contributions: HashMap<String, HashMap<&'static str, OracleContribution>> =
        HashMap::new();

    for list in ranked_lists {
        for (rank, result) in list.into_iter().enumerate() {
            // RRF score: 1 / (k + rank + 1)
            // rank is 0-indexed, so rank 0 -> 1/(k+1)
            let base_rrf_score = 1.0 / (k + rank + 1) as f32;

            // Apply oracle-specific weight if provided
            let weight = weights.map(|w| w.weight_for(result.source)).unwrap_or(1.0);
            let rrf_score = weight * base_rrf_score;

            *scores.entry(result.doc_id.clone()).or_default() += rrf_score;

            sources
                .entry(result.doc_id.clone())
                .or_default()
                .push(result.source);

            // Track per-oracle contribution (rank is 1-indexed for display)
            contributions
                .entry(result.doc_id.clone())
                .or_default()
                .insert(
                    result.source,
                    OracleContribution {
                        rank: rank + 1, // 1-indexed for display
                        raw_score: result.score,
                        score_type: result.score_type,
                        matches: result.metadata.matches.clone(),
                    },
                );

            docs.entry(result.doc_id.clone()).or_insert(result);
        }
    }

    // Sort by fused score descending
    let mut fused: Vec<_> = scores
        .into_iter()
        .map(|(doc_id, fused_score)| {
            let doc = docs.remove(&doc_id).unwrap();
            let doc_sources = sources.remove(&doc_id).unwrap_or_default();
            let doc_contributions = contributions.remove(&doc_id).unwrap_or_default();
            FusedResult {
                doc_id,
                content: doc.content,
                fused_score,
                sources: doc_sources,
                contributions: doc_contributions,
                metadata: doc.metadata,
                annotations: StructuralAnnotations::default(),
            }
        })
        .collect();

    fused.sort_by(|a, b| {
        b.fused_score
            .partial_cmp(&a.fused_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    fused.truncate(limit);
    fused
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(doc_id: &str, source: &'static str) -> OracleResult {
        OracleResult {
            doc_id: doc_id.to_string(),
            content: format!("Content for {}", doc_id),
            source,
            score: 0.5,
            score_type: "cosine",
            metadata: OracleMetadata::default(),
        }
    }

    #[test]
    fn test_rrf_single_list() {
        // Results ordered by rank (first = rank 0, highest RRF score)
        let lists = vec![vec![
            make_result("doc_a", "semantic"),
            make_result("doc_b", "semantic"),
        ]];

        let fused = rrf_fuse(lists, 60, 10);

        assert_eq!(fused.len(), 2);
        assert_eq!(fused[0].doc_id, "doc_a");
        assert_eq!(fused[1].doc_id, "doc_b");
        // Rank 0: 1/61, Rank 1: 1/62
        assert!(fused[0].fused_score > fused[1].fused_score);
    }

    #[test]
    fn test_rrf_multiple_lists_boost() {
        // doc_b appears in both lists, should be boosted
        let lists = vec![
            vec![
                make_result("doc_a", "semantic"),
                make_result("doc_b", "semantic"),
            ],
            vec![
                make_result("doc_b", "lexical"),
                make_result("doc_c", "lexical"),
            ],
        ];

        let fused = rrf_fuse(lists, 60, 10);

        // doc_b should be first (appears in both lists)
        assert_eq!(fused[0].doc_id, "doc_b");
        assert_eq!(fused[0].sources.len(), 2);
    }

    #[test]
    fn test_rrf_limit() {
        let lists = vec![vec![
            make_result("doc_a", "semantic"),
            make_result("doc_b", "semantic"),
            make_result("doc_c", "semantic"),
        ]];

        let fused = rrf_fuse(lists, 60, 2);

        assert_eq!(fused.len(), 2);
    }
}
