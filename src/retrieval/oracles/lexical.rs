//! Lexical oracle - BM25/FTS5 text search
//!
//! Searches code_fts, commits_fts, and pattern_fts using FTS5.
//! When commits are found, expands to include files touched by those commits.

use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;

use crate::commands::scry::{scry_lexical, ScryOptions};
use crate::retrieval::oracle::{Oracle, OracleMetadata, OracleResult};

pub struct LexicalOracle {
    db_path: PathBuf,
    include_issues: bool,
}

impl LexicalOracle {
    pub fn new() -> Self {
        Self::with_options(false)
    }

    pub fn with_options(include_issues: bool) -> Self {
        Self {
            db_path: PathBuf::from(".patina/data/patina.db"),
            include_issues,
        }
    }
}

impl LexicalOracle {
    /// Expand git.commit results to include files touched by those commits
    ///
    /// When a commit matches via commits_fts, this follows the commit→file
    /// relationship by looking up commit_files and adding those files as
    /// additional results with a derived score.
    fn expand_commit_files(
        &self,
        results: &mut Vec<OracleResult>,
        source: &'static str,
        query_terms: &[String],
    ) {
        // Open database connection
        let conn = match Connection::open(&self.db_path) {
            Ok(c) => c,
            Err(_) => return, // No database, skip expansion
        };

        // Collect commit SHAs and their scores for expansion
        let commit_results: Vec<(String, f32)> = results
            .iter()
            .filter_map(|r| {
                if r.metadata.event_type.as_deref() == Some("git.commit") {
                    Some((r.doc_id.clone(), r.score))
                } else {
                    None
                }
            })
            .collect();

        // For each commit, look up its files and add as results
        for (sha, parent_score) in commit_results {
            let file_query =
                conn.prepare("SELECT file_path FROM commit_files WHERE sha = ? LIMIT 15");

            let mut stmt = match file_query {
                Ok(s) => s,
                Err(_) => continue, // Table might not exist
            };

            let file_paths: Vec<String> = stmt
                .query_map([&sha], |row| row.get(0))
                .ok()
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
                .unwrap_or_default();

            // Add file results with derived score (parent * 0.9)
            let derived_score = parent_score * 0.9;
            for file_path in file_paths {
                results.push(OracleResult {
                    doc_id: file_path.clone(),
                    content: format!("Changed in commit {}", &sha[..sha.len().min(7)]),
                    source,
                    score: derived_score,
                    score_type: "bm25_derived",
                    metadata: OracleMetadata {
                        file_path: Some(file_path),
                        timestamp: None,
                        event_type: Some("commit_file".to_string()),
                        matches: Some(query_terms.to_vec()),
                    },
                });
            }
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
            include_issues: self.include_issues,
            ..Default::default()
        };

        let results = scry_lexical(query, &options)?;
        let source = self.name();

        // Extract query terms as matches (what we searched for)
        let query_terms: Vec<String> = query
            .split_whitespace()
            .filter(|t| t.len() > 1)
            .map(|t| t.to_string())
            .collect();

        let mut oracle_results: Vec<OracleResult> = results
            .into_iter()
            .map(|r| OracleResult {
                doc_id: r.source_id.clone(),
                content: r.content,
                source,
                score: r.score,
                score_type: "bm25",
                metadata: OracleMetadata {
                    file_path: Some(r.source_id),
                    timestamp: if r.timestamp.is_empty() {
                        None
                    } else {
                        Some(r.timestamp)
                    },
                    event_type: Some(r.event_type),
                    matches: Some(query_terms.clone()),
                },
            })
            .collect();

        // Expand git.commit results to include touched files
        self.expand_commit_files(&mut oracle_results, source, &query_terms);

        Ok(oracle_results)
    }

    fn is_available(&self) -> bool {
        self.db_path.exists()
    }
}
