//! QueryEngine - parallel multi-oracle retrieval with RRF fusion

use anyhow::Result;
use rayon::prelude::*;

use super::fusion::{rrf_fuse, FusedResult};
use super::oracle::Oracle;
use super::oracles::{LexicalOracle, PersonaOracle, SemanticOracle};

/// Retrieval configuration for QueryEngine
///
/// These are algorithm constants from the literature (Cormack et al., 2009).
/// See `RetrievalSection` in project config for persistence.
#[derive(Debug, Clone)]
pub struct RetrievalConfig {
    /// RRF smoothing constant (default: 60)
    pub rrf_k: usize,
    /// Over-fetch multiplier for fusion (default: 2)
    pub fetch_multiplier: usize,
    /// Filter to specific oracles (None = all available)
    /// Used for ablation testing: --oracle semantic
    pub oracle_filter: Option<Vec<String>>,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            rrf_k: 60,
            fetch_multiplier: 2,
            oracle_filter: None,
        }
    }
}

/// Query engine that coordinates parallel oracle retrieval
pub struct QueryEngine {
    oracles: Vec<Box<dyn Oracle>>,
    config: RetrievalConfig,
}

impl QueryEngine {
    /// Create engine with default oracles and config
    pub fn new() -> Self {
        Self::with_config(RetrievalConfig::default())
    }

    /// Create engine with custom retrieval config
    pub fn with_config(config: RetrievalConfig) -> Self {
        let oracles: Vec<Box<dyn Oracle>> = vec![
            Box::new(SemanticOracle::new()),
            Box::new(LexicalOracle::new()),
            Box::new(PersonaOracle::new()),
        ];

        Self { oracles, config }
    }

    /// Query all available oracles in parallel, fuse with RRF
    pub fn query(&self, query: &str, limit: usize) -> Result<Vec<FusedResult>> {
        // Over-fetch from each oracle for better fusion
        let fetch_limit = limit * self.config.fetch_multiplier;

        // Query available oracles in parallel (optionally filtered)
        let oracle_results: Vec<_> = self
            .oracles
            .par_iter()
            .filter(|o| o.is_available())
            .filter(|o| self.matches_filter(o.name()))
            .filter_map(|oracle| oracle.query(query, fetch_limit).ok())
            .collect();

        // Fuse with RRF
        Ok(rrf_fuse(oracle_results, self.config.rrf_k, limit))
    }

    /// Check if oracle matches the filter (if any)
    fn matches_filter(&self, oracle_name: &str) -> bool {
        match &self.config.oracle_filter {
            None => true, // No filter = include all
            Some(allowed) => allowed.iter().any(|a| a.eq_ignore_ascii_case(oracle_name)),
        }
    }

    /// List available oracles
    pub fn available_oracles(&self) -> Vec<&'static str> {
        self.oracles
            .iter()
            .filter(|o| o.is_available())
            .map(|o| o.name())
            .collect()
    }
}

impl Default for QueryEngine {
    fn default() -> Self {
        Self::new()
    }
}
