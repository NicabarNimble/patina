//! Result enrichment with SQLite metadata
//!
//! Enriches vector search results with metadata from the SQLite database.
//! Handles different content types (semantic, temporal, dependency).

use anyhow::{Context, Result};
use rusqlite::Connection;

use super::super::ScryResult;

/// Search results from USearch
pub struct SearchResults {
    pub keys: Vec<u64>,
    pub distances: Vec<f32>,
}

/// Enrich vector search results with SQLite metadata
pub fn enrich_results(
    conn: &Connection,
    results: &SearchResults,
    dimension: &str,
    min_score: f32,
) -> Result<Vec<ScryResult>> {
    let mut enriched = Vec::new();

    // ID offsets to distinguish different content types in semantic index
    const CODE_ID_OFFSET: i64 = 1_000_000_000;
    const PATTERN_ID_OFFSET: i64 = 2_000_000_000;

    match dimension {
        "semantic" => {
            // Semantic index contains eventlog entries, code facts, and patterns
            for i in 0..results.keys.len() {
                let key = results.keys[i] as i64;
                let distance = results.distances[i];
                // Convert distance to similarity score (cosine: 1 - distance)
                let score = 1.0 - distance;

                if score < min_score {
                    continue;
                }

                // Check content type based on ID range
                if key >= PATTERN_ID_OFFSET {
                    // Pattern - look up in patterns table
                    let rowid = key - PATTERN_ID_OFFSET;
                    let result = conn.query_row(
                        "SELECT rowid, id, title, purpose, layer, file_path
                         FROM patterns
                         WHERE rowid = ?",
                        [rowid],
                        |row| {
                            let id: String = row.get(1)?;
                            let title: String = row.get(2)?;
                            let purpose: Option<String> = row.get(3)?;
                            let layer: String = row.get(4)?;
                            let file_path: String = row.get(5)?;

                            // Build description
                            let desc = if let Some(p) = purpose {
                                format!("{}: {}", title, p)
                            } else {
                                title.clone()
                            };

                            Ok(ScryResult {
                                id: key,
                                event_type: format!("pattern.{}", layer),
                                source_id: id,
                                timestamp: String::new(),
                                content: format!("{} ({})", desc, file_path),
                                score,
                            })
                        },
                    );

                    if let Ok(r) = result {
                        enriched.push(r);
                    }
                } else if key >= CODE_ID_OFFSET {
                    // Code fact - look up in function_facts
                    let rowid = key - CODE_ID_OFFSET;
                    let result = conn.query_row(
                        "SELECT rowid, file, name, parameters, return_type, is_public, is_async
                         FROM function_facts
                         WHERE rowid = ?",
                        [rowid],
                        |row| {
                            let file: String = row.get(1)?;
                            let name: String = row.get(2)?;
                            let params: Option<String> = row.get(3)?;
                            let return_type: Option<String> = row.get(4)?;
                            let is_public: bool = row.get(5)?;
                            let is_async: bool = row.get(6)?;

                            // Reconstruct the description
                            let mut desc = format!("Function `{}` in `{}`", name, file);
                            if is_public {
                                desc.push_str(", public");
                            }
                            if is_async {
                                desc.push_str(", async");
                            }
                            if let Some(p) = params {
                                if !p.is_empty() {
                                    desc.push_str(&format!(", params: {}", p));
                                }
                            }
                            if let Some(rt) = return_type {
                                if !rt.is_empty() {
                                    desc.push_str(&format!(", returns: {}", rt));
                                }
                            }

                            Ok(ScryResult {
                                id: key,
                                event_type: "code.function".to_string(),
                                // Use :: to match eventlog source_id format (path::name)
                                source_id: format!("{}::{}", file, name),
                                timestamp: String::new(),
                                content: desc,
                                score,
                            })
                        },
                    );

                    if let Ok(r) = result {
                        enriched.push(r);
                    }
                } else {
                    // Eventlog entry
                    let result = conn.query_row(
                        "SELECT seq, event_type, source_id, timestamp,
                                json_extract(data, '$.content') as content
                         FROM eventlog
                         WHERE seq = ?",
                        [key],
                        |row| {
                            Ok(ScryResult {
                                id: row.get(0)?,
                                event_type: row.get(1)?,
                                source_id: row.get(2)?,
                                timestamp: row.get(3)?,
                                content: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                                score,
                            })
                        },
                    );

                    if let Ok(r) = result {
                        enriched.push(r);
                    }
                }
            }
        }
        "temporal" => {
            // Temporal index uses sequential file index as key
            // Need to look up file path from the index
            let files: Vec<String> = {
                let mut stmt = conn.prepare(
                    "SELECT DISTINCT file_a FROM co_changes
                     UNION
                     SELECT DISTINCT file_b FROM co_changes
                     ORDER BY 1",
                )?;
                let mut rows = stmt.query([])?;
                let mut files = Vec::new();
                while let Some(row) = rows.next()? {
                    files.push(row.get(0)?);
                }
                files
            };

            for i in 0..results.keys.len() {
                let key = results.keys[i] as usize;
                let distance = results.distances[i];
                let score = 1.0 - distance;

                if score < min_score {
                    continue;
                }

                if key < files.len() {
                    let file_path = &files[key];
                    enriched.push(ScryResult {
                        id: key as i64,
                        event_type: "file.cochange".to_string(),
                        source_id: file_path.clone(),
                        timestamp: String::new(),
                        content: format!("File: {} (temporal co-change relationship)", file_path),
                        score,
                    });
                }
            }
        }
        "dependency" => {
            // Dependency index uses sequential function index as key
            // Need to look up function name from call_graph
            let functions: Vec<String> = {
                let mut stmt = conn.prepare(
                    "SELECT DISTINCT caller FROM call_graph
                     UNION
                     SELECT DISTINCT callee FROM call_graph
                     ORDER BY 1",
                )?;
                let mut rows = stmt.query([])?;
                let mut funcs = Vec::new();
                while let Some(row) = rows.next()? {
                    funcs.push(row.get(0)?);
                }
                funcs
            };

            for i in 0..results.keys.len() {
                let key = results.keys[i] as usize;
                let distance = results.distances[i];
                let score = 1.0 - distance;

                if score < min_score {
                    continue;
                }

                if key < functions.len() {
                    let func_name = &functions[key];
                    enriched.push(ScryResult {
                        id: key as i64,
                        event_type: "function.dependency".to_string(),
                        source_id: func_name.clone(),
                        timestamp: String::new(),
                        content: format!("Function: {} (dependency relationship)", func_name),
                        score,
                    });
                }
            }
        }
        _ => {
            anyhow::bail!("Unknown dimension: {}", dimension);
        }
    }

    // Sort by score descending
    enriched.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(enriched)
}

/// Truncate content for display
pub fn truncate_content(content: &str, max_len: usize) -> String {
    let content = content.replace('\n', " ").trim().to_string();
    if content.len() <= max_len {
        content
    } else {
        format!("{}...", &content[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_content() {
        assert_eq!(truncate_content("short", 10), "short");
        assert_eq!(truncate_content("a very long string", 10), "a very lon...");
        assert_eq!(truncate_content("with\nnewlines", 20), "with newlines");
    }
}
