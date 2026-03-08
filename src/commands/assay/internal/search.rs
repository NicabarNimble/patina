//! Ranked factual search using FTS5
//!
//! Provides ranked text search across code, commits, and patterns using
//! FTS5 full-text search with per-table min-max normalization.
//!
//! Moved from scry during semantic-structural split (Phase 1).
//! Factual/keyword search is assay's domain — scry handles meaning.

use anyhow::{Context, Result};
use rusqlite::Connection;

use super::query_prep::prepare_fts_query;

const DB_PATH: &str = ".patina/local/data/patina.db";

/// A single ranked search result from FTS5
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub content: String,
    pub score: f32,
    pub event_type: String,
    pub source_id: String,
    pub timestamp: String,
}

/// Options for assay search
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub limit: usize,
    pub include_issues: bool,
    pub repo: Option<String>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            include_issues: false,
            repo: None,
        }
    }
}

/// Execute ranked factual search using FTS5 with per-table min-max normalization
///
/// Each FTS5 table (code, commits, patterns) has different column counts and
/// corpus sizes, making raw BM25 scores incomparable. We normalize per-table
/// using log1p + min-max to [0,1], preserving within-table score magnitude
/// while making cross-table scores comparable.
pub fn assay_search(query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>> {
    let db_path = match &options.repo {
        Some(name) => crate::commands::repo::get_db_path(name)?,
        None => DB_PATH.to_string(),
    };

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Prepare the FTS5 query
    let fts_query = prepare_fts_query(query);

    // Collect results per-table with raw BM25 scores
    let mut code_results = search_code_fts(&conn, &fts_query, options)?;
    let mut commit_results = search_commits_fts(&conn, &fts_query, options)?;
    let mut pattern_results = search_pattern_fts(&conn, &fts_query, options)?;
    let mut eventlog_results = search_eventlog_fts(&conn, &fts_query, options)?;

    // Min-max normalization per table: log1p transform + scale to [0,1]
    normalize_table(&mut code_results);
    normalize_table(&mut commit_results);
    normalize_table(&mut pattern_results);
    normalize_table(&mut eventlog_results);

    // Merge all tables, sort by normalized score desc
    let mut collected: Vec<SearchResult> = Vec::new();
    collected.extend(code_results);
    collected.extend(commit_results);
    collected.extend(pattern_results);
    collected.extend(eventlog_results);

    collected.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    collected.truncate(options.limit);

    Ok(collected)
}

/// Execute ranked factual search and return JSON
pub fn assay_search_json(query: &str, options: &SearchOptions) -> Result<String> {
    let results = assay_search(query, options)?;

    let json_results: Vec<serde_json::Value> = results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            serde_json::json!({
                "rank": i + 1,
                "score": r.score,
                "event_type": r.event_type,
                "source_id": r.source_id,
                "content": r.content,
            })
        })
        .collect();

    Ok(serde_json::to_string_pretty(&json_results)?)
}

/// Execute ranked factual search and print results (CLI)
pub fn execute_search(query: &str, options: &SearchOptions) -> Result<()> {
    let results = assay_search(query, options)?;

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    println!("Found {} results:\n", results.len());
    println!("{}", "─".repeat(60));

    for (i, result) in results.iter().enumerate() {
        let timestamp_display = if result.timestamp.is_empty() {
            String::new()
        } else {
            format!(" | {}", result.timestamp)
        };
        println!(
            "\n[{}] Score: {:.3} | {} | {}{}",
            i + 1,
            result.score,
            result.event_type,
            result.source_id,
            timestamp_display
        );
        println!("    {}", truncate(&result.content, 200));
    }

    println!("\n{}", "─".repeat(60));
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    let collapsed = s.replace('\n', " ");
    let trimmed = collapsed.trim();
    if trimmed.chars().count() <= max {
        trimmed.to_string()
    } else {
        let truncated: String = trimmed.chars().take(max).collect();
        format!("{}...", truncated)
    }
}

/// Search code_fts table
fn search_code_fts(
    conn: &Connection,
    fts_query: &str,
    options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
    let event_type_filter = if options.include_issues {
        "event_type LIKE 'code.%' OR event_type LIKE 'forge.%' OR event_type LIKE 'github.%'"
    } else {
        "event_type LIKE 'code.%'"
    };

    let sql = format!(
        "SELECT
            symbol_name,
            file_path,
            snippet(code_fts, 2, '>>>', '<<<', '...', 64) as snippet,
            event_type,
            bm25(code_fts) as score
         FROM code_fts
         WHERE code_fts MATCH ?
           AND ({})
         ORDER BY score
         LIMIT ?",
        event_type_filter
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params![fts_query, options.limit as i64], |row| {
        let symbol: String = row.get(0)?;
        let file_path: String = row.get(1)?;
        let snippet: String = row.get(2)?;
        let event_type: String = row.get(3)?;
        let bm25_score: f64 = row.get(4)?;

        let source_id = if event_type == "forge.issue" || event_type == "github.issue" {
            format!("[ISSUE] {}", symbol)
        } else if event_type == "forge.pr" || event_type == "github.pr" {
            format!("[PR] {}", symbol)
        } else {
            file_path
        };

        Ok(SearchResult {
            content: snippet,
            score: -bm25_score as f32,
            event_type,
            source_id,
            timestamp: String::new(),
        })
    })?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Search commits_fts table
fn search_commits_fts(
    conn: &Connection,
    fts_query: &str,
    options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
    let sql = "SELECT
            sha,
            snippet(commits_fts, 1, '>>>', '<<<', '...', 64) as snippet,
            author_name,
            bm25(commits_fts) as score
         FROM commits_fts
         WHERE commits_fts MATCH ?
         ORDER BY score
         LIMIT ?";

    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return Ok(Vec::new()),
    };

    let rows = stmt.query_map(rusqlite::params![fts_query, options.limit as i64], |row| {
        let sha: String = row.get(0)?;
        let snippet: String = row.get(1)?;
        let author: String = row.get(2)?;
        let bm25_score: f64 = row.get(3)?;

        Ok(SearchResult {
            content: format!("{} ({})", snippet, author),
            score: -bm25_score as f32,
            event_type: "git.commit".to_string(),
            source_id: sha,
            timestamp: String::new(),
        })
    })?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Search pattern_fts table
fn search_pattern_fts(
    conn: &Connection,
    fts_query: &str,
    options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
    let sql = "SELECT
            id,
            title,
            snippet(pattern_fts, 2, '>>>', '<<<', '...', 64) as snippet,
            file_path,
            bm25(pattern_fts) as score
         FROM pattern_fts
         WHERE pattern_fts MATCH ?
         ORDER BY score
         LIMIT ?";

    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return Ok(Vec::new()),
    };

    let rows = stmt.query_map(rusqlite::params![fts_query, options.limit as i64], |row| {
        let _id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let snippet: String = row.get(2)?;
        let file_path: String = row.get(3)?;
        let bm25_score: f64 = row.get(4)?;

        let layer = if file_path.contains("layer/core") {
            "core"
        } else {
            "surface"
        };

        Ok(SearchResult {
            content: format!("{}: {}", title, snippet),
            score: -bm25_score as f32,
            event_type: format!("pattern.{}", layer),
            source_id: file_path,
            timestamp: String::new(),
        })
    })?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Search eventlog_fts table (session decisions, patterns, work, context)
fn search_eventlog_fts(
    conn: &Connection,
    fts_query: &str,
    options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
    let sql = "SELECT
            source_id,
            event_type,
            snippet(eventlog_fts, 2, '>>>', '<<<', '...', 64) as snippet,
            bm25(eventlog_fts) as score
         FROM eventlog_fts
         WHERE eventlog_fts MATCH ?
         ORDER BY score
         LIMIT ?";

    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return Ok(Vec::new()),
    };

    let rows = stmt.query_map(rusqlite::params![fts_query, options.limit as i64], |row| {
        let source_id: String = row.get(0)?;
        let event_type: String = row.get(1)?;
        let snippet: String = row.get(2)?;
        let bm25_score: f64 = row.get(3)?;

        Ok(SearchResult {
            content: snippet,
            score: -bm25_score as f32,
            event_type,
            source_id,
            timestamp: String::new(),
        })
    })?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Normalize a table's BM25 scores to [0,1] using log1p + min-max.
///
/// log1p reduces outlier compression (a score of 15 vs 4 becomes 2.77 vs 1.61
/// instead of dominating the range). Min-max then scales to [0,1] within the
/// table. If all scores are identical (or only one result), all get 1.0.
fn normalize_table(results: &mut [SearchResult]) {
    if results.is_empty() {
        return;
    }

    // Transform: log1p to reduce outlier compression
    let transformed: Vec<f32> = results
        .iter()
        .map(|r| (r.score as f64 + 1.0).ln() as f32)
        .collect();

    let t_min = transformed.iter().cloned().fold(f32::INFINITY, f32::min);
    let t_max = transformed
        .iter()
        .cloned()
        .fold(f32::NEG_INFINITY, f32::max);
    let range = t_max - t_min;

    const EPS: f32 = 1e-8;

    for (result, &t) in results.iter_mut().zip(transformed.iter()) {
        if range < EPS {
            // All scores identical or single result — all equally "best"
            result.score = 1.0;
        } else {
            result.score = (t - t_min) / range;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_table_empty() {
        let mut results: Vec<SearchResult> = vec![];
        normalize_table(&mut results);
        assert!(results.is_empty());
    }

    #[test]
    fn test_normalize_table_single() {
        let mut results = vec![SearchResult {
            content: "test".to_string(),
            score: 5.0,
            event_type: "code.function".to_string(),
            source_id: "test.rs".to_string(),
            timestamp: String::new(),
        }];
        normalize_table(&mut results);
        assert_eq!(results[0].score, 1.0);
    }

    #[test]
    fn test_normalize_table_range() {
        let mut results = vec![
            SearchResult {
                content: "high".to_string(),
                score: 10.0,
                event_type: "code.function".to_string(),
                source_id: "a.rs".to_string(),
                timestamp: String::new(),
            },
            SearchResult {
                content: "low".to_string(),
                score: 1.0,
                event_type: "code.function".to_string(),
                source_id: "b.rs".to_string(),
                timestamp: String::new(),
            },
        ];
        normalize_table(&mut results);
        assert!((results[0].score - 1.0).abs() < 0.01);
        assert!((results[1].score - 0.0).abs() < 0.01);
    }
}
