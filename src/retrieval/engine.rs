//! QueryEngine - parallel multi-oracle retrieval with RRF fusion
//!
//! Handles federation across multiple repos while keeping oracles simple.
//! Oracles work on single projects; QueryEngine coordinates multi-repo queries.

use anyhow::Result;
use rayon::prelude::*;
use std::path::Path;

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

/// Options for multi-repo queries
///
/// These are MCP/interface-level options that QueryEngine handles.
/// Oracles stay simple - they don't know about repos.
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    /// Query a specific registered repo by name
    pub repo: Option<String>,
    /// Query all registered repos (current project + reference repos)
    pub all_repos: bool,
    /// Include GitHub issues in search results
    pub include_issues: bool,
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
    ///
    /// This is the simple single-project query. For multi-repo queries,
    /// use `query_with_options`.
    pub fn query(&self, query: &str, limit: usize) -> Result<Vec<FusedResult>> {
        self.query_local(query, limit)
    }

    /// Query with federation options (repo, all_repos, include_issues)
    ///
    /// Handles multi-repo queries by:
    /// 1. If all_repos: query current project + all registered repos
    /// 2. If repo: query only that specific repo
    /// 3. Otherwise: query current project only
    pub fn query_with_options(
        &self,
        query: &str,
        limit: usize,
        options: &QueryOptions,
    ) -> Result<Vec<FusedResult>> {
        // All repos mode: federate across current + registered repos
        if options.all_repos {
            return self.query_all_repos(query, limit, options);
        }

        // Specific repo mode: query that repo only
        if let Some(ref repo_name) = options.repo {
            return self.query_repo(query, limit, repo_name, options);
        }

        // Default: query current project with options
        self.query_local_with_options(query, limit, options)
    }

    /// Create oracles configured with the given options
    fn create_oracles(include_issues: bool) -> Vec<Box<dyn Oracle>> {
        vec![
            Box::new(SemanticOracle::new()),
            Box::new(LexicalOracle::with_options(include_issues)),
            Box::new(PersonaOracle::new()),
        ]
    }

    /// Query local project only (current directory) - uses default oracles
    fn query_local(&self, query: &str, limit: usize) -> Result<Vec<FusedResult>> {
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

    /// Query local project with options (creates oracles with include_issues if needed)
    fn query_local_with_options(
        &self,
        query: &str,
        limit: usize,
        options: &QueryOptions,
    ) -> Result<Vec<FusedResult>> {
        // If include_issues, create oracles with that config
        // Otherwise use default oracles for efficiency
        if options.include_issues {
            let oracles = Self::create_oracles(true);
            let fetch_limit = limit * self.config.fetch_multiplier;

            let oracle_results: Vec<_> = oracles
                .par_iter()
                .filter(|o| o.is_available())
                .filter(|o| self.matches_filter(o.name()))
                .filter_map(|oracle| oracle.query(query, fetch_limit).ok())
                .collect();

            Ok(rrf_fuse(oracle_results, self.config.rrf_k, limit))
        } else {
            self.query_local(query, limit)
        }
    }

    /// Query a specific registered repo
    fn query_repo(
        &self,
        query: &str,
        limit: usize,
        repo_name: &str,
        options: &QueryOptions,
    ) -> Result<Vec<FusedResult>> {
        use crate::commands::repo;

        // Get repo path from registry
        let repos = repo::list()?;
        let repo_entry = repos
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(repo_name))
            .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found in registry", repo_name))?;

        let repo_path = Path::new(&repo_entry.path);
        if !repo_path.exists() {
            anyhow::bail!("Repository path not found: {}", repo_entry.path);
        }

        // Query in repo context
        self.query_in_context(
            query,
            limit,
            repo_path,
            Some(repo_name),
            options.include_issues,
        )
    }

    /// Query all registered repos plus current project
    fn query_all_repos(
        &self,
        query: &str,
        limit: usize,
        options: &QueryOptions,
    ) -> Result<Vec<FusedResult>> {
        use crate::commands::repo;

        let mut all_results: Vec<Vec<super::oracle::OracleResult>> = Vec::new();

        // 1. Query current project if we're in one
        let current_dir = std::env::current_dir()?;
        if current_dir.join(".patina/data/patina.db").exists() {
            let local_results =
                self.collect_oracle_results(query, limit, options.include_issues)?;
            all_results.extend(local_results);
        }

        // 2. Query all registered repos
        let repos = repo::list()?;
        for repo_entry in repos {
            let repo_path = Path::new(&repo_entry.path);
            if !repo_path.exists() {
                eprintln!(
                    "patina: skipping repo '{}' - path not found",
                    repo_entry.name
                );
                continue;
            }

            match self.collect_oracle_results_in_context(
                query,
                limit,
                repo_path,
                &repo_entry.name,
                options.include_issues,
            ) {
                Ok(results) => all_results.extend(results),
                Err(e) => {
                    eprintln!("patina: repo '{}' query failed: {}", repo_entry.name, e);
                }
            }
        }

        // 3. RRF fuse all results together
        Ok(rrf_fuse(all_results, self.config.rrf_k, limit))
    }

    /// Query in a specific directory context
    fn query_in_context(
        &self,
        query: &str,
        limit: usize,
        context_path: &Path,
        repo_name: Option<&str>,
        include_issues: bool,
    ) -> Result<Vec<FusedResult>> {
        let results = self.collect_oracle_results_in_context(
            query,
            limit,
            context_path,
            repo_name.unwrap_or("unknown"),
            include_issues,
        )?;
        Ok(rrf_fuse(results, self.config.rrf_k, limit))
    }

    /// Collect raw oracle results (before RRF) for local context
    fn collect_oracle_results(
        &self,
        query: &str,
        limit: usize,
        include_issues: bool,
    ) -> Result<Vec<Vec<super::oracle::OracleResult>>> {
        let oracles = Self::create_oracles(include_issues);
        let fetch_limit = limit * self.config.fetch_multiplier;

        let results: Vec<_> = oracles
            .par_iter()
            .filter(|o| o.is_available())
            .filter(|o| self.matches_filter(o.name()))
            .filter_map(|oracle| oracle.query(query, fetch_limit).ok())
            .collect();

        Ok(results)
    }

    /// Collect raw oracle results in a different directory context
    fn collect_oracle_results_in_context(
        &self,
        query: &str,
        limit: usize,
        context_path: &Path,
        repo_name: &str,
        include_issues: bool,
    ) -> Result<Vec<Vec<super::oracle::OracleResult>>> {
        let original_dir = std::env::current_dir()?;

        // Change to repo directory
        std::env::set_current_dir(context_path)?;

        // Create fresh oracles for this context (they use relative paths)
        // Note: PersonaOracle is cross-project, only include once in main query
        let context_oracles: Vec<Box<dyn Oracle>> = vec![
            Box::new(SemanticOracle::new()),
            Box::new(LexicalOracle::with_options(include_issues)),
        ];

        let fetch_limit = limit * self.config.fetch_multiplier;

        // Query oracles in this context
        let results: Vec<Vec<super::oracle::OracleResult>> = context_oracles
            .par_iter()
            .filter(|o| o.is_available())
            .filter(|o| self.matches_filter(o.name()))
            .filter_map(|oracle| {
                oracle.query(query, fetch_limit).ok().map(|mut r| {
                    // Tag results with repo source for provenance
                    for result in &mut r {
                        result.doc_id = format!("[{}] {}", repo_name, result.doc_id);
                    }
                    r
                })
            })
            .collect();

        // Restore original directory
        std::env::set_current_dir(original_dir)?;

        Ok(results)
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
