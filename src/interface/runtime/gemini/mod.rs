//! Gemini interface for Patina (stub implementation)
//!
//! Placeholder for future Gemini AI integration.
//! Creates basic `.gemini/` structure with context file.

use crate::environment::Environment;
use anyhow::Result;
use std::path::{Path, PathBuf};

mod internal;
use super::InterfaceProvider;

// Export version for version management
pub const GEMINI_INTERFACE_VERSION: &str = "0.1.0";

/// Gemini interface implementation (stub)
pub struct GeminiInterface;

impl GeminiInterface {
    /// Create a new Gemini interface
    pub fn new() -> Self {
        Self
    }

    pub fn ensure_context(
        project_path: &Path,
        project_name: &str,
        environment: &Environment,
    ) -> Result<PathBuf> {
        internal::ensure_context_file(project_path, project_name, environment)
    }
}

impl Default for GeminiInterface {
    fn default() -> Self {
        Self::new()
    }
}

impl InterfaceProvider for GeminiInterface {
    fn name(&self) -> &'static str {
        "gemini"
    }

    fn init_project(
        &self,
        project_path: &Path,
        project_name: &str,
        environment: &Environment,
    ) -> Result<()> {
        internal::init_project(project_path, project_name, environment)
    }

    fn post_init(&self, _project_path: &Path) -> Result<()> {
        Ok(())
    }

    fn get_custom_commands(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "/session-start [name]",
                "Start session with Git branch creation",
            ),
            ("/session-update", "Update session with Git awareness"),
            ("/session-note [insight]", "Add insight with Git context"),
            ("/session-end", "End session with Git classification"),
            ("/patina-review", "Review recent sessions and git history"),
        ]
    }

    fn get_context_file_path(&self, project_path: &Path) -> PathBuf {
        internal::get_context_file_path(project_path)
    }

    fn check_for_updates(&self, _project_path: &Path) -> Result<Option<(String, String)>> {
        // No version tracking for stub
        Ok(None)
    }

    fn update_interface_files(&self, _project_path: &Path) -> Result<()> {
        // Nothing to update in stub
        Ok(())
    }

    fn get_sessions_path(&self, _project_path: &Path) -> Option<PathBuf> {
        // No session tracking yet
        None
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn get_version_changes(&self, _version: &str) -> Option<Vec<String>> {
        None
    }

    fn get_changelog_since(&self, _from_version: &str) -> Vec<String> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_name() {
        let iface = GeminiInterface::new();
        assert_eq!(iface.name(), "gemini");
    }

    #[test]
    fn test_custom_commands() {
        let iface = GeminiInterface::new();
        let commands = iface.get_custom_commands();
        assert_eq!(commands.len(), 5);
        assert!(commands.iter().any(|(cmd, _)| cmd.starts_with("/session-")));
        assert!(commands
            .iter()
            .any(|(cmd, _)| cmd.starts_with("/patina-review")));
    }
}
