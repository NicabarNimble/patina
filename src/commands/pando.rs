use anyhow::Result;
use std::path::Path;

const FOLDER_TEXT_TO_PARQUET_PANDO_NAME: &str = "folder-text-to-parquet";
const FOLDER_TEXT_TO_PARQUET_MANIFEST: &str =
    include_str!("../../resources/pandos/folder-text-to-parquet/pando.toml");

#[derive(Debug, Clone, clap::Subcommand)]
pub enum PandoCommands {
    /// List registered pandos
    List {
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
}

pub fn execute_cli(command: Option<PandoCommands>) -> Result<()> {
    let effective = command.unwrap_or(PandoCommands::List { json: false });
    match effective {
        PandoCommands::List { json } => {
            let state = patina::mother::control_plane_client()
                .pando_list()
                .map_err(|e| anyhow::anyhow!("pando registry unavailable via Mother: {}", e))?;

            if json {
                println!("{}", serde_json::to_string_pretty(&state)?);
                return Ok(());
            }

            if state.pandos.is_empty() {
                println!("No pandos registered.");
                return Ok(());
            }

            let has_commands = state.pandos.iter().any(|pando| !pando.commands.is_empty());
            println!("Pandos:");
            for pando in state.pandos {
                if has_commands {
                    let commands = if pando.commands.is_empty() {
                        "-".to_string()
                    } else {
                        pando.commands.join(",")
                    };
                    println!(
                        "  {:<24} status: {:<8} children: {:<3} commands: {}",
                        pando.name,
                        format!("{:?}", pando.status).to_ascii_lowercase(),
                        pando.child_count,
                        commands
                    );
                } else {
                    println!(
                        "  {:<24} status: {:<8} children: {:<3}",
                        pando.name,
                        format!("{:?}", pando.status).to_ascii_lowercase(),
                        pando.child_count
                    );
                }
            }
        }
    }
    Ok(())
}

fn seed_first_party_pandos(pandos_root: &Path) -> Result<()> {
    std::fs::create_dir_all(pandos_root)?;

    let folder_text_to_parquet = pandos_root.join(FOLDER_TEXT_TO_PARQUET_PANDO_NAME);
    std::fs::create_dir_all(&folder_text_to_parquet)?;
    std::fs::write(
        folder_text_to_parquet.join("pando.toml"),
        FOLDER_TEXT_TO_PARQUET_MANIFEST,
    )?;

    Ok(())
}

pub fn native_command_names() -> Vec<String> {
    let names = vec![
        "init",
        "doctor",
        "child",
        "version",
        "scrape",
        "oxidize",
        "rebuild",
        "scry",
        "context",
        "eval",
        "bench",
        "persona",
        "repo",
        "model",
        "connect",
        "lake",
        "mother",
        "pando",
        "secrets",
        "yolo",
        "serve",
        "interface",
        "report",
        "measure",
        "ai",
        "hook",
        "belief",
        "setup",
        "spec",
        "schema",
        "events",
        "assay",
    ];

    #[cfg(feature = "dev")]
    let names = {
        let mut names = names;
        names.push("upgrade");
        names.push("dev");
        names
    };

    names.into_iter().map(String::from).collect()
}

pub fn init_registry_best_effort() {
    let _ = seed_first_party_pandos(&patina::paths::pando::pandos_dir());

    let request = patina_protocol::PandoRegistryInit {
        protocol_version: patina_protocol::PANDO_REGISTRY_PROTOCOL_VERSION,
        native_commands: native_command_names(),
        binary_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let _ = patina::mother::control_plane_client().pando_registry_init(&request);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_first_party_pando_manifests() {
        let temp = tempfile::tempdir().unwrap();
        seed_first_party_pandos(temp.path()).unwrap();

        let folder_manifest_path = temp
            .path()
            .join(FOLDER_TEXT_TO_PARQUET_PANDO_NAME)
            .join("pando.toml");
        let folder_manifest = std::fs::read_to_string(folder_manifest_path).unwrap();
        assert!(folder_manifest.contains("name = \"folder-text-to-parquet\""));
        assert!(folder_manifest.contains("[[children]]"));
        assert!(folder_manifest.contains("[composition]"));
    }

    #[test]
    fn seeding_overwrites_manifest_without_touching_state_or_data() {
        let temp = tempfile::tempdir().unwrap();
        let pando_dir = temp.path().join(FOLDER_TEXT_TO_PARQUET_PANDO_NAME);
        std::fs::create_dir_all(pando_dir.join("state")).unwrap();
        std::fs::create_dir_all(pando_dir.join("data")).unwrap();
        std::fs::write(pando_dir.join("state/checkpoint.json"), "{\"offset\":7}").unwrap();
        std::fs::write(pando_dir.join("pando.toml"), "broken").unwrap();

        seed_first_party_pandos(temp.path()).unwrap();

        let checkpoint = std::fs::read_to_string(pando_dir.join("state/checkpoint.json")).unwrap();
        assert_eq!(checkpoint, "{\"offset\":7}");
        assert!(pando_dir.join("data").exists());

        let manifest = std::fs::read_to_string(pando_dir.join("pando.toml")).unwrap();
        assert!(manifest.contains("name = \"folder-text-to-parquet\""));
        assert!(!manifest.contains("broken"));
    }

    #[test]
    fn native_command_names_include_active_native_surfaces() {
        let names = native_command_names();
        assert!(names.iter().any(|name| name == "mother"));
        assert!(names.iter().any(|name| name == "spec"));
        assert!(names.iter().any(|name| name == "measure"));
        assert!(!names.iter().any(|name| name == "atlas"));
    }
}
