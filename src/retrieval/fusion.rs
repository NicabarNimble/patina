//! Result types for the retrieval pipeline
//!
//! After the semantic-structural split, scry uses a single oracle (semantic).
//! These types are still used by the QueryEngine for result representation,
//! MCP JSON serialization, and eval.

use std::collections::HashMap;

use super::oracle::OracleMetadata;

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
