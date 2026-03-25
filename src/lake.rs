use crate::paths;
use anyhow::Result;
use patina_core::lake::{Clock, LakeConfigDocument, LakeRepository};
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LakeCommand {
    Create { name: String },
    List,
}

pub fn execute_value(command: LakeCommand) -> Result<serde_json::Value> {
    let repository = FsLakeRepository::new(lakes_dir());
    let clock = UtcClock;

    match command {
        LakeCommand::Create { name } => {
            let created = patina_core::lake::create_lake(&repository, &clock, &name)
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
            eprintln!("Lake \"{}\" created at {}", created.name, created.location);
            Ok(serde_json::json!({
                "child": "lake-manager",
                "text": format!("Lake '{}' created", name),
                "data": {"name": name}
            }))
        }
        LakeCommand::List => {
            let lakes = patina_core::lake::list_lakes(&repository)
                .map_err(|err| anyhow::anyhow!(err.to_string()))?
                .into_iter()
                .map(|lake| {
                    serde_json::json!({
                        "name": lake.name,
                        "created_at": lake.created_at,
                        "path": lake.location,
                    })
                })
                .collect::<Vec<_>>();
            Ok(serde_json::json!({
                "child": "lake-manager",
                "data": {"lakes": lakes}
            }))
        }
    }
}

pub fn validate_name(name: &str) -> Result<()> {
    patina_core::lake::validate_lake_name(name).map_err(|err| anyhow::anyhow!(err.to_string()))
}

fn lakes_dir() -> PathBuf {
    paths::lakes::lakes_dir()
}

struct FsLakeRepository {
    root: PathBuf,
}

impl FsLakeRepository {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl LakeRepository for FsLakeRepository {
    fn lake_location(&self, name: &str) -> String {
        self.root.join(name).display().to_string()
    }

    fn lake_exists(&self, name: &str) -> Result<bool, String> {
        Ok(self.root.join(name).exists())
    }

    fn create_lake_dir(&self, name: &str) -> Result<(), String> {
        std::fs::create_dir_all(self.root.join(name)).map_err(|e| e.to_string())
    }

    fn write_lake_config(&self, name: &str, content: &str) -> Result<(), String> {
        std::fs::write(self.root.join(name).join("lake.toml"), content).map_err(|e| e.to_string())
    }

    fn list_lake_configs(&self) -> Result<Vec<LakeConfigDocument>, String> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }

        let mut documents = Vec::new();
        let entries = std::fs::read_dir(&self.root).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            let lake_dir = entry.path();
            let lake_toml = lake_dir.join("lake.toml");
            if !lake_toml.exists() {
                continue;
            }

            let content = std::fs::read_to_string(&lake_toml).unwrap_or_default();
            documents.push(LakeConfigDocument {
                location: lake_dir.display().to_string(),
                content,
            });
        }

        Ok(documents)
    }
}

struct UtcClock;

impl Clock for UtcClock {
    fn now_rfc3339(&self) -> String {
        chrono::Utc::now().to_rfc3339()
    }
}
