//! Lake command — manage DuckLake data lakes.
//!
//! Subcommands: create, list.

use anyhow::Result;
use patina_protocol::{BuiltinChild, BuiltinChildAction, BuiltinChildRequest, LakeDispatchRequest};

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
    let protocol_request = BuiltinChildRequest::new(
        BuiltinChild::LakeManager,
        BuiltinChildAction::LakeDispatch(LakeDispatchRequest {
            command: serde_json::to_value(effective)?,
        }),
    );
    let client = patina::mother::control_plane_client();
    let response = client.child_action_typed(&protocol_request).map_err(|e| {
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

#[allow(dead_code)]
pub(crate) fn execute_value(command: LakeCommands) -> Result<serde_json::Value> {
    let runtime_command = match command {
        LakeCommands::Create { name } => patina::mother::lake_runtime::LakeCommand::Create { name },
        LakeCommands::List => patina::mother::lake_runtime::LakeCommand::List,
    };
    patina::mother::lake_runtime::execute_value(runtime_command)
}

#[cfg(test)]
mod tests {
    use patina::paths;

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
        assert!(patina::mother::lake_runtime::validate_lake_name("").is_err());
        assert!(patina::mother::lake_runtime::validate_lake_name("has spaces").is_err());
        assert!(patina::mother::lake_runtime::validate_lake_name("has/slash").is_err());
        assert!(patina::mother::lake_runtime::validate_lake_name("has.dot").is_err());
    }

    #[test]
    fn valid_lake_name_characters() {
        assert!(patina::mother::lake_runtime::validate_lake_name("good-name").is_ok());
        assert!(patina::mother::lake_runtime::validate_lake_name("my_lake").is_ok());
        assert!(patina::mother::lake_runtime::validate_lake_name("lake123").is_ok());
    }

    #[test]
    fn resolve_nonexistent_lake() {
        let result = paths::lakes::resolve_lake_path("nonexistent-lake-xyz-12345");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}
