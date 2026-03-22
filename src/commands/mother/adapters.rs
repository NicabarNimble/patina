use anyhow::Result;

use crate::retrieval::{QueryEngine, QueryOptions};

#[derive(Debug, Clone)]
pub struct ScryHit {
    pub content: String,
    pub score: f32,
    pub event_type: String,
    pub source_id: String,
    pub timestamp: String,
}

pub trait ScryBackend: Send + Sync {
    fn query(
        &self,
        query: &str,
        limit: usize,
        repo: Option<String>,
        all_repos: bool,
    ) -> Result<Vec<ScryHit>>;
}

#[derive(Debug, Default)]
pub struct RetrievalScryBackend;

impl ScryBackend for RetrievalScryBackend {
    fn query(
        &self,
        query: &str,
        limit: usize,
        repo: Option<String>,
        all_repos: bool,
    ) -> Result<Vec<ScryHit>> {
        let engine = QueryEngine::new();
        let options = QueryOptions {
            repo,
            all_repos,
            ..Default::default()
        };
        let results = engine.query_with_options(query, limit, &options)?;
        Ok(results
            .into_iter()
            .map(|r| ScryHit {
                content: r.content,
                score: r.fused_score,
                event_type: r.sources.join("+"),
                source_id: r.doc_id,
                timestamp: r.metadata.timestamp.unwrap_or_default(),
            })
            .collect())
    }
}
