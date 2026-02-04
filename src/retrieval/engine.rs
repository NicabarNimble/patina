//! QueryEngine - parallel multi-oracle retrieval with RRF fusion
//!
//! Handles federation across multiple repos while keeping oracles simple.
//! Oracles work on single projects; QueryEngine coordinates multi-repo queries.

use anyhow::Result;
use rayon::prelude::*;
use rusqlite::Connection;
use std::path::Path;
use std::time::Instant;

use super::fusion::{rrf_fuse, rrf_fuse_weighted, FusedResult, StructuralAnnotations};
use super::intent::{detect_intent, IntentWeights};
use super::oracle::Oracle;
use super::oracles::{BeliefOracle, LexicalOracle, PersonaOracle, SemanticOracle, TemporalOracle};

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
        // Oracles for retrieval - structural signals available via `assay` tool directly
        let oracles: Vec<Box<dyn Oracle>> = vec![
            Box::new(SemanticOracle::new()),
            Box::new(LexicalOracle::new()),
            Box::new(TemporalOracle::new()),
            Box::new(PersonaOracle::new()),
            Box::new(BeliefOracle::new()),
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
            Box::new(TemporalOracle::new()),
            Box::new(PersonaOracle::new()),
            Box::new(BeliefOracle::new()),
        ]
    }

    /// Query local project only (current directory) - uses default oracles
    fn query_local(&self, query: &str, limit: usize) -> Result<Vec<FusedResult>> {
        let start = Instant::now();

        // Detect intent from query for weighted fusion
        let intent = detect_intent(query);
        let weights = IntentWeights::for_intent(intent);

        // Log intent detection if PATINA_LOG is set
        if std::env::var("PATINA_LOG").is_ok() {
            eprintln!(
                "[DEBUG retrieval::engine] detected intent: {:?} for query: \"{}\"",
                intent,
                &query[..query.len().min(50)]
            );
        }

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

        let oracle_elapsed = start.elapsed();

        // Log per-oracle contributions before fusion
        if std::env::var("PATINA_LOG").is_ok() {
            log_oracle_contributions(&oracle_results, query);
        }

        // Fuse with RRF using intent-aware weights
        let mut results =
            rrf_fuse_weighted(oracle_results, self.config.rrf_k, limit, Some(&weights));

        // Populate structural annotations from module_signals
        populate_annotations(&mut results);

        // Log timing if PATINA_LOG is set
        if std::env::var("PATINA_LOG").is_ok() {
            eprintln!(
                "[DEBUG retrieval::engine] query complete: {} results in {:?} (oracles: {:?})",
                results.len(),
                start.elapsed(),
                oracle_elapsed
            );
        }

        Ok(results)
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
            let start = Instant::now();

            // Detect intent from query for weighted fusion
            let intent = detect_intent(query);
            let weights = IntentWeights::for_intent(intent);

            let oracles = Self::create_oracles(true);
            let fetch_limit = limit * self.config.fetch_multiplier;

            let oracle_results: Vec<_> = oracles
                .par_iter()
                .filter(|o| o.is_available())
                .filter(|o| self.matches_filter(o.name()))
                .filter_map(|oracle| oracle.query(query, fetch_limit).ok())
                .collect();

            let oracle_elapsed = start.elapsed();

            // Log per-oracle contributions before fusion
            if std::env::var("PATINA_LOG").is_ok() {
                log_oracle_contributions(&oracle_results, query);
            }

            let mut results =
                rrf_fuse_weighted(oracle_results, self.config.rrf_k, limit, Some(&weights));
            populate_annotations(&mut results);

            if std::env::var("PATINA_LOG").is_ok() {
                eprintln!(
                    "[DEBUG retrieval::engine] query (with issues) complete: {} results in {:?} (oracles: {:?})",
                    results.len(),
                    start.elapsed(),
                    oracle_elapsed
                );
            }

            Ok(results)
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
        if current_dir.join(".patina/local/data/patina.db").exists() {
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
        let fused = rrf_fuse(results, self.config.rrf_k, limit);
        // Note: annotations for external repos would need context switch
        // For now, skip annotations for repo queries
        Ok(fused)
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
            Box::new(TemporalOracle::new()),
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

/// Populate structural annotations from module_signals table
///
/// Best-effort: if database or table doesn't exist, results are unchanged
fn populate_annotations(results: &mut [FusedResult]) {
    const DB_PATH: &str = ".patina/local/data/patina.db";

    let conn = match Connection::open(DB_PATH) {
        Ok(c) => c,
        Err(_) => return, // No database, skip annotations
    };

    // Check if module_signals table exists
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
        // Extract file path from doc_id
        // Handles: "./src/main.rs::fn:main" -> "./src/main.rs"
        //          "src/main.rs" -> "src/main.rs"
        //          "persona:direct:..." -> skip (no file)
        let file_path = extract_file_path(&result.doc_id);

        if file_path.is_empty() || file_path.starts_with("persona:") {
            continue;
        }

        // Try to find module signals for this file
        // Try with and without leading "./"
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

/// Log per-oracle contributions before RRF fusion
///
/// This data enables future intent analysis (Phase 3 of retrieval optimization).
/// Logs: query, oracle_name, doc_id, rank for each result
fn log_oracle_contributions(oracle_results: &[Vec<super::oracle::OracleResult>], query: &str) {
    // Count results per oracle
    let mut oracle_counts: Vec<(String, usize)> = Vec::new();

    for results in oracle_results {
        if let Some(first) = results.first() {
            let oracle_name = first.source;
            oracle_counts.push((oracle_name.to_string(), results.len()));

            // Log top-3 doc_ids from each oracle for debugging
            eprintln!(
                "[DEBUG retrieval::oracle] {} returned {} results, top-3: {:?}",
                oracle_name,
                results.len(),
                results
                    .iter()
                    .take(3)
                    .map(|r| &r.doc_id)
                    .collect::<Vec<_>>()
            );
        }
    }

    // Summary line
    let counts_str: String = oracle_counts
        .iter()
        .map(|(name, count)| format!("{}:{}", name, count))
        .collect::<Vec<_>>()
        .join(", ");
    eprintln!(
        "[DEBUG retrieval::engine] oracle contributions for \"{}\": [{}]",
        &query[..query.len().min(50)],
        counts_str
    );
}

/// Extract file path from doc_id
///
/// doc_id formats:
/// - "./src/main.rs::fn:main" -> "./src/main.rs"
/// - "./src/retrieval/fusion.rs::rrf_fuse:rrf_fuse" -> "./src/retrieval/fusion.rs"
/// - "src/main.rs" -> "src/main.rs"
/// - "persona:direct:2025-12-08" -> "persona:direct:2025-12-08" (no file, skip)
fn extract_file_path(doc_id: &str) -> String {
    // If starts with persona:, it's not a file
    if doc_id.starts_with("persona:") {
        return doc_id.to_string();
    }

    // Find :: which separates file path from symbol
    if let Some(idx) = doc_id.find("::") {
        doc_id[..idx].to_string()
    } else {
        doc_id.to_string()
    }
}
