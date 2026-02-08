//! QueryEngine - semantic vector search
//!
//! After the semantic-structural split, QueryEngine wraps the SemanticOracle only.
//! Factual search (FTS5, temporal, belief, persona) lives in assay.
//! The QueryEngine still handles multi-repo federation for semantic queries.

use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::time::Instant;

use super::fusion::{FusedResult, StructuralAnnotations};
use super::oracle::Oracle;
use super::oracles::SemanticOracle;

/// Retrieval configuration for QueryEngine
#[derive(Debug, Clone)]
pub struct RetrievalConfig {
    /// RRF smoothing constant (kept for backward compat, unused in semantic-only mode)
    pub rrf_k: usize,
    /// Over-fetch multiplier (default: 2)
    pub fetch_multiplier: usize,
    /// Filter to specific oracles (kept for backward compat with eval --oracle)
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

/// Options for multi-repo queries
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct QueryOptions {
    /// Query a specific registered repo by name
    pub repo: Option<String>,
    /// Query all registered repos (current project + reference repos)
    pub all_repos: bool,
    /// Include GitHub issues in search results (forwarded to assay, ignored by scry)
    pub include_issues: bool,
}

/// Query engine — semantic vector search with multi-repo federation
pub struct QueryEngine {
    oracle: SemanticOracle,
    config: RetrievalConfig,
}

impl QueryEngine {
    /// Create engine with default config
    pub fn new() -> Self {
        Self::with_config(RetrievalConfig::default())
    }

    /// Create engine with custom config
    pub fn with_config(config: RetrievalConfig) -> Self {
        Self {
            oracle: SemanticOracle::new(),
            config,
        }
    }

    /// Query the semantic oracle, returning ranked results
    pub fn query(&self, query: &str, limit: usize) -> Result<Vec<FusedResult>> {
        self.query_local(query, limit)
    }

    /// Query with federation options (repo, all_repos)
    pub fn query_with_options(
        &self,
        query: &str,
        limit: usize,
        options: &QueryOptions,
    ) -> Result<Vec<FusedResult>> {
        if options.all_repos {
            return self.query_all_repos(query, limit);
        }

        if let Some(ref repo_name) = options.repo {
            return self.query_repo(query, limit, repo_name);
        }

        self.query_local(query, limit)
    }

    /// Query local project's semantic index
    fn query_local(&self, query: &str, limit: usize) -> Result<Vec<FusedResult>> {
        let start = Instant::now();

        if !self.oracle.is_available() {
            // Graceful fallback: no semantic index available
            if std::env::var("PATINA_LOG").is_ok() {
                eprintln!("[DEBUG retrieval::engine] semantic oracle not available, returning empty");
            }
            return Ok(Vec::new());
        }

        // Check oracle filter (backward compat with eval --oracle)
        if let Some(ref filter) = self.config.oracle_filter {
            if !filter.iter().any(|f| f.eq_ignore_ascii_case("semantic")) {
                return Ok(Vec::new());
            }
        }

        let fetch_limit = limit * self.config.fetch_multiplier;
        let oracle_results = self.oracle.query(query, fetch_limit)?;

        // Convert OracleResult -> FusedResult (no fusion needed, single oracle)
        let mut results: Vec<FusedResult> = oracle_results
            .into_iter()
            .map(|r| {
                let mut contributions = std::collections::HashMap::new();
                contributions.insert(
                    r.source,
                    super::fusion::OracleContribution {
                        rank: 1,
                        raw_score: r.score,
                        score_type: r.score_type,
                        matches: r.metadata.matches.clone(),
                    },
                );

                FusedResult {
                    doc_id: r.doc_id,
                    content: r.content,
                    fused_score: r.score,
                    sources: vec![r.source],
                    contributions,
                    metadata: r.metadata,
                    annotations: StructuralAnnotations::default(),
                }
            })
            .collect();

        results.truncate(limit);
        populate_annotations(&mut results);

        if std::env::var("PATINA_LOG").is_ok() {
            eprintln!(
                "[DEBUG retrieval::engine] semantic query: {} results in {:?}",
                results.len(),
                start.elapsed()
            );
        }

        Ok(results)
    }

    /// Query a specific registered repo's semantic index
    fn query_repo(
        &self,
        query: &str,
        limit: usize,
        repo_name: &str,
    ) -> Result<Vec<FusedResult>> {
        use crate::commands::repo;

        let repos = repo::list()?;
        let repo_entry = repos
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(repo_name))
            .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found in registry", repo_name))?;

        let repo_path = Path::new(&repo_entry.path);
        if !repo_path.exists() {
            anyhow::bail!("Repository path not found: {}", repo_entry.path);
        }

        self.query_in_context(query, limit, repo_path, Some(repo_name))
    }

    /// Query all registered repos plus current project
    fn query_all_repos(&self, query: &str, limit: usize) -> Result<Vec<FusedResult>> {
        use crate::commands::repo;

        let mut all_results: Vec<FusedResult> = Vec::new();

        // 1. Query current project
        let current_dir = std::env::current_dir()?;
        if current_dir.join(".patina/local/data/patina.db").exists() {
            if let Ok(results) = self.query_local(query, limit) {
                all_results.extend(results);
            }
        }

        // 2. Query all registered repos
        let repos = repo::list()?;
        for repo_entry in repos {
            let repo_path = Path::new(&repo_entry.path);
            if !repo_path.exists() {
                continue;
            }

            if let Ok(results) =
                self.query_in_context(query, limit, repo_path, Some(&repo_entry.name))
            {
                all_results.extend(results);
            }
        }

        // Sort all results by score and truncate
        all_results.sort_by(|a, b| {
            b.fused_score
                .partial_cmp(&a.fused_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.truncate(limit);

        Ok(all_results)
    }

    /// Query in a specific directory context (for repo queries)
    fn query_in_context(
        &self,
        query: &str,
        limit: usize,
        context_path: &Path,
        repo_name: Option<&str>,
    ) -> Result<Vec<FusedResult>> {
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(context_path)?;

        // Create a fresh semantic oracle for this context
        let oracle = SemanticOracle::new();
        let fetch_limit = limit * self.config.fetch_multiplier;

        let result = if oracle.is_available() {
            oracle.query(query, fetch_limit).ok()
        } else {
            None
        };

        std::env::set_current_dir(original_dir)?;

        let results: Vec<FusedResult> = result
            .unwrap_or_default()
            .into_iter()
            .map(|mut r| {
                // Tag with repo name for provenance
                if let Some(name) = repo_name {
                    r.doc_id = format!("[{}] {}", name, r.doc_id);
                }

                let mut contributions = std::collections::HashMap::new();
                contributions.insert(
                    r.source,
                    super::fusion::OracleContribution {
                        rank: 1,
                        raw_score: r.score,
                        score_type: r.score_type,
                        matches: r.metadata.matches.clone(),
                    },
                );

                FusedResult {
                    doc_id: r.doc_id,
                    content: r.content,
                    fused_score: r.score,
                    sources: vec![r.source],
                    contributions,
                    metadata: r.metadata,
                    annotations: StructuralAnnotations::default(),
                }
            })
            .take(limit)
            .collect();

        Ok(results)
    }

    /// List available oracles (backward compat)
    pub fn available_oracles(&self) -> Vec<&'static str> {
        if self.oracle.is_available() {
            vec!["semantic"]
        } else {
            vec![]
        }
    }
}

impl Default for QueryEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Populate structural annotations from module_signals table
fn populate_annotations(results: &mut [FusedResult]) {
    const DB_PATH: &str = ".patina/local/data/patina.db";

    let conn = match Connection::open(DB_PATH) {
        Ok(c) => c,
        Err(_) => return,
    };

    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='module_signals'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !table_exists {
        return;
    }

    for result in results.iter_mut() {
        let file_path = extract_file_path(&result.doc_id);
        if file_path.is_empty() || file_path.starts_with("persona:") {
            continue;
        }

        let paths_to_try = vec![
            file_path.clone(),
            file_path.trim_start_matches("./").to_string(),
            format!("./{}", file_path.trim_start_matches("./")),
        ];

        for path in paths_to_try {
            if let Ok(annotations) = conn.query_row(
                "SELECT importer_count, activity_level, is_entry_point, is_test_file
                 FROM module_signals WHERE path = ?",
                [&path],
                |row| {
                    Ok(StructuralAnnotations {
                        importer_count: row.get(0).ok(),
                        activity_level: row.get(1).ok(),
                        is_entry_point: row.get::<_, Option<i32>>(2).ok().flatten().map(|v| v != 0),
                        is_test_file: row.get::<_, Option<i32>>(3).ok().flatten().map(|v| v != 0),
                    })
                },
            ) {
                result.annotations = annotations;
                break;
            }
        }
    }
}

/// Extract file path from doc_id
fn extract_file_path(doc_id: &str) -> String {
    if doc_id.starts_with("persona:") {
        return doc_id.to_string();
    }
    if let Some(idx) = doc_id.find("::") {
        doc_id[..idx].to_string()
    } else {
        doc_id.to_string()
    }
}
