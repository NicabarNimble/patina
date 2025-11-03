//! Domain types for storage layer
//!
//! These types are storage-agnostic - they don't know about SQLite or USearch.
//! Storage wrappers handle serialization/deserialization.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A belief captured from user interactions or patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    pub id: Uuid,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: BeliefMetadata,
}

/// Metadata associated with a belief
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeliefMetadata {
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub source: Option<String>,
    pub confidence: Option<f32>,
}

/// An observation captured from development sessions
/// Includes patterns, technologies, decisions, and challenges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub id: Uuid,
    pub observation_type: String, // "pattern", "technology", "decision", "challenge"
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: ObservationMetadata,
}

/// Metadata associated with an observation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObservationMetadata {
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub source: Option<String>,
}

/// Result from vector search
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: Uuid,
    pub similarity: f32,
}
