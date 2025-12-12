//! Lexical oracle - BM25/FTS5 text search

use anyhow::Result;
use std::path::PathBuf;

use crate::commands::scry::{scry_lexical, ScryOptions};
use crate::retrieval::oracle::{Oracle, OracleMetadata, OracleResult};

pub struct LexicalOracle {
    db_path: PathBuf,
}

impl LexicalOracle {
    pub fn new() -> Self {
        Self {
            db_path: PathBuf::from(".patina/data/patina.db"),
        }
    }
}

impl Oracle for LexicalOracle {
    fn name(&self) -> &'static str {
        "lexical"
    }

    fn query(&self, query: &str, limit: usize) -> Result<Vec<OracleResult>> {
        let options = ScryOptions {
            limit,
            include_persona: false,
            ..Default::default()
        };

        let results = scry_lexical(query, &options)?;
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
        self.db_path.exists()
    }
}
