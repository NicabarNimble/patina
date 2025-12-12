//! QueryEngine - parallel multi-oracle retrieval with RRF fusion

use anyhow::Result;
use rayon::prelude::*;

use super::fusion::{rrf_fuse, FusedResult};
use super::oracle::Oracle;
use super::oracles::{LexicalOracle, PersonaOracle, SemanticOracle};

/// Query engine that coordinates parallel oracle retrieval
pub struct QueryEngine {
    oracles: Vec<Box<dyn Oracle>>,
}

impl QueryEngine {
    /// Create engine with default oracles (semantic, lexical, persona)
    pub fn new() -> Self {
        let oracles: Vec<Box<dyn Oracle>> = vec![
            Box::new(SemanticOracle::new()),
            Box::new(LexicalOracle::new()),
            Box::new(PersonaOracle::new()),
        ];

        Self { oracles }
    }

    /// Query all available oracles in parallel, fuse with RRF
    pub fn query(&self, query: &str, limit: usize) -> Result<Vec<FusedResult>> {
        // Over-fetch from each oracle (2x limit) for better fusion
        let fetch_limit = limit * 2;

        // Query available oracles in parallel
        let oracle_results: Vec<_> = self
            .oracles
            .par_iter()
            .filter(|o| o.is_available())
            .filter_map(|oracle| oracle.query(query, fetch_limit).ok())
            .collect();

        // Fuse with RRF (k=60 is standard)
        Ok(rrf_fuse(oracle_results, 60, limit))
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
