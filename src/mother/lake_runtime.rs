use crate::paths;
use anyhow::{bail, Result};
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LakeCommand {
    Create { name: String },
    List,
}

pub fn execute_value(command: LakeCommand) -> Result<serde_json::Value> {
    match command {
        LakeCommand::Create { name } => {
            create(&name)?;
            Ok(serde_json::json!({
                "child": "lake-manager",
                "text": format!("Lake '{}' created", name),
                "data": {"name": name}
            }))
        }
        LakeCommand::List => {
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
                    let metadata = patina_core::lake::parse_lake_metadata(&content);
                    lakes.push(serde_json::json!({
                        "name": metadata.name,
                        "created_at": metadata.created_at,
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

fn lakes_dir() -> PathBuf {
    paths::lakes::lakes_dir()
}

fn create(name: &str) -> Result<()> {
    validate_lake_name(name)?;

    let lake_dir = lakes_dir().join(name);

    if lake_dir.exists() {
        bail!("lake '{}' already exists at {}", name, lake_dir.display());
    }

    std::fs::create_dir_all(&lake_dir)?;

    let now = chrono::Utc::now().to_rfc3339();
    let config = patina_core::lake::render_lake_config(name, &now);
    std::fs::write(lake_dir.join("lake.toml"), config)?;

    eprintln!("Lake \"{}\" created at {}", name, lake_dir.display());
    Ok(())
}

pub fn validate_lake_name(name: &str) -> Result<()> {
    patina_core::lake::validate_lake_name(name).map_err(|err| anyhow::anyhow!(err.to_string()))
}
