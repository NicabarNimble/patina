mod internal;

use anyhow::Result;

#[derive(Debug, Clone, clap::Subcommand)]
pub enum InterfaceCommands {
    /// Create or refresh Patina-managed projection for a project-local interface
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

pub fn setup(name: &str, path: Option<String>, force: bool) -> Result<()> {
    internal::setup(name, path, force)
}

pub fn ensure_ready(
    name: &str,
    project_path: &std::path::Path,
    force: bool,
) -> Result<(
    Box<dyn patina::interface::AiAdapter>,
    patina::interface::BootstrapResult,
)> {
    internal::ensure_interface_ready(name, project_path, force)
}
