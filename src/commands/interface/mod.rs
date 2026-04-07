mod internal;
pub(crate) mod manage;

pub use manage::{execute, InterfaceManageCommands};

use anyhow::Result;

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
