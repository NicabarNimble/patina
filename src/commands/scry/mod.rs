//! Scry command - Query knowledge using vector search
//!
//! Unified query interface for searching project knowledge.
//! Phase 2.5b: MVP implementation for validating retrieval quality.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

use patina::embeddings::create_embedder;

/// Result from a scry query
#[derive(Debug)]
pub struct ScryResult {
    pub id: i64,
    pub content: String,
    pub score: f32,
    pub event_type: String,
    pub source_id: String,
    pub timestamp: String,
}

/// Options for scry query
#[derive(Debug, Clone)]
pub struct ScryOptions {
    pub limit: usize,
    pub min_score: f32,
    pub dimension: Option<String>,
}

impl Default for ScryOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            min_score: 0.0,
            dimension: None, // Use semantic by default
        }
    }
}

/// Execute scry command
pub fn execute(query: &str, options: ScryOptions) -> Result<()> {
    println!("🔮 Scry - Searching knowledge base\n");
    println!("Query: \"{}\"\n", query);

    let results = scry(query, &options)?;

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    println!("Found {} results:\n", results.len());
    println!("{}", "─".repeat(60));

    for (i, result) in results.iter().enumerate() {
        println!(
            "\n[{}] Score: {:.3} | {} | {}",
            i + 1,
            result.score,
            result.event_type,
            result.source_id
        );
        println!("    {}", truncate_content(&result.content, 200));
    }

    println!("\n{}", "─".repeat(60));

    Ok(())
}

/// Main scry function - search and return results
pub fn scry(query: &str, options: &ScryOptions) -> Result<Vec<ScryResult>> {
    let db_path = ".patina/data/patina.db";
    let embeddings_dir = ".patina/data/embeddings/e5-base-v2/projections";

    // Determine which dimension to search
    let dimension = options.dimension.as_deref().unwrap_or("semantic");
    let index_path = format!("{}/{}.usearch", embeddings_dir, dimension);

    if !Path::new(&index_path).exists() {
        anyhow::bail!(
            "Index not found: {}. Run 'patina oxidize' first.",
            index_path
        );
    }

    // Create embedder and embed query
    println!("Embedding query...");
    let mut embedder = create_embedder()?;
    let query_embedding = embedder.embed_query(query)?;

    // Load projection and project query embedding
    let projection_path = format!("{}/{}.safetensors", embeddings_dir, dimension);
    let projected = if Path::new(&projection_path).exists() {
        use crate::commands::oxidize::trainer::Projection;
        let projection = Projection::load_safetensors(Path::new(&projection_path))?;
        projection.forward(&query_embedding)
    } else {
        query_embedding
    };

    // Search index
    println!("Searching {} index...", dimension);

    // Create index with matching options (256-dim projection output, cosine)
    let index_options = IndexOptions {
        dimensions: 256,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        ..Default::default()
    };

    let index = Index::new(&index_options)
        .with_context(|| "Failed to create index")?;

    index
        .load(&index_path)
        .with_context(|| format!("Failed to load index: {}", index_path))?;

    let matches = index
        .search(&projected, options.limit)
        .with_context(|| "Vector search failed")?;

    // Convert to our SearchResults struct
    let results = SearchResults {
        keys: matches.keys,
        distances: matches.distances,
    };

    // Enrich with metadata from SQLite
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    let enriched = enrich_results(&conn, &results, dimension, options.min_score)?;

    Ok(enriched)
}

/// Search results from USearch
struct SearchResults {
    keys: Vec<u64>,
    distances: Vec<f32>,
}

/// Enrich vector search results with SQLite metadata
fn enrich_results(
    conn: &Connection,
    results: &SearchResults,
    dimension: &str,
    min_score: f32,
) -> Result<Vec<ScryResult>> {
    let mut enriched = Vec::new();

    match dimension {
        "semantic" => {
            // Semantic index uses seq as key
            for i in 0..results.keys.len() {
                let key = results.keys[i];
                let distance = results.distances[i];
                // Convert distance to similarity score (cosine: 1 - distance)
                let score = 1.0 - distance;

                if score < min_score {
                    continue;
                }

                let result = conn.query_row(
                    "SELECT seq, event_type, source_id, timestamp,
                            json_extract(data, '$.content') as content
                     FROM eventlog
                     WHERE seq = ?",
                    [key as i64],
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
        _ => {
            anyhow::bail!("Unknown dimension: {}", dimension);
        }
    }

    // Sort by score descending
    enriched.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    Ok(enriched)
}

/// Truncate content for display
fn truncate_content(content: &str, max_len: usize) -> String {
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

    #[test]
    fn test_default_options() {
        let opts = ScryOptions::default();
        assert_eq!(opts.limit, 10);
        assert_eq!(opts.min_score, 0.0);
        assert!(opts.dimension.is_none());
    }
}
