//! Semantic query command - Search observations using semantic similarity

use anyhow::{Context, Result};
use patina::embeddings::create_embedder;
use patina::query::SemanticSearch;
use serde::Serialize;
use std::path::Path;

/// Result from semantic search with evidence strength mapping
#[derive(Debug, Serialize)]
pub struct SemanticQueryResult {
    pub id: String,
    pub session_id: Option<String>,
    #[serde(rename = "type")]
    pub observation_type: String,
    pub text: String,
    pub similarity: f32,
    pub evidence_strength: String,
    pub source_type: Option<String>,
    pub reliability: Option<f32>,
}

/// Evidence strength based on similarity score
fn map_evidence_strength(similarity: f32) -> &'static str {
    if similarity >= 0.70 {
        "strong"
    } else if similarity >= 0.50 {
        "medium"
    } else {
        "weak"
    }
}

/// Execute semantic query command
///
/// # Arguments
/// * `query` - Query text to search for
/// * `observation_types` - Optional filter by types (pattern, technology, decision, challenge)
/// * `min_score` - Minimum similarity score (0.0-1.0)
/// * `limit` - Maximum number of results to return
pub fn execute(
    query: &str,
    observation_types: Option<Vec<String>>,
    min_score: f32,
    limit: usize,
) -> Result<()> {
    // Check storage exists
    let storage_path = ".patina/data/observations";
    if !Path::new(storage_path).exists() {
        anyhow::bail!(
            "Observation storage not found at {}\n\nRun `patina embeddings generate` first.",
            storage_path
        );
    }

    // Create embedder
    let embedder = create_embedder().context("Failed to create embedder")?;

    // Open semantic search engine
    let mut search = SemanticSearch::new(".patina/data", embedder)
        .context("Failed to open semantic search engine")?;

    // Generate query embedding
    let query_embedding = search
        .observation_storage_mut()
        .count()
        .context("Failed to verify observation storage")?;

    if query_embedding == 0 {
        anyhow::bail!(
            "No observations found in storage.\n\nRun `patina embeddings generate` first."
        );
    }

    // Use quality-filtered search with scores
    // Filters: source_type in (session, session_distillation, documentation),
    // reliability > 0.85, deduplicated by content
    let type_filter = observation_types.as_ref().and_then(|types| {
        if types.len() == 1 {
            Some(types[0].as_str())
        } else {
            None // Multi-type filter not supported in filtered search yet
        }
    });

    let scored_results = search
        .search_observations_filtered_with_scores(query, type_filter, limit * 2)
        .context("Failed to search with quality filtering")?;

    // Filter and format results
    let mut results = Vec::new();
    for (observation, similarity) in scored_results {
        // Apply min score filter
        if similarity < min_score {
            continue;
        }

        // Apply multi-type filter if needed (single-type already handled above)
        if let Some(ref types) = observation_types {
            if types.len() > 1 && !types.contains(&observation.observation_type) {
                continue;
            }
        }

        // Extract session_id from metadata source if available
        let session_id = observation.metadata.source.as_ref().and_then(|s| {
            // Extract session ID from path like "sessions/20251008-061520.md"
            s.split('/')
                .nth(1)
                .and_then(|name| name.strip_suffix(".md"))
                .map(|id| id.to_string())
        });

        results.push(SemanticQueryResult {
            id: observation.id.to_string(),
            session_id,
            observation_type: observation.observation_type,
            text: observation.content,
            similarity,
            evidence_strength: map_evidence_strength(similarity).to_string(),
            source_type: observation.metadata.source_type,
            reliability: observation.metadata.reliability,
        });

        if results.len() >= limit {
            break;
        }
    }

    // Output JSON
    let json =
        serde_json::to_string_pretty(&results).context("Failed to serialize results to JSON")?;
    println!("{}", json);

    Ok(())
}
