//! Result enrichment with SQLite metadata
//!
//! Enriches vector search results with metadata from the SQLite database.
//! Handles different content types (semantic, temporal, dependency).

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

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
    const COMMIT_ID_OFFSET: i64 = 3_000_000_000;
    const BELIEF_ID_OFFSET: i64 = 4_000_000_000;

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

                // Check content type based on ID range (order matters: highest offset first)
                if key >= BELIEF_ID_OFFSET {
                    // Belief - look up in beliefs table with computed metrics
                    let rowid = key - BELIEF_ID_OFFSET;
                    let result = conn.query_row(
                        "SELECT id, statement, entrenchment, file_path,
                                cited_by_beliefs, cited_by_sessions, applied_in,
                                evidence_count, evidence_verified, defeated_attacks
                         FROM beliefs
                         WHERE rowid = ?",
                        [rowid],
                        |row| {
                            let id: String = row.get(0)?;
                            let statement: String = row.get(1)?;
                            let entrenchment: String = row.get(2)?;
                            let file_path: String = row.get(3)?;
                            let cited_by_beliefs: i32 = row.get(4)?;
                            let cited_by_sessions: i32 = row.get(5)?;
                            let applied_in: i32 = row.get(6)?;
                            let evidence_count: i32 = row.get(7)?;
                            let evidence_verified: i32 = row.get(8)?;
                            let defeated_attacks: i32 = row.get(9)?;

                            // Build description with computed use/truth metrics
                            let use_total = cited_by_beliefs + cited_by_sessions;
                            let mut parts = Vec::new();
                            parts.push(format!("use: {}+{}", cited_by_beliefs, cited_by_sessions));
                            parts.push(format!("truth: {}/{}", evidence_count, evidence_verified));
                            if defeated_attacks > 0 {
                                parts.push(format!("{} defeated", defeated_attacks));
                            }
                            if applied_in > 0 {
                                parts.push(format!("{} applied", applied_in));
                            }
                            let metrics_str = parts.join(" | ");

                            // Flag weak beliefs
                            let health = if evidence_count == 0 && use_total <= 1 {
                                " WEAK"
                            } else if evidence_verified == 0 && evidence_count > 0 {
                                " UNVERIFIED"
                            } else {
                                ""
                            };

                            let content = format!(
                                "{} [{}{}] ({}, {})",
                                statement, metrics_str, health, entrenchment, file_path
                            );

                            Ok(ScryResult {
                                id: key,
                                event_type: "belief.surface".to_string(),
                                source_id: id,
                                timestamp: String::new(),
                                content,
                                score,
                            })
                        },
                    );

                    if let Ok(r) = result {
                        enriched.push(r);
                    }
                } else if key >= COMMIT_ID_OFFSET {
                    // Commit - look up in commits table
                    let rowid = key - COMMIT_ID_OFFSET;
                    let result = conn.query_row(
                        "SELECT sha, message, author_name, timestamp
                         FROM commits
                         WHERE rowid = ?",
                        [rowid],
                        |row| {
                            let sha: String = row.get(0)?;
                            let message: String = row.get(1)?;
                            let author: String =
                                row.get::<_, Option<String>>(2)?.unwrap_or_default();
                            let timestamp: String =
                                row.get::<_, Option<String>>(3)?.unwrap_or_default();

                            let content = if author.is_empty() {
                                format!("{}: {}", &sha[..7.min(sha.len())], message)
                            } else {
                                format!("{}: {} ({})", &sha[..7.min(sha.len())], message, author)
                            };

                            Ok(ScryResult {
                                id: key,
                                event_type: "git.commit".to_string(),
                                source_id: sha,
                                timestamp,
                                content,
                                score,
                            })
                        },
                    );

                    if let Ok(r) = result {
                        enriched.push(r);
                    }
                } else if key >= PATTERN_ID_OFFSET {
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

/// Find beliefs semantically related to code results (E4.6a step 4)
///
/// Computes direct cosine similarity between each code result's vector and
/// all belief vectors. Standard kNN doesn't work here because beliefs are
/// sparse in the full index — code has too many closer code/commit neighbors.
pub fn find_belief_impact(results: &[ScryResult]) -> Result<HashMap<i64, Vec<(String, f32)>>> {
    const BELIEF_ID_OFFSET: i64 = 4_000_000_000;
    const MIN_IMPACT_SCORE: f32 = 0.85;

    // Collect code result keys
    let code_keys: Vec<i64> = results
        .iter()
        .filter(|r| r.event_type.starts_with("code."))
        .map(|r| r.id)
        .collect();

    if code_keys.is_empty() {
        return Ok(HashMap::new());
    }

    // Load semantic index
    let model = super::search::get_embedding_model();
    let index_path = format!(
        ".patina/local/data/embeddings/{}/projections/semantic.usearch",
        model
    );

    if !Path::new(&index_path).exists() {
        return Ok(HashMap::new());
    }

    let index_options = IndexOptions {
        dimensions: 256,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        ..Default::default()
    };

    let index = Index::new(&index_options)?;
    index.load(&index_path)?;

    let db_path = ".patina/local/data/patina.db";
    let conn = Connection::open(db_path)?;

    // Pre-load all belief vectors (47 beliefs → 47 vector lookups, fast)
    let belief_vectors: Vec<(String, Vec<f32>)> = {
        let mut stmt = conn.prepare("SELECT rowid, id FROM beliefs")?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        let mut vecs = Vec::new();
        for (rowid, belief_id) in rows {
            let belief_key = (BELIEF_ID_OFFSET + rowid) as u64;
            let mut vector = vec![0.0_f32; 256];
            if index.get(belief_key, &mut vector).is_ok() {
                let mag: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
                if mag > 0.001 {
                    vecs.push((belief_id, vector));
                }
            }
        }
        vecs
    };

    if belief_vectors.is_empty() {
        return Ok(HashMap::new());
    }

    let mut impact_map: HashMap<i64, Vec<(String, f32)>> = HashMap::new();

    for key in &code_keys {
        let mut code_vector = vec![0.0_f32; 256];
        if index.get(*key as u64, &mut code_vector).is_err() {
            continue;
        }

        let code_mag: f32 = code_vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if code_mag < 0.001 {
            continue;
        }

        // Compute cosine similarity with each belief
        let mut beliefs: Vec<(String, f32)> = belief_vectors
            .iter()
            .filter_map(|(belief_id, belief_vec)| {
                let dot: f32 = code_vector
                    .iter()
                    .zip(belief_vec.iter())
                    .map(|(a, b)| a * b)
                    .sum();
                let b_mag: f32 = belief_vec.iter().map(|v| v * v).sum::<f32>().sqrt();
                let similarity = dot / (code_mag * b_mag);

                if similarity >= MIN_IMPACT_SCORE {
                    Some((belief_id.clone(), similarity))
                } else {
                    None
                }
            })
            .collect();

        // Sort by similarity descending
        beliefs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        beliefs.truncate(3); // Top 3 beliefs per code result

        if !beliefs.is_empty() {
            impact_map.insert(*key, beliefs);
        }
    }

    Ok(impact_map)
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
