//! Reciprocal Rank Fusion (RRF) for combining ranked lists
//!
//! RRF is a simple, effective method for fusing results from multiple retrievers.
//! k=60 is the standard value from the original paper (Cormack et al., 2009).

use std::collections::HashMap;

use super::oracle::{OracleMetadata, OracleResult};

/// Fused result with combined score and provenance
#[derive(Debug)]
pub struct FusedResult {
    pub doc_id: String,
    pub content: String,
    pub fused_score: f32,
    pub sources: Vec<&'static str>,
    pub metadata: OracleMetadata,
}

/// Reciprocal Rank Fusion
///
/// Combines multiple ranked lists into a single ranking.
/// Score for document d = Σ 1/(k + rank_i) for each list i containing d
///
/// k=60 is standard (higher k reduces impact of top ranks)
pub fn rrf_fuse(ranked_lists: Vec<Vec<OracleResult>>, k: usize, limit: usize) -> Vec<FusedResult> {
    let mut scores: HashMap<String, f32> = HashMap::new();
    let mut docs: HashMap<String, OracleResult> = HashMap::new();
    let mut sources: HashMap<String, Vec<&'static str>> = HashMap::new();

    for list in ranked_lists {
        for (rank, result) in list.into_iter().enumerate() {
            // RRF score: 1 / (k + rank + 1)
            // rank is 0-indexed, so rank 0 -> 1/(k+1)
            let rrf_score = 1.0 / (k + rank + 1) as f32;

            *scores.entry(result.doc_id.clone()).or_default() += rrf_score;

            sources
                .entry(result.doc_id.clone())
                .or_default()
                .push(result.source);

            docs.entry(result.doc_id.clone()).or_insert(result);
        }
    }

    // Sort by fused score descending
    let mut fused: Vec<_> = scores
        .into_iter()
        .map(|(doc_id, fused_score)| {
            let doc = docs.remove(&doc_id).unwrap();
            let doc_sources = sources.remove(&doc_id).unwrap_or_default();
            FusedResult {
                doc_id,
                content: doc.content,
                fused_score,
                sources: doc_sources,
                metadata: doc.metadata,
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
