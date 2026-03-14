mod internal;

use anyhow::Result;

#[derive(Debug, Clone, clap::Subcommand)]
pub enum InterfaceCommands {
    /// Compatibility shim for refreshing the Patina AI surface
    Setup {
        name: String,

        #[arg(long)]
        path: Option<String>,

        #[arg(long)]
        force: bool,
    },
}

pub fn execute(command: InterfaceCommands) -> Result<()> {
    match command {
        InterfaceCommands::Setup { name, path, force } => internal::setup(&name, path, force),
    }
}

pub fn ensure_ready(
    name: &str,
    project_path: &std::path::Path,
    force: bool,
) -> Result<(
    Box<dyn patina::interface::AiInterface>,
    patina::interface::BootstrapResult,
)> {
    internal::ensure_interface_ready(name, project_path, force)
}
