//! OpenCode adapter for Patina
//!
//! OpenCode is a multi-provider AI CLI that supports Claude subscriptions.
//! Creates `.opencode/` structure with markdown commands (same format as Claude).

use crate::adapters::LLMAdapter;
use crate::environment::Environment;
use anyhow::Result;
use std::path::{Path, PathBuf};

mod internal;

// Export version for version management
pub const OPENCODE_ADAPTER_VERSION: &str = "0.1.0";

/// OpenCode adapter implementation
pub struct OpenCodeAdapter;

impl OpenCodeAdapter {
    /// Create a new OpenCode adapter
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenCodeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl LLMAdapter for OpenCodeAdapter {
    fn name(&self) -> &'static str {
        "opencode"
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
        // No version tracking yet
        Ok(None)
    }

    fn update_adapter_files(&self, _project_path: &Path) -> Result<()> {
        // Nothing to update yet
        Ok(())
    }

    fn get_sessions_path(&self, _project_path: &Path) -> Option<PathBuf> {
        // No session tracking yet (uses .opencode/context/sessions/)
        None
    }

    fn version(&self) -> &'static str {
        OPENCODE_ADAPTER_VERSION
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
    fn test_adapter_name() {
        let adapter = OpenCodeAdapter::new();
        assert_eq!(adapter.name(), "opencode");
    }

    #[test]
    fn test_custom_commands() {
        let adapter = OpenCodeAdapter::new();
        let commands = adapter.get_custom_commands();
        assert_eq!(commands.len(), 5);
        assert!(commands.iter().any(|(cmd, _)| cmd.starts_with("/session-")));
        assert!(commands
            .iter()
            .any(|(cmd, _)| cmd.starts_with("/patina-review")));
    }
}
