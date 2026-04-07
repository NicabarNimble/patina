use anyhow::Result;

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

            println!("Pandos:");
            for pando in state.pandos {
                let commands = if pando.commands.is_empty() {
                    "-".to_string()
                } else {
                    pando.commands.join(",")
                };
                println!(
                    "  {:<20} status: {:<8} children: {:<3} commands: {}",
                    pando.name,
                    format!("{:?}", pando.status).to_ascii_lowercase(),
                    pando.child_count,
                    commands
                );
            }
        }
    }
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
    {
        names.push("upgrade");
        names.push("dev");
    }

    names.into_iter().map(String::from).collect()
}

pub fn init_registry_best_effort() {
    let request = patina_protocol::PandoRegistryInit {
        protocol_version: patina_protocol::PANDO_REGISTRY_PROTOCOL_VERSION,
        native_commands: native_command_names(),
        binary_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let _ = patina::mother::control_plane_client().pando_registry_init(&request);
}
