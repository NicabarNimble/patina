//! Fact routing — validation, content-hash dedup, and eventlog write.
//!
//! Implements DESIGN.md §6 decision table for schema validation and
//! §11 content-hash dedup via INSERT OR IGNORE with partial unique index.

use anyhow::{bail, Result};
use patina_pipe_types::manifest::ChildManifest;
use rusqlite::Connection;
use std::collections::HashSet;

use super::lifecycle::BrokerFact;

/// Result of writing a batch of facts.
#[derive(Debug, Clone)]
pub struct WriteResult {
    pub inserted: u64,
    pub dedup_skipped: u64,
    pub cursor: Option<String>,
}

/// A fact that passed validation and is ready to write.
#[derive(Debug)]
pub struct ValidatedFact {
    pub event_type: String,
    pub data: String,
    pub content_hash: String,
    pub source_id: String,
}

/// Validate a fact against the child manifest (§6 decision table).
///
/// Returns a ValidatedFact ready for database insertion, or an error
/// describing why the fact was rejected.
pub fn validate_fact(
    fact: &BrokerFact,
    manifest: &ChildManifest,
    child_name: &str,
    _warned_schemas: &mut HashSet<String>,
) -> Result<ValidatedFact> {
    // Step 1: content_hash must be present (§11 validation gate)
    let content_hash = fact.content_hash.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "child '{}': fact missing content_hash (schema={}, fact_type={})",
            child_name,
            fact.schema,
            fact.fact_type
        )
    })?;

    // Step 2: schema must be declared in manifest
    if !manifest.schemas.contains_key(&fact.schema) {
        bail!(
            "child '{}': schema '{}' not declared in manifest — fact dropped",
            child_name,
            fact.schema
        );
    }

    // Step 3: check if schema is installed on disk
    // For now, we don't validate fact_type against installed schemas —
    // that requires loading schema files from the destination project.
    // Schema installation check is deferred to when we have project context.
    // If schema is declared but not installed: warn + pass-through (§14).
    // The warning is per-schema-per-run, not per-fact.
    let event_type = format!("{}.{}", fact.schema, fact.fact_type);

    // Build source_id
    let source_id = format!("child:{}", child_name);

    // Serialize data
    let data = serde_json::to_string(&fact.data)
        .map_err(|e| anyhow::anyhow!("failed to serialize fact data: {}", e))?;

    Ok(ValidatedFact {
        event_type,
        data,
        content_hash: content_hash.clone(),
        source_id,
    })
}

/// Write validated facts to events.db with content-hash dedup.
///
/// Uses INSERT OR IGNORE — SQLite's partial unique index on content_hash
/// handles dedup at B-tree insert time (no separate SELECT).
pub fn write_facts(
    conn: &Connection,
    facts: &[ValidatedFact],
) -> Result<WriteResult> {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let mut inserted: u64 = 0;
    let mut dedup_skipped: u64 = 0;

    for fact in facts {
        let rows = conn.execute(
            "INSERT OR IGNORE INTO eventlog (event_type, timestamp, source_id, data, provenance, content_hash)
             VALUES (?1, ?2, ?3, ?4, 'external', ?5)",
            rusqlite::params![
                &fact.event_type,
                &timestamp,
                &fact.source_id,
                &fact.data,
                &fact.content_hash,
            ],
        )?;

        if rows == 0 {
            dedup_skipped += 1;
            eprintln!(
                "[broker] dedup: skipped fact content_hash={} (already in eventlog)",
                &fact.content_hash
            );
        } else {
            inserted += 1;
        }
    }

    Ok(WriteResult {
        inserted,
        dedup_skipped,
        cursor: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manifest() -> ChildManifest {
        ChildManifest::from_toml(
            r#"
[child]
name = "test-child"
version = "0.1.0"
type = "connector"
runtime = "native"
lifecycle = "poll"

[schemas.github]
package = "patina:schema/github@1.0.0"
"#,
        )
        .unwrap()
    }

    #[test]
    fn validate_valid_fact() {
        let manifest = test_manifest();
        let fact = BrokerFact {
            schema: "github".to_string(),
            fact_type: "issue".to_string(),
            data: serde_json::json!({"title": "test"}),
            content_hash: Some("blake3:abc123".to_string()),
        };

        let mut warned = HashSet::new();
        let validated = validate_fact(&fact, &manifest, "test-child", &mut warned).unwrap();
        assert_eq!(validated.event_type, "github.issue");
        assert_eq!(validated.source_id, "child:test-child");
        assert_eq!(validated.content_hash, "blake3:abc123");
    }

    #[test]
    fn validate_undeclared_schema_drops() {
        let manifest = test_manifest();
        let fact = BrokerFact {
            schema: "bogus".to_string(),
            fact_type: "item".to_string(),
            data: serde_json::json!({}),
            content_hash: Some("blake3:xyz".to_string()),
        };

        let mut warned = HashSet::new();
        let result = validate_fact(&fact, &manifest, "test-child", &mut warned);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not declared"));
    }

    #[test]
    fn validate_missing_content_hash_rejects() {
        let manifest = test_manifest();
        let fact = BrokerFact {
            schema: "github".to_string(),
            fact_type: "issue".to_string(),
            data: serde_json::json!({}),
            content_hash: None,
        };

        let mut warned = HashSet::new();
        let result = validate_fact(&fact, &manifest, "test-child", &mut warned);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing content_hash"));
    }

    #[test]
    fn write_facts_dedup() {
        let dir = tempfile::tempdir().unwrap();
        let conn = crate::eventlog::open_events_db_at(dir.path()).unwrap();

        let facts = vec![
            ValidatedFact {
                event_type: "github.issue".to_string(),
                data: r#"{"title":"first"}"#.to_string(),
                content_hash: "blake3:aaa".to_string(),
                source_id: "child:test".to_string(),
            },
            ValidatedFact {
                event_type: "github.issue".to_string(),
                data: r#"{"title":"second"}"#.to_string(),
                content_hash: "blake3:bbb".to_string(),
                source_id: "child:test".to_string(),
            },
        ];

        let result = write_facts(&conn, &facts).unwrap();
        assert_eq!(result.inserted, 2);
        assert_eq!(result.dedup_skipped, 0);

        // Write again — should be all dedup
        let result2 = write_facts(&conn, &facts).unwrap();
        assert_eq!(result2.inserted, 0);
        assert_eq!(result2.dedup_skipped, 2);
    }
}
