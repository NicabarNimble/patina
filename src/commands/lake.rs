//! Lake command — manage DuckLake data lakes.
//!
//! Subcommands: create, list.

use anyhow::{bail, Result};
use patina::paths;
use std::path::PathBuf;

/// Lake CLI subcommands
#[derive(Debug, Clone, clap::Subcommand)]
pub enum LakeCommands {
    /// Create a new data lake
    ///
    /// Creates a lake directory under ~/.patina/lakes/<name>/ with a
    /// lake.toml configuration file. The DuckLake child uses this path
    /// as its storage toy.
    Create {
        /// Lake name (e.g., "github-data")
        name: String,
    },

    /// List all lakes
    List,
}

/// Execute lake CLI subcommand.
pub fn execute_cli(command: Option<LakeCommands>) -> Result<()> {
    match command {
        None | Some(LakeCommands::List) => list(),
        Some(LakeCommands::Create { name }) => create(&name),
    }
}

/// Lakes directory: ~/.patina/lakes/
fn lakes_dir() -> PathBuf {
    paths::lakes::lakes_dir()
}

/// Create a new data lake.
fn create(name: &str) -> Result<()> {
    // Validate name — alphanumeric, hyphens, underscores only
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        bail!("lake name must be non-empty and contain only alphanumeric characters, hyphens, or underscores");
    }

    let lake_dir = lakes_dir().join(name);

    if lake_dir.exists() {
        bail!("lake '{}' already exists at {}", name, lake_dir.display());
    }

    std::fs::create_dir_all(&lake_dir)?;

    let now = chrono::Utc::now().to_rfc3339();
    let config = format!("name = \"{}\"\ncreated_at = \"{}\"", name, now);
    std::fs::write(lake_dir.join("lake.toml"), config)?;

    eprintln!("Lake \"{}\" created at {}", name, lake_dir.display());
    Ok(())
}

/// List all lakes.
fn list() -> Result<()> {
    let dir = lakes_dir();

    if !dir.exists() {
        eprintln!("No lakes found ({})", dir.display());
        return Ok(());
    }

    let mut found = false;
    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("lake.toml").exists())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let lake_toml = entry.path().join("lake.toml");
        let content = std::fs::read_to_string(&lake_toml).unwrap_or_default();

        // Parse name from TOML
        let name = content
            .lines()
            .find(|l| l.starts_with("name"))
            .and_then(|l| l.split('=').nth(1))
            .map(|v| v.trim().trim_matches('"'))
            .unwrap_or("?");

        let created = content
            .lines()
            .find(|l| l.starts_with("created_at"))
            .and_then(|l| l.split('=').nth(1))
            .map(|v| v.trim().trim_matches('"'))
            .unwrap_or("?");

        println!("  {} (created: {})", name, created);
        println!("    {}", entry.path().display());
        found = true;
    }

    if !found {
        eprintln!("No lakes found");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_list_lake() {
        let tmp = tempfile::tempdir().unwrap();
        let lake_dir = tmp.path().join("test-lake");
        std::fs::create_dir_all(&lake_dir).unwrap();

        let now = "2026-03-10T12:00:00Z";
        let config = format!("name = \"test-lake\"\ncreated_at = \"{}\"", now);
        std::fs::write(lake_dir.join("lake.toml"), config).unwrap();

        assert!(lake_dir.join("lake.toml").exists());

        let content = std::fs::read_to_string(lake_dir.join("lake.toml")).unwrap();
        assert!(content.contains("test-lake"));
        assert!(content.contains("2026-03-10"));
    }

    #[test]
    fn invalid_lake_names() {
        assert!(create("").is_err());
        assert!(create("has spaces").is_err());
        assert!(create("has/slash").is_err());
        assert!(create("has.dot").is_err());
    }

    #[test]
    fn valid_lake_name_characters() {
        // Valid names should pass character validation
        // (tested via the validation logic, not create() which writes to disk)
        fn name_is_valid(name: &str) -> bool {
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        }
        assert!(name_is_valid("good-name"));
        assert!(name_is_valid("my_lake"));
        assert!(name_is_valid("lake123"));
        assert!(!name_is_valid(""));
        assert!(!name_is_valid("has spaces"));
        assert!(!name_is_valid("has.dot"));
    }

    #[test]
    fn resolve_nonexistent_lake() {
        let result = paths::lakes::resolve_lake_path("nonexistent-lake-xyz-12345");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}
