//! Belief validation command - Validate belief candidates using neuro-symbolic reasoning

use anyhow::{Context, Result};
use patina::embeddings::create_embedder;
use patina::query::SemanticSearch;
use patina::reasoning::{ReasoningEngine, ScoredObservation};
use serde::Serialize;
use std::path::Path;

/// Result from belief validation
#[derive(Debug, Serialize)]
pub struct BeliefValidationOutput {
    pub query: String,
    pub valid: bool,
    pub reason: String,
    pub metrics: ValidationMetrics,
    pub observations: Vec<ObservationSummary>,
}

#[derive(Debug, Serialize)]
pub struct ValidationMetrics {
    pub weighted_score: f32,
    pub strong_evidence_count: usize,
    pub has_diverse_sources: bool,
    pub avg_reliability: f32,
    pub avg_similarity: f32,
}

#[derive(Debug, Serialize)]
pub struct ObservationSummary {
    pub id: String,
    pub content: String,
    pub similarity: f32,
    pub reliability: f32,
    pub source_type: String,
}

/// Execute belief validation command
///
/// # Arguments
/// * `query` - Belief statement to validate
/// * `min_score` - Minimum similarity score for observations (0.0-1.0)
/// * `limit` - Maximum number of observations to consider
pub fn execute(query: &str, min_score: f32, limit: usize) -> Result<()> {
    // Check storage exists
    let storage_path = ".patina/storage/observations";
    if !Path::new(storage_path).exists() {
        anyhow::bail!(
            "Observation storage not found at {}\n\nRun `patina embeddings generate` first.",
            storage_path
        );
    }

    // Create embedder
    let embedder = create_embedder().context("Failed to create embedder")?;

    // Open semantic search engine
    let search = SemanticSearch::new(".patina/storage", embedder)
        .context("Failed to open semantic search engine")?;

    // Verify storage has observations
    let count = search
        .observation_storage()
        .count()
        .context("Failed to verify observation storage")?;

    if count == 0 {
        anyhow::bail!(
            "No observations found in storage.\n\nRun `patina embeddings generate` first."
        );
    }

    // Generate query embedding and search
    let mut embedder = create_embedder().context("Failed to create embedder")?;
    let query_embedding = embedder
        .embed(query)
        .context("Failed to generate query embedding")?;

    let scored_results = search
        .observation_storage()
        .search_with_scores(&query_embedding, limit * 2)
        .context("Failed to search with scores")?;

    // Filter by min_score and convert to ScoredObservation
    let mut scored_observations = Vec::new();
    let mut observation_summaries = Vec::new();

    for (observation, similarity) in scored_results {
        if similarity < min_score {
            continue;
        }

        let scored_obs = ScoredObservation {
            id: observation.id.to_string(),
            observation_type: observation.observation_type.clone(),
            content: observation.content.clone(),
            similarity,
            reliability: observation.metadata.reliability.unwrap_or(0.70),
            source_type: observation
                .metadata
                .source_type
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
        };

        observation_summaries.push(ObservationSummary {
            id: observation.id.to_string(),
            content: observation.content,
            similarity,
            reliability: scored_obs.reliability,
            source_type: scored_obs.source_type.clone(),
        });

        scored_observations.push(scored_obs);

        if scored_observations.len() >= limit {
            break;
        }
    }

    if scored_observations.is_empty() {
        anyhow::bail!(
            "No observations found matching query with similarity >= {}\n\nTry lowering --min-score",
            min_score
        );
    }

    // Initialize reasoning engine
    let mut engine = ReasoningEngine::new().context("Failed to create reasoning engine")?;

    // Load observations into Prolog
    engine
        .load_observations(&scored_observations)
        .context("Failed to load observations into reasoning engine")?;

    // Validate belief
    let validation = engine
        .validate_belief()
        .context("Failed to validate belief")?;

    // Prepare output
    let output = BeliefValidationOutput {
        query: query.to_string(),
        valid: validation.valid,
        reason: validation.reason,
        metrics: ValidationMetrics {
            weighted_score: validation.weighted_score,
            strong_evidence_count: validation.strong_evidence_count,
            has_diverse_sources: validation.has_diverse_sources,
            avg_reliability: validation.avg_reliability,
            avg_similarity: validation.avg_similarity,
        },
        observations: observation_summaries,
    };

    // Output JSON
    let json = serde_json::to_string_pretty(&output)
        .context("Failed to serialize validation result to JSON")?;
    println!("{}", json);

    Ok(())
}
