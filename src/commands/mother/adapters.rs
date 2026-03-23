use anyhow::Result;
use std::path::PathBuf;

use crate::commands::repo::internal::Registry;
use crate::retrieval::{QueryEngine, QueryOptions};

#[derive(Debug, Clone)]
pub struct RegistryProject {
    pub name: String,
    pub path: String,
    pub domains: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RegistryRepo {
    pub name: String,
    pub path: String,
    pub domains: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RegistrySnapshot {
    pub projects: Vec<RegistryProject>,
    pub repos: Vec<RegistryRepo>,
}

pub trait RegistryBackend: Send + Sync {
    fn load_snapshot(&self) -> Result<RegistrySnapshot>;
}

#[derive(Debug, Default)]
pub struct RepoRegistryBackend;

impl RegistryBackend for RepoRegistryBackend {
    fn load_snapshot(&self) -> Result<RegistrySnapshot> {
        let registry = Registry::load()?;
        let projects = registry
            .projects
            .iter()
            .map(|(name, entry)| RegistryProject {
                name: name.clone(),
                path: entry.path.clone(),
                domains: entry.domains.clone(),
            })
            .collect();
        let repos = registry
            .repos
            .iter()
            .map(|(name, entry)| RegistryRepo {
                name: name.clone(),
                path: entry.path.clone(),
                domains: entry.domains.clone(),
            })
            .collect();
        Ok(RegistrySnapshot { projects, repos })
    }
}

pub trait ProjectRootProvider: Send + Sync {
    fn find_current_project_root(&self) -> Option<PathBuf>;
}

#[derive(Debug, Default)]
pub struct SessionProjectRootProvider;

impl ProjectRootProvider for SessionProjectRootProvider {
    fn find_current_project_root(&self) -> Option<PathBuf> {
        patina::session::SessionManager::find_project_root().ok()
    }
}

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
