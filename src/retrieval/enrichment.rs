//! Result enrichment with SQLite metadata.

use anyhow::Result;
use rusqlite::Connection;

/// Search results from USearch.
pub struct SearchResults {
    pub keys: Vec<u64>,
    pub distances: Vec<f32>,
}

/// Enriched retrieval result.
pub struct EnrichedResult {
    pub id: i64,
    pub content: String,
    pub score: f32,
    pub event_type: String,
    pub source_id: String,
    pub timestamp: String,
}

/// Enrich vector search results with SQLite metadata.
pub fn enrich_results(
    conn: &Connection,
    results: &SearchResults,
    dimension: &str,
    min_score: f32,
) -> Result<Vec<EnrichedResult>> {
    let mut enriched = Vec::new();

    use patina::embeddings::offsets::*;

    match dimension {
        "knowledge" | "semantic" | "sessions" => {
            for i in 0..results.keys.len() {
                let key = results.keys[i] as i64;
                let distance = results.distances[i];
                let score = 1.0 - distance;

                if score < min_score {
                    continue;
                }

                if key >= FORGE_ID_OFFSET {
                    let event_seq = key - FORGE_ID_OFFSET;
                    let result = conn.query_row(
                        "SELECT event_type, source_id, timestamp,
                                json_extract(data, '$.title') as title,
                                json_extract(data, '$.body') as body
                         FROM eventlog
                         WHERE seq = ?",
                        [event_seq],
                        |row| {
                            let event_type: String = row.get(0)?;
                            let source_id: String = row.get(1)?;
                            let timestamp: String = row.get(2)?;
                            let title: String =
                                row.get::<_, Option<String>>(3)?.unwrap_or_default();
                            let body: String = row.get::<_, Option<String>>(4)?.unwrap_or_default();

                            let kind = display_kind_for_event_type(&event_type);
                            let preview: String = body.chars().take(200).collect();
                            let content = if preview.is_empty() {
                                format!("{} #{}: {}", kind, source_id, title)
                            } else {
                                format!("{} #{}: {}\n{}", kind, source_id, title, preview)
                            };

                            Ok(EnrichedResult {
                                id: key,
                                event_type,
                                source_id: format!("{}#{}", kind.to_lowercase(), source_id),
                                timestamp,
                                content,
                                score,
                            })
                        },
                    );

                    if let Ok(r) = result {
                        enriched.push(r);
                    }
                } else if key >= BELIEF_ID_OFFSET {
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

                            Ok(EnrichedResult {
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

                            Ok(EnrichedResult {
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
                    let rowid = key - PATTERN_ID_OFFSET;
                    let result = conn.query_row(
                        "SELECT rowid, id, title, purpose, layer, file_path
                         FROM patterns
                         WHERE rowid = ?",
                        [rowid],
                        |row| {
                            let _id: String = row.get(1)?;
                            let title: String = row.get(2)?;
                            let purpose: Option<String> = row.get(3)?;
                            let layer: String = row.get(4)?;
                            let file_path: String = row.get(5)?;

                            let desc = if let Some(p) = purpose {
                                format!("{}: {}", title, p)
                            } else {
                                title.clone()
                            };

                            Ok(EnrichedResult {
                                id: key,
                                event_type: format!("pattern.{}", layer),
                                source_id: file_path.clone(),
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

                            Ok(EnrichedResult {
                                id: key,
                                event_type: "code.function".to_string(),
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
                    let result = conn.query_row(
                        "SELECT seq, event_type, source_id, timestamp,
                                json_extract(data, '$.content') as content
                         FROM eventlog
                         WHERE seq = ?",
                        [key],
                        |row| {
                            Ok(EnrichedResult {
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
                    enriched.push(EnrichedResult {
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
                    enriched.push(EnrichedResult {
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

    enriched.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(enriched)
}

fn display_kind_for_event_type(event_type: &str) -> String {
    if let Ok(schemas) = crate::commands::schema::load_all_installed() {
        for schema in &schemas {
            for contract in &schema.contracts {
                if contract.event_type == event_type {
                    return contract.display_kind.clone();
                }
            }
        }
    }
    event_type
        .rsplit('.')
        .next()
        .unwrap_or(event_type)
        .to_string()
}

pub fn truncate_content(content: &str, max_len: usize) -> String {
    let content = content.replace('\n', " ").trim().to_string();
    if content.len() <= max_len {
        content
    } else {
        let boundary = content
            .char_indices()
            .map(|(idx, _)| idx)
            .take_while(|idx| *idx <= max_len)
            .last()
            .unwrap_or(0);
        format!("{}...", &content[..boundary])
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

    #[test]
    fn test_truncate_content_utf8_char_boundary_safe() {
        let content = "Hello 世界 🌍";
        assert_eq!(truncate_content(content, 7), "Hello ...");
        assert_eq!(truncate_content(content, 10), "Hello 世...");
    }
}
