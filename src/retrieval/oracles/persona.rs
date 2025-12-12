//! Persona oracle - cross-project user knowledge

use anyhow::Result;
use std::path::PathBuf;

use crate::commands::persona;
use crate::retrieval::oracle::{Oracle, OracleMetadata, OracleResult};

pub struct PersonaOracle {
    db_path: PathBuf,
}

impl PersonaOracle {
    pub fn new() -> Self {
        let persona_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".patina/personas/default/materialized");

        Self {
            db_path: persona_dir.join("persona.db"),
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
