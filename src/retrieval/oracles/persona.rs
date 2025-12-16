//! Persona oracle - cross-project user knowledge

use anyhow::Result;
use std::path::PathBuf;

use crate::commands::persona;
use patina::paths::persona as persona_paths;
use crate::retrieval::oracle::{Oracle, OracleMetadata, OracleResult};

pub struct PersonaOracle {
    db_path: PathBuf,
}

impl PersonaOracle {
    pub fn new() -> Self {
        Self {
            db_path: persona_paths::cache_dir().join("persona.db"),
        }
    }
}

impl Oracle for PersonaOracle {
    fn name(&self) -> &'static str {
        "persona"
    }

    fn query(&self, query: &str, limit: usize) -> Result<Vec<OracleResult>> {
        let results = persona::query(query, limit, 0.0, None)?;
        let source = self.name();

        Ok(results
            .into_iter()
            .map(|r| OracleResult {
                doc_id: format!("{}:{}:{}", source, r.source, r.timestamp),
                content: r.content,
                source,
                metadata: OracleMetadata {
                    file_path: None,
                    timestamp: Some(r.timestamp),
                    event_type: Some(r.source),
                },
            })
            .collect())
    }

    fn is_available(&self) -> bool {
        self.db_path.exists()
    }
}
