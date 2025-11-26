//! Training pair generators for projections
//!
//! Generate (anchor, positive, negative) triplets from eventlog for contrastive learning

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::HashMap;

/// A training triplet: anchor, positive (similar), negative (dissimilar)
#[derive(Debug, Clone)]
pub struct TrainingPair {
    pub anchor: String,
    pub positive: String,
    pub negative: String,
}

/// Generate training pairs where observations from same session are similar
///
/// Strategy:
/// - Anchor: random observation from a session
/// - Positive: different observation from same session
/// - Negative: random observation from different session
pub fn generate_same_session_pairs(db_path: &str, num_pairs: usize) -> Result<Vec<TrainingPair>> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open database: {}", db_path))?;

    // Load all observations grouped by session
    let mut sessions: HashMap<String, Vec<String>> = HashMap::new();

    let mut stmt = conn.prepare(
        "SELECT source_id, json_extract(data, '$.content') as content
         FROM eventlog
         WHERE event_type IN ('session.decision', 'session.observation', 'session.pattern')
           AND content IS NOT NULL
           AND length(content) > 20
         ORDER BY timestamp",
    )?;

    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let session_id: String = row.get(0)?;
        let content: String = row.get(1)?;
        sessions.entry(session_id).or_default().push(content);
    }

    // Filter to sessions with at least 2 observations
    let valid_sessions: Vec<_> = sessions.iter().filter(|(_, obs)| obs.len() >= 2).collect();

    if valid_sessions.is_empty() {
        anyhow::bail!("No sessions with multiple observations found");
    }

    println!(
        "  Found {} sessions with {} total observations",
        valid_sessions.len(),
        sessions.values().map(|v| v.len()).sum::<usize>()
    );

    // Generate pairs
    let mut pairs = Vec::new();
    let mut rng = fastrand::Rng::new();

    for _ in 0..num_pairs {
        // Pick random session for anchor
        let anchor_idx = rng.usize(..valid_sessions.len());
        let (anchor_session_id, anchor_observations) = valid_sessions[anchor_idx];

        // Pick two different observations from same session
        let anchor_obs_idx = rng.usize(..anchor_observations.len());
        let mut positive_obs_idx = rng.usize(..anchor_observations.len());
        while positive_obs_idx == anchor_obs_idx && anchor_observations.len() > 1 {
            positive_obs_idx = rng.usize(..anchor_observations.len());
        }

        let anchor = anchor_observations[anchor_obs_idx].clone();
        let positive = anchor_observations[positive_obs_idx].clone();

        // Pick observation from different session
        let mut negative_session_idx = rng.usize(..valid_sessions.len());
        while valid_sessions[negative_session_idx].0 == anchor_session_id
            && valid_sessions.len() > 1
        {
            negative_session_idx = rng.usize(..valid_sessions.len());
        }

        let negative_observations = valid_sessions[negative_session_idx].1;
        let negative_obs_idx = rng.usize(..negative_observations.len());
        let negative = negative_observations[negative_obs_idx].clone();

        pairs.push(TrainingPair {
            anchor,
            positive,
            negative,
        });
    }

    Ok(pairs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

    fn create_test_db() -> NamedTempFile {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        // Create eventlog table
        conn.execute(
            "CREATE TABLE eventlog (
                seq INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                source_id TEXT NOT NULL,
                data JSON NOT NULL
            )",
            [],
        )
        .unwrap();

        // Insert test observations from multiple sessions
        conn.execute(
            "INSERT INTO eventlog (event_type, timestamp, source_id, data) VALUES
             ('session.observation', '2025-01-01T10:00:00Z', 'session-1',
              json('{\"content\": \"Rust error handling uses Result types for recoverable errors\"}')),
             ('session.observation', '2025-01-01T10:05:00Z', 'session-1',
              json('{\"content\": \"Pattern matching on Result makes error handling explicit and safe\"}')),
             ('session.decision', '2025-01-01T10:10:00Z', 'session-1',
              json('{\"content\": \"Use Result<T,E> instead of exceptions for all fallible operations\"}')),
             ('session.observation', '2025-01-02T14:00:00Z', 'session-2',
              json('{\"content\": \"Vector embeddings require L2 normalization for cosine similarity\"}')),
             ('session.pattern', '2025-01-02T14:05:00Z', 'session-2',
              json('{\"content\": \"Mean pooling over attention mask gives better sentence embeddings than CLS token\"}'))",
            [],
        )
        .unwrap();

        temp_file
    }

    #[test]
    fn test_generate_same_session_pairs() {
        let temp_db = create_test_db();
        let pairs = generate_same_session_pairs(temp_db.path().to_str().unwrap(), 5).unwrap();

        assert_eq!(pairs.len(), 5);

        // Verify structure
        for pair in &pairs {
            assert!(!pair.anchor.is_empty());
            assert!(!pair.positive.is_empty());
            assert!(!pair.negative.is_empty());
            assert_ne!(pair.anchor, pair.negative);
        }
    }

    #[test]
    fn test_empty_database() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        conn.execute(
            "CREATE TABLE eventlog (
                seq INTEGER PRIMARY KEY,
                event_type TEXT,
                timestamp TEXT,
                source_id TEXT,
                data JSON
            )",
            [],
        )
        .unwrap();

        let result = generate_same_session_pairs(temp_file.path().to_str().unwrap(), 5);
        assert!(result.is_err());
    }
}
