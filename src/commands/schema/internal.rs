//! Schema command implementation.
//!
//! Handles parsing schema.toml, validating WIT presence, installing to
//! .patina/schemas/, and listing/showing installed schemas.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use patina::paths;

// =========================================================================
// Schema metadata types (parsed from schema.toml)
// =========================================================================

/// Top-level schema metadata from schema.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SchemaMetadata {
    pub schema: SchemaInfo,
    #[serde(default)]
    pub facts: Vec<FactDef>,
    pub embedding: Option<EmbeddingConfig>,
    #[serde(default)]
    pub indexes: Vec<IndexConfig>,
}

/// The [schema] section — identity and description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SchemaInfo {
    pub name: String,
    pub version: String,
    pub package: String,
    #[serde(default)]
    pub description: String,
}

/// A [[facts]] entry — maps a logical name to an event type and WIT record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FactDef {
    pub name: String,
    pub event_type: String,
    pub record: String,
}

/// The [embedding] section — offset slot and corpus query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EmbeddingConfig {
    pub offset_slot: i64,
    pub corpus_query: String,
}

/// An [[indexes]] entry — FTS5 and materialized view config per fact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IndexConfig {
    pub fact: String,
    pub fts_fields: Vec<String>,
    pub table: String,
}

// =========================================================================
// Schema package operations
// =========================================================================

/// Parse schema.toml from a directory.
fn parse_schema_toml(dir: &Path) -> Result<SchemaMetadata> {
    let toml_path = dir.join("schema.toml");
    if !toml_path.exists() {
        bail!("schema.toml not found in {}", dir.display());
    }
    let content = std::fs::read_to_string(&toml_path)
        .with_context(|| format!("reading {}", toml_path.display()))?;
    let metadata: SchemaMetadata =
        toml::from_str(&content).with_context(|| format!("parsing {}", toml_path.display()))?;
    Ok(metadata)
}

/// Validate that a schema package directory has required files.
fn validate_package(dir: &Path) -> Result<SchemaMetadata> {
    if !dir.is_dir() {
        bail!("{} is not a directory", dir.display());
    }

    // Must have schema.toml
    let metadata = parse_schema_toml(dir)?;

    // Must have at least one .wit file
    let has_wit = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .any(|e| e.path().extension().is_some_and(|ext| ext == "wit"));
    if !has_wit {
        bail!(
            "no .wit files found in {} — schema packages require at least one WIT file",
            dir.display()
        );
    }

    // Validate required fields
    if metadata.schema.name.is_empty() {
        bail!("schema.name is empty");
    }
    if metadata.schema.version.is_empty() {
        bail!("schema.version is empty");
    }
    if metadata.schema.package.is_empty() {
        bail!("schema.package is empty");
    }
    if metadata.facts.is_empty() {
        bail!("schema must define at least one [[facts]] entry");
    }

    Ok(metadata)
}

/// Find project root by walking up from cwd.
fn find_project_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let mut dir = cwd.as_path();
    loop {
        if dir.join(".patina").is_dir() {
            return Ok(dir.to_path_buf());
        }
        dir = dir
            .parent()
            .ok_or_else(|| anyhow::anyhow!("not in a patina project (no .patina/ found)"))?;
    }
}

// =========================================================================
// Commands
// =========================================================================

/// Install a schema package from a local path.
pub fn install_schema(source_path: &str) -> Result<()> {
    let source = PathBuf::from(source_path);
    let metadata = validate_package(&source)?;
    let name = &metadata.schema.name;

    let root = find_project_root()?;
    let schemas_dir = paths::project::schemas_dir(&root);
    let target_dir = schemas_dir.join(name);

    // Create target directory
    std::fs::create_dir_all(&target_dir)
        .with_context(|| format!("creating {}", target_dir.display()))?;

    // Copy all files from source to target
    let mut copied = 0;
    for entry in std::fs::read_dir(&source)? {
        let entry = entry?;
        let file_name = entry.file_name();
        if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), target_dir.join(&file_name))?;
            copied += 1;
        }
    }

    println!(
        "Installed schema '{}' v{} ({} files) → {}",
        name,
        metadata.schema.version,
        copied,
        target_dir.display()
    );
    println!("  Package: {}", metadata.schema.package);
    println!("  Facts: {}", metadata.facts.len());
    if let Some(ref emb) = metadata.embedding {
        println!("  Embedding: slot {}", emb.offset_slot);
    }

    Ok(())
}

/// List installed schemas.
pub fn list_schemas(json: bool) -> Result<()> {
    let root = find_project_root()?;
    let schemas_dir = paths::project::schemas_dir(&root);

    if !schemas_dir.exists() {
        if json {
            println!("[]");
        } else {
            println!("No schemas installed");
        }
        return Ok(());
    }

    let mut schemas = Vec::new();
    for entry in std::fs::read_dir(&schemas_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let schema_dir = entry.path();
        match parse_schema_toml(&schema_dir) {
            Ok(metadata) => schemas.push(metadata),
            Err(e) => {
                eprintln!("Warning: skipping {}: {}", schema_dir.display(), e);
            }
        }
    }

    if json {
        let json_schemas: Vec<serde_json::Value> = schemas
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.schema.name,
                    "version": s.schema.version,
                    "package": s.schema.package,
                    "description": s.schema.description,
                    "facts": s.facts.len(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_schemas)?);
    } else if schemas.is_empty() {
        println!("No schemas installed");
    } else {
        println!("Installed schemas:");
        for s in &schemas {
            println!(
                "  {} v{} — {} ({} facts)",
                s.schema.name,
                s.schema.version,
                s.schema.description,
                s.facts.len()
            );
        }
    }

    Ok(())
}

/// Show details of an installed schema.
pub fn show_schema(name: &str, json: bool) -> Result<()> {
    let root = find_project_root()?;
    let schema_dir = paths::project::schemas_dir(&root).join(name);

    if !schema_dir.exists() {
        bail!("schema '{}' is not installed", name);
    }

    let metadata = parse_schema_toml(&schema_dir)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&metadata)?);
        return Ok(());
    }

    println!(
        "Schema: {} v{}",
        metadata.schema.name, metadata.schema.version
    );
    println!("Package: {}", metadata.schema.package);
    if !metadata.schema.description.is_empty() {
        println!("Description: {}", metadata.schema.description);
    }

    println!("\nFacts:");
    for fact in &metadata.facts {
        println!(
            "  {} → event_type: {}, record: {}",
            fact.name, fact.event_type, fact.record
        );
    }

    if let Some(ref emb) = metadata.embedding {
        println!("\nEmbedding:");
        println!("  Offset slot: {}", emb.offset_slot);
        println!("  Corpus query: {}", emb.corpus_query.trim());
    }

    if !metadata.indexes.is_empty() {
        println!("\nIndexes:");
        for idx in &metadata.indexes {
            println!(
                "  {} → table: {}, FTS: [{}]",
                idx.fact,
                idx.table,
                idx.fts_fields.join(", ")
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_schema(dir: &Path) {
        fs::write(
            dir.join("schema.toml"),
            r#"
[schema]
name = "test"
version = "1.0.0"
package = "patina:schema/test@1.0.0"
description = "Test schema"

[[facts]]
name = "item"
event_type = "test.item"
record = "item"
"#,
        )
        .unwrap();
        fs::write(
            dir.join("test.wit"),
            "package patina:schema/test@1.0.0;\ninterface types { record item { id: s64 } }\n",
        )
        .unwrap();
    }

    #[test]
    fn parse_valid_schema_toml() {
        let dir = TempDir::new().unwrap();
        create_test_schema(dir.path());
        let meta = parse_schema_toml(dir.path()).unwrap();
        assert_eq!(meta.schema.name, "test");
        assert_eq!(meta.schema.version, "1.0.0");
        assert_eq!(meta.facts.len(), 1);
        assert_eq!(meta.facts[0].event_type, "test.item");
    }

    #[test]
    fn validate_rejects_missing_wit() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("schema.toml"),
            r#"
[schema]
name = "no-wit"
version = "1.0.0"
package = "patina:schema/no-wit@1.0.0"

[[facts]]
name = "x"
event_type = "no-wit.x"
record = "x"
"#,
        )
        .unwrap();
        let err = validate_package(dir.path()).unwrap_err();
        assert!(err.to_string().contains(".wit"), "got: {}", err);
    }

    #[test]
    fn validate_rejects_missing_toml() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("test.wit"), "package test;").unwrap();
        let err = validate_package(dir.path()).unwrap_err();
        assert!(err.to_string().contains("schema.toml"), "got: {}", err);
    }

    #[test]
    fn validate_rejects_empty_facts() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("schema.toml"),
            r#"
[schema]
name = "empty"
version = "1.0.0"
package = "patina:schema/empty@1.0.0"
"#,
        )
        .unwrap();
        fs::write(dir.path().join("empty.wit"), "package test;").unwrap();
        let err = validate_package(dir.path()).unwrap_err();
        assert!(err.to_string().contains("facts"), "got: {}", err);
    }

    #[test]
    fn validate_accepts_good_package() {
        let dir = TempDir::new().unwrap();
        create_test_schema(dir.path());
        let meta = validate_package(dir.path()).unwrap();
        assert_eq!(meta.schema.name, "test");
    }

    #[test]
    fn parse_forge_schema_toml() {
        let forge_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("wit/schema/forge");
        if !forge_dir.exists() {
            return; // skip if not in repo root
        }
        let meta = parse_schema_toml(&forge_dir).unwrap();
        assert_eq!(meta.schema.name, "forge");
        assert_eq!(meta.schema.package, "patina:schema/forge@1.0.0");
        assert_eq!(meta.facts.len(), 2);
        assert_eq!(meta.facts[0].event_type, "forge.issue");
        assert_eq!(meta.facts[1].event_type, "forge.pr");
        assert!(meta.embedding.is_some());
        assert_eq!(meta.embedding.unwrap().offset_slot, 5);
        assert_eq!(meta.indexes.len(), 2);
    }
}
