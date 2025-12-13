//! Semantic oracle - E5 embeddings + USearch vector search

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::commands::scry::{scry_text, ScryOptions};
use crate::retrieval::oracle::{Oracle, OracleMetadata, OracleResult};

pub struct SemanticOracle {
    db_path: PathBuf,
    index_path: PathBuf,
}

impl SemanticOracle {
    pub fn new() -> Self {
        // Read model from project config
        let model = patina::project::load(Path::new("."))
            .ok()
            .map(|c| c.embeddings.model)
            .unwrap_or_else(|| "e5-base-v2".to_string());

        Self {
            db_path: PathBuf::from(".patina/data/patina.db"),
            index_path: PathBuf::from(format!(
                ".patina/data/embeddings/{}/projections/semantic.usearch",
                model
            )),
        }
    }
}

impl Oracle for SemanticOracle {
    fn name(&self) -> &'static str {
        "semantic"
    }

    fn query(&self, query: &str, limit: usize) -> Result<Vec<OracleResult>> {
        let options = ScryOptions {
            limit,
            dimension: Some("semantic".to_string()),
            include_persona: false, // Persona is separate oracle
            ..Default::default()
        };

        let results = scry_text(query, &options)?;
        let source = self.name();

        Ok(results
            .into_iter()
            .map(|r| OracleResult {
                doc_id: r.source_id.clone(),
                content: r.content,
                source,
                metadata: OracleMetadata {
                    file_path: Some(r.source_id),
                    timestamp: if r.timestamp.is_empty() {
                        None
                    } else {
                        Some(r.timestamp)
                    },
                    event_type: Some(r.event_type),
                },
            })
            .collect())
    }

    fn is_available(&self) -> bool {
        self.index_path.exists() && self.db_path.exists()
    }
}
