//! Lake command — manage DuckLake data lakes.
//!
//! Subcommands: create, list.

use anyhow::{bail, Result};
use patina::paths;
use std::path::PathBuf;

/// Lake CLI subcommands
#[derive(Debug, Clone, clap::Subcommand, serde::Serialize, serde::Deserialize)]
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
    let effective = command.unwrap_or(LakeCommands::List);
    let payload = serde_json::json!({ "command": effective });
    let client = patina::mother::control_plane_client();
    let response = client
        .child_action("lake-manager", "dispatch", &payload)
        .map_err(|e| {
            anyhow::anyhow!(
                "lake-manager unavailable via Mother (start with `patina mother start`): {}",
                e
            )
        })?;

    if let Some(text) = response.get("text").and_then(|v| v.as_str()) {
        if !text.is_empty() {
            println!("{}", text);
        }
    }
    if let Some(data) = response.get("data") {
        if response.get("text").is_none() {
            println!("{}", serde_json::to_string_pretty(data)?);
        }
    }
    Ok(())
}

pub(crate) fn execute_value(command: LakeCommands) -> Result<serde_json::Value> {
    match command {
        LakeCommands::Create { name } => {
            create(&name)?;
            Ok(serde_json::json!({
                "child": "lake-manager",
                "text": format!("Lake '{}' created", name),
                "data": {"name": name}
            }))
        }
        LakeCommands::List => {
            let dir = lakes_dir();
            let mut lakes = Vec::new();
            if dir.exists() {
                let mut entries: Vec<_> = std::fs::read_dir(&dir)?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().join("lake.toml").exists())
                    .collect();
                entries.sort_by_key(|e| e.file_name());
                for entry in entries {
                    let lake_toml = entry.path().join("lake.toml");
                    let content = std::fs::read_to_string(&lake_toml).unwrap_or_default();
                    let name = content
                        .lines()
                        .find(|l| l.starts_with("name"))
                        .and_then(|l| l.split('=').nth(1))
                        .map(|v| v.trim().trim_matches('"'))
                        .unwrap_or("?")
                        .to_string();
                    let created = content
                        .lines()
                        .find(|l| l.starts_with("created_at"))
                        .and_then(|l| l.split('=').nth(1))
                        .map(|v| v.trim().trim_matches('"'))
                        .unwrap_or("?")
                        .to_string();
                    lakes.push(serde_json::json!({
                        "name": name,
                        "created_at": created,
                        "path": entry.path().display().to_string()
                    }));
                }
            }
            Ok(serde_json::json!({
                "child": "lake-manager",
                "data": {"lakes": lakes}
            }))
        }
    }
}

/// Lakes directory: ~/.patina/lakes/
fn lakes_dir() -> PathBuf {
    paths::lakes::lakes_dir()
}

/// Create a new data lake.
fn create(name: &str) -> Result<()> {
    validate_lake_name(name)?;

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

fn validate_lake_name(name: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        bail!("lake name must be non-empty and contain only alphanumeric characters, hyphens, or underscores");
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
        assert!(validate_lake_name("good-name").is_ok());
        assert!(validate_lake_name("my_lake").is_ok());
        assert!(validate_lake_name("lake123").is_ok());
        assert!(validate_lake_name("").is_err());
        assert!(validate_lake_name("has spaces").is_err());
        assert!(validate_lake_name("has.dot").is_err());
    }

    #[test]
    fn resolve_nonexistent_lake() {
        let result = paths::lakes::resolve_lake_path("nonexistent-lake-xyz-12345");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}
