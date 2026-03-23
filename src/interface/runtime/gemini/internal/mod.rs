//! Internal implementation for Gemini adapter

use anyhow::Result;
use std::path::{Path, PathBuf};

use super::super::templates;
use crate::environment::Environment;

/// Initialize Gemini project structure.
///
/// Native Patina projection now uses root `AGENTS.md` as the canonical
/// instruction surface. `.gemini/` only carries adapter-local command assets.
pub fn init_project(
    project_path: &Path,
    _project_name: &str,
    _environment: &Environment,
) -> Result<()> {
    templates::copy_to_project("gemini", project_path)
}

/// Get canonical context file path.
pub fn get_context_file_path(project_path: &Path) -> PathBuf {
    project_path.join("AGENTS.md")
}

/// Legacy helper retained for compatibility with older call sites.
pub fn ensure_context_file(
    project_path: &Path,
    _project_name: &str,
    _environment: &Environment,
) -> Result<PathBuf> {
    Ok(get_context_file_path(project_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn context_path_is_root_agents() {
        let temp = TempDir::new().unwrap();
        assert_eq!(
            get_context_file_path(temp.path()),
            temp.path().join("AGENTS.md")
        );
    }
}
