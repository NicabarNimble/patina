//! Fact routing — validation, content-hash dedup, and eventlog write.
//!
//! Implements DESIGN.md §6 decision table for schema validation and
//! §11 content-hash dedup via INSERT OR IGNORE with partial unique index.

use anyhow::{bail, Context, Result};
use patina_pipe_types::manifest::ChildManifest;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::Path;

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

/// Validate a fact against the child manifest and installed schema.
///
/// Returns a ValidatedFact ready for database insertion, or an error
/// describing why the fact was rejected.
///
/// **Fail closed:** If a schema is declared in the manifest but not installed,
/// all facts for that schema are rejected. The `schema_cache` maps schema name
/// to the set of valid event_types, populated on first fact per schema.
pub fn validate_fact(
    fact: &BrokerFact,
    manifest: &ChildManifest,
    child_name: &str,
    project_root: &Path,
    schema_cache: &mut HashMap<String, HashSet<String>>,
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

    // Step 3: validate fact_type against installed schema (fail closed)
    let event_type = format!("{}.{}", fact.schema, fact.fact_type);

    let valid_types = match schema_cache.entry(fact.schema.clone()) {
        std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
        std::collections::hash_map::Entry::Vacant(e) => {
            // Load installed schema — fail closed if missing
            let schema_dir = crate::paths::project::schemas_dir(project_root).join(&fact.schema);
            let types = load_fact_types(&schema_dir).with_context(|| {
                format!(
                    "schema '{}' declared in manifest but not installed — \
                     run: patina schema install wit/schema/{}",
                    fact.schema, fact.schema
                )
            })?;
            e.insert(types)
        }
    };

    if !valid_types.contains(&event_type) {
        bail!(
            "child '{}': fact_type '{}' not declared in installed schema '{}' — fact dropped",
            child_name,
            event_type,
            fact.schema
        );
    }

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

/// Load the set of valid event_types from an installed schema's schema.toml.
///
/// Minimal TOML parse — only extracts `[[facts]].event_type` values.
/// Keeps broker self-contained without depending on the commands crate.
fn load_fact_types(schema_dir: &Path) -> Result<HashSet<String>> {
    let toml_path = schema_dir.join("schema.toml");
    let content = std::fs::read_to_string(&toml_path)
        .with_context(|| format!("reading {}", toml_path.display()))?;
    let table: toml::Table =
        toml::from_str(&content).with_context(|| format!("parsing {}", toml_path.display()))?;
    let facts = table
        .get("facts")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("no [[facts]] in {}", toml_path.display()))?;
    let types: HashSet<String> = facts
        .iter()
        .filter_map(|f| {
            f.get("event_type")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .collect();
    if types.is_empty() {
        bail!("no facts with event_type in {}", toml_path.display());
    }
    Ok(types)
}

/// Write validated facts to events.db with content-hash dedup.
///
/// Uses INSERT OR IGNORE — SQLite's partial unique index on content_hash
/// handles dedup at B-tree insert time (no separate SELECT).
pub fn write_facts(conn: &Connection, facts: &[ValidatedFact]) -> Result<WriteResult> {
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

    /// Create a temp project with an installed github schema containing
    /// fact event_types: github.issue, github.pull_request
    fn setup_project_with_schema() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let schema_dir = dir.path().join(".patina/schemas/github");
        std::fs::create_dir_all(&schema_dir).unwrap();

        std::fs::write(
            schema_dir.join("schema.toml"),
            r#"
[schema]
name = "github"
version = "1.0.0"
package = "patina:schema/github@1.0.0"
description = "GitHub data"

[[facts]]
name = "issue"
event_type = "github.issue"
record = "issue"
identity_fields = ["number"]

[[facts]]
name = "pull_request"
event_type = "github.pull_request"
record = "pull-request"
identity_fields = ["number"]
"#,
        )
        .unwrap();

        // Minimal WIT file (not parsed by validation, but present for completeness)
        std::fs::write(
            schema_dir.join("github.wit"),
            "package patina:schema/github@1.0.0;\n",
        )
        .unwrap();

        dir
    }

    #[test]
    fn validate_valid_fact() {
        let manifest = test_manifest();
        let project = setup_project_with_schema();
        let fact = BrokerFact {
            schema: "github".to_string(),
            fact_type: "issue".to_string(),
            data: serde_json::json!({"title": "test"}),
            content_hash: Some("blake3:abc123".to_string()),
        };

        let mut cache = HashMap::new();
        let validated =
            validate_fact(&fact, &manifest, "test-child", project.path(), &mut cache).unwrap();
        assert_eq!(validated.event_type, "github.issue");
        assert_eq!(validated.source_id, "child:test-child");
        assert_eq!(validated.content_hash, "blake3:abc123");
    }

    #[test]
    fn validate_undeclared_schema_drops() {
        let manifest = test_manifest();
        let project = setup_project_with_schema();
        let fact = BrokerFact {
            schema: "bogus".to_string(),
            fact_type: "item".to_string(),
            data: serde_json::json!({}),
            content_hash: Some("blake3:xyz".to_string()),
        };

        let mut cache = HashMap::new();
        let result = validate_fact(&fact, &manifest, "test-child", project.path(), &mut cache);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not declared"));
    }

    #[test]
    fn validate_missing_content_hash_rejects() {
        let manifest = test_manifest();
        let project = setup_project_with_schema();
        let fact = BrokerFact {
            schema: "github".to_string(),
            fact_type: "issue".to_string(),
            data: serde_json::json!({}),
            content_hash: None,
        };

        let mut cache = HashMap::new();
        let result = validate_fact(&fact, &manifest, "test-child", project.path(), &mut cache);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing content_hash"));
    }

    #[test]
    fn validate_unknown_fact_type_rejects() {
        let manifest = test_manifest();
        let project = setup_project_with_schema();
        let fact = BrokerFact {
            schema: "github".to_string(),
            fact_type: "typo".to_string(),
            data: serde_json::json!({}),
            content_hash: Some("blake3:abc".to_string()),
        };

        let mut cache = HashMap::new();
        let result = validate_fact(&fact, &manifest, "test-child", project.path(), &mut cache);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not declared in installed schema"),
            "got: {}",
            err
        );
    }

    #[test]
    fn validate_known_fact_type_passes() {
        let manifest = test_manifest();
        let project = setup_project_with_schema();

        let mut cache = HashMap::new();

        // Both issue and pull_request should pass
        for fact_type in &["issue", "pull_request"] {
            let fact = BrokerFact {
                schema: "github".to_string(),
                fact_type: fact_type.to_string(),
                data: serde_json::json!({}),
                content_hash: Some(format!("blake3:{}", fact_type)),
            };
            let result = validate_fact(&fact, &manifest, "test-child", project.path(), &mut cache);
            assert!(
                result.is_ok(),
                "expected {} to pass, got: {:?}",
                fact_type,
                result
            );
        }

        // Cache should be populated
        assert!(cache.contains_key("github"));
        assert_eq!(cache["github"].len(), 2);
    }

    #[test]
    fn validate_missing_installed_schema_rejects() {
        let manifest = test_manifest();
        // Empty project — no .patina/schemas/
        let project = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(project.path().join(".patina")).unwrap();

        let fact = BrokerFact {
            schema: "github".to_string(),
            fact_type: "issue".to_string(),
            data: serde_json::json!({}),
            content_hash: Some("blake3:abc".to_string()),
        };

        let mut cache = HashMap::new();
        let result = validate_fact(&fact, &manifest, "test-child", project.path(), &mut cache);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not installed"), "got: {}", err);
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
